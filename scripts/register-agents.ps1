# Register 3 formula agents on-chain (run after deploy, with AdminCap + AgentRegistry IDs).
param(
    [Parameter(Mandatory = $true)][string]$PackageId,
    [Parameter(Mandatory = $true)][string]$AdminCap,
    [Parameter(Mandatory = $true)][string]$AgentRegistry,
    [string]$SuiCli = "sui",
    [int]$GasBudget = 100_000_000
)

$env:Path = "$env:LOCALAPPDATA\bin;$env:Path"

function Invoke-RegisterAgent([int]$AgentId) {
    $hash = (1..32 | ForEach-Object { $AgentId }) -join ","
    & $SuiCli client call `
        --package $PackageId `
        --module agent_quorum `
        --function register_agent `
        --gas-budget $GasBudget `
        --args $AdminCap `
        --args $AgentRegistry `
        --args $AgentId `
        --args "[$hash]" `
        --args 0
}

Write-Host "Registering formula agents 0..2 on AgentRegistry $AgentRegistry"
foreach ($id in 0..2) {
    Write-Host "  agent $id"
    Invoke-RegisterAgent $id | Out-Null
}
Write-Host "Done."
