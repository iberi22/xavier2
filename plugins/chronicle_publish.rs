use std::collections::HashMap;
use xavier::chronicle::publish::{
    ChroniclePost, ChroniclePublishHook, FilePublishHook, StdoutPublishHook,
};

fn main() -> anyhow::Result<()> {
    println!("--- Chronicle Publish Example ---");

    // 1. Create a post
    let mut metadata = HashMap::new();
    metadata.insert("author".to_string(), "Xavier2".to_string());
    metadata.insert("tags".to_string(), "rust, ai, memory".to_string());

    let post = ChroniclePost {
        date: "2024-05-20".to_string(),
        title: "The Future of Cognitive Memory".to_string(),
        markdown: r#"
# The Future of Cognitive Memory

Artificial intelligence is evolving from simple stateless models to complex systems with long-term memory.
Xavier2 is at the forefront of this revolution.

## Key Features
- Trait-based plugin system
- Durable memory storage
- Agentic workflows
"#.to_string(),
        metadata,
    };

    // 2. Setup hooks
    let hooks: Vec<Box<dyn ChroniclePublishHook>> = vec![
        Box::new(StdoutPublishHook),
        Box::new(FilePublishHook::new("src/content/blog")),
    ];

    // 3. Publish using all hooks
    for hook in hooks {
        println!("Publishing using hook: {}", hook.name());
        let result = hook.publish(&post)?;

        if result.success {
            println!("✅ Successfully published to: {}", result.destination);
        } else {
            println!("❌ Failed to publish to: {}", result.destination);
        }
    }

    Ok(())
}
