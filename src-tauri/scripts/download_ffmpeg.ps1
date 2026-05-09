$ErrorActionPreference = "Stop"
$BinDir = "..\bin"
$FfmpegExe = "$BinDir\ffmpeg-x86_64-pc-windows-msvc.exe"

If (Test-Path $FfmpegExe) {
    Write-Host "ffmpeg sidecar already exists."
    exit 0
}

If (-Not (Test-Path $BinDir)) {
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
}

Write-Host "Downloading FFmpeg release (this may take a minute)..."
$Url = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip"
$ZipPath = "ffmpeg.zip"
Invoke-WebRequest -Uri $Url -OutFile $ZipPath

Write-Host "Extracting FFmpeg..."
Expand-Archive -Path $ZipPath -DestinationPath "ffmpeg_extracted" -Force

$ExtractedExe = "ffmpeg_extracted\ffmpeg-master-latest-win64-gpl\bin\ffmpeg.exe"
Copy-Item -Path $ExtractedExe -Destination $FfmpegExe -Force

Write-Host "Cleaning up..."
Remove-Item -Path $ZipPath -Force
Remove-Item -Path "ffmpeg_extracted" -Recurse -Force

Write-Host "FFmpeg sidecar setup complete!"
