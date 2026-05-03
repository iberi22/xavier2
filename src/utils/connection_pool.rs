//! Simple SQLite connection pool
//!
//! A lightweight connection pool for SQLite. For SQLite, true pooling isn't needed
//! since it has limited concurrent write capacity. This provides a simple wrapper
//! around a shared connection.

use rusqlite::{Connection, Result as SqliteResult};
use std::sync::Mutex;
use std::time::Duration;

/// Configuration for the connection pool
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of connections in the pool
    pub max_size: usize,
    /// Connection timeout
    pub connection_timeout: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 4,
            connection_timeout: Duration::from_secs(30),
        }
    }
}

/// A simple connection pool wrapper for SQLite connections
pub struct ConnectionPool {
    // TODO: Dead code - remove or use pool configuration for connection lifecycle.
    #[allow(dead_code)]
    config: PoolConfig,
    connection: Connection,
}

impl ConnectionPool {
    /// Create a new connection pool with a single shared connection
    pub fn new(database_path: &str) -> SqliteResult<Self> {
        Self::with_config(database_path, PoolConfig::default())
    }

    /// Create a new connection pool with custom configuration
    pub fn with_config(database_path: &str, config: PoolConfig) -> SqliteResult<Self> {
        let conn = Connection::open(database_path)?;

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;

        Ok(Self {
            config,
            connection: conn,
        })
    }

    /// Get a reference to the connection
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    /// Execute a query that doesn't return results
    pub fn execute(&self, sql: &str) -> SqliteResult<()> {
        self.connection.execute_batch(sql)?;
        Ok(())
    }
}

/// Thread-safe connection pool using Mutex
pub type MutexPool = Mutex<ConnectionPool>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_creation() {
        let test_db = "file::memory:?cache=shared";
        let pool = ConnectionPool::new(test_db);
        assert!(pool.is_ok());
    }

    #[test]
    fn test_pool_execute() {
        let test_db = "file::memory:?cache=shared";
        let pool = ConnectionPool::new(test_db).unwrap();
        let result = pool.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)");
        assert!(result.is_ok());
    }

    #[test]
    fn test_pool_query() {
        let test_db = "file::memory:?cache=shared";
        let pool = ConnectionPool::new(test_db).unwrap();

        pool.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, name TEXT)")
            .unwrap();
        pool.execute("INSERT INTO test (name) VALUES ('test1')")
            .unwrap();

        let conn = pool.connection();
        let mut stmt = conn.prepare("SELECT id, name FROM test").unwrap();
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap();

        let results: Vec<(i32, String)> = rows.filter_map(|r| r.ok()).collect();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1, "test1");
    }
}
