# Install the latest (or a pinned) whylog release on Windows.
#   irm https://raw.githubusercontent.com/stuyk/whylog/main/install.ps1 | iex
# Env: WHYLOG_VERSION=0.1.0 (default: latest), WHYLOG_INSTALL_DIR (default: %LOCALAPPDATA%\whylog\bin)
$ErrorActionPreference = 'Stop'
$repo = 'stuyk/whylog'
$dir = if ($env:WHYLOG_INSTALL_DIR) { $env:WHYLOG_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'whylog\bin' }
$arch = if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -eq 'Arm64') { 'arm64' } else { 'amd64' }

if ($env:WHYLOG_VERSION) { $version = $env:WHYLOG_VERSION.TrimStart('v') }
else { $version = (Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest").tag_name.TrimStart('v') }

$archive = "whylog_${version}_windows_${arch}.zip"
$base = "https://github.com/$repo/releases/download/v$version"
$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ([System.Guid]::NewGuid())
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
  Write-Host "downloading whylog v$version (windows/$arch)..."
  Invoke-WebRequest "$base/$archive" -OutFile (Join-Path $tmp $archive)
  Invoke-WebRequest "$base/checksums.txt" -OutFile (Join-Path $tmp 'checksums.txt')
  $expected = (Get-Content (Join-Path $tmp 'checksums.txt') | Where-Object { $_ -match " $([regex]::Escape($archive))$" }) -split ' ' | Select-Object -First 1
  $actual = (Get-FileHash (Join-Path $tmp $archive) -Algorithm SHA256).Hash.ToLower()
  if (-not $expected -or $expected -ne $actual) { throw "checksum mismatch for $archive" }
  Expand-Archive (Join-Path $tmp $archive) -DestinationPath $tmp -Force
  New-Item -ItemType Directory -Path $dir -Force | Out-Null
  Copy-Item (Get-ChildItem $tmp -Recurse -Filter whylog.exe | Select-Object -First 1).FullName (Join-Path $dir 'whylog.exe') -Force
  Write-Host "installed $dir\whylog.exe ($(& (Join-Path $dir 'whylog.exe') version))"
  $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
  if (($userPath -split ';') -notcontains $dir) {
    [Environment]::SetEnvironmentVariable('Path', "$userPath;$dir", 'User')
    Write-Host "added $dir to your user PATH (restart the terminal)"
  }
  Write-Host "agents: run 'whylog skill --install' to add the Claude Code skill"
} finally { Remove-Item $tmp -Recurse -Force }
