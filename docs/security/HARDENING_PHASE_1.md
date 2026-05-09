# Phase 1 Security Hardening - Root Cause Analysis and Fixes

This document details the critical security fixes applied in Phase 1 to address vulnerabilities and harden the Xavier system.

## 1. Hardcoded Credentials Removal
**File:** `src/tools/kanban.rs`
**Issue:** The `PlankaConfig` struct contained hardcoded default credentials (`swaladmin2026`).
**Fix:** Removed the hardcoded fallback. The system now strictly requires `PLANKA_URL`, `PLANKA_EMAIL`, and `PLANKA_PASSWORD` environment variables to be set.
**Regression Prevention:** Added tests to ensure `PlankaConfig::from_env()` returns `None` if any required environment variable is missing, and never falls back to hardcoded secrets.

## 2. Token Enforcement in CLI and Workspace
**Files:** `src/cli.rs`, `src/main_tui.rs`, `src/workspace.rs`
**Issue:** Several modules used a default `dev-token` when the required security tokens (`XAVIER_TOKEN` or `X-CORTEX-TOKEN`) were missing from the environment.
**Fix:** Removed `unwrap_or_else(|_| "dev-token".to_string())` and replaced it with `.expect()` or proper error handling to enforce token presence.
**Regression Prevention:** Integration tests verify that requests without valid tokens are rejected and that the CLI fails gracefully with clear error messages when tokens are missing.

## 3. Redaction of Sensitive Data in Debug Logs
**Files:** `src/embedding/openai.rs`, `src/security/auth.rs`
**Issue:** Sensitive structs like `OpenAIEmbedder`, `User`, `Claims`, and `LoginRequest` used `#[derive(Debug)]`, which could leak API keys, passwords, and tokens into logs.
**Fix:** Implemented manual `fmt::Debug` for these structs to redact sensitive fields (e.g., `api_key: "<redacted>"`).
**Regression Prevention:** Unit tests verify that the `{:?}` output for these structs does not contain the actual sensitive values.

## 4. Enhanced Prompt Injection Detection
**File:** `src/security/prompt_guard.rs`
**Issue:** The `PromptInjectionDetector` was missing detection for zero-width character bypasses and various template/HTML injection patterns.
**Fix:**
- Added detection for zero-width characters (U+200B, U+200C, etc.) used to bypass keyword filters.
- Added regex patterns for template injections (`{{...}}`, `${...}`) and HTML/JavaScript injections (`<script>`, `onerror=`).
- Improved sanitization to filter out these new patterns.
**Regression Prevention:** Added comprehensive test cases covering zero-width bypasses, template injections, and XSS-like patterns in the `PromptInjectionDetector` tests.
