use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::agents::provider::ModelProviderClient;

use crate::memory::belief_graph::Belief;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurationResult {
    pub domain: String,
    pub topic: String,
    pub subtopic: Option<String>,
    pub importance: f32,
    pub beliefs: Vec<Belief>,
}

pub struct CurationAgent {
    client: ModelProviderClient,
}

impl CurationAgent {
    pub fn new() -> Self {
        Self {
            client: ModelProviderClient::from_env(),
        }
    }

    pub async fn curate(&self, content: &str) -> Result<CurationResult> {
        info!(
            "🧠 Curating content: {}...",
            content.chars().take(50).collect::<String>()
        );

        let prompt = format!(
            "Analyze the following content and categorize it into a hierarchical structure (Domain > Topic > Fact).\n\
             Extract key facts as SPO triples (Subject-Predicate-Object).\n\
             Return ONLY a JSON object with fields:\n\
             - domain (string): High-level area (e.g., Technology, Health, Business)\n\
             - topic (string): Specific subject within the domain (e.g., Rust Programming, AI Agents)\n\
             - subtopic (optional string): Granular detail (e.g., Memory Management, RAG Pipeline)\n\
             - importance (float 0.0-1.0): How critical this information is\n\
             - beliefs (array of objects): Extract specific facts as SPO triples.\n\
               Fields for each belief: 'subject', 'predicate', 'object', and 'confidence' [High/Medium/Low].\n\n\
             Content:\n\"\"\"\n{}\n\"\"\"",
            content
        );

        // We use an empty context for raw classification
        let response = self.client.generate_response(&prompt, &[]).await?;

        // Extract JSON from response (handling potential markdown blocks)
        let json_str = if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                &response[start..]
            }
        } else {
            &response
        };

        let result: CurationResult = serde_json::from_str(json_str)?;
        Ok(result)
    }
}

impl Default for CurationAgent {
    fn default() -> Self {
        Self::new()
    }
}
