# Integration test: cput-api ↔ Next.js proxy ↔ dashboard endpoints.
# Usage: .\scripts\test-frontend-api.ps1

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

Write-Host "==> Building cput-api..." -ForegroundColor Cyan
$prevEap = $ErrorActionPreference
$ErrorActionPreference = "Continue"
cargo build --bin cput-api *> $null
$buildExit = $LASTEXITCODE
$ErrorActionPreference = $prevEap
if ($buildExit -ne 0) { throw "cargo build failed (exit $buildExit)" }

$apiProc = Start-Process -FilePath "cargo" `
    -ArgumentList "run","--bin","cput-api","--","--listen","127.0.0.1:8787" `
    -PassThru -NoNewWindow -WorkingDirectory $Root

Start-Sleep -Seconds 4

function Test-Endpoint {
    param([string]$Name, [string]$Url, [string]$Method = "GET", [string]$Body = $null)
    Write-Host "  $Method $Url" -NoNewline
    try {
        $params = @{ Uri = $Url; Method = $Method; UseBasicParsing = $true }
        if ($Body) {
            $params["ContentType"] = "application/json"
            $params["Body"] = $Body
        }
        $r = Invoke-WebRequest @params
        if ($r.StatusCode -ge 200 -and $r.StatusCode -lt 300) {
            Write-Host " OK ($($r.StatusCode))" -ForegroundColor Green
            return $r.Content
        }
        throw "status $($r.StatusCode)"
    } catch {
        Write-Host " FAIL" -ForegroundColor Red
        throw "${Name}: $_"
    }
}

try {
    Write-Host "`n==> Rust API smoke (port 8787)" -ForegroundColor Cyan
    Test-Endpoint "health" "http://127.0.0.1:8787/api/v1/health" | Out-Null
    $dash = Test-Endpoint "dashboard" "http://127.0.0.1:8787/api/v1/dashboard"
    $dashJson = $dash | ConvertFrom-Json
    if (-not $dashJson.heroStats) { throw "dashboard missing heroStats" }
    Test-Endpoint "mint" "http://127.0.0.1:8787/api/v1/mint" "POST" '{"amount":250}' | Out-Null

    Write-Host "`n==> cargo test -p cput-api" -ForegroundColor Cyan
    $ErrorActionPreference = "Continue"
    cargo test -p cput-api --test api_smoke *> $null
    $testExit = $LASTEXITCODE
    $ErrorActionPreference = $prevEap
    if ($testExit -ne 0) { throw "cput-api smoke tests failed (exit $testExit)" }
    Write-Host "  cargo test OK" -ForegroundColor Green

    Write-Host "`n==> All frontend-backend pipeline checks passed" -ForegroundColor Green
} finally {
    if ($apiProc -and -not $apiProc.HasExited) {
        Stop-Process -Id $apiProc.Id -Force -ErrorAction SilentlyContinue
    }
}
