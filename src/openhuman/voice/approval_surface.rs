//! Voice-native approval surface — speak a parked approval prompt aloud.
//!
//! Phase 4 of #3148. The [`ApprovalGate`] already classifies sensitive agent
//! tool calls and parks them for a yes/no decision, but the prompt is
//! visual-only (the in-app approval card). A hands-free / always-on user looking
//! away from the screen never hears it.
//!
//! This subscriber mirrors `channels::providers::telegram::approval_surface`:
//! it watches [`DomainEvent::ApprovalRequested`] and, **only when the turn was
//! voice-initiated** (`is_voice == true`), publishes a [`SpeakRequest`] so the
//! assistant speaks the confirmation aloud. The user answers by voice — the
//! spoken "yes"/"no" rides the existing transcription → auto-send → web.rs
//! ingress yes/no path straight to `approval_decide`, so no answer-side wiring
//! is needed here.
//!
//! Typed-chat approvals (`is_voice == false`) are left untouched — they stay
//! visual-only, per the agreed scope.
//!
//! [`ApprovalGate`]: crate::openhuman::approval::ApprovalGate

use crate::core::event_bus::{subscribe_global, DomainEvent, EventHandler, SubscriptionHandle};
use crate::openhuman::voice::speak_bus::{publish_speak, SpeakRequest};
use async_trait::async_trait;
use std::sync::{Arc, OnceLock};

const LOG_PREFIX: &str = "[voice-approval]";

/// Keeps the subscription alive for the process lifetime. `OnceLock` makes
/// [`register_voice_approval_surface`] idempotent — subsequent calls no-op.
static VOICE_APPROVAL_HANDLE: OnceLock<SubscriptionHandle> = OnceLock::new();

/// Register the voice approval surface so spoken approval prompts fire for
/// voice-initiated turns. Idempotent; safe to call from multiple startup paths.
pub fn register_voice_approval_surface() {
    if VOICE_APPROVAL_HANDLE.get().is_some() {
        return;
    }
    match subscribe_global(Arc::new(VoiceApprovalSurfaceSubscriber)) {
        Some(handle) => {
            let _ = VOICE_APPROVAL_HANDLE.set(handle);
            log::info!(
                "{LOG_PREFIX} registered voice approval surface (domain=approval) — will speak \
                 approval prompts for voice-initiated turns"
            );
        }
        None => {
            log::warn!(
                "{LOG_PREFIX} failed to register voice approval surface — bus not initialized"
            );
        }
    }
}

/// `SpeakRequest.source` tag for spoken approval prompts.
pub const VOICE_APPROVAL_SOURCE: &str = "approval";

/// Render an approval request's redacted `action_summary` into a short spoken
/// confirmation line. Kept as a free function so tests pin the wording without a
/// bus round-trip. Returns `None` for an empty summary — never speak silence.
pub(crate) fn spoken_prompt(action_summary: &str) -> Option<String> {
    let summary = action_summary.trim();
    if summary.is_empty() {
        return None;
    }
    // Drop a trailing period so the joined sentence reads cleanly.
    let summary = summary.trim_end_matches('.');
    Some(format!("{summary}. Say yes to confirm, or no to cancel."))
}

/// Subscriber that speaks approval prompts for voice-initiated turns.
pub struct VoiceApprovalSurfaceSubscriber;

#[async_trait]
impl EventHandler for VoiceApprovalSurfaceSubscriber {
    fn name(&self) -> &str {
        "voice::approval_surface"
    }

    fn domains(&self) -> Option<&[&str]> {
        Some(&["approval"])
    }

    async fn handle(&self, event: &DomainEvent) {
        if let DomainEvent::ApprovalRequested {
            request_id,
            tool_name,
            action_summary,
            is_voice,
            ..
        } = event
        {
            if !*is_voice {
                // Typed/visual approval — stays on the in-app card.
                return;
            }
            let Some(line) = spoken_prompt(action_summary) else {
                tracing::warn!(
                    "{LOG_PREFIX} voice approval request_id={request_id} tool={tool_name} \
                     has an empty action_summary — not speaking"
                );
                return;
            };
            tracing::info!(
                "{LOG_PREFIX} speaking approval prompt request_id={request_id} tool={tool_name}"
            );
            publish_speak(SpeakRequest::new(line).with_source(VOICE_APPROVAL_SOURCE));
        }
    }
}

#[cfg(test)]
#[path = "approval_surface_tests.rs"]
mod tests;
