//! Re-bindable channel-runtime memory store (issue #4398).
//!
//! The channel runtime builds its OWN local memory store at boot
//! (`create_memory_with_local_ai`) and hands it to the dispatch loop via
//! [`ChannelRuntimeContext`](crate::openhuman::channels::context::ChannelRuntimeContext).
//! Channel-level memory writes — conversation auto-save and memory-context
//! retrieval — go through that store (agent *tools* separately use the
//! already-rebound `memory::global` client). When the runtime starts pre-login,
//! that store is rooted at `~/.openhuman/users/local/`, so post-login channel
//! turns keep reading/writing the wrong workspace's memory DB until a restart.
//!
//! This module holds a process-global handle to the context's memory slot
//! (`Arc<RwLock<Arc<dyn Memory>>>`) registered at boot, plus [`rebind_memory`]
//! which rebuilds the store for the activated user's workspace and swaps it in
//! place — the same shape as the workspace / live-policy re-bind seams. The
//! swap is race-safe: an in-flight turn keeps the old `Arc<dyn Memory>` it
//! already cloned; new turns pick up the new store.

use std::sync::{Arc, OnceLock, RwLock};

use anyhow::Result;

use crate::openhuman::config::Config;
use crate::openhuman::memory::Memory;
use crate::openhuman::memory_store;

/// In-place-swappable memory store held by the live runtime context.
pub(crate) type MemoryHandle = Arc<RwLock<Arc<dyn Memory>>>;

type HandleSlot = RwLock<Option<MemoryHandle>>;

static MEMORY_HANDLE: OnceLock<HandleSlot> = OnceLock::new();

fn handle_slot() -> &'static HandleSlot {
    MEMORY_HANDLE.get_or_init(|| RwLock::new(None))
}

/// Build the channel-runtime memory store for `config`'s workspace.
///
/// Mirrors the boot construction, including the keyword-only fallback (#3712):
/// if the configured embedder can't be built, degrade to
/// `embedding_provider = "none"` (NoopEmbedding) rather than failing, so the
/// channel runtime keeps working with reduced (keyword-only) memory. Shared by
/// `start_channels` (boot) and [`rebind_memory`] (post-login) so both paths
/// build the store identically.
pub(crate) fn build_channel_memory(config: &Config) -> Result<Arc<dyn Memory>> {
    let local_embedding = config.workload_local_model("embeddings");
    let embedding_api_key =
        crate::openhuman::embeddings::resolve_api_key(config, &config.memory.embedding_provider);
    match memory_store::create_memory_with_local_ai(
        &config.memory,
        local_embedding.as_deref(),
        &embedding_api_key,
        &[],
        Some(&config.storage.provider.config),
        &config.workspace_dir,
    ) {
        Ok(mem) => Ok(Arc::from(mem)),
        Err(e) => {
            tracing::error!(
                error = %format!("{e:#}"),
                provider = %config.memory.embedding_provider,
                "[channels] memory embedder build failed — falling back to keyword-only \
                 memory so channels still start"
            );
            let mut fallback_memory = config.memory.clone();
            fallback_memory.embedding_provider = "none".to_string();
            Ok(Arc::from(memory_store::create_memory_with_local_ai(
                &fallback_memory,
                local_embedding.as_deref(),
                &embedding_api_key,
                &[],
                Some(&config.storage.provider.config),
                &config.workspace_dir,
            )?))
        }
    }
}

fn register_in(slot: &HandleSlot, handle: MemoryHandle) {
    match slot.write() {
        Ok(mut guard) => *guard = Some(handle),
        Err(e) => log::warn!("[channels:runtime] register_memory_handle: slot poisoned: {e}"),
    }
}

fn rebind_in(slot: &HandleSlot, config: &Config) {
    let handle = match slot.read() {
        Ok(guard) => guard.as_ref().map(Arc::clone),
        Err(e) => {
            log::warn!("[channels:runtime] rebind_memory: slot read poisoned: {e}");
            return;
        }
    };
    let Some(handle) = handle else {
        log::debug!(
            "[channels:runtime] rebind_memory: no live runtime memory handle registered; \
             skipping re-bind to {}",
            config.workspace_dir.display()
        );
        return;
    };
    match build_channel_memory(config) {
        Ok(new_mem) => match handle.write() {
            Ok(mut current) => {
                log::info!(
                    "[channels:runtime] re-binding channel memory store to workspace {}",
                    config.workspace_dir.display()
                );
                *current = new_mem;
            }
            Err(e) => log::warn!("[channels:runtime] rebind_memory: handle poisoned: {e}"),
        },
        Err(e) => log::warn!(
            "[channels:runtime] rebind_memory: rebuild for {} failed; keeping existing store: {e:#}",
            config.workspace_dir.display()
        ),
    }
}

/// Register the live runtime's memory handle so [`rebind_memory`] can swap it
/// after login. Called once from `start_channels`.
pub(crate) fn register_memory_handle(handle: MemoryHandle) {
    register_in(handle_slot(), handle);
}

/// Rebuild the channel-runtime memory store for `config`'s workspace and swap
/// it into the live context. Invoked from `credentials::ops::store_session`
/// after activation. No-op when the channel runtime is not running (no handle
/// registered) or when the rebuild fails (keeps the existing store).
pub fn rebind_memory(config: &Config) {
    rebind_in(handle_slot(), config);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebind_without_registered_handle_is_noop() {
        // With no handle registered, `rebind_in` must return before it ever
        // tries to build a store — so this neither panics nor touches disk.
        let slot: HandleSlot = RwLock::new(None);
        let config = crate::openhuman::config::Config::default();
        rebind_in(&slot, &config);
        assert!(slot.read().unwrap().is_none());
    }
}
