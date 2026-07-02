//! Process-global, re-bindable workspace handle for the channel runtime.
//!
//! The channel runtime is started **once** with a `Config` snapshot — often
//! *before* any user has signed in, so `config.workspace_dir` resolves to
//! `~/.openhuman/users/local/`. When a later sign-in writes
//! `~/.openhuman/active_user.toml` and creates
//! `~/.openhuman/users/<user_id>/`, `credentials::ops::store_session` must
//! re-point the running runtime at the activated user's workspace — otherwise
//! every channel turn keeps reading/writing under `users/local/` until the
//! process restarts (issue #4398).
//!
//! [`ChannelRuntimeContext`](crate::openhuman::channels::context::ChannelRuntimeContext)
//! holds an `Arc<RwLock<PathBuf>>` handle and registers a clone of it here at
//! boot ([`register_workspace_handle`]). [`rebind_workspace`] swaps the path in
//! place through that shared handle, so the live dispatch loop and every
//! workspace-scoped read/write (`ctx.workspace_dir()`) re-resolves on the next
//! turn without a restart. This mirrors the memory rebind
//! (`memory::global::init`) and the active-channel handle
//! (`channels::proactive::register_active_channel_handle`) precedents.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock, RwLock};

/// Shared, in-place-swappable workspace path held by the live runtime context.
pub(crate) type WorkspaceHandle = Arc<RwLock<PathBuf>>;

type HandleSlot = RwLock<Option<WorkspaceHandle>>;

/// Process-global slot pointing at the live runtime's workspace handle.
static WORKSPACE_HANDLE: OnceLock<HandleSlot> = OnceLock::new();

fn handle_slot() -> &'static HandleSlot {
    WORKSPACE_HANDLE.get_or_init(|| RwLock::new(None))
}

fn register_in(slot: &HandleSlot, handle: WorkspaceHandle) {
    match slot.write() {
        Ok(mut guard) => *guard = Some(handle),
        Err(e) => log::warn!("[channels:runtime] register_workspace_handle: slot poisoned: {e}"),
    }
}

fn rebind_in(slot: &HandleSlot, workspace_dir: PathBuf) {
    let handle = match slot.read() {
        Ok(guard) => guard.as_ref().map(Arc::clone),
        Err(e) => {
            log::warn!("[channels:runtime] rebind_workspace: slot read poisoned: {e}");
            return;
        }
    };
    let Some(handle) = handle else {
        // The channel runtime isn't running in this process (e.g. CLI-only
        // paths, or login before `start_channels`). Nothing to re-bind — the
        // runtime will resolve the active user from its startup `Config` load.
        log::debug!(
            "[channels:runtime] rebind_workspace: no live runtime handle registered; \
             skipping re-bind to {}",
            workspace_dir.display()
        );
        return;
    };
    match handle.write() {
        Ok(mut current) => {
            if *current != workspace_dir {
                log::info!(
                    "[channels:runtime] re-binding channel workspace {} -> {}",
                    current.display(),
                    workspace_dir.display()
                );
                *current = workspace_dir;
            }
        }
        Err(e) => log::warn!("[channels:runtime] rebind_workspace: handle poisoned: {e}"),
    };
}

/// Register the live runtime's workspace handle so [`rebind_workspace`] can
/// re-point it after login. Called once from `start_channels`.
pub(crate) fn register_workspace_handle(handle: WorkspaceHandle) {
    register_in(handle_slot(), handle);
}

/// Re-point the running channel runtime at `workspace_dir`.
///
/// Invoked from `credentials::ops::store_session` after activation, against
/// `effective_config.workspace_dir`. No-op when the channel runtime is not
/// running (no handle registered yet).
pub fn rebind_workspace(workspace_dir: PathBuf) {
    rebind_in(handle_slot(), workspace_dir);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn rebind_reresolves_registered_handle() {
        // A fresh, test-local slot — the `memory::global` test pattern — so this
        // assertion is deterministic under parallel test execution instead of
        // racing the process-global singleton.
        let slot: HandleSlot = RwLock::new(None);
        // The live `ChannelRuntimeContext` holds a clone of this same handle
        // and reads it via `workspace_dir()`, so a re-bind here is what every
        // channel turn observes on its next workspace resolution.
        let handle: WorkspaceHandle = Arc::new(RwLock::new(PathBuf::from("/users/local")));
        register_in(&slot, Arc::clone(&handle));
        assert_eq!(*handle.read().unwrap(), Path::new("/users/local"));

        rebind_in(&slot, PathBuf::from("/users/user-42"));

        assert_eq!(*handle.read().unwrap(), Path::new("/users/user-42"));
    }

    #[test]
    fn rebind_without_registered_handle_is_noop() {
        let slot: HandleSlot = RwLock::new(None);
        // Must not panic when the channel runtime isn't started in-process.
        rebind_in(&slot, PathBuf::from("/users/user-7"));
        assert!(slot.read().unwrap().is_none());
    }
}
