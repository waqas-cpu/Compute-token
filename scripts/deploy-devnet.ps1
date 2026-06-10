# CPUT Sui devnet deploy — test-publish + init + config template.
# Devnet uses `sui client test-publish --build-env testnet` (no devnet entry in Move.toml).
#
# Usage:
#   .\scripts\deploy-devnet.ps1

param(
    [string]$SuiCli = $(if (Test-Path "$env:LOCALAPPDATA\bin\sui.exe") { "$env:LOCALAPPDATA\bin\sui.exe" } else { "sui" }),
    [string]$RepoRoot = (Split-Path -Parent $PSScriptRoot),
    [uint64]$GasBudget = 200000000,
    [uint64]$MintCeiling = 1000000000000
)

$ErrorActionPreference = "Stop"
Set-Location $RepoRoot

Write-Host "== CPUT devnet deploy ==" -ForegroundColor Cyan
& $SuiCli client switch --env devnet | Out-Null

$gas = & $SuiCli client gas 2>&1 | Out-String
if ($gas -match "No gas coins") {
    Write-Host "Requesting devnet faucet..." -ForegroundColor Yellow
    $addr = (& $SuiCli client active-address 2>&1 | Select-Object -Last 1).Trim()
    $body = '{"FixedAmountRequest":{"recipient":"' + $addr + '"}}'
    Invoke-RestMethod -Uri "https://faucet.devnet.sui.io/v2/gas" -Method POST -ContentType "application/json" -Body $body | Out-Null
    Start-Sleep -Seconds 8
}

Set-Location "$RepoRoot\sui-move"
& $SuiCli move build | Out-Null
$publishJson = & $SuiCli client test-publish --build-env testnet --gas-budget $GasBudget.ToString() --json 2>&1 | Out-String | ConvertFrom-Json
$packageId = ($publishJson.objectChanges | Where-Object { $_.type -eq "published" }).packageId
$adminCap = ($publishJson.objectChanges | Where-Object { $_.objectType -like "*AdminCap*" }).objectId
$protocolState = ($publishJson.objectChanges | Where-Object { $_.objectType -like "*ProtocolState*" }).objectId
$deployer = (& $SuiCli client active-address 2>&1 | Select-Object -Last 1).Trim()

Write-Host "Package: $packageId"
Write-Host "AdminCap: $adminCap"

foreach ($mod in @("oracle_verifier","minting","governance","staking","audit_registry","pqc_anchor")) {
    & $SuiCli client call --package $packageId --module $mod --function initialize --args $adminCap --gas-budget 100000000 | Out-Null
}

& $SuiCli client call --package $packageId --module cput --function configure_pools `
    --args $adminCap $protocolState $deployer $deployer $deployer $deployer --gas-budget 100000000 | Out-Null
& $SuiCli client call --package $packageId --module cput --function set_ceiling `
    --args $adminCap $protocolState $MintCeiling false --gas-budget 100000000 | Out-Null

Write-Host "Done. Fill shared object IDs from init txs into config/cput.devnet.toml" -ForegroundColor Green
