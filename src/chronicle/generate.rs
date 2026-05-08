use anyhow::Result;
use crate::agents::provider::ModelProviderClient;
use crate::chronicle::harvest::HarvestOutput;
use crate::chronicle::redact::Redactor;

pub struct ChronicleGenerator {
    llm_client: ModelProviderClient,
    redactor: Redactor,
}

impl ChronicleGenerator {
    pub fn new(model_override: Option<String>) -> Self {
        Self {
            llm_client: ModelProviderClient::from_model_override(model_override),
            redactor: Redactor::new(),
        }
    }

    pub async fn generate(&self, harvest: &HarvestOutput) -> Result<String> {
        let harvest_json = serde_json::to_string_pretty(harvest)?;
        let redacted_harvest = self.redactor.redact(&harvest_json);

        let system_prompt = "You are a technical blog writer for the Xavier2 project.
Your goal is to write an engaging, technical, and informative 'Daily Chronicle' post based on the provided activity data.
Focus on:
- A summary of the day's achievements.
- Explaining key technical decisions found in the data.
- Describing resolved bugs and lessons learned.
- Providing architectural context for the changes.

The tone should be professional yet conversational, like a developer log.
Output the post in Markdown format.";

        let user_prompt = format!(
            "{}\n\nHere is the redacted activity data for today:\n\n```json\n{}\n```\n\nPlease generate the Daily Chronicle blog post.",
            system_prompt,
            redacted_harvest
        );

        let response = self.llm_client.generate_response(&user_prompt, &[]).await?;

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_creation() {
        let generator = ChronicleGenerator::new(None);
        assert!(generator.redactor.redact("test").contains("test"));
    }
}
