# Sui development setup (Windows) — mirrors MystenLabs suiup install flow.
# Usage: .\scripts\setup-sui-dev.ps1

$ErrorActionPreference = "Stop"

Write-Host "== Installing suiup ==" -ForegroundColor Cyan
$installDir = Join-Path $env:USERPROFILE ".local\bin"
New-Item -ItemType Directory -Force -Path $installDir | Out-Null
$zip = Join-Path $env:TEMP "suiup-Windows-msvc-x86_64.zip"
curl.exe -sSfL -o $zip "https://github.com/MystenLabs/suiup/releases/download/v0.0.13/suiup-Windows-msvc-x86_64.zip"
Expand-Archive -Path $zip -DestinationPath $installDir -Force

$suiupBin = Join-Path $installDir "suiup.exe"
$suiBinDir = Join-Path $env:LOCALAPPDATA "bin"

Write-Host "== Installing sui@testnet ==" -ForegroundColor Cyan
& $suiupBin install -y sui@testnet
& $suiupBin default set sui@testnet

$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$suiBinDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$suiBinDir;$userPath", "User")
    Write-Host "Added $suiBinDir to user PATH" -ForegroundColor Green
}

$env:Path = "$suiBinDir;$installDir;$env:Path"
& "$suiBinDir\sui.exe" --version
& $suiupBin doctor

Write-Host ""
Write-Host "== Sui client ==" -ForegroundColor Cyan
& "$suiBinDir\sui.exe" client active-env
$addr = (& "$suiBinDir\sui.exe" client active-address 2>&1 | Select-Object -Last 1).Trim()
Write-Host "Active address: $addr"
Write-Host ""
Write-Host "Fund this address at: https://faucet.sui.io/?address=$addr" -ForegroundColor Yellow
