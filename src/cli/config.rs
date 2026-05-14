//! CLI configuration utilities

use anyhow::{anyhow, Result};
use rand::{rngs::OsRng, RngCore};
use std::path::PathBuf;
use tracing::warn;

use crate::settings::XavierSettings;

pub fn resolve_http_token() -> Result<String> {
    match std::env::var("XAVIER_TOKEN") {
        Ok(token) => Ok(token),
        Err(_) if xavier_dev_mode_enabled() => {
            let mut bytes = [0u8; 16];
            OsRng.fill_bytes(&mut bytes);
            let token = hex::encode(bytes);
            warn!("XAVIER_TOKEN not set, generated random token because XAVIER_DEV_MODE=true");
            Ok(token)
        }
        Err(_) => Err(anyhow!(
            "XAVIER_TOKEN environment variable must be set to start the HTTP server. Set XAVIER_DEV_MODE=true only for explicit local development."
        )),
    }
}

pub fn xavier_dev_mode_enabled() -> bool {
    std::env::var("XAVIER_DEV_MODE")
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "TRUE"))
}

pub fn resolve_http_bind_host() -> String {
    std::env::var("XAVIER_HOST").unwrap_or_else(|_| XavierSettings::current().server.host)
}

pub fn resolve_base_url_for_port(port: u16) -> String {
    std::env::var("XAVIER_URL").unwrap_or_else(|_| {
        let settings = XavierSettings::current();
        if port == settings.server.port {
            return settings.client_base_url();
        }
        let host = match settings.server.host.as_str() {
            "0.0.0.0" | "::" => "127.0.0.1",
            other => other,
        };
        format!("http://{}:{}", host, port)
    })
}

pub fn resolve_base_url() -> String {
    let port = resolve_http_port();
    resolve_base_url_for_port(port)
}

pub fn resolve_http_port() -> u16 {
    std::env::var("XAVIER_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or_else(|| XavierSettings::current().server.port)
}

pub fn xavier_token() -> String {
    std::env::var("XAVIER_TOKEN")
        .expect("XAVIER_TOKEN environment variable must be set for CLI client commands")
}

pub fn require_xavier_token() -> Result<String> {
    std::env::var("XAVIER_TOKEN").map_err(|_| {
        anyhow!("XAVIER_TOKEN environment variable must be set for CLI client commands")
    })
}

pub fn code_graph_db_path() -> PathBuf {
    std::env::var("XAVIER_CODE_GRAPH_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data").join("code_graph.db"))
}

pub fn state_panel_root(workspace_dir: &std::path::Path, workspace_id: &str) -> PathBuf {
    std::env::var("XAVIER_PANEL_STORE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            workspace_dir
                .join("data")
                .join("workspaces")
                .join(workspace_id)
                .join("panel_threads")
        })
}

pub fn default_token_budget() -> usize {
    800
}

pub fn default_limit() -> usize {
    10
}

pub fn default_compaction_threshold() -> f64 {
    80.0
}
