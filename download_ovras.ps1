$outPath = "c:\Users\27457\Desktop\Project\UnityEXE\src-tauri\OVRAS\OVRAS_Installer.exe"
$url = "https://github.com/OpenVR-Advanced-Settings/OpenVR-AdvancedSettings/releases/download/v5.8.11/AdvancedSettings-5.8.11-Installer.exe"
Invoke-WebRequest -Uri $url -OutFile $outPath
Write-Host "Downloaded successfully to $outPath"
