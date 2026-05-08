use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;

#[async_trait]
pub trait ChroniclePublishPlugin: Send + Sync {
    fn name(&self) -> &str;
    async fn publish(&self, content: &str, dest_path: Option<String>) -> anyhow::Result<String>;
}

pub struct FilePublishPlugin {
    default_dir: PathBuf,
}

impl FilePublishPlugin {
    pub fn new(default_dir: PathBuf) -> Self {
        Self { default_dir }
    }
}

#[async_trait]
impl ChroniclePublishPlugin for FilePublishPlugin {
    fn name(&self) -> &str {
        "file_publish"
    }

    async fn publish(&self, content: &str, dest_path: Option<String>) -> anyhow::Result<String> {
        let dest = dest_path.map(PathBuf::from).unwrap_or_else(|| {
            let date = chrono::Local::now().format("%Y-%m-%d");
            self.default_dir.join(format!("chronicle-{}.md", date))
        });

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&dest, content).await?;
        Ok(dest.to_string_lossy().to_string())
    }
}
