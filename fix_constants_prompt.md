You are at E:\scripts-python\xavier2 on branch fix/issue-115.

Apply these fixes. ONLY modify the files listed below. Do NOT touch any other file.

## Fix 1: src/retrieval/gating.rs
Change line 47: replace hardcoded `0.001` with `config::WEIGHT_SUM_TOLERANCE`
So `(sum - 1.0).abs() < 0.001` becomes `(sum - 1.0).abs() < config::WEIGHT_SUM_TOLERANCE`

## Fix 2: src/memory/layers_config.rs
Change `from_env()` in `WorkingMemoryLayerConfig` to use `Self::default()` as base instead of hardcoding default values again:
```rust
impl WorkingMemoryLayerConfig {
    pub fn from_env() -> Self {
        let default = Self::default();
        Self {
            capacity: std::env::var("XAVIER2_WORKING_MEMORY_CAPACITY")
                .ok().and_then(|v| v.parse().ok()).unwrap_or(default.capacity),
            lru_exempt_access_threshold: std::env::var("XAVIER2_WORKING_LRU_THRESHOLD")
                .ok().and_then(|v| v.parse().ok()).unwrap_or(default.lru_exempt_access_threshold),
            bm25_k1: std::env::var("XAVIER2_WORKING_BM25_K1")
                .ok().and_then(|v| v.parse().ok()).unwrap_or(default.bm25_k1),
            bm25_b: std::env::var("XAVIER2_WORKING_BM25_B")
                .ok().and_then(|v| v.parse().ok()).unwrap_or(default.bm25_b),
        }
    }
}
```
Do the same for `EpisodicLayerConfig::from_env()`:
```rust
impl EpisodicLayerConfig {
    pub fn from_env() -> Self {
        let default = Self::default();
        Self {
            summary_window: std::env::var("XAVIER2_EPISODIC_SUMMARY_WINDOW")
                .ok().and_then(|v| v.parse().ok()).unwrap_or(default.summary_window),
            max_sessions: std::env::var("XAVIER2_MAX_EPISODIC_SESSIONS")
                .ok().and_then(|v| v.parse().ok()).unwrap_or(default.max_sessions),
            min_event_importance: std::env::var("XAVIER2_EPISODIC_MIN_EVENT_IMPORTANCE")
                .ok().and_then(|v| v.parse().ok()).unwrap_or(default.min_event_importance),
        }
    }
}
```
Also add doc comments for the missing env vars in the module-level doc comment at the top of the file.

## Fix 3: src/context/executor.rs
Line 20: `Ok(Err(e))` should already be `Ok(Err(_e))` — if not, fix it.

After making ALL changes, run:
cargo build --lib 2>&1 | tail -5
cargo test --lib tools_list_returns_all_xavier2_tools 2>&1 | tail -5
cargo test --lib test_default_config test_working_layer_config_from_env test_episodic_layer_config_from_env test_full_layers_config_from_env 2>&1 | tail -5

Do NOT modify any test files, Cargo.toml, or any other files outside the 3 listed.
