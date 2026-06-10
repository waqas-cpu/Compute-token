# Wait for testnet gas, then publish CPUT package and print config template.
# Run after funding at https://faucet.sui.io

param(
    [int]$PollSeconds = 15,
    [int]$MaxWaitMinutes = 30,
    [uint64]$GasBudget = 200000000
)

$ErrorActionPreference = "Stop"
$env:Path = "$env:LOCALAPPDATA\bin;$env:Path"
$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

$addr = (sui client active-address 2>&1 | Select-Object -Last 1).Trim()
Write-Host "Waiting for gas on $addr (fund at https://faucet.sui.io/?address=$addr)" -ForegroundColor Yellow

$deadline = (Get-Date).AddMinutes($MaxWaitMinutes)
while ((Get-Date) -lt $deadline) {
    $gas = sui client gas 2>&1 | Out-String
    if ($gas -notmatch "No gas coins") {
        Write-Host "Gas detected — publishing..." -ForegroundColor Green
        & "$PSScriptRoot\deploy-testnet.ps1" -GasBudget $GasBudget
        exit $LASTEXITCODE
    }
    Write-Host "No gas yet — retrying in ${PollSeconds}s..."
    Start-Sleep -Seconds $PollSeconds
}
Write-Host "Timed out waiting for faucet funding." -ForegroundColor Red
exit 1
