use tracing::info;

use crate::session::event_mapper::PanelThreadEntry;
use crate::session::types::SessionEvent;

/// Maps session events to PanelThreadEntry and indexes them into Xavier2 memory stores
pub struct SessionIndexer;

impl SessionIndexer {
    /// Map a session event to a thread entry (returns None for session start/end)
    pub fn index_event(event: &SessionEvent) -> Option<PanelThreadEntry> {
        let entry = PanelThreadEntry::from_session_event(event)?;

        info!(
            session_id = %event.session_id,
            role = %entry.role,
            content_len = entry.content.len(),
            "mapping session event"
        );

        Some(entry)
    }
}
