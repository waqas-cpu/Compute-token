# CPUT Sui testnet deploy — publish package, initialize modules, emit config.
# Prerequisites: sui CLI on PATH (or .sui-bin/sui.exe), active-env = testnet, funded address.
#
# Usage:
#   .\scripts\deploy-testnet.ps1
#   .\scripts\deploy-testnet.ps1 -SuiCli ".\.sui-bin\sui.exe" -GasBudget 200000000

param(
    [string]$SuiCli = $(if (Test-Path "$env:LOCALAPPDATA\bin\sui.exe") { "$env:LOCALAPPDATA\bin\sui.exe" } else { "sui" }),
    [string]$RepoRoot = (Split-Path -Parent $PSScriptRoot),
    [uint64]$GasBudget = 200000000,
    [uint64]$MintCeiling = 1000000000000,
    [switch]$SkipPublish
)

$ErrorActionPreference = "Stop"
Set-Location $RepoRoot

function Invoke-Sui {
    param([string[]]$Args)
    $out = & $SuiCli @Args 2>&1
    if ($LASTEXITCODE -ne 0) { throw "sui $($Args -join ' ') failed: $out" }
    return $out
}

Write-Host "== CPUT testnet deploy ==" -ForegroundColor Cyan
$env = (Invoke-Sui @("client", "active-env")) | Select-Object -Last 1
if ($env -ne "testnet") {
    Write-Warning "Active env is '$env' — switching to testnet"
    Invoke-Sui @("client", "switch", "--env", "testnet") | Out-Null
}

$gas = Invoke-Sui @("client", "gas")
if ($gas -match "No gas coins") {
    Write-Host "Requesting testnet faucet..." -ForegroundColor Yellow
    Invoke-Sui @("client", "faucet") | Out-Null
    Start-Sleep -Seconds 5
    $gas = Invoke-Sui @("client", "gas")
    if ($gas -match "No gas coins") {
        throw "No testnet SUI — fund address via https://faucet.sui.io then re-run"
    }
}

$deployer = (Invoke-Sui @("client", "active-address")) | Select-Object -Last 1
Write-Host "Deployer: $deployer"

# Build Move package
Set-Location "$RepoRoot\sui-move"
Invoke-Sui @("move", "build") | Out-Null
Invoke-Sui @("move", "test") | Out-Null

$packageId = $null
if (-not $SkipPublish) {
    Write-Host "Publishing package (gas budget $GasBudget)..." -ForegroundColor Cyan
    $publishOut = Invoke-Sui @("client", "publish", "--gas-budget", $GasBudget.ToString())
    $publishJson = $publishOut | ConvertFrom-Json
    $packageId = $publishJson.objectChanges | Where-Object { $_.type -eq "published" } | Select-Object -ExpandProperty packageId
    if (-not $packageId) {
        $packageId = ($publishOut | Select-String -Pattern '0x[a-f0-9]{64}' -AllMatches).Matches[-1].Value
    }
    Write-Host "Package ID: $packageId"
} else {
    $packageId = Read-Host "Enter existing package ID"
}

function Call-Init {
    param([string]$Module, [string]$Function, [string[]]$ExtraArgs = @())
    $args = @(
        "client", "call",
        "--package", $packageId,
        "--module", $Module,
        "--function", $Function,
        "--gas-budget", $GasBudget.ToString(),
        "--args", $adminCap
    ) + $ExtraArgs
    Invoke-Sui $args | Out-Null
}

# Resolve AdminCap from publish or user input
$adminCap = Read-Host "AdminCap object ID (from publish output)"
if ([string]::IsNullOrWhiteSpace($adminCap)) { throw "AdminCap required" }

Write-Host "Initializing on-chain modules..." -ForegroundColor Cyan
Call-Init "oracle_verifier" "initialize"
Call-Init "minting" "initialize"
Call-Init "governance" "initialize"
Call-Init "staking" "initialize"
Call-Init "audit_registry" "initialize"
Call-Init "pqc_anchor" "initialize"
Call-Init "gas_sponsor" "initialize"
Call-Init "agent_quorum" "initialize_registry"
Call-Init "agent_quorum" "initialize_quorum_book"

Write-Host "Configure pools (using deployer address for all pools on testnet)..." -ForegroundColor Cyan
Invoke-Sui @(
    "client", "call",
    "--package", $packageId,
    "--module", "cput",
    "--function", "configure_pools",
    "--gas-budget", $GasBudget.ToString(),
    "--args", $adminCap,
    "--args", $deployer,
    "--args", $deployer,
    "--args", $deployer,
    "--args", $deployer
) | Out-Null

Write-Host "Setting mint ceiling to $MintCeiling..." -ForegroundColor Cyan
Invoke-Sui @(
    "client", "call",
    "--package", $packageId,
    "--module", "cput",
    "--function", "set_ceiling",
    "--gas-budget", $GasBudget.ToString(),
    "--args", $adminCap,
    "--args", $MintCeiling,
    "--args", "false"
) | Out-Null

Write-Host ""
Write-Host "Collect shared object IDs from publish/initialize transactions, then fill config/cput.toml:" -ForegroundColor Yellow
Write-Host @"
rpc_url = `"https://fullnode.testnet.sui.io:443`"
package_id = `"$packageId`"
admin_cap = `"$adminCap`"
# protocol_state, oracle_state, mint_log, governance_state, stake_book,
# audit_registry, pqc_anchor_book, agent_registry, agent_quorum_book — from objectChanges
# After deploy, register 3 formula agents (IDs 0..2) via agent_quorum::register_agent
dry_run = true

[pools]
provider = `"$deployer`"
oracle = `"$deployer`"
treasury = `"$deployer`"
burn_reserve = `"$deployer`"
"@

Write-Host ""
Write-Host "Smoke test (dry-run):" -ForegroundColor Cyan
Write-Host "  cargo run --release --bin cput-relayer -- --config config/cput.toml --dry-run --epoch 1000"
Write-Host "  cargo run --release --bin cput-agent -- --health"
