//! Event-bus subscriber for Telegram remote-control lifecycle signals.

use crate::core::event_bus::{DomainEvent, EventHandler};
use crate::openhuman::channels::providers::telegram::session_store::with_store;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

const LOG_PREFIX: &str = "[telegram-remote]";

/// Tracks Telegram turn lifecycle via channel domain events and exposes busy
/// state for `/status`.
pub struct TelegramRemoteSubscriber {
    /// Re-bindable workspace handle (issue #4398). Shared with the channel
    /// runtime context so a post-login `rebind_workspace` re-points busy-state
    /// persistence at the activated user's workspace. Read at event time so the
    /// stale-workspace guard below compares against the CURRENT workspace — the
    /// dispatch loop stamps events with `ctx.workspace_dir()`, which after a
    /// re-bind is the new path; a baked snapshot here would drop every event.
    workspace_handle: Arc<RwLock<PathBuf>>,
}

impl TelegramRemoteSubscriber {
    pub fn new(workspace_handle: Arc<RwLock<PathBuf>>) -> Self {
        Self { workspace_handle }
    }

    /// Current workspace directory, re-resolved through the shared handle.
    fn workspace_dir(&self) -> PathBuf {
        match self.workspace_handle.read() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    async fn set_busy(&self, reply_target: &str, busy: bool) {
        let workspace_dir = self.workspace_dir();
        let reply_target_owned = reply_target.to_string();
        let join_result = tokio::task::spawn_blocking(move || {
            with_store(&workspace_dir, |store| {
                store.set_busy(&reply_target_owned, busy);
                Ok(())
            })
        })
        .await;

        match join_result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::warn!(
                "{LOG_PREFIX} failed to persist busy={busy} reply_target={reply_target}: {error}"
            ),
            Err(error) => tracing::warn!(
                "{LOG_PREFIX} join error persisting busy={busy} reply_target={reply_target}: {error}"
            ),
        }
    }
}

#[async_trait]
impl EventHandler for TelegramRemoteSubscriber {
    fn name(&self) -> &str {
        "telegram::remote_control"
    }

    fn domains(&self) -> Option<&[&str]> {
        Some(&["channel"])
    }

    async fn handle(&self, event: &DomainEvent) {
        match event {
            DomainEvent::ChannelMessageReceived {
                channel,
                reply_target,
                workspace_dir,
                ..
            } if channel == "telegram" => {
                if *workspace_dir != self.workspace_dir() {
                    tracing::debug!(
                        "{LOG_PREFIX} dropping stale-workspace ChannelMessageReceived \
                         event_ws={} self_ws={}",
                        workspace_dir.display(),
                        self.workspace_dir().display()
                    );
                    return;
                }
                tracing::debug!("{LOG_PREFIX} turn started reply_target={reply_target}");
                self.set_busy(reply_target, true).await;
            }
            DomainEvent::ChannelMessageProcessed {
                channel,
                reply_target,
                success,
                elapsed_ms,
                workspace_dir,
                ..
            } if channel == "telegram" => {
                if *workspace_dir != self.workspace_dir() {
                    tracing::debug!(
                        "{LOG_PREFIX} dropping stale-workspace ChannelMessageProcessed \
                         event_ws={} self_ws={}",
                        workspace_dir.display(),
                        self.workspace_dir().display()
                    );
                    return;
                }
                tracing::debug!(
                    "{LOG_PREFIX} turn finished reply_target={reply_target} success={success} elapsed_ms={elapsed_ms}"
                );
                self.set_busy(reply_target, false).await;
            }
            _ => {}
        }
    }
}
