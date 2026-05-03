use serde::{Deserialize, Serialize};
use crate::context::{ContextLevel, ContextDocument};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBuilderConfig {
    pub persona: String,
    pub rules: Vec<String>,
    pub goals: Vec<String>,
    pub constraints: Vec<String>,
    pub recent_messages_limit: usize,
}

impl Default for ContextBuilderConfig {
    fn default() -> Self {
        Self {
            persona: "You are Xavier2, a cognitive memory runtime for AI agents.".to_string(),
            rules: vec![],
            goals: vec![],
            constraints: vec![],
            recent_messages_limit: 5,
        }
    }
}

pub struct ContextBuilder {
    config: ContextBuilderConfig,
}

impl ContextBuilder {
    pub fn new(config: ContextBuilderConfig) -> Self {
        Self { config }
    }

    pub fn build(
        &self,
        level: ContextLevel,
        recent_messages: &[ContextDocument],
        memories: &[ContextDocument],
        skills: &[String],
    ) -> String {
        let mut context = String::new();

        // 1. System Prompt & Persona
        context.push_str("# System Prompt\n");
        context.push_str(&self.config.persona);
        context.push_str("\n\n");

        // 2. Rules, Goals, Constraints
        if !self.config.rules.is_empty() {
            context.push_str("## Rules\n");
            for rule in &self.config.rules {
                context.push_str(&format!("- {}\n", rule));
            }
            context.push_str("\n");
        }

        if !self.config.goals.is_empty() {
            context.push_str("## Goals\n");
            for goal in &self.config.goals {
                context.push_str(&format!("- {}\n", goal));
            }
            context.push_str("\n");
        }

        if !self.config.constraints.is_empty() {
            context.push_str("## Constraints\n");
            for constraint in &self.config.constraints {
                context.push_str(&format!("- {}\n", constraint));
            }
            context.push_str("\n");
        }

        match level {
            ContextLevel::Minimal => {
                // Phase 0: minimo -> only system prompt
                // (Already added above)
            }
            ContextLevel::Medium => {
                // Phase 0: medio -> system + recent context
                self.append_recent_messages(&mut context, recent_messages);
            }
            ContextLevel::Maximum => {
                // Phase 0: maximo -> full retrieval + skills
                self.append_memories(&mut context, memories);
                self.append_skills(&mut context, skills);
                self.append_recent_messages(&mut context, recent_messages);
            }
        }

        context
    }

    fn append_recent_messages(&self, context: &mut String, messages: &[ContextDocument]) {
        if messages.is_empty() {
            return;
        }

        context.push_str("# Recent Messages\n");
        let limit = self.config.recent_messages_limit.min(messages.len());
        let start = messages.len() - limit;
        
        for msg in &messages[start..] {
            context.push_str(&format!("{}: {}\n", msg.role, msg.content));
        }
        context.push_str("\n");
    }

    fn append_memories(&self, context: &mut String, memories: &[ContextDocument]) {
        if memories.is_empty() {
            return;
        }

        context.push_str("# Relevant Memories\n");
        for mem in memories {
            context.push_str(&format!("- {}\n", mem.content));
        }
        context.push_str("\n");
    }

    fn append_skills(&self, context: &mut String, skills: &[String]) {
        if skills.is_empty() {
            return;
        }

        context.push_str("# Available Skills\n");
        for skill in skills {
            context.push_str(&format!("- {}\n", skill));
        }
        context.push_str("\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_minimal_context() {
        let config = ContextBuilderConfig::default();
        let builder = ContextBuilder::new(config);
        let ctx = builder.build(ContextLevel::Minimal, &[], &[], &[]);

        assert!(ctx.contains("# System Prompt"));
        assert!(!ctx.contains("# Recent Messages"));
    }

    #[test]
    fn builds_medium_context() {
        let config = ContextBuilderConfig::default();
        let builder = ContextBuilder::new(config);
        let messages = vec![ContextDocument::new("1", "s1", "user", "hello")];
        let ctx = builder.build(ContextLevel::Medium, &messages, &[], &[]);

        assert!(ctx.contains("# System Prompt"));
        assert!(ctx.contains("# Recent Messages"));
        assert!(ctx.contains("user: hello"));
    }

    #[test]
    fn builds_maximum_context() {
        let config = ContextBuilderConfig::default();
        let builder = ContextBuilder::new(config);
        let memories = vec![ContextDocument::new("m1", "s1", "system", "remember this")];
        let skills = vec!["skill-1".to_string()];
        let ctx = builder.build(ContextLevel::Maximum, &[], &memories, &skills);

        assert!(ctx.contains("# System Prompt"));
        assert!(ctx.contains("# Relevant Memories"));
        assert!(ctx.contains("remember this"));
        assert!(ctx.contains("# Available Skills"));
        assert!(ctx.contains("skill-1"));
    }
}
