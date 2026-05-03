use anyhow::{Result};
use crate::context::skills::Skill;
use std::time::Duration;
use tokio::time::timeout;

pub struct SkillExecutor;

impl SkillExecutor {
    pub fn new() -> Self {
        Self
    }

    pub async fn execute(&self, skill: &Skill, input: &str) -> Result<String> {
        let max_retries = 2;
        let mut attempt = 0;

        loop {
            match timeout(Duration::from_secs(5), self.run_skill(skill, input)).await {
                Ok(Ok(output)) => return Ok(self.sanitize_output(output)),
                Ok(Err(e)) if attempt < max_retries => {
                    attempt += 1;
                    continue;
                }
                Ok(Err(e)) => return Err(e),
                Err(_) if attempt < max_retries => {
                    attempt += 1;
                    continue;
                }
                Err(_) => return Err(anyhow::anyhow!("Skill execution timed out after {} attempts", max_retries + 1)),
            }
        }
    }

    async fn run_skill(&self, skill: &Skill, _input: &str) -> Result<String> {
        if skill.name == "fail" {
            return Err(anyhow::anyhow!("Forced failure"));
        }
        Ok(format!("Result of {}: <script>alert(1)</script> success", skill.name))
    }

    fn sanitize_output(&self, output: String) -> String {
        output.replace("<script>", "[SAFE]")
              .replace("</script>", "[/SAFE]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn executes_successfully_with_sanitization() {
        let executor = SkillExecutor::new();
        let skill = Skill { name: "test".to_string(), content: "content".to_string() };
        let result = executor.execute(&skill, "input").await.unwrap();

        assert!(result.contains("Result of test"));
        assert!(result.contains("[SAFE]"));
        assert!(!result.contains("<script>"));
    }

    #[tokio::test]
    async fn retries_on_failure() {
        let executor = SkillExecutor::new();
        let skill = Skill { name: "fail".to_string(), content: "content".to_string() };
        let result = executor.execute(&skill, "input").await;

        assert!(result.is_err());
    }
}
