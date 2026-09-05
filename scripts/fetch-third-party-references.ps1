param(
  [string]$Destination = (Join-Path (Split-Path -Parent $PSScriptRoot) 'references')
)

$ErrorActionPreference = 'Stop'
$repositories = @(
  @{ Name = 'aps-notecast'; Url = 'https://github.com/Alexs-Piano-Service/aps-notecast.git' },
  @{ Name = 'MioVRC_Translator'; Url = 'https://github.com/CokoIya/MioVRC_Translator.git' },
  @{ Name = 'VRCLS'; Url = 'https://github.com/VoiceLinkVR/VRCLS.git' },
  @{ Name = 'VRC-Draw'; Url = 'https://github.com/FlyPig01/VRC-Draw.git' },
  @{ Name = 'openvr'; Url = 'https://github.com/ValveSoftware/openvr.git' },
  @{ Name = 'OpenVR-AdvancedSettings'; Url = 'https://github.com/OpenVR-Advanced-Settings/OpenVR-AdvancedSettings.git' }
)

New-Item -ItemType Directory -Force -Path $Destination | Out-Null
foreach ($repository in $repositories) {
  $target = Join-Path $Destination $repository.Name
  if (Test-Path (Join-Path $target '.git')) {
    git -C $target fetch --depth 1 origin
    git -C $target reset --hard origin/HEAD
  } else {
    git clone --depth 1 $repository.Url $target
  }
  Write-Output "$($repository.Name): $(git -C $target rev-parse --short HEAD)"
}
