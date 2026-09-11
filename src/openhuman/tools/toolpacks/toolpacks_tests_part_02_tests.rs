//! Tool-pack advertisement: the index the model reads, and the disclosure /
//! execution distinction the whole pack mechanism rests on.

use super::*;

// ── the index must agree with the gate too ──────────────────────────────────

/// A spec shaped like the one `LoadSkillTool` publishes.
fn load_skill_spec() -> crate::openhuman::tools::traits::ToolSpec {
    let tools = registry_with_all(&["build_workflow"]);
    let tool = find(&tools, LOAD_SKILL);
    crate::openhuman::tools::traits::ToolSpec {
        name: tool.name().to_string(),
        description: tool.description().to_string(),
        parameters: tool.parameters_schema(),
    }
}

/// The landing was fixed first; this is the invitation. A pack the session can
/// call nothing in must not be advertised as loadable — the model would go,
/// find out, and come back, which is a wasted round trip on every turn it is
/// tempted.
#[test]
fn the_index_drops_a_pack_this_session_can_call_nothing_in() {
    let mut spec = load_skill_spec();
    assert!(
        spec.description.contains("`system`"),
        "precondition: the unscoped index advertises every pack"
    );

    // Only the workflows pack is reachable.
    let workflows = pack("workflows").expect("workflows pack");
    let kept = scope_load_skill_spec(&mut spec, &|name| workflows.tools.contains(&name));
    assert!(
        kept,
        "workflows is callable, so load_skill stays on the wire"
    );

    assert!(
        spec.description.contains("`workflows`"),
        "a reachable pack must still be offered: {}",
        spec.description
    );
    for dead in ["`system`", "`crypto`", "`audio`", "`documents`"] {
        assert!(
            !spec.description.contains(dead),
            "{dead} has no callable tool here and must not be advertised: {}",
            spec.description
        );
    }
}

/// The schema is the stronger half: an unusable pack becomes unrepresentable,
/// not merely discouraged in prose.
#[test]
fn the_skill_enum_offers_only_reachable_packs() {
    let mut spec = load_skill_spec();
    let workflows = pack("workflows").expect("workflows pack");
    scope_load_skill_spec(&mut spec, &|name| workflows.tools.contains(&name));

    let values = spec
        .parameters
        .pointer("/properties/skill/enum")
        .and_then(Value::as_array)
        .expect("the skill enum survives scoping")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    assert_eq!(
        values,
        vec!["workflows"],
        "the enum must name exactly the reachable packs"
    );
}

/// An empty index and an empty enum are not a tool. The caller is told to drop
/// `load_skill` rather than ship one that can do nothing.
#[test]
fn a_session_that_can_reach_no_pack_loses_load_skill() {
    let mut spec = load_skill_spec();
    assert!(
        !scope_load_skill_spec(&mut spec, &|_| false),
        "with nothing reachable, load_skill must be dropped, not emptied"
    );
}

/// Scoping must not quietly cost more context than it saves — the index is
/// charged to every turn.
#[test]
fn scoping_the_index_only_ever_shrinks_it() {
    let mut spec = load_skill_spec();
    let before = spec.description.len();
    let workflows = pack("workflows").expect("workflows pack");
    scope_load_skill_spec(&mut spec, &|name| workflows.tools.contains(&name));
    assert!(
        spec.description.len() < before,
        "scoped index ({}) must be smaller than the full one ({before})",
        spec.description.len()
    );
}

/// **The contract this fix must not break** (AGENTS.md: `Withheld` = "Registered
/// and callable: yes"). A withheld packed tool is hidden from the prompt and
/// still callable through `use_skill` — that is the entire point of a pack.
///
/// `ToolPolicySession` records that state as `HideFromPrompt`, and
/// `ToolPolicyDecision::is_denied()` is `!matches!(action, Allow)` — so it
/// answers **true** for a perfectly callable packed tool. Any "can this session
/// call it" predicate built on `is_denied()` therefore eats the whole pack.
#[test]
fn a_withheld_packed_tool_is_hidden_but_not_denied() {
    use crate::openhuman::tools::agent_policy::{ToolPolicyAction, ToolPolicyEngine};

    let tools = registry_with_all(&["goal_set", "build_workflow"]);
    // The real shape: the harness seeds `visible` with everything, then
    // `strip_packed_from_visible` removes the packed names for a non-owner.
    let mut visible: HashSet<String> = tools.iter().map(|t| t.name().to_string()).collect();
    strip_packed_from_visible(&mut visible, "orchestrator");
    assert!(
        !visible.contains("goal_set"),
        "precondition: the packed tool is withheld from the prompt"
    );

    let session = ToolPolicyEngine::build_session(
        "orchestrator",
        "web_chat",
        "chat",
        &Default::default(),
        &tools,
        &visible,
    );

    let decision = session.decision_for("goal_set");
    assert_eq!(
        decision.action,
        ToolPolicyAction::HideFromPrompt,
        "a withheld packed tool is classified as prompt-hidden, not denied"
    );
    assert!(
        decision.is_denied(),
        "…and `is_denied()` nevertheless answers true for it — this is the trap"
    );
}

/// **The permission ceiling survives the disclosure exemption.**
///
/// `build_session_from_refs` tests `explicitly_hidden` before
/// `exceeds_permission`, so a tool that is both hidden and over the ceiling is
/// recorded as `HideFromPrompt` and never gets its permission verdict.
/// `is_denied()` masked that by blocking every hidden tool. A predicate that
/// deliberately admits hidden tools must carry the ceiling itself, or `use_skill`
/// becomes a laundering route into a tool the channel would refuse.
#[test]
fn a_hidden_tool_over_the_permission_ceiling_still_blocks_execution() {
    use crate::openhuman::tools::agent_policy::{ToolPolicyAction, ToolPolicyDecision};

    let over = ToolPolicyDecision {
        tool_name: "dangerous_packed_tool".to_string(),
        action: ToolPolicyAction::HideFromPrompt,
        required_permission: Some(PermissionLevel::Dangerous),
        allowed_permission: PermissionLevel::ReadOnly,
    };
    assert!(
        over.blocks_execution(),
        "a hidden tool above the session's ceiling must not be executable"
    );

    let within = ToolPolicyDecision {
        tool_name: "ordinary_packed_tool".to_string(),
        action: ToolPolicyAction::HideFromPrompt,
        required_permission: Some(PermissionLevel::ReadOnly),
        allowed_permission: PermissionLevel::Dangerous,
    };
    assert!(
        !within.blocks_execution(),
        "a hidden tool within the ceiling is the normal packed case and must stay callable"
    );
}
