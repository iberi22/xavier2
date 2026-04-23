use anyhow::Result;
use std::sync::Arc;
use crate::memory::qmd_memory::QmdMemory;
use crate::memory::session_store::SessionStore;

/// Compacts a session if it exceeds 80% of the token limit.
///
/// If triggered:
/// 1. Creates a summary of the current session messages.
/// 2. Saves this summary as a new document in Xavier2 (QmdMemory).
/// 3. Clears the messages from the session in SessionStore.
///
/// Returns `true` if compaction was triggered, `false` otherwise.
pub async fn session_compact(
    memory: Arc<QmdMemory>,
    session_store: Arc<SessionStore>,
    session_id: &str,
    current_token_count: usize,
    max_tokens: usize,
) -> Result<bool> {
    if max_tokens == 0 {
        return Ok(false);
    }

    let ratio = current_token_count as f64 / max_tokens as f64;
    if ratio <= 0.8 {
        return Ok(false);
    }

    // Trigger compaction
    if let Some(mut thread) = session_store.get_thread(session_id).await {
        if thread.messages.is_empty() {
            return Ok(false);
        }

        // 1. Create summary (concatenation of messages for now)
        let mut summary_content = format!("Compact Summary for Session {}:\n\n", thread.title);
        for msg in &thread.messages {
            summary_content.push_str(&format!("[{}] {}: {}\n",
                msg.created_at.format("%Y-%m-%d %H:%M:%S"),
                msg.role,
                msg.plain_text
            ));
        }

        // 2. Save to Xavier2 (QmdMemory)
        let path = format!("sessions/{}/compaction/{}", session_id, ulid::Ulid::new());
        let metadata = serde_json::json!({
            "session_id": session_id,
            "category": "session_summary",
            "memory_kind": "session_compaction",
            "original_token_count": current_token_count,
            "max_tokens": max_tokens,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        memory.add_document(path, summary_content, metadata).await?;

        // 3. Clear messages in SessionStore
        thread.messages.clear();
        thread.updated_at = chrono::Utc::now();
        thread.last_preview = "(Session compacted)".to_string();
        session_store.insert_thread(thread).await?;

        return Ok(true);
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::RwLock;

    async fn setup_test_stores() -> (Arc<QmdMemory>, Arc<SessionStore>) {
        let memory = Arc::new(QmdMemory::new(Arc::new(RwLock::new(Vec::new()))));

        let temp_dir = std::env::temp_dir().join(format!("xavier2-test-{}", ulid::Ulid::new()));
        let session_store = Arc::new(SessionStore::new(temp_dir).await.unwrap());

        (memory, session_store)
    }

    #[tokio::test]
    async fn test_session_compact_threshold() {
        let (memory, session_store) = setup_test_stores().await;

        // Below threshold (79%)
        let triggered = session_compact(
            Arc::clone(&memory),
            Arc::clone(&session_store),
            "test-session",
            79,
            100
        ).await.unwrap();
        assert!(!triggered);

        // At/Above threshold (80%)
        // Create a thread first so it can find it
        let thread = session_store.create_thread("Test Thread").await.unwrap();
        let session_id = thread.id.clone();

        // Add a message
        session_store.append_message(&session_id, crate::memory::session_store::PanelMessage {
            id: ulid::Ulid::new().to_string(),
            role: "user".to_string(),
            plain_text: "Hello".to_string(),
            openui_lang: None,
            created_at: chrono::Utc::now(),
            metadata: serde_json::json!({}),
        }).await.unwrap();

        let triggered = session_compact(
            Arc::clone(&memory),
            Arc::clone(&session_store),
            &session_id,
            81,
            100
        ).await.unwrap();
        assert!(triggered);

        // Verify messages cleared
        let updated_thread = session_store.get_thread(&session_id).await.unwrap();
        assert!(updated_thread.messages.is_empty());

        // Verify summary saved to memory
        let docs = memory.all_documents().await;
        assert_eq!(docs.len(), 1);
        assert!(docs[0].content.contains("Compact Summary"));
        assert_eq!(docs[0].metadata["session_id"], session_id);
    }
}
