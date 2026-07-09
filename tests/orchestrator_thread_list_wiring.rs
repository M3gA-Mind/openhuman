//! Pins the orchestrator's conversation-thread routing (#4744).
//!
//! An agent-efficiency eval found that "list my recent conversation threads"
//! made the chat-tier orchestrator spawn a *Memory Agent* that walked the
//! memory tree (`memory_tree` / `memory_doctor`) instead of listing threads.
//! Root cause: the orchestrator had no correct route to the thread index — it
//! lacked the read-only `thread_list` tool directly, and the thread-owning
//! `context_scout` is not in its subagent allowlist — so the model fell back
//! to the closest specialist (`agent_memory`), the wrong tool.
//!
//! The fix makes `thread_list` a direct read-only tool on the orchestrator
//! (a zero-arg, local, single-call lookup — the same class as `file_read` /
//! `list`) and teaches the prompt to use it directly rather than delegating
//! to memory retrieval. These invariants pin that wiring.
//!
//! Exact-line matching (not substring) so a commented-out entry or a
//! prefixed name (`thread_list_v2`) can't satisfy the assertion accidentally.

const ORCHESTRATOR_TOML: &str =
    include_str!("../src/openhuman/agent_registry/agents/orchestrator/agent.toml");

const ORCHESTRATOR_PROMPT: &str =
    include_str!("../src/openhuman/agent_registry/agents/orchestrator/prompt.md");

/// True if `toml` lists `name` as a bare quoted array entry (`"name"` or
/// `"name",`), matching how the `named = [ … ]` / subagent allowlists are
/// written. Ignores leading indentation and trailing whitespace.
fn lists_named_tool(toml: &str, name: &str) -> bool {
    let bare = format!("\"{name}\"");
    let trailing = format!("\"{name}\",");
    toml.lines()
        .map(str::trim)
        .any(|line| line == bare || line == trailing)
}

#[test]
fn orchestrator_lists_thread_list_as_direct_tool() {
    assert!(
        lists_named_tool(ORCHESTRATOR_TOML, "thread_list"),
        "orchestrator must have `thread_list` as a direct read-only tool so \
         'list my recent threads' is one direct call, not a memory sub-agent \
         spawn that walks the memory tree (#4744)"
    );
}

#[test]
fn orchestrator_does_not_route_thread_listing_through_memory_subagent() {
    // The orchestrator reaches memory retrieval via the `agent_memory`
    // subagent (surfaced as `retrieve_memory`). It must NOT own `context_scout`
    // (the other `thread_list` holder) as a subagent — the direct tool is the
    // intended route. If a future change adds `context_scout` here, revisit
    // whether thread listing should still be direct.
    assert!(
        !lists_named_tool(ORCHESTRATOR_TOML, "context_scout"),
        "orchestrator is not expected to delegate to context_scout; thread \
         listing is served by the direct `thread_list` tool (#4744)"
    );
}

#[test]
fn prompt_teaches_direct_thread_listing() {
    // The capability alone isn't enough — the prompt must steer the model to
    // call `thread_list` directly instead of delegating to memory retrieval.
    assert!(
        ORCHESTRATOR_PROMPT.contains("thread_list"),
        "orchestrator prompt must mention `thread_list` so the model uses it \
         directly for thread-listing requests (#4744)"
    );
}
