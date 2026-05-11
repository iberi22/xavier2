# Dogfooding ADR-006 — Using Change Control Plane to fix its own bug
# ------------------------------------------------------------------
# This script demonstrates the full Change Control workflow:
#   create task → claim lease → fix code → complete task → merge plan
#
# The bug fixed: deadlock in complete_task() — write lock + read lock on same RwLock.
# Found via 5-axis SWAL code review (correctness axis).
#
# PREREQUISITE: Xavier server running on localhost:19010
#   $env:XAVIER_DEV_MODE="true"
#   xavier.exe http 19010

$BASE = "http://localhost:19010"
$AGENT = "xavier2-ceo"
$TASK = ""

Write-Host "🐶 DOGFOODING: ADR-006 Change Control Plane" -ForegroundColor Cyan
Write-Host "═══════════════════════════════════════════`n"

# ── Step 1: Create task ──────────────────────────────────────────
Write-Host "[1/5] POST /change/tasks — Create task" -ForegroundColor Yellow
$response = Invoke-RestMethod -Uri "$BASE/change/tasks" -Method Post `
  -ContentType "application/json" -Body (@{
    agent_id = $AGENT
    title    = "Fix deadlock in complete_task() — write+read lock on same RwLock"
    intent   = "ADR-006 code review found a tokio::sync::RwLock deadlock"
    scope    = @{
      allowed_write       = @("src/app/change_control_service.rs")
      read_only           = @("src/domain/change_control/", "src/ports/inbound/change_control_port.rs")
      blocked             = @("src/memory/qmd_memory.rs")
      contracts_affected  = @("ChangeControlPort::complete_task")
      layers_affected     = @("app")
    }
  } | ConvertTo-Json)
$TASK = $response.task_id
Write-Host "  ✅ Task created: $TASK"
Write-Host "     Status: $($response.status)`n"

# ── Step 2: Claim lease ──────────────────────────────────────────
Write-Host "[2/5] POST /change/leases/claim — Claim write lease" -ForegroundColor Yellow
$leaseResp = Invoke-RestMethod -Uri "$BASE/change/leases/claim" -Method Post `
  -ContentType "application/json" -Body (@{
    agent_id    = $AGENT
    task_id     = $TASK
    patterns    = @("src/app/change_control_service.rs")
    mode        = "write"
    ttl_seconds = 900
  } | ConvertTo-Json)
Write-Host "  ✅ Lease: $($leaseResp.lease_id)"
Write-Host "     Status: $($leaseResp.status)"
Write-Host "     Conflicts: $($leaseResp.conflicts.Count)"
Write-Host "     Memory context: $($leaseResp.memory_context -join ', ')"
Write-Host "     Required checks: $($leaseResp.required_checks -join ', ')"
Write-Host "     🧠 ADR references returned by search_decisions()`n"

# ── Step 3: Check conflicts ──────────────────────────────────────
Write-Host "[3/5] POST /change/conflicts/check — Verify no conflicts" -ForegroundColor Yellow
$conflicts = Invoke-RestMethod -Uri "$BASE/change/conflicts/check" -Method Post `
  -ContentType "application/json" -Body (@{ task_id = $TASK } | ConvertTo-Json)
if ($conflicts.Count -eq 0) {
    Write-Host "  ✅ No conflicts — safe to proceed`n"
} else {
    Write-Host "  ⚠️  Conflicts detected: $($conflicts.Count)`n"
}

# ── Step 4: Fix the bug (manual, done by agent) ──────────────────
Write-Host "[4/5] Apply fix to src/app/change_control_service.rs" -ForegroundColor Yellow
Write-Host "  🔧 Fix: Clone task under write lock in scoped block, then drop lock"
Write-Host "  Before:  tasks.write().await → ... → tasks.read().await  # DEADLOCK"
Write-Host "  After:   { let mut tasks = write().await; clone; }  // lock dropped"
Write-Host "           generate_summary(clone)  # safe, no lock held`n"

# ── Step 5: Complete task ────────────────────────────────────────
Write-Host "[5/5] POST /change/complete — Complete task" -ForegroundColor Yellow
$complete = Invoke-RestMethod -Uri "$BASE/change/complete" -Method Post `
  -ContentType "application/json" -Body (@{
    task_id = $TASK
    result  = @{
      commit  = "4eef6cd"
      message = "fix(change-control): deadlock in complete_task()"
      files   = @("src/app/change_control_service.rs")
      review  = "5-axis SWAL code review — correctness"
    }
  } | ConvertTo-Json)
Write-Host "  ✅ Task completed"
Write-Host "     Summary: $($complete.summary)`n"

# ── Bonus: Merge Plan ────────────────────────────────────────────
Write-Host "───────────────────────────────────────────────────" -ForegroundColor DarkGray
Write-Host "GET /change/merge-plan" -ForegroundColor Cyan
$plan = Invoke-RestMethod -Uri "$BASE/change/merge-plan"
Write-Host "  Safe parallel groups: $($plan.safe_parallel_groups.Count)"
Write-Host "  Sequential: $($plan.sequential.Count)"
Write-Host "  Blocked: $($plan.blocked.Count)"
if ($plan.blocked.Count -gt 0) {
    foreach ($b in $plan.blocked) {
        Write-Host "    🚫 $($b.task): $($b.reason)" -ForegroundColor Red
    }
}

# ── Release lease ────────────────────────────────────────────────
Write-Host "`nPOST /change/leases/release — Release lease" -ForegroundColor DarkGray
Invoke-RestMethod -Uri "$BASE/change/leases/release" -Method Post `
  -ContentType "application/json" -Body (@{ lease_id = $leaseResp.lease_id } | ConvertTo-Json) | Out-Null
Write-Host "  ✅ Lease released"

Write-Host "`n🐶 Dogfooding complete — ADR-006 Change Control Plane WORKS!" -ForegroundColor Green
