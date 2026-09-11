use super::*;

// ── the listing must agree with the gate (#workflow-routing) ────────────────
//
// End to end at the only layer that holds both halves: the session allowlist
// that `use_skill` is gated on, and the tool set the listing is rendered from.
// The unit tests in `tools/toolpacks/toolpacks_tests.rs` pin the filter itself;
// these pin that the middleware feeds it the *session's* answer and resolves
// the route out of the session's own delegation tools.

use crate::openhuman::agent::orchestration::tools::{ArchetypeDelegationTool, DelegationTarget};
use crate::openhuman::tools::agent_policy::{
    TaskProfile, TaskRiskLevel, ToolCapability, ToolPolicyAction, ToolPolicyDecision,
    ToolPolicySession,
};
use crate::openhuman::tools::toolpacks::{append_pack_tools, bind_pack_registry, LOAD_SKILL};
use crate::openhuman::tools::traits::{PermissionLevel, ToolResult};

struct RoutingFakeTool(&'static str);

#[async_trait]
impl crate::openhuman::tools::traits::Tool for RoutingFakeTool {
    fn name(&self) -> &str {
        self.0
    }
    fn description(&self) -> &str {
        "fake"
    }
    fn parameters_schema(&self) -> serde_json::Value {
        json!({ "type": "object" })
    }
    async fn execute(&self, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        Ok(ToolResult::success("ok"))
    }
    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::ReadOnly
    }
}

fn allow(name: &str) -> (String, ToolPolicyDecision) {
    (
        name.to_string(),
        ToolPolicyDecision {
            tool_name: name.to_string(),
            action: ToolPolicyAction::Allow,
            required_permission: None,
            allowed_permission: PermissionLevel::Dangerous,
        },
    )
}

/// A non-owner session: it holds the whole `workflows` pack in its registry and
/// a `build_workflow` delegate, but may only call the delegate and the two pack
/// tools — exactly the orchestrator's shape. Anything not named here is denied,
/// because `decision_for` defaults to `Deny`.
fn non_owner_middleware() -> ToolPolicyMiddleware {
    let mut tools: Vec<Box<dyn crate::openhuman::tools::traits::Tool>> = vec![
        Box::new(RoutingFakeTool("build_workflow")),
        Box::new(RoutingFakeTool("propose_workflow")),
        Box::new(ArchetypeDelegationTool {
            tool_name: "build_workflow".to_string(),
            agent_id: DelegationTarget("workflow_builder".to_string()),
            tool_description: "Build a workflow".to_string(),
        }),
    ];
    append_pack_tools(&mut tools);
    let tools = Arc::new(tools);
    bind_pack_registry(&tools);

    let session = ToolPolicySession {
        profile: TaskProfile {
            agent_id: "orchestrator".to_string(),
            channel: "web_chat".to_string(),
            entrypoint: "chat".to_string(),
            risk_level: TaskRiskLevel::Low,
            allowed_permission: PermissionLevel::Dangerous,
        },
        capabilities: vec![ToolCapability {
            name: "build_workflow".to_string(),
            required_permission: PermissionLevel::ReadOnly,
        }],
        allowed_tool_names: ["build_workflow", LOAD_SKILL, "use_skill"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        blocked_tool_names: ["propose_workflow"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        hidden_tool_names: Default::default(),
        decisions: [
            allow("build_workflow"),
            allow(LOAD_SKILL),
            allow("use_skill"),
        ]
        .into_iter()
        .collect(),
    };

    ToolPolicyMiddleware::new(
        Arc::new(crate::openhuman::agent::tool_policy::AllowAllToolPolicy::default()),
        session,
        vec![tools],
        "sess".to_string(),
        "web_chat".to_string(),
        "orchestrator".to_string(),
    )
}

fn call(name: &str, args: serde_json::Value) -> TaToolCall {
    TaToolCall {
        id: "call-1".to_string(),
        name: name.to_string(),
        arguments: args,
        invalid: None,
    }
}

/// The bug: the orchestrator loaded `workflows`, read `propose_workflow` off
/// the listing, called it, and was refused. The listing must not offer it.
#[tokio::test]
async fn load_skill_hides_what_this_session_cannot_call_and_names_the_route() {
    let mw = non_owner_middleware();
    let result = mw
        .render_skill_for_session(&call(LOAD_SKILL, json!({ "skill": "workflows" })))
        .expect("a load_skill call with a skill argument renders here");

    assert!(
        !result.content.contains("propose_workflow"),
        "a non-owner must not be offered a tool the gate will refuse:\n{}",
        result.content
    );
    // Assert the LISTING entry in its exact rendered form, not the bare name.
    //
    // `build_workflow` occurs in two different outputs: a rendered listing
    // (`## \`build_workflow\``) and the route sentence used when nothing is
    // callable (``Call `build_workflow` instead``). A bare `contains` therefore
    // passed even when the filter had dropped every tool and the render had
    // fallen through to its error path — i.e. it passed in the exact case this
    // test exists to catch. A check that cannot fail reads as coverage and is
    // worse than none.
    assert!(
        result.content.contains("## `build_workflow`"),
        "the tool it CAN call must still be LISTED, not merely mentioned in a \
         route sentence:\n{}",
        result.content
    );
    // And this really is a listing, not the "nothing callable" error.
    assert!(
        result.content.starts_with("# Skill `workflows`"),
        "expected a rendered pack listing:\n{}",
        result.content
    );
}

/// And when it calls the denied tool anyway, the refusal has to carry the way
/// out — resolved from a delegate this session actually holds.
#[tokio::test]
async fn a_use_skill_denial_names_a_delegate_this_session_can_call() {
    let mw = non_owner_middleware();
    let message = mw
        .channel_permission_block(&call(
            "use_skill",
            json!({ "skill": "workflows", "tool": "propose_workflow" }),
        ))
        .expect("propose_workflow is denied for a non-owner");

    assert!(
        message.contains("not allowed in the current session"),
        "the denial itself must survive: {message}"
    );
    assert!(
        message.contains("build_workflow"),
        "the denial must name the delegate to call instead: {message}"
    );
}

/// The route is only trustworthy because it is read off the session's own tool
/// set. A session with no delegate to `workflow_builder` must fall back to
/// naming the owning agents rather than inventing a call it cannot make.
#[tokio::test]
async fn a_session_without_the_delegate_is_not_told_to_call_it() {
    let mut tools: Vec<Box<dyn crate::openhuman::tools::traits::Tool>> =
        vec![Box::new(RoutingFakeTool("propose_workflow"))];
    append_pack_tools(&mut tools);
    let tools = Arc::new(tools);
    bind_pack_registry(&tools);

    let session = ToolPolicySession {
        profile: TaskProfile {
            agent_id: "orchestrator".to_string(),
            channel: "web_chat".to_string(),
            entrypoint: "chat".to_string(),
            risk_level: TaskRiskLevel::Low,
            allowed_permission: PermissionLevel::Dangerous,
        },
        capabilities: vec![],
        allowed_tool_names: [LOAD_SKILL, "use_skill"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        blocked_tool_names: ["propose_workflow"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        hidden_tool_names: Default::default(),
        decisions: [allow(LOAD_SKILL), allow("use_skill")]
            .into_iter()
            .collect(),
    };
    let mw = ToolPolicyMiddleware::new(
        Arc::new(crate::openhuman::agent::tool_policy::AllowAllToolPolicy::default()),
        session,
        vec![tools],
        "sess".to_string(),
        "web_chat".to_string(),
        "orchestrator".to_string(),
    );

    let message = mw
        .channel_permission_block(&call(
            "use_skill",
            json!({ "skill": "workflows", "tool": "propose_workflow" }),
        ))
        .expect("propose_workflow is denied");

    assert!(
        message.contains("workflow_builder"),
        "with no delegate to call, name the owning agent: {message}"
    );
    assert!(
        !message.contains("Call `build_workflow`"),
        "must not instruct a call this session cannot make: {message}"
    );
}

/// **Pre-existing, and larger than the bug this branch was opened for.**
///
/// `channel_permission_block`'s inner `use_skill` check denies on
/// `decision.is_denied()`, which is true for `HideFromPrompt`. Every withheld
/// packed tool is `HideFromPrompt` for a non-owner — that is what "withheld"
/// means. So `use_skill`, the *only* advertised route to a withheld tool, was
/// refusing all of them. AGENTS.md's disclosure table says `Withheld` is
/// "Registered and callable: yes".
#[tokio::test]
async fn use_skill_reaches_a_withheld_packed_tool() {
    use crate::openhuman::tools::agent_policy::ToolPolicyEngine;
    use crate::openhuman::tools::toolpacks::strip_packed_from_visible;

    let mut tools: Vec<Box<dyn crate::openhuman::tools::traits::Tool>> =
        vec![Box::new(RoutingFakeTool("goal_set"))];
    append_pack_tools(&mut tools);
    let tools = Arc::new(tools);
    bind_pack_registry(&tools);

    // The real harness shape: seed `visible` with everything, then withhold the
    // packed names for a non-owner.
    let mut visible: std::collections::HashSet<String> =
        tools.iter().map(|t| t.name().to_string()).collect();
    strip_packed_from_visible(&mut visible, "orchestrator");
    let session = ToolPolicyEngine::build_session(
        "orchestrator",
        "web_chat",
        "chat",
        &Default::default(),
        &tools,
        &visible,
    );

    let mw = ToolPolicyMiddleware::new(
        Arc::new(crate::openhuman::agent::tool_policy::AllowAllToolPolicy::default()),
        session,
        vec![tools],
        "sess".to_string(),
        "web_chat".to_string(),
        "orchestrator".to_string(),
    );

    assert!(
        mw.channel_permission_block(&call(
            "use_skill",
            json!({ "skill": "goals", "tool": "goal_set" }),
        ))
        .is_none(),
        "a withheld packed tool must stay reachable through use_skill — that is \
         the only route it has"
    );
}

/// **A route hint must use the predicate of the gate it points at.**
///
/// `route_sentence` tells the model to make a *direct* call, and the direct-call
/// gate rejects `HideFromPrompt`. So a prompt-hidden delegate is not a route:
/// naming it would send the model into the same refusal this branch exists to
/// remove, one hop later. The hint must fall back to the owning agent instead.
#[tokio::test]
async fn a_prompt_hidden_delegate_is_not_offered_as_a_direct_route() {
    use crate::openhuman::tools::agent_policy::ToolPolicyEngine;
    use crate::openhuman::tools::toolpacks::strip_packed_from_visible;

    let mut tools: Vec<Box<dyn crate::openhuman::tools::traits::Tool>> = vec![
        Box::new(RoutingFakeTool("propose_workflow")),
        Box::new(ArchetypeDelegationTool {
            tool_name: "build_workflow".to_string(),
            agent_id: DelegationTarget("workflow_builder".to_string()),
            tool_description: "Build a workflow".to_string(),
        }),
    ];
    append_pack_tools(&mut tools);
    let tools = Arc::new(tools);
    bind_pack_registry(&tools);

    // `build_workflow` is itself a member of the `workflows` pack, so
    // `strip_packed_from_visible` withholds it for a non-owner — leaving the
    // delegate prompt-hidden and therefore NOT directly callable.
    let mut visible: std::collections::HashSet<String> =
        tools.iter().map(|t| t.name().to_string()).collect();
    strip_packed_from_visible(&mut visible, "orchestrator");
    assert!(
        !visible.contains("build_workflow"),
        "precondition: the delegate is prompt-hidden here"
    );

    let session = ToolPolicyEngine::build_session(
        "orchestrator",
        "web_chat",
        "chat",
        &Default::default(),
        &tools,
        &visible,
    );
    let mw = ToolPolicyMiddleware::new(
        Arc::new(crate::openhuman::agent::tool_policy::AllowAllToolPolicy::default()),
        session,
        vec![tools],
        "sess".to_string(),
        "web_chat".to_string(),
        "orchestrator".to_string(),
    );

    let pack = crate::openhuman::tools::toolpacks::pack("workflows").expect("workflows pack");
    let route = mw.route_for_pack(pack);
    assert!(
        !route.contains("Call `build_workflow`"),
        "a prompt-hidden delegate must not be advertised as a direct call: {route}"
    );
    assert!(
        route.contains("workflow_builder"),
        "the hint must fall back to naming the owning agent: {route}"
    );
}

/// **The third variant of the same defect, pinned.**
///
/// A delegate can be `Allow` on the action and still be refused: the gate's
/// second check is `call_required > decision.allowed_permission`, and a
/// synthesised `delegate_*` tool declares `PermissionLevel::Execute`. On any
/// session whose ceiling is lower, *every* delegate is over it.
///
/// Testing the action alone let such a delegate into the route sentence, and
/// the gate then refused the call it had just been told to make. The hint now
/// calls `direct_call_refusal` — the gate itself — so the ceiling is included
/// by construction rather than by a third special case.
#[tokio::test]
async fn a_delegate_over_the_permission_ceiling_is_not_a_route() {
    use crate::openhuman::tools::agent_policy::{
        TaskProfile, TaskRiskLevel, ToolPolicyAction, ToolPolicyDecision, ToolPolicySession,
    };

    let mut tools: Vec<Box<dyn crate::openhuman::tools::traits::Tool>> = vec![
        Box::new(RoutingFakeTool("propose_workflow")),
        // Declares `PermissionLevel::Execute`, like every synthesised delegate.
        Box::new(ArchetypeDelegationTool {
            tool_name: "build_workflow".to_string(),
            agent_id: DelegationTarget("workflow_builder".to_string()),
            tool_description: "Build a workflow".to_string(),
        }),
    ];
    append_pack_tools(&mut tools);
    let tools = Arc::new(tools);
    bind_pack_registry(&tools);

    // ALLOW on the action — the delegate is visible and permitted — but the
    // session's ceiling is below what the delegate requires.
    let allowed_ceiling = PermissionLevel::ReadOnly;
    let session = ToolPolicySession {
        profile: TaskProfile {
            agent_id: "orchestrator".to_string(),
            channel: "web_chat".to_string(),
            entrypoint: "chat".to_string(),
            risk_level: TaskRiskLevel::Low,
            allowed_permission: allowed_ceiling,
        },
        capabilities: vec![],
        allowed_tool_names: ["build_workflow"].into_iter().map(str::to_string).collect(),
        blocked_tool_names: Default::default(),
        hidden_tool_names: Default::default(),
        decisions: [(
            "build_workflow".to_string(),
            ToolPolicyDecision {
                tool_name: "build_workflow".to_string(),
                action: ToolPolicyAction::Allow,
                required_permission: Some(PermissionLevel::Execute),
                allowed_permission: allowed_ceiling,
            },
        )]
        .into_iter()
        .collect(),
    };

    let mw = ToolPolicyMiddleware::new(
        Arc::new(crate::openhuman::agent::tool_policy::AllowAllToolPolicy::default()),
        session,
        vec![tools],
        "sess".to_string(),
        "web_chat".to_string(),
        "orchestrator".to_string(),
    );

    // Precondition: the gate really would refuse this call, so the hint has
    // something to be wrong about.
    assert!(
        mw.direct_call_refusal("build_workflow", &serde_json::Value::Null)
            .is_some(),
        "precondition: the ceiling refuses this delegate"
    );

    let pack = crate::openhuman::tools::toolpacks::pack("workflows").expect("workflows pack");
    let route = mw.route_for_pack(pack);
    assert!(
        !route.contains("Call `build_workflow`"),
        "a delegate the gate will refuse on the ceiling must not be advertised \
         as a direct call: {route}"
    );
    assert!(
        route.contains("workflow_builder"),
        "the hint must fall back to naming the owning agent: {route}"
    );
}
