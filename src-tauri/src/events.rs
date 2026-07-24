//! Events emitted from the backend to the overlay webview.

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::config::STREAM_EVENT;

/// Streaming rewrite lifecycle. Serialized with a `kind` tag matching the
/// TypeScript `StreamEvent` union.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum StreamEvent {
    /// A new rewrite began; carries the original captured text.
    Start { original: String },
    /// An incremental text fragment.
    Delta { text: String },
    /// The rewrite finished; carries the full result.
    Done { full: String },
    /// Something went wrong; carries a user-facing message.
    Error { message: String },
}

impl StreamEvent {
    /// Emit this event to all windows listening on the stream channel.
    pub fn emit(self, app: &AppHandle) {
        if let Err(e) = app.emit(STREAM_EVENT, &self) {
            log::warn!("failed to emit stream event: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The frontend `StreamEvent` union relies on this exact shape:
    // `{ kind: "...", ... }`.
    #[test]
    fn serializes_with_kind_tag() {
        let json = serde_json::to_string(&StreamEvent::Delta {
            text: "hi".into(),
        })
        .unwrap();
        assert_eq!(json, r#"{"kind":"delta","text":"hi"}"#);

        let json = serde_json::to_string(&StreamEvent::Done {
            full: "done".into(),
        })
        .unwrap();
        assert_eq!(json, r#"{"kind":"done","full":"done"}"#);

        let json = serde_json::to_string(&StreamEvent::Error {
            message: "boom".into(),
        })
        .unwrap();
        assert_eq!(json, r#"{"kind":"error","message":"boom"}"#);
    }
}
