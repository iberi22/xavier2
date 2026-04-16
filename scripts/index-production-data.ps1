# Xavier2 Production Data Indexer
# Pobla las capas de memoria con datos reales de la operación

param(
    [int]$BatchSize = 10
)

$XAVIER2_URL = "http://127.0.0.1:8003"
$TOKEN = "dev-token"
$HEADER = @{"X-Xavier2-Token" = $TOKEN; "Content-Type" = "application/json"}

function Add-Memory {
    param(
        [string]$Content,
        [string]$Path,
        [string]$Category = "general"
    )
    
    $body = @{
        content = $Content
        path = $Path
        metadata = @{
            category = $Category
            indexed_at = (Get-Date).ToString("o")
            source = "production-indexer"
        }
    } | ConvertTo-Json -Compress
    
    try {
        $null = Invoke-RestMethod -Uri "$XAVIER2_URL/memory/add" -Method Post -Headers $HEADER -Body $body -TimeoutSec 30
        return $true
    }
    catch {
        Write-Host "Error adding $Path : $_"
        return $false
    }
}

Write-Host "============================================================"
Write-Host "XAVIER2 PRODUCTION DATA INDEXER"
Write-Host "============================================================"

# SWAL Business Context
Write-Host "`n[1/8] SWAL Business Context..."
Add-Memory -Content "SouthWest AI Labs (SWAL) - AI development company. Builds autonomous agents, SaaS products, and custom software. Founder: BELA (BeRi0n3)." -Path "swal/business/overview" -Category "business"
Add-Memory -Content "SWAL Products: ManteniApp (machinery monitoring SaaS), Xavier2 (memory system), Gestalt-Rust (runtime), ZeroClaw (runtime), Cortex (enterprise memory)." -Path "swal/products" -Category "product"
Add-Memory -Content "BELA - Developer and founder of SWAL. Manages multiple ventures: AI Labs, Laboratory Engineering, Content Creator Career, Influencer Manager." -Path "swal/team/bela" -Category "person"

# ManteniApp
Write-Host "[2/8] ManteniApp..."
Add-Memory -Content "ManteniApp - Machinery monitoring SaaS built with Rust. Pricing: Starter $499/mo, Pro $999/mo, Enterprise $2,499/mo. Target: industrial clients." -Path "product/manteniapp/pricing" -Category "product"
Add-Memory -Content "ManteniApp Features: Real-time monitoring, AI anomaly detection, predictive maintenance alerts, dashboard analytics." -Path "product/manteniapp/features" -Category "product"
Add-Memory -Content "ManteniApp Demo: tripro.cl/manteniapp - Demo site maintained by Leonardo Duque at Rodacenter Chile." -Path "product/manteniapp/demo" -Category "product"

# Clients
Write-Host "[3/8] Clients..."
Add-Memory -Content "Leonardo Duque - External seller/partner at Rodacenter Chile. Client company: tripro.cl. Interested in ManteniApp for Antofagasta operations." -Path "client/leonardo-duque" -Category "client"
Add-Memory -Content "Rodacenter - Client prospect in Chile (Antofagasta). Company website: tripro.cl. Interested in ManteniApp machinery monitoring." -Path "client/rodacenter" -Category "client"
Add-Memory -Content "Tripro - tripro.cl - Industrial company in Chile. Part of Rodacenter. Target customer for ManteniApp." -Path "client/tripro" -Category "client"

# OpenClaw Configuration
Write-Host "[4/8] OpenClaw Configuration..."
Add-Memory -Content "OpenClaw - Agent orchestration platform. Runs SWAL agents (main, ventas). Connects to Telegram. Manages heartbeats, cron jobs, memory." -Path "system/openclaw" -Category "system"
Add-Memory -Content "OpenClaw Agent 'ventas' - Sales agent. Manages prospects, RFI generation, proposals for SWAL products. Connected via Telegram." -Path "system/openclaw/agents/ventas" -Category "agent"
Add-Memory -Content "OpenClaw Agent 'main' - Primary agent for BELA. Handles overall operations, coding, research, management tasks." -Path "system/openclaw/agents/main" -Category "agent"

# Xavier2 Memory System
Write-Host "[5/8] Xavier2 Memory System..."
Add-Memory -Content "Xavier2 - Multi-layer memory system for AI agents. Layers: Working Memory (recent), Episodic (sessions), Semantic (entities). Uses RRF fusion." -Path "system/xavier2" -Category "system"
Add-Memory -Content "Xavier2 API: POST /memory/add (content, path), POST /memory/search (query, limit), POST /memory/query (query). Auth: X-Xavier2-Token header." -Path "system/xavier2/api" -Category "system"
Add-Memory -Content "Xavier2 vs Cortex: Xavier2 is OSS MIT, Cortex is Enterprise. Both use same core memory architecture." -Path "system/xavier2/comparison" -Category "system"

# Sales Operations
Write-Host "[6/8] Sales Operations..."
Add-Memory -Content "Sales Process: 1) Prospect research, 2) Initial contact, 3) RFI generation, 4) Proposal creation, 5) Negotiation, 6) Close." -Path "sales/process" -Category "sales"
Add-Memory -Content "RFI Template: Used for gathering client requirements. Includes business context, current systems, pain points, expected outcomes." -Path "sales/templates/rfi" -Category "sales"
Add-Memory -Content "Proposal Template: Includes executive summary, solution description, pricing, timeline, terms, call to action." -Path "sales/templates/proposal" -Category "sales"

# Benchmark Results
Write-Host "[7/8] Benchmark History..."
Add-Memory -Content "Benchmark 2026-04-15: Xavier2 avg=516ms p95=1145ms with Ollama local. Cortex avg=962ms. Xavier2 is faster but retrieval needs tuning." -Path "benchmark/results/20260415" -Category "benchmark"

# Skills and Tools
Write-Host "[8/8] Skills and Tools..."
Add-Memory -Content "Available Skills: sales-pro (RFI/proposals), src-generator (SRC docs), market-research, generate-presentation, email-daily-summary." -Path "skills/list" -Category "skill"
Add-Memory -Content "Web Research: MiniMax MCP (primary), Brave Search (fast), Tavily Search (AI-optimized). APIs configured in TOOLS.md." -Path "tools/web-research" -Category "tool"

Write-Host "`n============================================================"
Write-Host "INDEXING COMPLETE"
Write-Host "============================================================"

# Verify
Write-Host "`n[VERIFY] Testing retrieval..."
$test = Invoke-RestMethod -Uri "$XAVIER2_URL/memory/query" -Method Post -Headers $HEADER -Body (@{query="ManteniApp"; limit=3} | ConvertTo-Json -Compress) -TimeoutSec 30
if ($test.status -eq "ok") {
    Write-Host "Test query 'ManteniApp': $($test.response.Substring(0, [math]::Min(80, $test.response.Length)))..."
}

Write-Host "`nIndexing complete! Xavier2 is now populated with production data."
