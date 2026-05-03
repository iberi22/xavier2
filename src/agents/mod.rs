pub mod curation;
pub mod provider;
pub mod router;
pub mod runtime;
pub mod supervisor;
pub mod system1;
pub mod system2;
pub mod system3;
pub mod ui_render;
pub mod unregister_agent_handler;

use std::collections::HashMap;

pub use runtime::{AgentRuntime, RuntimeConfig};
pub use unregister_agent_handler::unregister_agent_handler;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentStatus {
    Idle,
    Running,
}

#[derive(Debug, Clone, Default)]
pub struct AgentConfig {
    pub name: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub tools: Vec<String>,
    pub context: HashMap<String, String>,
    pub skills: Vec<String>,
}

impl AgentConfig {
    pub fn new(name: String) -> Self {
        Self {
            name,
            provider: None,
            model: None,
            tools: Vec::new(),
            context: HashMap::new(),
            skills: Vec::new(),
        }
    }

    pub fn with_provider(mut self, provider: String) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }

    pub fn with_tools(mut self, tools: Vec<String>) -> Self {
        self.tools = tools;
        self
    }

    pub fn with_context(mut self, context: HashMap<String, String>) -> Self {
        self.context = context;
        self
    }

    pub fn with_skills(mut self, skills: Vec<String>) -> Self {
        self.skills = skills;
        self
    }
}

#[derive(Debug, Clone)]
pub struct Agent {
    pub name: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub tools: Vec<String>,
    pub context: HashMap<String, String>,
    pub skills: Vec<String>,
    pub status: AgentStatus,
}

impl Agent {
    pub fn new(config: AgentConfig) -> Self {
        Self {
            name: config.name,
            provider: config.provider,
            model: config.model,
            tools: config.tools,
            context: config.context,
            skills: config.skills,
            status: AgentStatus::Idle,
        }
    }

    pub fn start(&mut self) {
        self.status = AgentStatus::Running;
    }

    pub fn stop(&mut self) {
        self.status = AgentStatus::Idle;
    }

    pub async fn execute(&self, prompt: String) -> anyhow::Result<String> {
        Ok(format!("{}:{}", self.name, prompt))
    }
}

pub type Context = HashMap<String, String>;

#[derive(Debug, Clone, Default)]
pub struct AgentState {
    pub agent_id: String,
    pub context: Context,
}

impl AgentState {
    pub fn new(agent_id: String) -> Self {
        Self {
            agent_id,
            context: HashMap::new(),
        }
    }

    pub fn add_context(&mut self, key: String, value: String) {
        self.context.insert(key, value);
    }

    pub fn update_context(&mut self, key: String, value: String) {
        self.context.insert(key, value);
    }

    pub fn get_context(&self, key: &str) -> Option<&String> {
        self.context.get(key)
    }

    pub fn remove_context(&mut self, key: String) {
        self.context.remove(&key);
    }
}

pub mod coordination {
    use std::collections::HashMap;
    use std::sync::RwLock;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum MessageType {
        Task,
        Notification,
    }

    #[derive(Debug, Clone)]
    pub struct AgentMessage {
        pub from: String,
        pub to: String,
        pub message_type: MessageType,
        pub payload: String,
    }

    impl AgentMessage {
        pub fn new(from: String, to: String, message_type: MessageType, payload: String) -> Self {
            Self {
                from,
                to,
                message_type,
                payload,
            }
        }
    }

    #[derive(Default)]
    pub struct AgentCoordinator {
        queues: RwLock<HashMap<String, Vec<AgentMessage>>>,
    }

    impl AgentCoordinator {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn agents(&self) -> Vec<String> {
            self.queues
                .read()
                .map(|queues| queues.keys().cloned().collect())
                .unwrap_or_default()
        }

        pub fn register_agent(&mut self, agent: String) {
            self.queues
                .write()
                .expect("agent coordinator lock poisoned")
                .entry(agent)
                .or_default();
        }

        pub async fn send_message(&self, message: AgentMessage) {
            self.queues
                .write()
                .expect("agent coordinator lock poisoned")
                .entry(message.to.clone())
                .or_default()
                .push(message);
        }

        pub async fn get_messages<S: AsRef<str>>(&self, agent: S) -> Vec<AgentMessage> {
            self.queues
                .read()
                .expect("agent coordinator lock poisoned")
                .get(agent.as_ref())
                .cloned()
                .unwrap_or_default()
        }

        pub async fn broadcast(&self, from: String, message_type: MessageType, payload: String) {
            let agents: Vec<String> = self
                .queues
                .read()
                .expect("agent coordinator lock poisoned")
                .keys()
                .cloned()
                .collect();
            for agent in agents {
                self.send_message(AgentMessage::new(
                    from.clone(),
                    agent,
                    message_type.clone(),
                    payload.clone(),
                ))
                .await;
            }
        }
    }
}
