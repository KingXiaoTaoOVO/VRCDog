$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$BinDir = Join-Path $ScriptDir "..\bin"
$FfmpegExe = Join-Path $BinDir "ffmpeg-x86_64-pc-windows-msvc.exe"
$WorkDir = Join-Path $ScriptDir ".ffmpeg-download"
$ZipPath = Join-Path $WorkDir "ffmpeg.zip"
$ExtractDir = Join-Path $WorkDir "ffmpeg_extracted"

Function Test-FfmpegSidecar {
    param([string]$Path)

    If (-Not (Test-Path $Path)) {
        return $false
    }

    $File = Get-Item -LiteralPath $Path
    If ($File.Length -lt 10485760) {
        Write-Host "Existing ffmpeg sidecar is too small ($($File.Length) bytes)."
        return $false
    }

    $Stream = [System.IO.File]::OpenRead($Path)
    Try {
        $Reader = New-Object System.IO.BinaryReader($Stream)

        $Mz = $Reader.ReadBytes(2)
        If ($Mz.Length -ne 2 -Or $Mz[0] -ne 0x4D -Or $Mz[1] -ne 0x5A) {
            Write-Host "Existing ffmpeg sidecar is not a Windows executable."
            return $false
        }

        $Stream.Seek(0x3C, [System.IO.SeekOrigin]::Begin) | Out-Null
        $PeOffset = $Reader.ReadUInt32()
        If ($PeOffset -le 0 -Or $PeOffset -gt ($Stream.Length - 6)) {
            Write-Host "Existing ffmpeg sidecar has an invalid PE header offset."
            return $false
        }

        $Stream.Seek($PeOffset, [System.IO.SeekOrigin]::Begin) | Out-Null
        $Pe = $Reader.ReadBytes(4)
        If ($Pe.Length -ne 4 -Or $Pe[0] -ne 0x50 -Or $Pe[1] -ne 0x45 -Or $Pe[2] -ne 0x00 -Or $Pe[3] -ne 0x00) {
            Write-Host "Existing ffmpeg sidecar has an invalid PE signature."
            return $false
        }

        $Machine = $Reader.ReadUInt16()
        If ($Machine -ne 0x8664) {
            Write-Host ("Existing ffmpeg sidecar is not x64. Machine=0x{0:X4}" -f $Machine)
            return $false
        }
    }
    Finally {
        $Stream.Dispose()
    }

    return $true
}

If (Test-FfmpegSidecar -Path $FfmpegExe) {
    Write-Host "ffmpeg sidecar already exists and is valid."
    exit 0
}

If (Test-Path $FfmpegExe) {
    Write-Host "Removing invalid ffmpeg sidecar."
    Remove-Item -LiteralPath $FfmpegExe -Force
}

If (-Not (Test-Path $BinDir)) {
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
}

If (Test-Path $WorkDir) {
    Remove-Item -LiteralPath $WorkDir -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $WorkDir | Out-Null

Write-Host "Downloading FFmpeg release (this may take a minute)..."
$Url = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip"
Invoke-WebRequest -Uri $Url -OutFile $ZipPath

Write-Host "Extracting FFmpeg..."
Expand-Archive -Path $ZipPath -DestinationPath $ExtractDir -Force

$ExtractedExe = Join-Path $ExtractDir "ffmpeg-master-latest-win64-gpl\bin\ffmpeg.exe"
If (-Not (Test-FfmpegSidecar -Path $ExtractedExe)) {
    throw "Downloaded FFmpeg executable is invalid."
}
Copy-Item -LiteralPath $ExtractedExe -Destination $FfmpegExe -Force

If (-Not (Test-FfmpegSidecar -Path $FfmpegExe)) {
    throw "Installed FFmpeg sidecar is invalid."
}

Write-Host "Cleaning up..."
Remove-Item -LiteralPath $WorkDir -Recurse -Force

Write-Host "FFmpeg sidecar setup complete!"
