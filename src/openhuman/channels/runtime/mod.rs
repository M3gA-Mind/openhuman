//! Channel runtime entry points.

mod dispatch;
mod memory_rebind;
mod startup;
mod supervision;
mod workspace;

pub use memory_rebind::rebind_memory;
pub use startup::start_channels;
pub use workspace::rebind_workspace;

#[cfg(any(test, debug_assertions))]
pub mod test_support;

// Re-exported for `channels::tests` only; omit in normal lib builds to avoid unused-import warnings.
#[cfg(test)]
pub(crate) use dispatch::{process_channel_message, run_message_dispatch_loop};
#[cfg(test)]
pub(crate) use supervision::spawn_supervised_listener;
