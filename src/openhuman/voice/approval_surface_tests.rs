//! Unit tests for the voice approval surface — pure prompt formatting plus the
//! `is_voice` gate that decides whether to speak.

use super::*;
use crate::core::event_bus::DomainEvent;
use crate::openhuman::voice::speak_bus::{subscribe_speak_events, SpeakRequest};
use tokio::sync::broadcast::error::TryRecvError;

fn approval_event(action_summary: &str, is_voice: bool) -> DomainEvent {
    DomainEvent::ApprovalRequested {
        request_id: "req-1".to_string(),
        tool_name: "composio".to_string(),
        action_summary: action_summary.to_string(),
        args_redacted: serde_json::json!({}),
        thread_id: Some("thread-1".to_string()),
        client_id: Some("client-1".to_string()),
        is_voice,
    }
}

/// Drain the speak bus looking for a request with the given source. Returns the
/// matching request text, or `None` if none arrived (tolerates lagged events
/// from other tests sharing the process-global bus).
fn drain_for_source(
    rx: &mut tokio::sync::broadcast::Receiver<SpeakRequest>,
    source: &str,
) -> Option<String> {
    for _ in 0..32 {
        match rx.try_recv() {
            Ok(req) if req.source.as_deref() == Some(source) => return Some(req.text),
            Ok(_) => continue,
            Err(TryRecvError::Lagged(_)) => continue,
            Err(_) => return None,
        }
    }
    None
}

// ── spoken_prompt ────────────────────────────────────────────────────────────

#[test]
fn spoken_prompt_appends_confirmation() {
    let line = spoken_prompt("Send a message to #general").unwrap();
    assert_eq!(
        line,
        "Send a message to #general. Say yes to confirm, or no to cancel."
    );
}

#[test]
fn spoken_prompt_strips_trailing_period() {
    let line = spoken_prompt("Delete 3 files.").unwrap();
    assert_eq!(line, "Delete 3 files. Say yes to confirm, or no to cancel.");
}

#[test]
fn spoken_prompt_empty_is_none() {
    assert_eq!(spoken_prompt("   "), None);
}

// ── handle: the is_voice gate ────────────────────────────────────────────────

#[tokio::test]
async fn speaks_for_voice_initiated_approval() {
    let mut rx = subscribe_speak_events();
    let sub = VoiceApprovalSurfaceSubscriber;
    sub.handle(&approval_event("Post to Slack", true)).await;
    let spoken = drain_for_source(&mut rx, VOICE_APPROVAL_SOURCE);
    assert_eq!(
        spoken.as_deref(),
        Some("Post to Slack. Say yes to confirm, or no to cancel.")
    );
}

#[tokio::test]
async fn silent_for_typed_approval() {
    let mut rx = subscribe_speak_events();
    let sub = VoiceApprovalSurfaceSubscriber;
    sub.handle(&approval_event("Post to Slack", false)).await;
    assert_eq!(
        drain_for_source(&mut rx, VOICE_APPROVAL_SOURCE),
        None,
        "typed approvals must stay visual-only"
    );
}

#[tokio::test]
async fn ignores_non_approval_events() {
    let mut rx = subscribe_speak_events();
    let sub = VoiceApprovalSurfaceSubscriber;
    sub.handle(&DomainEvent::ApprovalDecided {
        request_id: "req-1".to_string(),
        tool_name: "composio".to_string(),
        decision: "approve_once".to_string(),
    })
    .await;
    assert_eq!(drain_for_source(&mut rx, VOICE_APPROVAL_SOURCE), None);
}

#[tokio::test]
async fn empty_summary_does_not_speak() {
    let mut rx = subscribe_speak_events();
    let sub = VoiceApprovalSurfaceSubscriber;
    sub.handle(&approval_event("   ", true)).await;
    assert_eq!(drain_for_source(&mut rx, VOICE_APPROVAL_SOURCE), None);
}
