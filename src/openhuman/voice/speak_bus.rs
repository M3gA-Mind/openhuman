//! Broadcast bus for **proactive assistant speech** (core → UI).
//!
//! Mirrors `overlay::bus`: a single `tokio::sync::broadcast` channel wrapped in
//! a `Lazy` static so any core module can ask the assistant to *speak* a line
//! without threading a sender around. The Socket.IO bridge in
//! `core::socketio::spawn_web_channel_bridge` subscribes here and forwards every
//! request to the desktop UI as a `voice:speak` message, which the frontend
//! plays through the existing TTS pipeline (`openhuman.voice_reply_synthesize`).
//!
//! Today's only producer is the voice-native approval surface
//! (`voice::approval_surface`): when a sensitive action is parked for approval
//! during a **voice-initiated** turn, it speaks the confirmation prompt aloud so
//! a hands-free user can answer "yes"/"no" by voice (Phase 4 of #3148).

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

const LOG_PREFIX: &str = "[voice-speak]";

/// A request for the assistant to speak a line aloud.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpeakRequest {
    /// The text to synthesize and play.
    pub text: String,
    /// Originating subsystem, for diagnostics/UI (e.g. `"approval"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

impl SpeakRequest {
    /// Build a speak request with no source label.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            source: None,
        }
    }

    /// Tag the request with an originating subsystem.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
}

static SPEAK_BUS: Lazy<broadcast::Sender<SpeakRequest>> = Lazy::new(|| {
    let (tx, _rx) = broadcast::channel(32);
    tx
});

/// Subscribe to speak requests. Used by the Socket.IO bridge.
pub fn subscribe_speak_events() -> broadcast::Receiver<SpeakRequest> {
    SPEAK_BUS.subscribe()
}

/// Publish a request for the assistant to speak `request.text`.
///
/// Fire-and-forget: if nobody is subscribed (bridge not started, UI offline) the
/// request is dropped. Empty/whitespace text is a no-op — never synthesize
/// silence. Returns the number of subscribers that received it, for diagnostics.
pub fn publish_speak(request: SpeakRequest) -> usize {
    if request.text.trim().is_empty() {
        log::debug!("{LOG_PREFIX} ignoring empty speak request");
        return 0;
    }
    log::debug!(
        "{LOG_PREFIX} publish speak source={:?} chars={}",
        request.source,
        request.text.len()
    );
    match SPEAK_BUS.send(request) {
        Ok(n) => n,
        Err(_) => {
            log::debug!("{LOG_PREFIX} no speak subscribers — request dropped");
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn publish_is_received_by_subscriber() {
        let mut rx = subscribe_speak_events();
        let delivered = publish_speak(SpeakRequest::new("hello there").with_source("test"));
        assert!(delivered >= 1);
        // The process-global bus may carry lagged events from parallel tests;
        // drain until we find ours.
        let mut found = false;
        for _ in 0..16 {
            match rx.try_recv() {
                Ok(req) if req.text == "hello there" => {
                    assert_eq!(req.source.as_deref(), Some("test"));
                    found = true;
                    break;
                }
                Ok(_) => continue,
                Err(broadcast::error::TryRecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
        assert!(found, "expected our speak request from the broadcast bus");
    }

    #[test]
    fn empty_text_is_not_published() {
        let _rx = subscribe_speak_events();
        assert_eq!(publish_speak(SpeakRequest::new("   ")), 0);
    }

    #[test]
    fn builder_sets_source() {
        let req = SpeakRequest::new("hi").with_source("approval");
        assert_eq!(req.source.as_deref(), Some("approval"));
        assert_eq!(SpeakRequest::new("hi").source, None);
    }
}
