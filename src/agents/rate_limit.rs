use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::ports::outbound::schema_init::SchemaInitializer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaStatus {
    pub provider: String,
    pub used_hourly: usize,
    pub used_today: usize,
    pub used_weekly: usize,
    pub used_monthly: usize,
    pub weekly_quota: usize,
    pub cache_hits: usize,
    pub rate_limited_until: Option<DateTime<Utc>>,
    pub last_update: DateTime<Utc>,
}

pub struct RateLimitManager {
    db: Arc<Mutex<Connection>>,
}

impl RateLimitManager {
    pub fn new(db: Arc<Mutex<Connection>>) -> Self {
        Self { db }
    }

    pub async fn track_request(
        &self,
        provider: &str,
        tokens: usize,
        status: u16,
        cost_usd: f64,
        is_cache_hit: bool,
    ) -> Result<()> {
        let conn = self.db.lock();
        conn.execute(
            "INSERT INTO rate_limit_usage (provider, tokens_used, cost_usd, status_code, is_error)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![provider, tokens as i64, cost_usd, status, status >= 400],
        )?;

        if is_cache_hit {
            conn.execute(
                "UPDATE rate_limit_usage SET cache_hits = cache_hits + 1 WHERE id = last_insert_rowid()",
                [],
            )?;
        }

        if status == 429 {
            // Default cooldown of 5 minutes if not specified
            let until = Utc::now() + Duration::minutes(5);
            conn.execute(
                "INSERT INTO provider_quotas (provider, rate_limited_until)
                 VALUES (?1, ?2)
                 ON CONFLICT(provider) DO UPDATE SET rate_limited_until = ?2",
                params![provider, until],
            )?;
        }

        Ok(())
    }

    pub async fn get_status(&self, provider: &str) -> Result<QuotaStatus> {
        let conn = self.db.lock();

        let now = Utc::now();
        let hour_ago = now - Duration::hours(1);
        let day_ago = now - Duration::days(1);
        let week_ago = now - Duration::days(7);
        let month_ago = now - Duration::days(30);

        let (used_hourly, used_today, used_weekly, used_monthly, cache_hits): (
            i64,
            i64,
            i64,
            i64,
            i64,
        ) = conn.query_row(
            "SELECT
                COALESCE(SUM(CASE WHEN timestamp > ?1 THEN tokens_used ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN timestamp > ?2 THEN tokens_used ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN timestamp > ?3 THEN tokens_used ELSE 0 END), 0),
                COALESCE(SUM(CASE WHEN timestamp > ?4 THEN tokens_used ELSE 0 END), 0),
                COALESCE(SUM(cache_hits), 0)
             FROM rate_limit_usage
             WHERE provider = ?5",
            params![hour_ago, day_ago, week_ago, month_ago, provider],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )?;

        let (rate_limited_until, weekly_quota): (Option<DateTime<Utc>>, usize) = conn.query_row(
            "SELECT rate_limited_until, COALESCE(weekly_quota, 1000000) FROM provider_quotas WHERE provider = ?1",
            params![provider],
            |row| Ok((row.get(0)?, row.get::<_, i64>(1)? as usize)),
        ).optional()?.unwrap_or((None, 1000000));

        Ok(QuotaStatus {
            provider: provider.to_string(),
            used_hourly: used_hourly as usize,
            used_today: used_today as usize,
            used_weekly: used_weekly as usize,
            used_monthly: used_monthly as usize,
            weekly_quota,
            cache_hits: cache_hits as usize,
            rate_limited_until,
            last_update: now,
        })
    }

    pub async fn get_daily_summary(&self, provider: &str) -> Result<serde_json::Value> {
        let now = Utc::now();
        let day_ago = now - Duration::days(1);

        let (requests, daily_total, daily_tokens) = {
            let conn = self.db.lock();

            let mut stmt = conn.prepare(
                "SELECT timestamp, tokens_used, status_code FROM rate_limit_usage
                 WHERE provider = ?1 AND timestamp > ?2
                 ORDER BY timestamp ASC",
            )?;

            let requests: Vec<serde_json::Value> = stmt
                .query_map(params![provider, day_ago], |row| {
                    Ok(serde_json::json!({
                        "ts": row.get::<_, DateTime<Utc>>(0)?,
                        "tokens": row.get::<_, i64>(1)?,
                        "status": row.get::<_, u16>(2)?,
                    }))
                })?
                .filter_map(|r| r.ok())
                .collect();

            let total = requests.len();
            let tokens: i64 = requests
                .iter()
                .map(|r| r["tokens"].as_i64().unwrap_or(0))
                .sum();

            (requests, total, tokens)
        }; // lock dropped here before .await

        let status = self.get_status(provider).await?;

        Ok(serde_json::json!({
            "requests": requests,
            "daily_total": daily_total,
            "daily_tokens": daily_tokens,
            "rate_limited": status.rate_limited_until.is_some_and(|until| until > now),
            "cooldown_until": status.rate_limited_until,
        }))
    }

    pub async fn is_quota_low(&self, provider: &str) -> Result<bool> {
        let status = self.get_status(provider).await?;
        if status.weekly_quota == 0 {
            return Ok(false);
        }

        let used_ratio = status.used_weekly as f32 / status.weekly_quota as f32;
        // Quota is low if more than 90% is used (less than 10% remaining)
        Ok(used_ratio > 0.9)
    }

    pub async fn report_429(&self, provider: &str, cooldown_minutes: i64) -> Result<()> {
        let conn = self.db.lock();
        let until = Utc::now() + Duration::minutes(cooldown_minutes);
        conn.execute(
            "INSERT INTO provider_quotas (provider, rate_limited_until)
             VALUES (?1, ?2)
             ON CONFLICT(provider) DO UPDATE SET rate_limited_until = ?2",
            params![provider, until],
        )?;
        Ok(())
    }

    pub async fn get_all_providers(&self) -> Result<Vec<String>> {
        let conn = self.db.lock();
        let mut stmt = conn.prepare("SELECT DISTINCT provider FROM rate_limit_usage")?;
        let providers = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(providers)
    }

    pub async fn update_manual_limit(&self, provider: &str, percentage: f32) -> Result<()> {
        let conn = self.db.lock();
        conn.execute(
            "INSERT INTO provider_quotas (provider, manual_limit_percentage, last_manual_update)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(provider) DO UPDATE SET manual_limit_percentage = ?2, last_manual_update = ?3",
            params![provider, percentage, Utc::now()],
        )?;
        Ok(())
    }
}

impl SchemaInitializer for RateLimitManager {
    fn init_schema(&self) -> Result<()> {
        let conn = self.db.lock();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS rate_limit_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider TEXT NOT NULL,
                timestamp DATETIME DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
                tokens_used INTEGER DEFAULT 0,
                cost_usd REAL DEFAULT 0.0,
                status_code INTEGER,
                is_error BOOLEAN DEFAULT 0
            )",
            [],
        )?;

        // Migration: Add cache_hits column if it doesn't exist
        let has_cache_hits: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('rate_limit_usage') WHERE name='cache_hits'",
            [],
            |row| row.get(0),
        ).unwrap_or(0) > 0;

        if !has_cache_hits {
            conn.execute(
                "ALTER TABLE rate_limit_usage ADD COLUMN cache_hits INTEGER DEFAULT 0",
                [],
            )?;
        }

        // Migration: Add cost_usd column if it doesn't exist
        let column_exists: bool = conn
            .query_row(
                "SELECT count(*) FROM pragma_table_info('rate_limit_usage') WHERE name='cost_usd'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0)
            > 0;

        if !column_exists {
            conn.execute(
                "ALTER TABLE rate_limit_usage ADD COLUMN cost_usd REAL DEFAULT 0.0",
                [],
            )?;
        }

        conn.execute(
            "CREATE TABLE IF NOT EXISTS provider_quotas (
                provider TEXT PRIMARY KEY,
                rate_limited_until DATETIME,
                manual_limit_percentage REAL DEFAULT 0.0,
                last_manual_update DATETIME,
                weekly_quota INTEGER DEFAULT 1000000
            )",
            [],
        )?;

        // Defensive schema evolution: add weekly_quota if it doesn't exist
        let has_weekly_quota: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('provider_quotas') WHERE name = 'weekly_quota'",
            [],
            |row| row.get::<_, i32>(0),
        ).map(|count| count > 0).unwrap_or(false);

        if !has_weekly_quota {
            let _ = conn.execute(
                "ALTER TABLE provider_quotas ADD COLUMN weekly_quota INTEGER DEFAULT 1000000",
                [],
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use rusqlite::Connection;
    use std::sync::Arc;

    async fn setup_manager() -> RateLimitManager {
        let conn = Connection::open_in_memory().unwrap();
        let manager = RateLimitManager::new(Arc::new(Mutex::new(conn)));
        manager.init_schema().unwrap();
        manager
    }

    #[tokio::test]
    async fn test_is_quota_low() {
        let manager = setup_manager().await;
        let provider = "test-provider";

        // Initial state: not low (uses default 1,000,000 quota)
        assert!(!manager.is_quota_low(provider).await.unwrap());

        // Use 91% of quota
        manager
            .track_request(provider, 910_000, 200, 0.0, false)
            .await
            .unwrap();
        assert!(manager.is_quota_low(provider).await.unwrap());

        // Use 89% of quota
        let manager = setup_manager().await;
        manager
            .track_request(provider, 890_000, 200, 0.0, false)
            .await
            .unwrap();
        assert!(!manager.is_quota_low(provider).await.unwrap());
    }

    #[tokio::test]
    async fn test_custom_weekly_quota() {
        let manager = setup_manager().await;
        let provider = "test-provider";

        {
            let conn = manager.db.lock();
            conn.execute(
                "INSERT INTO provider_quotas (provider, weekly_quota) VALUES (?1, ?2)",
                params![provider, 1000],
            )
            .unwrap();
        }

        // Use 950 tokens (95%)
        manager
            .track_request(provider, 950, 200, 0.0, false)
            .await
            .unwrap();
        assert!(manager.is_quota_low(provider).await.unwrap());

        // Use 850 tokens (85%)
        let manager = setup_manager().await;
        {
            let conn = manager.db.lock();
            conn.execute(
                "INSERT INTO provider_quotas (provider, weekly_quota) VALUES (?1, ?2)",
                params![provider, 1000],
            )
            .unwrap();
        }
        manager
            .track_request(provider, 850, 200, 0.0, false)
            .await
            .unwrap();
        assert!(!manager.is_quota_low(provider).await.unwrap());
    }

    #[tokio::test]
    async fn test_daily_summary_and_hourly_usage() {
        let manager = setup_manager().await;
        let provider = "test-provider";

        manager
            .track_request(provider, 100, 200, 0.01, false)
            .await
            .unwrap();
        manager
            .track_request(provider, 200, 200, 0.02, false)
            .await
            .unwrap();

        let status = manager.get_status(provider).await.unwrap();
        assert_eq!(status.used_hourly, 300);
        assert_eq!(status.used_today, 300);

        let summary = manager.get_daily_summary(provider).await.unwrap();
        assert_eq!(summary["daily_total"], 2);
        assert_eq!(summary["daily_tokens"], 300);
        assert!(summary["requests"].as_array().unwrap().len() == 2);
    }
}
