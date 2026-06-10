# Deploy agent-centric CPUT package on testnet and run on-chain agent quorum + mint test.
# Usage: .\scripts\test-agent-quorum-testnet.ps1 [-Epoch 1006]

param(
    [string]$SuiCli = $(if (Test-Path "$env:LOCALAPPDATA\bin\sui.exe") { "$env:LOCALAPPDATA\bin\sui.exe" } else { "sui" }),
    [string]$RepoRoot = (Split-Path -Parent $PSScriptRoot),
    [uint64]$GasBudget = 200000000,
    [uint64]$MintCeiling = 1000000000000,
    [uint64]$Epoch = 1006,
    [uint64]$VerifiedGflops = 33000000
)

$ErrorActionPreference = "Stop"
$env:Path = "$env:LOCALAPPDATA\bin;$env:Path"
Set-Location $RepoRoot

function Invoke-SuiJson {
    param([string[]]$Args)
    $raw = & $SuiCli @Args 2>$null | Out-String
    if ($LASTEXITCODE -ne 0) { throw "sui failed (exit $LASTEXITCODE): $raw" }
    return ($raw | ConvertFrom-Json)
}

function Object-FromTx($tx, [string]$TypeSuffix) {
    $tx.objectChanges | Where-Object { $_.objectType -like "*$TypeSuffix*" } | Select-Object -First 1 -ExpandProperty objectId
}

Write-Host "== Agent-centric on-chain test (testnet) ==" -ForegroundColor Cyan
& $SuiCli client switch --env testnet | Out-Null
$deployer = (& $SuiCli client active-address 2>&1 | Select-Object -Last 1).Trim()
Write-Host "Deployer: $deployer"

Set-Location "$RepoRoot\sui-move"
& $SuiCli move build | Out-Null
Write-Host "Publishing package (test-publish)..." -ForegroundColor Cyan
$pub = Invoke-SuiJson @("client", "test-publish", "--build-env", "testnet", "--gas-budget", $GasBudget.ToString(), "--json")
$packageId = ($pub.objectChanges | Where-Object { $_.type -eq "published" }).packageId
$adminCap = Object-FromTx $pub "AdminCap"
$protocolState = Object-FromTx $pub "ProtocolState"
Write-Host "Package: $packageId"

function Init-Module([string]$Module, [string]$Function = "initialize") {
    $tx = Invoke-SuiJson @(
        "client", "call", "--package", $packageId, "--module", $Module,
        "--function", $Function, "--gas-budget", "100000000", "--args", $adminCap, "--json"
    )
    return $tx
}

Write-Host "Initializing modules..." -ForegroundColor Cyan
$oracleTx = Init-Module "oracle_verifier"
$mintTx = Init-Module "minting"
Init-Module "governance" | Out-Null
Init-Module "staking" | Out-Null
Init-Module "audit_registry" | Out-Null
Init-Module "pqc_anchor" | Out-Null
Init-Module "gas_sponsor" | Out-Null
$registryTx = Init-Module "agent_quorum" "initialize_registry"
$quorumTx = Init-Module "agent_quorum" "initialize_quorum_book"

$oracleState = Object-FromTx $oracleTx "OracleState"
$mintLog = Object-FromTx $mintTx "MintLog"
$agentRegistry = Object-FromTx $registryTx "AgentRegistry"
$agentQuorumBook = Object-FromTx $quorumTx "AgentQuorumBook"

Invoke-SuiJson @(
    "client", "call", "--package", $packageId, "--module", "cput",
    "--function", "configure_pools", "--gas-budget", "100000000",
    "--args", $adminCap, $protocolState, $deployer, $deployer, $deployer, $deployer, "--json"
) | Out-Null

Invoke-SuiJson @(
    "client", "call", "--package", $packageId, "--module", "cput",
    "--function", "set_ceiling", "--gas-budget", "100000000",
    "--args", $adminCap, $protocolState, $MintCeiling.ToString(), "false", "--json"
) | Out-Null

Write-Host "Registering formula agents 0..2..." -ForegroundColor Cyan
foreach ($id in 0..2) {
    $hash = "[" + ((1..32 | ForEach-Object { $id }) -join ",") + "]"
    Invoke-SuiJson @(
        "client", "call", "--package", $packageId, "--module", "agent_quorum",
        "--function", "register_agent", "--gas-budget", "100000000",
        "--args", $adminCap, $agentRegistry, $id.ToString(), $hash, "0", "--json"
    ) | Out-Null
}

$zkCommit = "0x" + ("c0" * 32)
$signers = "[0,1,2,3,4]"
Write-Host "submit_report epoch=$Epoch gflops=$VerifiedGflops" -ForegroundColor Cyan
Invoke-SuiJson @(
    "client", "call", "--package", $packageId, "--module", "oracle_verifier",
    "--function", "submit_report", "--gas-budget", "100000000",
    "--args", $adminCap, $oracleState, $Epoch.ToString(), $VerifiedGflops.ToString(),
    $zkCommit, $signers, "--json"
) | Out-Null

$policyHash = "0x" + ("ab" * 32)
$reasoningHash = "0x" + ("cd" * 32)
$agentIds = "[0,1,2]"
$proposals = "[$VerifiedGflops,$VerifiedGflops,$VerifiedGflops]"

Write-Host "approve_epoch_mint (agent quorum)" -ForegroundColor Cyan
$approveTx = Invoke-SuiJson @(
    "client", "call", "--package", $packageId, "--module", "agent_quorum",
    "--function", "approve_epoch_mint", "--gas-budget", "100000000",
    "--args", $adminCap, $agentRegistry, $agentQuorumBook, $oracleState,
    $Epoch.ToString(), $agentIds, $proposals, $VerifiedGflops.ToString(),
    $policyHash, $reasoningHash, "--json"
)

Write-Host "execute_mint epoch=$Epoch total=$VerifiedGflops" -ForegroundColor Cyan
$mintTx = Invoke-SuiJson @(
    "client", "call", "--package", $packageId, "--module", "minting",
    "--function", "execute_mint", "--gas-budget", "100000000",
    "--args", $adminCap, $protocolState, $oracleState, $agentQuorumBook, $mintLog,
    $Epoch.ToString(), $VerifiedGflops.ToString(), "--json"
)

$mintDigest = $mintTx.digest
Write-Host ""
Write-Host "== ON-CHAIN AGENT-CENTRIC TEST PASSED ==" -ForegroundColor Green
Write-Host "Package:          $packageId"
Write-Host "AgentRegistry:    $agentRegistry"
Write-Host "AgentQuorumBook:  $agentQuorumBook"
Write-Host "OracleState:      $oracleState"
Write-Host "MintLog:          $mintLog"
Write-Host "ProtocolState:  $protocolState"
Write-Host "AdminCap:         $adminCap"
Write-Host "Epoch minted:     $Epoch ($VerifiedGflops CPUT)"
Write-Host "Mint tx digest:   $mintDigest"
Write-Host ""
Write-Host "Update config/cput.toml with the IDs above, then:" -ForegroundColor Yellow
Write-Host "  cargo run --bin cput-relayer -- --config config/cput.toml --epoch $Epoch"
