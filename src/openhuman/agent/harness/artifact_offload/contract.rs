//! The prompt-side half of the artifact-offload convention (#3883).
//!
//! Single source of truth for the directory names the model is told about, so
//! the prompt and [`super::paths::resolve_artifact_path`] can never drift.

use super::types::{OUTPUTS_DIR, SCRATCH_DIR};

/// Approximate token budget quoted to the model as the offload trigger.
/// Mirrors [`super::types::DEFAULT_OFFLOAD_THRESHOLD_BYTES`] at the harness-wide
/// 4-chars-per-token estimate.
const CONTRACT_THRESHOLD_TOKENS: usize = 2_000;

/// Heading the contract is rendered under. Used by the idempotence check in
/// `subagent_runner::ops::prompt` so a re-rendered prompt never stacks two
/// copies.
pub const ARTIFACT_OFFLOAD_HEADING: &str = "## Long-horizon Artifact Offload";

/// Render the offload contract appended to every typed sub-agent prompt.
///
/// Deliberately parameter-free and byte-stable: the sub-agent system prompt is
/// prefix-cached, so this must render identically on every run. Absolute paths
/// are never interpolated for the same reason (and because a worktree-isolated
/// worker resolves its own action root).
pub fn render_artifact_offload_contract() -> String {
    format!(
        "{ARTIFACT_OFFLOAD_HEADING}\n\n\
Large results belong on disk, not in your reply. Two directories sit under your action directory:\n\
- `{OUTPUTS_DIR}/` for deliverables, anything the parent or a later step needs to read.\n\
- `{SCRATCH_DIR}/` for scratch, intermediate files you do not intend to hand back.\n\
\n\
When a result would run past roughly {CONTRACT_THRESHOLD_TOKENS} tokens:\n\
1. Write the full content to a file under `{OUTPUTS_DIR}/` with `file_write`.\n\
2. Reply with that relative path plus a short abstract, not the full payload.\n\
3. Quote the path exactly, so the parent can `file_read` it verbatim.\n\
\n\
Rules:\n\
- Offload paths are always relative to your action directory. Never write outside it, and never target the core's internal workspace state.\n\
- Keep the abstract honest: say what the file holds and what is still open. Never present it as the complete result.\n\
- Small results stay inline. A pointer to a two-line file costs the parent more than the two lines.\n\
- If you inline an oversized result anyway, the harness persists it under `{OUTPUTS_DIR}/` and hands the parent the path instead.\n"
    )
}
