//! Structured, actionable messages for policy / permission denials.
//!
//! When the harness blocks a tool call at a policy or permission boundary, the
//! agent must not dead-end with a bare "blocked" line. Each denial is rendered
//! into a structured message — **what** was blocked, **why**, and a concrete
//! **workaround** (how to enable it, or a permitted alternative) — followed by
//! an explicit instruction to relay it to the user rather than halting
//! silently. The rendered string is returned as the (failed) tool result, so it
//! flows back into the turn the same way the unknown-tool corrective error is
//! surfaced to the model (see PR #4360).

use crate::openhuman::tools::PermissionLevel;

/// The boundary that blocked a tool call, with the context needed to explain it
/// and suggest a way forward.
pub(super) enum PolicyDenial<'a> {
    /// The session tool policy forbids this tool for the channel's permission
    /// tier (it is not in the allowed set).
    SessionForbidden {
        tool: &'a str,
        required: Option<PermissionLevel>,
        allowed: PermissionLevel,
        channel: &'a str,
    },
    /// The tool is allowed in general, but *this call's* arguments require a
    /// higher permission than the channel grants.
    PermissionTooLow {
        tool: &'a str,
        required: PermissionLevel,
        allowed: PermissionLevel,
        channel: &'a str,
    },
    /// A pluggable `ToolPolicy`
    /// denied the call outright.
    PolicyDenied {
        tool: &'a str,
        policy: &'a str,
        reason: &'a str,
    },
    /// A pluggable `ToolPolicy`
    /// requires an approval handoff this executor cannot complete inline.
    ApprovalRequired {
        tool: &'a str,
        policy: &'a str,
        reason: &'a str,
    },
}

/// Suffix appended to every denial so the agent relays the block instead of
/// silently stopping.
const RELAY_INSTRUCTION: &str = "Relay this to the user: explain what was \
    blocked and why, then offer the workaround as the next step. Do not stop \
    silently.";

impl PolicyDenial<'_> {
    /// Render the denial as a structured `Blocked / Reason / Workaround / relay`
    /// message for the model.
    pub(super) fn render(&self) -> String {
        let (blocked, reason, workaround) = match self {
            PolicyDenial::SessionForbidden {
                tool,
                required,
                allowed,
                channel,
            } => {
                let reason = match required {
                    Some(required) => format!(
                        "it requires {required} permission, but the '{channel}' channel only \
                         grants {allowed} access"
                    ),
                    None => format!(
                        "it is not permitted at the '{channel}' channel's {allowed} access tier"
                    ),
                };
                (
                    format!("Tool '{tool}' is blocked by the session tool policy"),
                    reason,
                    raise_tier_workaround(
                        required.map(|p| p.to_string()).as_deref(),
                        *allowed,
                        channel,
                    ),
                )
            }
            PolicyDenial::PermissionTooLow {
                tool,
                required,
                allowed,
                channel,
            } => (
                format!("Tool '{tool}' is blocked by a per-call permission check"),
                format!(
                    "this call needs {required} permission, but the '{channel}' channel only \
                     grants {allowed} access"
                ),
                raise_tier_workaround(Some(&required.to_string()), *allowed, channel),
            ),
            PolicyDenial::PolicyDenied {
                tool,
                policy,
                reason,
            } => (
                format!("Tool '{tool}' was denied by policy '{policy}'"),
                (*reason).to_string(),
                "Address the reason above, or reach the goal with a permitted alternative tool / \
                 path. If this action is genuinely required, ask the user to adjust the policy."
                    .to_string(),
            ),
            PolicyDenial::ApprovalRequired {
                tool,
                policy,
                reason,
            } => (
                format!("Tool '{tool}' requires approval under policy '{policy}'"),
                (*reason).to_string(),
                "Ask the user to approve this action, then retry — or choose an alternative that \
                 does not require approval."
                    .to_string(),
            ),
        };

        format!(
            "Blocked: {blocked}. Reason: {reason}. Workaround: {workaround} {RELAY_INSTRUCTION}"
        )
    }
}

/// Workaround shared by the permission-tier denials: raise the channel's
/// agent-access tier, or fall back to a lower-permission tool.
fn raise_tier_workaround(
    required: Option<&str>,
    allowed: PermissionLevel,
    channel: &str,
) -> String {
    match required {
        Some(required) => format!(
            "Raise the '{channel}' channel's agent-access tier to at least {required} \
             (Settings → Agent access, or the `config.update_autonomy_settings` RPC / \
             `[autonomy]` config), or accomplish the goal with a tool that needs only \
             {allowed} access."
        ),
        None => format!(
            "Raise the '{channel}' channel's agent-access tier (Settings → Agent access, or the \
             `config.update_autonomy_settings` RPC / `[autonomy]` config), or accomplish the goal \
             with a tool that needs only {allowed} access."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_forbidden_with_required_lists_reason_and_workaround() {
        let msg = PolicyDenial::SessionForbidden {
            tool: "run_script",
            required: Some(PermissionLevel::Execute),
            allowed: PermissionLevel::ReadOnly,
            channel: "web",
        }
        .render();

        assert!(msg.starts_with("Blocked: Tool 'run_script'"));
        assert!(msg.contains("Reason:"));
        assert!(msg.contains("requires Execute permission"));
        assert!(msg.contains("Workaround:"));
        assert!(msg.contains("agent-access tier"));
        // The relay instruction is what keeps the agent from halting silently.
        assert!(msg.contains("Relay this to the user"));
    }

    #[test]
    fn session_forbidden_without_required_still_has_workaround() {
        let msg = PolicyDenial::SessionForbidden {
            tool: "run_script",
            required: None,
            allowed: PermissionLevel::ReadOnly,
            channel: "cron",
        }
        .render();

        assert!(msg.contains("not permitted"));
        assert!(msg.contains("Workaround:"));
        assert!(msg.contains("Relay this to the user"));
    }

    #[test]
    fn permission_too_low_names_both_levels() {
        let msg = PolicyDenial::PermissionTooLow {
            tool: "shell",
            required: PermissionLevel::Write,
            allowed: PermissionLevel::ReadOnly,
            channel: "web",
        }
        .render();

        assert!(msg.contains("needs Write permission"));
        assert!(msg.contains("only grants ReadOnly"));
        assert!(msg.contains("Workaround:"));
    }

    #[test]
    fn policy_denied_carries_reason_and_alternative() {
        let msg = PolicyDenial::PolicyDenied {
            tool: "run_script",
            policy: "sandbox",
            reason: "sandbox restriction",
        }
        .render();

        assert!(msg.contains("denied by policy 'sandbox'"));
        assert!(msg.contains("sandbox restriction"));
        assert!(msg.contains("permitted alternative"));
        assert!(msg.contains("Relay this to the user"));
    }

    #[test]
    fn approval_required_suggests_approval_then_retry() {
        let msg = PolicyDenial::ApprovalRequired {
            tool: "send_email",
            policy: "approval_gate",
            reason: "outbound message needs sign-off",
        }
        .render();

        assert!(msg.contains("requires approval under policy 'approval_gate'"));
        assert!(msg.contains("approve this action"));
        assert!(msg.contains("Relay this to the user"));
    }
}
