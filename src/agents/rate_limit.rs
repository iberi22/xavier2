use std::sync::Arc;
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use anyhow::Result;
use chrono::{DateTime, Utc, Duration};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaStatus {
    pub provider: String,
    pub used_today: usize,
    pub used_weekly: usize,
    pub used_monthly: usize,
    pub weekly_quota: usize,
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

    pub fn init_schema(conn: &Connection) -> Result<()> {
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

        // Migration: Add cost_usd column if it doesn't exist
        let column_exists: bool = conn.query_row(
            "SELECT count(*) FROM pragma_table_info('rate_limit_usage') WHERE name='cost_usd'",
            [],
            |row| row.get(0),
        ).unwrap_or(0) > 0;

        if !column_exists {
            conn.execute("ALTER TABLE rate_limit_usage ADD COLUMN cost_usd REAL DEFAULT 0.0", [])?;
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
            let _ = conn.execute("ALTER TABLE provider_quotas ADD COLUMN weekly_quota INTEGER DEFAULT 1000000", []);
        }

        Ok(())
    }

    pub async fn track_request(
        &self,
        provider: &str,
        tokens: usize,
        status: u16,
        cost_usd: f64,
    ) -> Result<()> {
        let conn = self.db.lock();
        conn.execute(
            "INSERT INTO rate_limit_usage (provider, tokens_used, cost_usd, status_code, is_error)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![provider, tokens as i64, cost_usd, status, status >= 400],
        )?;

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
        let day_ago = now - Duration::days(1);
        let week_ago = now - Duration::days(7);
        let month_ago = now - Duration::days(30);

        let used_today: i64 = conn.query_row(
            "SELECT COALESCE(SUM(tokens_used), 0) FROM rate_limit_usage WHERE provider = ?1 AND timestamp > ?2",
            params![provider, day_ago],
            |row| row.get(0),
        )?;

        let used_weekly: i64 = conn.query_row(
            "SELECT COALESCE(SUM(tokens_used), 0) FROM rate_limit_usage WHERE provider = ?1 AND timestamp > ?2",
            params![provider, week_ago],
            |row| row.get(0),
        )?;

        let used_monthly: i64 = conn.query_row(
            "SELECT COALESCE(SUM(tokens_used), 0) FROM rate_limit_usage WHERE provider = ?1 AND timestamp > ?2",
            params![provider, month_ago],
            |row| row.get(0),
        )?;

        let (rate_limited_until, weekly_quota): (Option<DateTime<Utc>>, usize) = conn.query_row(
            "SELECT rate_limited_until, COALESCE(weekly_quota, 1000000) FROM provider_quotas WHERE provider = ?1",
            params![provider],
            |row| Ok((row.get(0)?, row.get::<_, i64>(1)? as usize)),
        ).optional()?.unwrap_or((None, 1000000));

        Ok(QuotaStatus {
            provider: provider.to_string(),
            used_today: used_today as usize,
            used_weekly: used_weekly as usize,
            used_monthly: used_monthly as usize,
            weekly_quota,
            rate_limited_until,
            last_update: now,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use rusqlite::Connection;
    use std::sync::Arc;

    async fn setup_manager() -> RateLimitManager {
        let conn = Connection::open_in_memory().unwrap();
        RateLimitManager::init_schema(&conn).unwrap();
        RateLimitManager::new(Arc::new(Mutex::new(conn)))
    }

    #[tokio::test]
    async fn test_is_quota_low() {
        let manager = setup_manager().await;
        let provider = "test-provider";

        // Initial state: not low (uses default 1,000,000 quota)
        assert!(!manager.is_quota_low(provider).await.unwrap());

        // Use 91% of quota
        manager.track_request(provider, 910_000, 200).await.unwrap();
        assert!(manager.is_quota_low(provider).await.unwrap());

        // Use 89% of quota
        let manager = setup_manager().await;
        manager.track_request(provider, 890_000, 200).await.unwrap();
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
            ).unwrap();
        }

        // Use 950 tokens (95%)
        manager.track_request(provider, 950, 200).await.unwrap();
        assert!(manager.is_quota_low(provider).await.unwrap());

        // Use 850 tokens (85%)
        let manager = setup_manager().await;
        {
            let conn = manager.db.lock();
            conn.execute(
                "INSERT INTO provider_quotas (provider, weekly_quota) VALUES (?1, ?2)",
                params![provider, 1000],
            ).unwrap();
        }
        manager.track_request(provider, 850, 200).await.unwrap();
        assert!(!manager.is_quota_low(provider).await.unwrap());
    }
}
