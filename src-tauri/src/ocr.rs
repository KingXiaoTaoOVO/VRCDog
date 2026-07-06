use std::process::Stdio;

pub struct OcrEngine;

impl OcrEngine {
    pub async fn extract_text_from_screen(
        ocr_lang: &str,
        image_enhance: bool,
    ) -> Result<String, String> {
        let enhance_block = if image_enhance {
            r#"
# Image Enhancement: Sharpen + Contrast boost
$enhancedFile = "$env:TEMP\vrcdog_scan_enhanced.png"
$srcBmp = [System.Drawing.Bitmap]::new($tmpFile)
$g2 = [System.Drawing.Graphics]::FromImage($srcBmp)
$matrix = New-Object System.Drawing.Imaging.ColorMatrix
$contrast = 1.3
$t = (1.0 - $contrast) / 2.0
$matrix.Matrix00 = $contrast; $matrix.Matrix11 = $contrast; $matrix.Matrix22 = $contrast
$matrix.Matrix40 = $t; $matrix.Matrix41 = $t; $matrix.Matrix42 = $t
$attrs = New-Object System.Drawing.Imaging.ImageAttributes
$attrs.SetColorMatrix($matrix)
$rect = New-Object System.Drawing.Rectangle(0, 0, $srcBmp.Width, $srcBmp.Height)
$g2.DrawImage($srcBmp, $rect, 0, 0, $srcBmp.Width, $srcBmp.Height, [System.Drawing.GraphicsUnit]::Pixel, $attrs)
$g2.Dispose()
$srcBmp.Save($enhancedFile, [System.Drawing.Imaging.ImageFormat]::Png)
$srcBmp.Dispose()
$tmpFile = $enhancedFile
"#
        } else {
            ""
        };

        let ps_script = format!(
            r#"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap($screen.Width, $screen.Height)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($screen.Location, [System.Drawing.Point]::Empty, $screen.Size)
$g.Dispose()
$tmpFile = "$env:TEMP\vrcdog_scan.png"
$bmp.Save($tmpFile, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()

{enhance}

try {{
    $null = [Windows.Media.Ocr.OcrEngine,Windows.Foundation.UniversalApiContract,ContentType=WindowsRuntime]
    $null = [Windows.Graphics.Imaging.BitmapDecoder,Windows.Foundation.UniversalApiContract,ContentType=WindowsRuntime]
    $null = [Windows.Storage.StorageFile,Windows.Foundation.UniversalApiContract,ContentType=WindowsRuntime]

    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    $asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {{ $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' }})[0]
    Function Await($WinRtTask, $ResultType) {{
        $asTask = $asTaskGeneric.MakeGenericMethod($ResultType)
        $netTask = $asTask.Invoke($null, @($WinRtTask))
        $netTask.Wait(-1) | Out-Null
        $netTask.Result
    }}

    $file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync($tmpFile)) ([Windows.Storage.StorageFile])
    $stream = Await ($file.OpenAsync([Windows.Storage.FileAccessMode]::Read)) ([Windows.Storage.Streams.IRandomAccessStream])
    $decoder = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
    $bitmap = Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])

    $engine = $null
    $primaryLangs = @("{lang}")
    if ("{lang}" -ne "ja") {{ $primaryLangs += "ja" }}
    if ("{lang}" -ne "en-US") {{ $primaryLangs += "en-US" }}
    if ("{lang}" -ne "zh-Hans-CN") {{ $primaryLangs += "zh-Hans-CN" }}

    foreach ($langTag in $primaryLangs) {{
        try {{
            $l = New-Object Windows.Globalization.Language $langTag
            $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage($l)
            if ($engine) {{ break }}
        }} catch {{}}
    }}
    if (-not $engine) {{
        $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
    }}

    if ($engine) {{
        $result = Await ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
        $result.Text
    }} else {{
        "OCR_ENGINE_NOT_AVAILABLE"
    }}
}} catch {{
    "OCR_ERROR: $($_.Exception.Message)"
}}
"#,
            enhance = enhance_block,
            lang = ocr_lang
        );

        let output = tokio::task::spawn_blocking(move || {
            let child = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    &ps_script,
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()?;

            let child_id = child.id();
            let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
            let watchdog = std::thread::spawn(move || {
                match done_rx.recv_timeout(std::time::Duration::from_secs(30)) {
                    Ok(_) => {}
                    Err(_) => {
                        #[cfg(windows)]
                        let _ = std::process::Command::new("taskkill")
                            .args(["/F", "/PID", &child_id.to_string()])
                            .output();
                    }
                }
            });

            let output = child.wait_with_output()?;
            let _ = done_tx.send(());
            let _ = watchdog.join();

            Ok::<std::process::Output, std::io::Error>(output)
        })
        .await
        .map_err(|e| format!("spawn error: {}", e))?
        .map_err(|e| format!("PowerShell error: {}", e))?;

        let ocr_text = String::from_utf8_lossy(&output.stdout).trim().to_string();

        if ocr_text.is_empty()
            || ocr_text.starts_with("OCR_ERROR")
            || ocr_text == "OCR_ENGINE_NOT_AVAILABLE"
        {
            Err(format!(
                "OCR 失败: {}",
                if ocr_text.is_empty() {
                    "未识别到文字"
                } else {
                    &ocr_text
                }
            ))
        } else {
            Ok(ocr_text)
        }
    }
}
