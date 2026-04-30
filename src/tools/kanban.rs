//! Kanban Integration - Direct native integration with Planka API
//! No wrappers - direct HTTP calls to Planka REST API

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Planka API configuration
#[derive(Clone)]
pub struct PlankaConfig {
    pub base_url: String,
    pub email: String,
    pub password: String,
}

impl std::fmt::Debug for PlankaConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlankaConfig")
            .field("base_url", &self.base_url)
            .field("email", &self.email)
            .field("password", &"<redacted>")
            .finish()
    }
}

impl PlankaConfig {
    pub fn from_env() -> Option<Self> {
        Some(Self {
            base_url: std::env::var("PLANKA_URL").ok()?,
            email: std::env::var("PLANKA_EMAIL").ok()?,
            password: std::env::var("PLANKA_PASSWORD").ok()?,
        })
    }
}

/// Planka Project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Planka Board
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub id: String,
    pub name: String,
    pub project_id: String,
}

/// Planka List (Column)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    pub id: String,
    pub name: String,
    pub board_id: String,
    pub position: i32,
}

/// Planka Card (Task)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub list_id: String,
    pub position: i32,
}

/// Planka Label
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: String,
    pub name: String,
    pub color: String,
}

/// API Response wrapper
#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    #[serde(rename = "item")]
    pub item: Option<T>,
    #[serde(rename = "items")]
    pub items: Option<Vec<T>>,
}

/// Login response
#[derive(Debug, Deserialize)]
struct LoginResponse {
    #[serde(rename = "item")]
    pub item: TokenItem,
}

#[derive(Debug, Deserialize)]
struct TokenItem {
    #[serde(rename = "token")]
    pub token: String,
}

/// Native Planka Client - Direct API integration
#[derive(Clone)]
pub struct PlankaClient {
    client: Client,
    config: PlankaConfig,
    token: Arc<RwLock<Option<String>>>,
}

impl PlankaClient {
    /// Create a new Planka client
    pub fn new(config: PlankaConfig) -> Self {
        Self {
            client: Client::new(),
            config,
            token: Arc::new(RwLock::new(None)),
        }
    }

    /// Create with environment config
    pub fn from_env() -> Self {
        Self::new(PlankaConfig::from_env().expect(
            "PLANKA_URL, PLANKA_EMAIL, PLANKA_PASSWORD must be set"
        ))
    }

    /// Login and get access token
    pub async fn login(&self) -> Result<String, String> {
        let response = self
            .client
            .post(format!("{}/api/access-tokens", self.config.base_url))
            .json(&serde_json::json!({
                "emailOrUsername": self.config.email,
                "password": self.config.password
            }))
            .send()
            .await
            .map_err(|e| format!("Login request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Login failed: {}", response.status()));
        }

        let login_resp: LoginResponse = response
            .json()
            .await
            .map_err(|e| format!("Parse login response: {}", e))?;

        let token = login_resp.item.token;
        info!("[OK] Logged in to Planka");

        // Store token
        *self.token.write().await = Some(token.clone());

        Ok(token)
    }

    /// Ensure we're logged in
    async fn ensure_auth(&self) -> Result<String, String> {
        {
            let token = self.token.read().await;
            if token.is_some() {
                return Ok(token.as_ref().unwrap().clone());
            }
        }
        // Need to login
        self.login().await
    }

    // ============ PROJECTS ============

    /// List all projects
    pub async fn list_projects(&self) -> Result<Vec<Project>, String> {
        let auth = self.ensure_auth().await?;

        let response = self
            .client
            .get(format!("{}/api/projects", self.config.base_url))
            .header("Authorization", auth)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("List projects failed: {}", response.status()));
        }

        let api_resp: ApiResponse<Project> = response
            .json()
            .await
            .map_err(|e| format!("Parse response: {}", e))?;

        Ok(api_resp.items.unwrap_or_default())
    }

    /// Create a new project
    pub async fn create_project(&self, name: &str, description: &str) -> Result<Project, String> {
        let auth = self.ensure_auth().await?;

        let response = self
            .client
            .post(format!("{}/api/projects", self.config.base_url))
            .header("Authorization", auth)
            .json(&serde_json::json!({
                "name": name,
                "description": description,
                "type": "private"
            }))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Create project failed: {}", response.status()));
        }

        let api_resp: ApiResponse<Project> = response
            .json()
            .await
            .map_err(|e| format!("Parse response: {}", e))?;

        api_resp
            .item
            .ok_or_else(|| "No project in response".to_string())
    }

    // ============ BOARDS ============

    /// List boards in a project
    pub async fn list_boards(&self, project_id: &str) -> Result<Vec<Board>, String> {
        let auth = self.ensure_auth().await?;

        let response = self
            .client
            .get(format!(
                "{}/api/projects/{}/boards",
                self.config.base_url, project_id
            ))
            .header("Authorization", auth)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("List boards failed: {}", response.status()));
        }

        let api_resp: ApiResponse<Board> = response
            .json()
            .await
            .map_err(|e| format!("Parse response: {}", e))?;

        Ok(api_resp.items.unwrap_or_default())
    }

    /// Create a board in a project
    pub async fn create_board(&self, project_id: &str, name: &str) -> Result<Board, String> {
        let auth = self.ensure_auth().await?;

        let response = self
            .client
            .post(format!(
                "{}/api/projects/{}/boards",
                self.config.base_url, project_id
            ))
            .header("Authorization", auth)
            .json(&serde_json::json!({
                "name": name,
                "position": 0
            }))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Create board failed: {}", response.status()));
        }

        let api_resp: ApiResponse<Board> = response
            .json()
            .await
            .map_err(|e| format!("Parse response: {}", e))?;

        api_resp
            .item
            .ok_or_else(|| "No board in response".to_string())
    }

    // ============ LISTS (COLUMNS) ============

    /// List lists in a board
    pub async fn list_lists(&self, board_id: &str) -> Result<Vec<List>, String> {
        let auth = self.ensure_auth().await?;

        let response = self
            .client
            .get(format!(
                "{}/api/boards/{}/lists",
                self.config.base_url, board_id
            ))
            .header("Authorization", auth)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("List lists failed: {}", response.status()));
        }

        let api_resp: ApiResponse<List> = response
            .json()
            .await
            .map_err(|e| format!("Parse response: {}", e))?;

        Ok(api_resp.items.unwrap_or_default())
    }

    /// Create a list (column) in a board
    pub async fn create_list(
        &self,
        board_id: &str,
        name: &str,
        position: i32,
    ) -> Result<List, String> {
        let auth = self.ensure_auth().await?;

        let response = self
            .client
            .post(format!(
                "{}/api/boards/{}/lists",
                self.config.base_url, board_id
            ))
            .header("Authorization", auth)
            .json(&serde_json::json!({
                "name": name,
                "position": position
            }))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Create list failed: {}", response.status()));
        }

        let api_resp: ApiResponse<List> = response
            .json()
            .await
            .map_err(|e| format!("Parse response: {}", e))?;

        api_resp
            .item
            .ok_or_else(|| "No list in response".to_string())
    }

    // ============ CARDS (TASKS) ============

    /// List cards in a list
    pub async fn list_cards(&self, list_id: &str) -> Result<Vec<Card>, String> {
        let auth = self.ensure_auth().await?;

        let response = self
            .client
            .get(format!(
                "{}/api/lists/{}/cards",
                self.config.base_url, list_id
            ))
            .header("Authorization", auth)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("List cards failed: {}", response.status()));
        }

        let api_resp: ApiResponse<Card> = response
            .json()
            .await
            .map_err(|e| format!("Parse response: {}", e))?;

        Ok(api_resp.items.unwrap_or_default())
    }

    /// Create a card (task)
    pub async fn create_card(
        &self,
        list_id: &str,
        name: &str,
        description: &str,
    ) -> Result<Card, String> {
        let auth = self.ensure_auth().await?;

        let response = self
            .client
            .post(format!("{}/api/cards", self.config.base_url))
            .header("Authorization", auth)
            .json(&serde_json::json!({
                "listId": list_id,
                "name": name,
                "description": description,
                "position": 0
            }))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Create card failed: {} - {}",
                response.status(),
                response.text().await.unwrap_or_default()
            ));
        }

        let api_resp: ApiResponse<Card> = response
            .json()
            .await
            .map_err(|e| format!("Parse response: {}", e))?;

        api_resp
            .item
            .ok_or_else(|| "No card in response".to_string())
    }

    /// Move a card to another list
    pub async fn move_card(
        &self,
        card_id: &str,
        list_id: &str,
        position: i32,
    ) -> Result<Card, String> {
        let auth = self.ensure_auth().await?;

        let response = self
            .client
            .patch(format!("{}/api/cards/{}", self.config.base_url, card_id))
            .header("Authorization", auth)
            .json(&serde_json::json!({
                "listId": list_id,
                "position": position
            }))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Move card failed: {}", response.status()));
        }

        let api_resp: ApiResponse<Card> = response
            .json()
            .await
            .map_err(|e| format!("Parse response: {}", e))?;

        api_resp
            .item
            .ok_or_else(|| "No card in response".to_string())
    }

    /// Delete a card
    pub async fn delete_card(&self, card_id: &str) -> Result<(), String> {
        let auth = self.ensure_auth().await?;

        let response = self
            .client
            .delete(format!("{}/api/cards/{}", self.config.base_url, card_id))
            .header("Authorization", auth)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Delete card failed: {}", response.status()));
        }

        Ok(())
    }

    // ============ LABELS ============

    /// Create a label in a board
    pub async fn create_label(
        &self,
        board_id: &str,
        name: &str,
        color: &str,
    ) -> Result<Label, String> {
        let auth = self.ensure_auth().await?;

        let response = self
            .client
            .post(format!(
                "{}/api/boards/{}/labels",
                self.config.base_url, board_id
            ))
            .header("Authorization", auth)
            .json(&serde_json::json!({
                "name": name,
                "color": color
            }))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Create label failed: {}", response.status()));
        }

        let api_resp: ApiResponse<Label> = response
            .json()
            .await
            .map_err(|e| format!("Parse response: {}", e))?;

        api_resp
            .item
            .ok_or_else(|| "No label in response".to_string())
    }

    // ============ HIGH-LEVEL OPERATIONS ============

    /// Quick create task - finds board and list automatically
    pub async fn quick_create_task(
        &self,
        project_name: &str,
        list_name: &str,
        task_name: &str,
        description: &str,
    ) -> Result<Card, String> {
        // Find project
        let projects = self.list_projects().await?;
        let project = projects
            .iter()
            .find(|p| p.name.to_lowercase() == project_name.to_lowercase())
            .ok_or_else(|| format!("Project '{}' not found", project_name))?;

        // Find board
        let boards = self.list_boards(&project.id).await?;
        let board = boards
            .first()
            .ok_or_else(|| "No boards in project".to_string())?;

        // Find list
        let lists = self.list_lists(&board.id).await?;
        let list = lists
            .iter()
            .find(|l| l.name.to_lowercase().contains(&list_name.to_lowercase()))
            .ok_or_else(|| format!("List '{}' not found", list_name))?;

        // Create card
        self.create_card(&list.id, task_name, description).await
    }

    /// Get full board with all data
    pub async fn get_full_board(
        &self,
        project_name: &str,
    ) -> Result<(Project, Board, Vec<List>, Vec<Card>), String> {
        // Find project
        let projects = self.list_projects().await?;
        let project = projects
            .iter()
            .find(|p| p.name.to_lowercase() == project_name.to_lowercase())
            .ok_or_else(|| format!("Project '{}' not found", project_name))?;

        // Find board
        let boards = self.list_boards(&project.id).await?;
        let board = boards
            .first()
            .ok_or_else(|| "No boards in project".to_string())?;

        // Get lists and cards
        let lists = self.list_lists(&board.id).await?;

        let mut all_cards = Vec::new();
        for list in &lists {
            let cards = self.list_cards(&list.id).await?;
            all_cards.extend(cards);
        }

        Ok((project.clone(), board.clone(), lists, all_cards))
    }
}

/// Tool definitions for Xavier2 agents
#[derive(Debug, Clone, serde::Serialize)]
pub struct KanbanTool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

pub fn get_kanban_tools() -> Vec<KanbanTool> {
    vec![
        KanbanTool {
            name: "planka_list_projects".to_string(),
            description: "List all projects in Planka".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        KanbanTool {
            name: "planka_create_task".to_string(),
            description: "Create a task in a specific project and list".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "project": {"type": "string", "description": "Project name (e.g., Xavier2, ZeroClaw)"},
                    "list": {"type": "string", "description": "List/column name (e.g., Backlog, In Progress)"},
                    "task": {"type": "string", "description": "Task title"},
                    "description": {"type": "string", "description": "Task description"}
                },
                "required": ["project", "list", "task"]
            }),
        },
        KanbanTool {
            name: "planka_move_task".to_string(),
            description: "Move a task to another list/column".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "project": {"type": "string", "description": "Project name"},
                    "task_name": {"type": "string", "description": "Task name to move"},
                    "to_list": {"type": "string", "description": "Target list name"}
                },
                "required": ["project", "task_name", "to_list"]
            }),
        },
        KanbanTool {
            name: "planka_get_board".to_string(),
            description: "Get full board status with all tasks".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "project": {"type": "string", "description": "Project name"}
                },
                "required": ["project"]
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_client_creation() {
        // Test requires PLANKA_* env vars - skip in unit tests
        // PlankaClient::from_env();
    }
}
