<#
.SYNOPSIS
  Builds and signs the MSIX sparse package.

.DESCRIPTION
  The package exists only to give the installed `bw.exe` package identity, so
  that `UserNotificationListener` will report what other applications have
  posted to the Action Center. Everything else in the shell works without it.

  This is separate from `tauri build` on purpose. Signing needs a certificate,
  and a certificate is a decision about the machine — which one to trust, and
  who is allowed to install one — that no build script should make on somebody
  else's behalf. So this takes a certificate you already have and does not
  create, install or trust anything.

  See docs/msix.md for what to do with the result, including how to make a
  certificate for testing and what trusting one actually means.

.PARAMETER Certificate
  Path to the .pfx to sign with. Its subject is written into the manifest's
  Publisher, which Windows requires to match exactly.

.PARAMETER Password
  The .pfx password, if it has one.

.PARAMETER Output
  Where to write the .msix. Defaults to beside this script.

.EXAMPLE
  .\build.ps1 -Certificate .\mine.pfx -Password (Read-Host -AsSecureString)
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$Certificate,
  [System.Security.SecureString]$Password,
  [string]$Output = (Join-Path $PSScriptRoot 'beautiful-wallpaper-sparse.msix')
)

$ErrorActionPreference = 'Stop'

# makeappx and signtool come with the Windows SDK and are not on PATH by
# default. Looking them up beats telling somebody to "add the SDK to PATH",
# which is the step people get wrong.
function Find-SdkTool([string]$Name) {
  $found = Get-Command $Name -ErrorAction SilentlyContinue
  if ($found) { return $found.Source }

  $roots = @(
    "${env:ProgramFiles(x86)}\Windows Kits\10\bin",
    "$env:ProgramFiles\Windows Kits\10\bin"
  ) | Where-Object { Test-Path $_ }

  $tool = $roots |
    ForEach-Object { Get-ChildItem $_ -Recurse -Filter $Name -ErrorAction SilentlyContinue } |
    Where-Object { $_.FullName -match '\\x64\\' } |
    Sort-Object FullName -Descending |
    Select-Object -First 1

  if (-not $tool) {
    throw "$Name was not found. It comes with the Windows SDK; install it, or run this from a Developer Command Prompt."
  }
  $tool.FullName
}

$makeappx = Find-SdkTool 'makeappx.exe'
$signtool = Find-SdkTool 'signtool.exe'

# The subject has to go into the manifest verbatim. A mismatch here is the
# single most common reason Windows refuses a package, and the error it gives
# says nothing useful about which of the two is wrong.
$plain = ''
if ($Password) {
  $plain = [Runtime.InteropServices.Marshal]::PtrToStringAuto(
    [Runtime.InteropServices.Marshal]::SecureStringToBSTR($Password))
}
$cert = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new(
  (Resolve-Path $Certificate), $plain)
$subject = $cert.Subject
Write-Host "Signing as: $subject"

$staging = Join-Path ([System.IO.Path]::GetTempPath()) ("bw-sparse-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $staging -Force | Out-Null
try {
  $manifest = Get-Content (Join-Path $PSScriptRoot 'AppxManifest.xml') -Raw
  $manifest = $manifest -replace 'Publisher="[^"]*"', ('Publisher="' + $subject + '"')
  Set-Content -Path (Join-Path $staging 'AppxManifest.xml') -Value $manifest -Encoding UTF8

  # The logos the manifest names. A package whose manifest points at an image
  # that is not in it fails to pack, with a message about the image rather than
  # about the manifest.
  $images = Join-Path $staging 'images'
  New-Item -ItemType Directory -Path $images -Force | Out-Null
  $icons = Join-Path $PSScriptRoot '..\icons'
  Copy-Item (Join-Path $icons '128x128.png') (Join-Path $images 'StoreLogo.png')
  Copy-Item (Join-Path $icons '128x128.png') (Join-Path $images 'Square150x150Logo.png')
  Copy-Item (Join-Path $icons '32x32.png')   (Join-Path $images 'Square44x44Logo.png')

  # `/nv` skips validation that only applies to packages carrying payload.
  & $makeappx pack /d $staging /p $Output /nv /o
  if ($LASTEXITCODE -ne 0) { throw "makeappx failed with $LASTEXITCODE" }

  $signArgs = @('sign', '/fd', 'SHA256', '/a', '/f', (Resolve-Path $Certificate))
  if ($plain) { $signArgs += @('/p', $plain) }
  $signArgs += $Output

  & $signtool @signArgs
  if ($LASTEXITCODE -ne 0) { throw "signtool failed with $LASTEXITCODE" }

  Write-Host "Wrote $Output"
  Write-Host "Put it beside bw.exe in the install folder, then switch on hacks.readOtherNotifications and restart the shell."
}
finally {
  Remove-Item $staging -Recurse -Force -ErrorAction SilentlyContinue
}
