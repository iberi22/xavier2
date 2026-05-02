use crate::memory::qmd_memory::types::MemoryDocument;
use crate::memory::surreal_store::MemoryRecord;

pub fn memory_record_from_document(workspace_id: &str, document: &MemoryDocument) -> MemoryRecord {
    let primary = document
        .metadata
        .get("source_path")
        .and_then(|value| value.as_str())
        .is_none();
    let parent_id = document
        .metadata
        .get("parent_id")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .or_else(|| {
            (!primary)
                .then(|| {
                    document
                        .metadata
                        .get("source_path")
                        .and_then(|value| value.as_str())
                        .map(|value| value.to_string())
                })
                .flatten()
        });

    MemoryRecord::from_document(workspace_id, document, primary, parent_id)
}
