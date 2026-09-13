$ErrorActionPreference = 'Stop'
$version = if ($env:KURIR_VERSION) { $env:KURIR_VERSION } else { '0.1.0' }
$installDir = if ($env:KURIR_INSTALL_DIR) { $env:KURIR_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\Kurir\bin' }
$arch = if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq 'Arm64') { 'aarch64' } else { 'x86_64' }
$base = "https://github.com/suiflex/kurir/releases/download/v$version"
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("kurir-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  $archive = Join-Path $tmp 'kurir.zip'
  Invoke-WebRequest "$base/kurir-$version-windows-$arch.zip" -OutFile $archive
  Expand-Archive -Path $archive -DestinationPath $tmp -Force
  New-Item -ItemType Directory -Path $installDir -Force | Out-Null
  Copy-Item (Join-Path $tmp 'kurir.exe') (Join-Path $installDir 'kurir.exe') -Force
  Write-Output "Installed kurir $version to $installDir\kurir.exe"
} finally {
  Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
