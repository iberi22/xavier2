//! Authentication Module for Xavier2
//! JWT-based authentication and RBAC

use tokio::sync::RwLock;
use rand::rngs::OsRng;
use rand::RngCore;
use serde::{Deserialize, Serialize};

/// JWT Claims for authentication
#[derive(Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,    // User ID
    pub email: String,  // User email
    pub role: UserRole, // User role
    pub exp: u64,       // Expiration timestamp
    pub iat: u64,       // Issued at
}

impl Claims {
    pub fn new(user_id: String, email: String, role: UserRole, expires_in: u64) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("SystemTime::duration_since failed - clock is before UNIX epoch")
            .as_secs();

        Self {
            sub: user_id,
            email,
            role,
            exp: now + expires_in,
            iat: now,
        }
    }

    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("SystemTime::duration_since failed - clock is before UNIX epoch")
            .as_secs();
        now > self.exp
    }
}

/// User roles for RBAC
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    User,
    Readonly,
}

impl Default for UserRole {
    fn default() -> Self {
        UserRole::User
    }
}

/// User representation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: UserRole,
    pub api_key: String,
    pub created_at: u64,
    pub updated_at: u64,
}

impl User {
    pub fn new(email: String, name: String, role: UserRole) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("SystemTime::duration_since failed - clock is before UNIX epoch")
            .as_secs();

        Self {
            id: ulid::Ulid::new().to_string(),
            email,
            name,
            role,
            api_key: {
                let mut bytes = [0u8; 32];
                OsRng.fill_bytes(&mut bytes);
                hex::encode(bytes)
            },
            created_at: now,
            updated_at: now,
        }
    }
}

/// Login request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub refresh_token: String,
    pub user: User,
}

/// User store for managing users
pub struct UserStore {
    users: RwLock<Vec<User>>,
}

impl Default for UserStore {
    fn default() -> Self {
        Self::new()
    }
}

impl UserStore {
    pub fn new() -> Self {
        Self {
            users: RwLock::new(Vec::new()),
        }
    }

    pub async fn add_user(&self, user: User) {
        let mut users = self.users.write().await;
        users.push(user);
    }

    pub async fn get_user(&self, id: &str) -> Option<User> {
        let users = self.users.read().await;
        users.iter().find(|u| u.id == id).cloned()
    }

    pub async fn get_by_email(&self, email: &str) -> Option<User> {
        let users = self.users.read().await;
        users.iter().find(|u| u.email == email).cloned()
    }

    pub async fn list_users(&self) -> Vec<User> {
        let users = self.users.read().await;
        users.clone()
    }

    pub async fn delete_user(&self, id: &str) -> bool {
        let mut users = self.users.write().await;
        let len_before = users.len();
        users.retain(|u| u.id != id);
        users.len() < len_before
    }
}

/// Permission check
pub trait Permission {
    fn can_view_dashboard(&self) -> bool;
    fn can_search_memory(&self) -> bool;
    fn can_add_memory(&self) -> bool;
    fn can_delete_memory(&self) -> bool;
    fn can_manage_beliefs(&self) -> bool;
    fn can_run_agents(&self) -> bool;
    fn can_view_config(&self) -> bool;
    fn can_edit_config(&self) -> bool;
    fn can_manage_users(&self) -> bool;
}

impl Permission for UserRole {
    fn can_view_dashboard(&self) -> bool {
        true
    }
    fn can_search_memory(&self) -> bool {
        true
    }

    fn can_add_memory(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::User)
    }

    fn can_delete_memory(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::User)
    }

    fn can_manage_beliefs(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::User)
    }

    fn can_run_agents(&self) -> bool {
        matches!(self, UserRole::Admin | UserRole::User)
    }

    fn can_view_config(&self) -> bool {
        true
    }

    fn can_edit_config(&self) -> bool {
        matches!(self, UserRole::Admin)
    }

    fn can_manage_users(&self) -> bool {
        matches!(self, UserRole::Admin)
    }
}

/// Rate limiter for API protection
#[derive(Debug)]
pub struct RateLimiter {
    requests: RwLock<std::collections::HashMap<String, Vec<u64>>>,
    max_requests: usize,
    window_seconds: u64,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(1000, 60) // 1000 requests per minute
    }
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_seconds: u64) -> Self {
        Self {
            requests: RwLock::new(std::collections::HashMap::new()),
            max_requests,
            window_seconds,
        }
    }

    pub async fn check(&self, key: &str) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("SystemTime::duration_since failed - clock is before UNIX epoch")
            .as_secs();

        let mut requests = self.requests.write().await;

        // Clean old entries
        let window_start = now - self.window_seconds;
        requests.entry(key.to_string()).and_modify(|times| {
            times.retain(|&t| t > window_start);
        });

        // Check limit
        let count = requests.get(key).map(|v| v.len()).unwrap_or(0);

        if count >= self.max_requests {
            return false;
        }

        // Add current request
        requests
            .entry(key.to_string())
            .or_insert_with(Vec::new)
            .push(now);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_role_permissions() {
        let admin = UserRole::Admin;
        let user = UserRole::User;
        let readonly = UserRole::Readonly;

        assert!(admin.can_manage_users());
        assert!(!user.can_manage_users());
        assert!(!readonly.can_manage_users());

        assert!(admin.can_add_memory());
        assert!(user.can_add_memory());
        assert!(!readonly.can_add_memory());
    }

    #[tokio::test]
    async fn test_user_store() {
        let store = UserStore::new();

        let user = User::new(
            "test@example.com".to_string(),
            "Test User".to_string(),
            UserRole::User,
        );

        store.add_user(user.clone()).await;

        let found = store.get_by_email("test@example.com").await;
        assert!(found.is_some());
    }
}
