//! Models for the DevLog system.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Post {
    pub title: String,
    pub date: String,
    pub tags: Vec<String>,
    pub author: String,
    pub source_files: Vec<String>,
    pub content_html: String,
}
