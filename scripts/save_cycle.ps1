$XAVIER2_URL = "http://localhost:8003"
$TOKEN = "dev-token"
$body = @{
  path="trading/cycle/2026-04-10-0048"
  content="## Trading Cycle 171 — 2026-04-10 05:48 UTC

### Leaderboard
| Rank | Agent | Balance | Daily PnL | Win Rate | Trades |
|------|-------|---------|-----------|----------|--------|
| 1 | Qwen Trader | 101.45 | +1.45 | 60% | 5 |
| 2 | GLM Trader | 101.00 | +1.00 | 100% | 1 |
| 3 | Kimi Trader | 100.60 | +0.60 | 100% | 1 |
| 4 | Claude Trader | 100.00 | 0.00 | 0% | 0 |
| 5 | ESTRATEGIA | 100.00 | 0.00 | 0% | 0 |

### Bot Status
- Balance: 100.0 USDT (paper)
- Equity: 100.0 USDT
- Open Positions: 3 (SOLUSDT Long, ETHUSDT Long, BTCUSDT Long)
- Unrealized PnL: +0.031 USDT
- All containers healthy

### AutoResearch Result
- Targeted: GLM Trader (Trend Follower)
- Result: No improvement — reverted to best
- Next target: Qwen Trader

### System Health
- Docker: All containers healthy
- Bot API: Responding
- Best strategy: Qwen Trader (60% WR, +1.45)
- Top performer style: Mean Reversion (Qwen), Trend Follower (GLM)

### Issues & Recommendations
1. 3 agents have 0 trades — need better signal generation
2. Win rate stuck at 0 for new agents
3. Suggested fix: Lower volatility threshold from 5% to 3%
4. Consider increasing position sizing for high-confidence signals
"
  metadata=@{
    type="cycle_report"
    cycle=171
    date="2026-04-10"
    best_strategy="Qwen Trader"
  }
} | ConvertTo-Json -Compress
$temp = [System.IO.Path]::GetTempFileName() + ".json"
[System.IO.File]::WriteAllText($temp, $body)
curl.exe -s -X POST "$XAVIER2_URL/memory/add" -H "X-Xavier2-Token: $TOKEN" -H "Content-Type: application/json" --data-binary "@$temp"
Remove-Item $temp
Write-Host "Saved to Xavier2"