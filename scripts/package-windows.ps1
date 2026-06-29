#Requires -Version 5.1
# Windows release: portable zip + Inno Setup installer.
# Usage: .\scripts\package-windows.ps1 -Version 0.4.0 [-OutDir dist]

param(
    [Parameter(Mandatory = $true)]
    [string]$Version,
    [string]$OutDir = "dist",
    [string]$OrtVersion = "1.20.1"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path $PSScriptRoot -Parent

$Staging = Join-Path $Root "packaging\windows\staging"
$ZipName = "openpolysphere-$Version-windows-x64"
$ZipStaging = Join-Path $OutDir $ZipName
$OrtDll = Join-Path $Root "ort\onnxruntime-win-x64-$OrtVersion\lib\onnxruntime.dll"
$Exe = Join-Path $Root "target\release\translator.exe"
$AppExe = Join-Path $Root "target\release\openpolysphere.exe"

if (-not (Test-Path $Exe)) { throw "missing $Exe — build release first" }
if (-not (Test-Path $AppExe)) { throw "missing $AppExe — run: cargo build --release -p openpolysphere-app" }
if (-not (Test-Path $OrtDll)) { throw "missing $OrtDll" }

New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

# Staging for Inno (and zip)
if (Test-Path $Staging) { Remove-Item -Recurse -Force $Staging }
New-Item -ItemType Directory -Force -Path $Staging | Out-Null
Copy-Item $AppExe $Staging\
Copy-Item $Exe $Staging\
Copy-Item $OrtDll $Staging\
Copy-Item -Recurse (Join-Path $Root "web") (Join-Path $Staging "web")
Copy-Item (Join-Path $Root ".env.example") $Staging\
Copy-Item (Join-Path $Root "README.md") $Staging\
if (Test-Path (Join-Path $Root "README.ru.md")) {
    Copy-Item (Join-Path $Root "README.ru.md") $Staging\
}
@(
    "OpenPolySphere $Version — Windows x64",
    "",
    "Installer (recommended): run openpolysphere-*-windows-x64-setup.exe",
    "",
    "Portable zip:",
    "  1. Unzip to a folder",
    "  2. .\openpolysphere.exe setup",
    "  3. .\openpolysphere.exe   (embedded window — close to quit)",
    "",
    "Virtual audio: VB-Audio Virtual Cable (see docs/windows.md)",
    "espeak-ng: choco install espeak-ng if TTS phonemization fails"
) | Set-Content -Path (Join-Path $Staging "WINDOWS.txt")

# Portable zip
if (Test-Path $ZipStaging) { Remove-Item -Recurse -Force $ZipStaging }
Copy-Item -Recurse $Staging $ZipStaging
$ZipPath = Join-Path $OutDir "$ZipName.zip"
if (Test-Path $ZipPath) { Remove-Item -Force $ZipPath }
Compress-Archive -Path $ZipStaging -DestinationPath $ZipPath -Force
Remove-Item -Recurse -Force $ZipStaging

# Inno Setup
$Iscc = "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe"
if (-not (Test-Path $Iscc)) {
    $Iscc = "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
}
if (-not (Test-Path $Iscc)) {
    throw "Inno Setup ISCC.exe not found — install with: choco install innosetup"
}

Push-Location (Join-Path $Root "packaging\windows")
& $Iscc "/DMyAppVersion=$Version" "openpolysphere.iss"
Pop-Location

Write-Host "Created:"
Write-Host "  $ZipPath"
Write-Host "  $(Join-Path $OutDir "openpolysphere-$Version-windows-x64-setup.exe")"
