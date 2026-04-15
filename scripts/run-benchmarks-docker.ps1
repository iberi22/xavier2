param(
    [int]$SampleLimit = 10,
    [int]$QuestionLimit = 5,
    [string]$OutputDir = "benchmark-results/locomo-docker",
    [int]$HostPort = 8003,
    [switch]$SWEbench
)

$ErrorActionPreference = "Stop"

function Invoke-Compose {
    param(
        [Parameter(ValueFromRemainingArguments = $true)]
        [string[]]$ComposeArgs
    )

    $env:XAVIER2_BENCHMARK_PORT = [string]$HostPort
    & docker compose -f $compose @ComposeArgs
    if ($LASTEXITCODE -ne 0) {
        throw "docker compose failed with exit code $LASTEXITCODE"
    }
}

$compose = "docker-compose.benchmarks.yml"

Write-Host "Building Xavier2 benchmark images..."
Invoke-Compose build xavier2-benchmark locomo-benchmark

try {
    Write-Host "Starting Xavier2 benchmark service..."
    Invoke-Compose @("up", "-d", "--wait", "xavier2-benchmark")

    Write-Host "Running LoCoMo benchmark in Docker..."
    Invoke-Compose @(
        "run",
        "--rm",
        "-e",
        "PYTHONUNBUFFERED=1",
        "locomo-benchmark",
        "python",
        "scripts/benchmarks/run_locomo_benchmark.py",
        "--base-url",
        "http://xavier2-benchmark:8003",
        "--output-dir",
        $OutputDir,
        "--sample-limit",
        "$SampleLimit",
        "--question-limit",
        "$QuestionLimit",
        "--use-existing-server"
    )

    Write-Host "LoCoMo results written under $OutputDir"

    if ($SWEbench) {
        Write-Host "Running SWE-bench benchmark in Docker..."
        Invoke-Compose @(
            "--profile",
            "swebench",
            "up",
            "--abort-on-container-exit",
            "--exit-code-from",
            "swebench-benchmark",
            "swebench-benchmark"
        )
        Write-Host "SWE-bench results written under benchmark-results\\swebench-docker"
    }
}
finally {
    & docker compose -f $compose down | Out-Null
}
