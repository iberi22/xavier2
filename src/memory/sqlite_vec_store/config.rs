use std::path::PathBuf;
use std::sync::OnceLock;
use anyhow::Result;
use crate::settings::XavierSettings;

pub const DB_FILENAME: &str = "xavier_memory_vec.db";
pub const DEFAULT_EMBEDDING_DIMENSIONS: usize = 768;
pub const DEFAULT_RRF_K: usize = 60;
pub const DEFAULT_VECTOR_WEIGHT: f32 = 0.40;
pub const DEFAULT_FTS_WEIGHT: f32 = 0.35;
pub const DEFAULT_KG_WEIGHT: f32 = 0.25;
pub const DEFAULT_QJL_THRESHOLD: usize = 30_000;
pub const QJL_MAGIC: &[u8; 4] = b"QJL2";
pub static SQLITE_VEC_EXTENSION_INIT: OnceLock<Result<(), String>> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct VecSqliteStoreConfig {
    pub path: PathBuf,
    pub embedding_dimensions: usize,
}

impl VecSqliteStoreConfig {
    pub fn from_env() -> Self {
        let settings = XavierSettings::current();
        let embedding_dimensions = std::env::var("XAVIER_EMBEDDING_DIMENSIONS")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or({
                if settings.memory.embedding_dimensions == 0 {
                    DEFAULT_EMBEDDING_DIMENSIONS
                } else {
                    settings.memory.embedding_dimensions
                }
            });

        Self {
            path: std::env::var("XAVIER_MEMORY_VEC_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    if settings.memory.vec_path.trim().is_empty() {
                        PathBuf::from(&settings.memory.data_dir).join(DB_FILENAME)
                    } else {
                        PathBuf::from(&settings.memory.vec_path)
                    }
                }),
            embedding_dimensions,
        }
    }

    pub fn detail(&self) -> String {
        format!(
            "{} ({}d embeddings)",
            self.path.display(),
            self.embedding_dimensions
        )
    }
}
