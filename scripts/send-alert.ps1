$body = @{
    chat_id = '777007827'
    text = 'Xavier Optimizer Alert - Benchmark Score: 20% (1/5 tests passed). Below 70% threshold - human review required. Passed: bench/person. Failed: bench/city, bench/company, bench/project, bench/tech. Xavier container healthy (v0.4.1, vec backend). Project changes detected: xavier(3), manteniapp(3). Time: 2026-04-14 08:37 UTC'
}
Invoke-RestMethod -Uri 'https://api.telegram.org/bot7304978495:AAEQ4hzBaJA6ZvNPabU_PQv5R9xRiT9gK40/sendMessage' -Method Post -Body $body