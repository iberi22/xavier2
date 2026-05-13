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

    pub fn init_schema(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS rate_limit_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                provider TEXT NOT NULL,
                timestamp DATETIME DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
                tokens_used INTEGER DEFAULT 0,
                status_code INTEGER,
                is_error BOOLEAN DEFAULT 0,
                cache_hits INTEGER DEFAULT 0
            )",
            [],
        )?;

        // Migration: Add cache_hits column if missing
        let mut stmt = conn.prepare("PRAGMA table_info(rate_limit_usage)")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
        let mut has_cache_hits = false;
        for col in rows {
            if col? == "cache_hits" {
                has_cache_hits = true;
                break;
            }
        }
        if !has_cache_hits {
            conn.execute("ALTER TABLE rate_limit_usage ADD COLUMN cache_hits INTEGER DEFAULT 0", [])?;
        }

        conn.execute(
            "CREATE TABLE IF NOT EXISTS provider_quotas (
                provider TEXT PRIMARY KEY,
                rate_limited_until DATETIME,
                manual_limit_percentage REAL DEFAULT 0.0,
                last_manual_update DATETIME
            )",
            [],
        )?;

        Ok(())
    }

    pub async fn track_request(&self, provider: &str, tokens: usize, status: u16, is_cache_hit: bool) -> Result<()> {
        let conn = self.db.lock();
        conn.execute(
            "INSERT INTO rate_limit_usage (provider, tokens_used, status_code, is_error, cache_hits)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![provider, tokens as i64, status, status >= 400, if is_cache_hit { 1 } else { 0 }],
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

        let cache_hits: i64 = conn.query_row(
            "SELECT COALESCE(SUM(cache_hits), 0) FROM rate_limit_usage WHERE provider = ?1",
            params![provider],
            |row| row.get(0),
        )?;

        let rate_limited_until: Option<DateTime<Utc>> = conn.query_row(
            "SELECT rate_limited_until FROM provider_quotas WHERE provider = ?1",
            params![provider],
            |row| row.get(0),
        ).optional()?;

        Ok(QuotaStatus {
            provider: provider.to_string(),
            used_today: used_today as usize,
            used_weekly: used_weekly as usize,
            used_monthly: used_monthly as usize,
            cache_hits: cache_hits as usize,
            rate_limited_until,
            last_update: now,
        })
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
    use std::sync::Arc;
    use parking_lot::Mutex;

    #[tokio::test]
    async fn test_rate_limit_cache_hits() -> Result<()> {
        let conn = Connection::open_in_memory()?;
        RateLimitManager::init_schema(&conn)?;
        let manager = RateLimitManager::new(Arc::new(Mutex::new(conn)));

        manager.track_request("test-provider", 100, 200, false).await?;
        manager.track_request("test-provider", 100, 200, true).await?;
        manager.track_request("test-provider", 100, 200, true).await?;

        let status = manager.get_status("test-provider").await?;
        assert_eq!(status.cache_hits, 2);
        assert_eq!(status.used_today, 300);

        Ok(())
    }
}
