# Xavier2 Validation Prompts

Use these prompts to validate Xavier2 installation and performance.

---

## 1. Basic Health Check

```
Ask Xavier2: "What is your version and status?"
Expected: {version, status: ok}
```

## 2. Memory Operations

### Add Memory
```
Add: "Xavier2 validation test - [DATE]"
Metadata: source=validation, priority=low
Expected: {status: ok}
```

### Search Memory
```
Search: "validation test"
Expected: Results containing "Xavier2 validation"
```

### Stats Check
```
Get stats
Expected: total_documents > 0
```

---

## 3. Performance Validation

### Latency Test
```
Time 10 sequential searches
Target: < 100ms average
```

### Recall Test
```
Add fact: "Validation fact unique id [RANDOM]"
Search for it
Expected: Found within 3 results
```

---

## 4. Auto-Curation Validation

### Decay Test
```
Apply decay (dry run)
Expected: {processed: N, actions: [...]}
```

### Consolidate Test
```
Add duplicate facts
Run consolidate
Expected: Duplicates merged
```

### Evict Test
```
Check quality
Expected: Low quality count > 0 OR = 0
```

---

## 5. Integration Tests

### OpenClaw Sync
```bash
node sync-all-to-xavier2.js
Expected: X memories synced
```

### Benchmark
```bash
powershell -File swal-locomo-benchmark.ps1
Expected: Recall >= 95%, Precision >= 4.0
```

---

## 6. Cloud Tier Validation (if deployed)

### Web Endpoint
```
curl https://[your-domain]/health
Expected: {status: ok}
```

### Token Auth
```
curl -H "X-Xavier2-Token: wrong-token" /health
Expected: 401 Unauthorized
```

---

## Automated Validation Script

```powershell
# xavier2-validation.ps1
$ErrorActionPreference = "Stop"

Write-Host "🧪 Xavier2 Validation" -ForegroundColor Cyan

# 1. Health
$h = Invoke-RestMethod http://localhost:8003/health
if ($h.status -ne "ok") { throw "Health check failed" }
Write-Host "✅ Health: $($h.version)"

# 2. Add memory
$add = Invoke-RestMethod http://localhost:8003/memory/add -Method Post `
  -Body (@{content="Test $(Get-Random)"; metadata=@{source="validation"}} | ConvertTo-Json) `
  -ContentType "application/json"
if ($add.status -ne "ok") { throw "Add failed" }
Write-Host "✅ Add memory: $($add.workspace_id)"

# 3. Search
$search = Invoke-RestMethod http://localhost:8003/memory/search -Method Post `
  -Body (@{query="Test"; limit=5} | ConvertTo-Json) `
  -ContentType "application/json"
if ($search.results.Count -eq 0) { throw "Search returned no results" }
Write-Host "✅ Search: $($search.results.Count) results"

# 4. Stats
$stats = Invoke-RestMethod http://localhost:8003/memory/stats
Write-Host "✅ Stats: $($stats.total_documents) documents"

Write-Host "`n🎉 All validations passed!" -ForegroundColor Green
```

Run with:
```powershell
.\xavier2-validation.ps1
```

---

## Success Criteria

| Test | Minimum | Ideal |
|------|---------|-------|
| Health check | ✅ ok | ✅ ok |
| Add memory | ✅ success | ✅ < 50ms |
| Search | ✅ finds results | ✅ < 100ms |
| Stats | ✅ returns | ✅ accurate |
| Decay | ✅ runs | ✅ 0 actions needed |
| Recall (benchmark) | ≥ 90% | ≥ 95% |
| Precision (benchmark) | ≥ 3.5/5 | ≥ 4.0/5 |
