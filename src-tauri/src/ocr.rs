use std::process::Stdio;
use crate::translate::{translate, GlossaryTerm, TranslateRequest};
use serde::{Deserialize, Serialize};

pub struct OcrEngine;

#[derive(Debug, Clone, Deserialize)]
pub struct PhotoTranslateRequest {
    pub image_path: String,
    #[serde(default = "default_photo_source_lang")]
    pub source_lang: String,
    #[serde(default = "default_photo_target_lang")]
    pub target_lang: String,
    #[serde(default = "default_photo_service")]
    pub service: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub custom_api_url: String,
    #[serde(default)]
    pub glossary: Vec<GlossaryTerm>,
    #[serde(default = "default_photo_retry_count")]
    pub retry_count: u8,
    #[serde(default = "default_photo_ocr_lang")]
    pub ocr_lang: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PhotoTranslateResult {
    pub image_path: String,
    pub original: String,
    pub translated: String,
    pub service: String,
}

fn default_photo_source_lang() -> String { "auto".into() }
fn default_photo_target_lang() -> String { "zh-CN".into() }
fn default_photo_service() -> String { "google_free".into() }
fn default_photo_retry_count() -> u8 { 2 }
fn default_photo_ocr_lang() -> String { "auto".into() }

#[tauri::command]
pub async fn vrct_translate_image(request: PhotoTranslateRequest) -> crate::AppResult<PhotoTranslateResult> {
    let path = std::path::Path::new(request.image_path.trim());
    if !path.is_file() {
        return Err("图片文件不存在".into());
    }
    let original = extract_text_from_image(path, &request.ocr_lang).await.map_err(crate::AppError::from)?;
    let result = translate(&TranslateRequest {
        text: original.clone(),
        source_lang: request.source_lang,
        target_lang: request.target_lang,
        service: request.service,
        api_key: request.api_key,
        model: request.model,
        prompt: request.prompt,
        custom_api_url: request.custom_api_url,
        glossary: request.glossary,
        context: Vec::new(),
        retry_count: request.retry_count,
    }).await.map_err(crate::AppError::from)?;
    Ok(PhotoTranslateResult {
        image_path: request.image_path,
        original,
        translated: result.translated,
        service: result.service,
    })
}

/// OCR a user-selected image without persisting a processed copy.
pub async fn extract_text_from_image(path: &std::path::Path, ocr_lang: &str) -> Result<String, String> {
    let image_path = path.to_string_lossy().replace('\'', "''");
    let lang = ocr_lang.replace('\'', "''");
    let script = format!(r#"
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$null = [Windows.Media.Ocr.OcrEngine,Windows.Foundation.UniversalApiContract,ContentType=WindowsRuntime]
$null = [Windows.Graphics.Imaging.BitmapDecoder,Windows.Foundation.UniversalApiContract,ContentType=WindowsRuntime]
$null = [Windows.Storage.StorageFile,Windows.Foundation.UniversalApiContract,ContentType=WindowsRuntime]
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {{ $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' }})[0]
Function Await($operation, $resultType) {{ $task = $asTaskGeneric.MakeGenericMethod($resultType).Invoke($null, @($operation)); $task.Wait(-1) | Out-Null; $task.Result }}
$file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync('{image_path}')) ([Windows.Storage.StorageFile])
$stream = Await ($file.OpenAsync([Windows.Storage.FileAccessMode]::Read)) ([Windows.Storage.Streams.IRandomAccessStream])
$decoder = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
$bitmap = Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
$engine = $null
if ('{lang}' -ne 'auto') {{ try {{ $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage((New-Object Windows.Globalization.Language '{lang}')) }} catch {{ $engine = $null }} }}
if (-not $engine) {{ $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages() }}
if ($engine) {{ (Await ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])).Text }} else {{ "OCR_ENGINE_NOT_AVAILABLE" }}
"#);
    let output = tokio::task::spawn_blocking(move || std::process::Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &script]).output())
        .await.map_err(|error| format!("OCR task failed: {error}"))?.map_err(|error| format!("OCR process failed: {error}"))?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() || text == "OCR_ENGINE_NOT_AVAILABLE" { Err("OCR 未识别到文字或当前语言包不可用".into()) } else { Ok(text) }
}

impl OcrEngine {
    /// Capture the primary desktop surface to a PNG. SteamVR mirrors its
    /// compositor to a desktop window, so this is the VR capture fallback.
    pub async fn capture_primary_screen_to_file(path: &std::path::Path) -> Result<(), String> {
        let destination = path.to_string_lossy().replace('\'', "''");
        let ps_script = format!(
            r#"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$bmp = New-Object System.Drawing.Bitmap($screen.Width, $screen.Height)
$graphics = [System.Drawing.Graphics]::FromImage($bmp)
try {{
    $graphics.CopyFromScreen($screen.Location, [System.Drawing.Point]::Empty, $screen.Size)
    $bmp.Save('{destination}', [System.Drawing.Imaging.ImageFormat]::Png)
}} finally {{
    $graphics.Dispose()
    $bmp.Dispose()
}}
"#
        );

        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-ExecutionPolicy",
                    "Bypass",
                    "-Command",
                    &ps_script,
                ])
                .output()
        })
        .await
        .map_err(|error| format!("screen capture task failed: {error}"))?
        .map_err(|error| format!("failed to start screen capture: {error}"))?;

        if !output.status.success() {
            let details = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if details.is_empty() {
                "screen capture failed".to_string()
            } else {
                format!("screen capture failed: {details}")
            });
        }

        if !path.is_file() {
            return Err("screen capture did not create a PNG file".to_string());
        }
        Ok(())
    }

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
