use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use clap::Subcommand;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock as AsyncRwLock;

use crate::chronicle::generate::{ChronicleGenerator, ChronicleInput};
use crate::chronicle::ssg::DevLogSSG;
use crate::chronicle::harvest::{HarvestOutput, Harvester};
use crate::chronicle::publish::{
    ChroniclePost, ChroniclePublishHook, FilePublishHook, StdoutPublishHook,
};
use crate::chronicle::redact::process_output;
use crate::memory::qmd_memory::QmdMemory;
use crate::memory::sqlite_vec_store::VecSqliteMemoryStore;
use crate::memory::store::{MemoryRecord, MemoryStore};

use crate::settings::XavierSettings;

#[derive(Subcommand, Debug, Clone, PartialEq)]
pub enum ChronicleCommand {
    /// Recolecta datos locales: commits, memoria, sesiones, cambios de código
    Harvest {
        /// Recolectar desde esta fecha
        #[arg(long)]
        since: Option<String>,
        /// Ruta al workspace
        #[arg(long)]
        workspace: Option<String>,
    },
    /// Toma el harvest más reciente, aplica redact y genera el post vía LLM
    Generate {
        /// Generar desde esta fecha
        #[arg(long)]
        since: Option<String>,
        /// Archivo de salida
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Muestra el post generado en la terminal
    Preview {
        /// Archivo a previsualizar
        #[arg(long)]
        file: Option<String>,
    },
    /// Exporta el post a markdown en la ruta especificada
    Publish {
        /// Ruta de destino del archivo markdown
        #[arg(long)]
        to: Option<String>,
    },
    /// Generates the static blog from docs/devlog/*.md
    Build,
}

pub async fn handle_chronicle_command(cmd: ChronicleCommand) -> Result<()> {
    match cmd {
        ChronicleCommand::Harvest { since, workspace } => {
            let workspace_path = resolve_workspace_path(workspace);
            let since = parse_since_arg(since.as_deref())?;
            let memory = load_memory_from_env().await?;
            let code_db = Arc::new(code_graph::db::CodeGraphDB::new(&resolve_code_graph_db_path())?);
            let harvester = Harvester::new(workspace_path, memory, code_db);
            let output_path = harvester.run(since).await?;
            println!("Chronicle harvest written to {}", output_path.display());
        }
        ChronicleCommand::Generate { since, output } => {
            let workspace_path = resolve_workspace_path(None);
            let harvest_path = resolve_harvest_path(&workspace_path, since.as_deref())?;
            let harvest = read_harvest(&harvest_path)?;
            let raw_json = fs::read_to_string(&harvest_path)
                .with_context(|| format!("failed to read {}", harvest_path.display()))?;
            let redacted = process_output(&raw_json)?;
            let input = ChronicleInput {
                date: harvest.date.clone(),
                active_projects: count_active_projects(&harvest),
                commits: harvest.commits.len(),
                files_modified: count_modified_files(&harvest),
                sessions: harvest.sessions.len(),
                raw_data: redacted,
            };
            let markdown = ChronicleGenerator::new().generate(input).await?;
            let output_path = output
                .map(PathBuf::from)
                .unwrap_or_else(|| chronicle_dir(&workspace_path).join(format!("daily-{}.md", harvest.date)));
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&output_path, markdown)
                .with_context(|| format!("failed to write {}", output_path.display()))?;
            println!("Chronicle post written to {}", output_path.display());
        }
        ChronicleCommand::Preview { file } => {
            let workspace_path = resolve_workspace_path(None);
            let path = file
                .map(PathBuf::from)
                .unwrap_or(resolve_latest_daily_path(&workspace_path)?);
            let content =
                fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
            println!("{content}");
        }
        ChronicleCommand::Publish { to } => {
            let workspace_path = resolve_workspace_path(None);
            let post_path = resolve_latest_daily_path(&workspace_path)?;
            let markdown = fs::read_to_string(&post_path)
                .with_context(|| format!("failed to read {}", post_path.display()))?;
            let post = ChroniclePost {
                date: extract_date_from_filename(&post_path).unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string()),
                title: extract_title(&markdown).unwrap_or_else(|| "Daily Chronicle".to_string()),
                markdown,
                metadata: HashMap::new(),
            };

            let result = if let Some(destination) = to {
                FilePublishHook::new(destination).publish(&post)?
            } else {
                StdoutPublishHook.publish(&post)?
            };

            println!("Chronicle published to {}", result.destination);
        }
        ChronicleCommand::Build => {
            DevLogSSG::new().build()?;
            println!("DevLog static blog built successfully in public/devlog/");
        }
    }
    Ok(())
}

fn parse_since_arg(value: Option<&str>) -> Result<DateTime<Utc>> {
    match value {
        None => Ok(Utc::now() - Duration::days(1)),
        Some(raw) => {
            if let Ok(date_time) = DateTime::parse_from_rfc3339(raw) {
                return Ok(date_time.with_timezone(&Utc));
            }

            let date = NaiveDate::parse_from_str(raw, "%Y-%m-%d")
                .with_context(|| format!("invalid --since value '{raw}', expected YYYY-MM-DD or RFC3339"))?;
            let naive = date
                .and_hms_opt(0, 0, 0)
                .ok_or_else(|| anyhow!("failed to build timestamp for '{raw}'"))?;
            Ok(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc))
        }
    }
}

fn resolve_workspace_path(workspace: Option<String>) -> PathBuf {
    workspace
        .map(PathBuf::from)
        .or_else(|| std::env::var("XAVIER_WORKSPACE_DIR").ok().map(PathBuf::from))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}

fn chronicle_dir(workspace_path: &Path) -> PathBuf {
    workspace_path.join(".chronicle")
}

fn resolve_code_graph_db_path() -> PathBuf {
    let settings = XavierSettings::current();
    std::env::var("XAVIER_CODE_GRAPH_DB_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(settings.server.code_graph_db_path))
}

async fn load_memory_from_env() -> Result<Arc<QmdMemory>> {
    XavierSettings::current().apply_to_env();
    if std::env::var_os("XAVIER_TOKEN").is_none() {
        std::env::set_var("XAVIER_TOKEN", "chronicle-local-token");
    }

    let workspace_id =
        std::env::var("XAVIER_DEFAULT_WORKSPACE_ID").unwrap_or_else(|_| "default".to_string());
    let store = VecSqliteMemoryStore::from_env().await?;
    let state = store.load_workspace_state(&workspace_id).await?;
    let docs = Arc::new(AsyncRwLock::new(
        state
            .memories
            .iter()
            .map(MemoryRecord::to_document)
            .collect::<Vec<_>>(),
    ));
    let memory = Arc::new(QmdMemory::new_with_workspace(docs, workspace_id));
    let dyn_store: Arc<dyn MemoryStore> = Arc::new(store);
    memory.set_store(dyn_store).await;
    memory.init().await?;
    Ok(memory)
}

fn resolve_harvest_path(workspace_path: &Path, since: Option<&str>) -> Result<PathBuf> {
    if let Some(raw) = since {
        let date = if let Ok(date_time) = DateTime::parse_from_rfc3339(raw) {
            date_time.format("%Y-%m-%d").to_string()
        } else {
            NaiveDate::parse_from_str(raw, "%Y-%m-%d")
                .with_context(|| format!("invalid --since value '{raw}', expected YYYY-MM-DD or RFC3339"))?
                .format("%Y-%m-%d")
                .to_string()
        };
        let path = chronicle_dir(workspace_path).join(format!("harvest-{date}.json"));
        if path.exists() {
            return Ok(path);
        }
        return Err(anyhow!("harvest file not found for date {date}: {}", path.display()));
    }

    latest_file_with_prefix(&chronicle_dir(workspace_path), "harvest-", "json")
}

fn resolve_latest_daily_path(workspace_path: &Path) -> Result<PathBuf> {
    latest_file_with_prefix(&chronicle_dir(workspace_path), "daily-", "md")
}

fn latest_file_with_prefix(dir: &Path, prefix: &str, extension: &str) -> Result<PathBuf> {
    let mut entries = fs::read_dir(dir)
        .with_context(|| format!("failed to read chronicle directory {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|value| value.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(prefix) && name.ends_with(extension))
        })
        .collect::<Vec<_>>();

    entries.sort();
    entries
        .pop()
        .ok_or_else(|| anyhow!("no chronicle artifact found in {}", dir.display()))
}

fn read_harvest(path: &Path) -> Result<HarvestOutput> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read harvest file {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse harvest file {}", path.display()))
}

fn count_active_projects(harvest: &HarvestOutput) -> usize {
    let mut roots = harvest
        .code_changes
        .iter()
        .filter_map(|change| change.file.split('/').next().map(str::to_string))
        .collect::<Vec<_>>();
    roots.sort();
    roots.dedup();
    roots.len().max(1)
}

fn count_modified_files(harvest: &HarvestOutput) -> usize {
    let mut files = harvest
        .commits
        .iter()
        .flat_map(|commit| commit.files.iter().cloned())
        .collect::<Vec<_>>();
    files.sort();
    files.dedup();
    files.len()
}

fn extract_date_from_filename(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    stem.strip_prefix("daily-").map(|value| value.to_string())
}

fn extract_title(markdown: &str) -> Option<String> {
    markdown
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(|value| value.trim().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct TestCli {
        #[command(subcommand)]
        cmd: ChronicleCommand,
    }

    #[test]
    fn test_chronicle_harvest_parsing() {
        let args = vec!["xavier", "harvest", "--since", "2026-05-01", "--workspace", "/path/to/ws"];
        let cli = TestCli::parse_from(args);

        match cli.cmd {
            ChronicleCommand::Harvest { since, workspace } => {
                assert_eq!(since, Some("2026-05-01".to_string()));
                assert_eq!(workspace, Some("/path/to/ws".to_string()));
            }
            _ => panic!("Expected Harvest command"),
        }
    }

    #[test]
    fn test_chronicle_generate_parsing() {
        let args = vec!["xavier", "generate", "--since", "2026-05-01", "--output", "post.md"];
        let cli = TestCli::parse_from(args);

        match cli.cmd {
            ChronicleCommand::Generate { since, output } => {
                assert_eq!(since, Some("2026-05-01".to_string()));
                assert_eq!(output, Some("post.md".to_string()));
            }
            _ => panic!("Expected Generate command"),
        }
    }

    #[test]
    fn test_parse_since_arg_supports_date() {
        let parsed = parse_since_arg(Some("2026-05-01")).unwrap();
        assert_eq!(parsed.format("%Y-%m-%d").to_string(), "2026-05-01");
    }
}
