//! Which billing route a recorded cost belongs to (issue #5016).
//!
//! The local `[cost]` daily/monthly limits exist to cap spend against
//! **OpenHuman-managed credits**. They were being enforced against *every*
//! recorded call, including bring-your-own-key (BYOK) and local inference that
//! OpenHuman never bills for. A BYOK user therefore accumulated phantom spend
//! — priced locally from [`super::catalog`] because their provider echoes no
//! `charged_amount_usd` — until they tripped the default $10/day cap and got
//! "You're out of credits", despite OpenHuman having charged them nothing.
//!
//! The route is derived from the recorded model id rather than threaded
//! through as a new parameter, which matters for two reasons:
//!
//! 1. Every recording site already has the model id, so no call site has to
//!    learn about routing.
//! 2. Records persisted by older builds carry a model id too, so history
//!    classifies correctly with **no migration and no schema change** — a
//!    stored-route field would have had to guess a default for every existing
//!    record and would keep mis-gating real users for up to a month.

/// The billing route a cost record belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostRoute {
    /// Served by the OpenHuman managed backend and paid for with OpenHuman
    /// credits. Counts toward — and is gated by — the local `[cost]` limits.
    Managed,
    /// Served by the user's own key (OpenRouter, Anthropic, a self-hosted
    /// OpenAI-compatible gateway, …) or a local model. OpenHuman bills nothing
    /// for it, so it is recorded for the dashboard but never gated.
    Byok,
}

impl CostRoute {
    /// Whether spend on this route counts toward the local `[cost]` budget.
    pub fn counts_toward_budget(self) -> bool {
        matches!(self, CostRoute::Managed)
    }
}

/// Model ids the managed backend serves. These are the backend's own tier
/// slugs (`crate::openhuman::config::MODEL_*_V1`) — a BYOK
/// provider is always addressed by its real model id (`minimax/minimax-m3`,
/// `anthropic/claude-sonnet-4-20250514`, `llama3:8b`), never by a tier slug,
/// because the tier vocabulary only means something to the managed backend.
///
/// Kept as a local list, deliberately: this is the set of ids that imply
/// *managed billing*, which is a narrower question than "is this a known tier
/// constant". A new tier must be added here consciously, and the
/// `managed_tier_slugs_stay_in_sync` test fails if one is added upstream
/// without doing so.
const MANAGED_MODEL_SLUGS: &[&str] = &[
    "chat-v1",
    "reasoning-v1",
    "reasoning-quick-v1",
    "agentic-v1",
    "burst-v1",
    "coding-v1",
    "vision-v1",
    "summarization-v1",
];

/// Classify a recorded model id into its billing route.
///
/// Unknown ids classify as [`CostRoute::Byok`], which is the safe direction:
/// the failure mode is under-enforcing a *local* convenience cap, not
/// over-charging. Real managed-credit exhaustion is still enforced
/// server-side, where the backend returns its own out-of-credits error — that
/// path is untouched by this classification.
pub fn route_for_model(model: &str) -> CostRoute {
    let normalized = normalize_model_id(model);
    if MANAGED_MODEL_SLUGS.contains(&normalized.as_str()) {
        CostRoute::Managed
    } else {
        CostRoute::Byok
    }
}

/// Lower-case, trim, and strip the decorations a model id can pick up on its
/// way to a cost record: a `hint:` prefix from the tier-resolution helpers and
/// an `openhuman/` provider qualifier.
fn normalize_model_id(model: &str) -> String {
    let lowered = model.trim().to_ascii_lowercase();
    let without_hint = lowered
        .as_str()
        .strip_prefix("hint:")
        .unwrap_or(lowered.as_str());
    without_hint
        .strip_prefix("openhuman/")
        .unwrap_or(without_hint)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::openhuman::config::{
        MODEL_AGENTIC_V1, MODEL_BURST_V1, MODEL_CHAT_V1, MODEL_CODING_V1, MODEL_REASONING_QUICK_V1,
        MODEL_REASONING_V1, MODEL_SUMMARIZATION_V1, MODEL_VISION_V1,
    };

    #[test]
    fn managed_tier_slugs_are_managed() {
        for slug in MANAGED_MODEL_SLUGS {
            assert_eq!(
                route_for_model(slug),
                CostRoute::Managed,
                "tier slug {slug} must bill as managed"
            );
        }
    }

    /// The local slug list must not drift from the backend tier constants. If a
    /// new tier is introduced upstream, this fails until it is classified here
    /// — otherwise managed spend on the new tier would silently stop counting
    /// toward the budget.
    #[test]
    fn managed_tier_slugs_stay_in_sync() {
        let upstream = [
            MODEL_CHAT_V1,
            MODEL_REASONING_V1,
            MODEL_REASONING_QUICK_V1,
            MODEL_AGENTIC_V1,
            MODEL_BURST_V1,
            MODEL_CODING_V1,
            MODEL_VISION_V1,
            MODEL_SUMMARIZATION_V1,
        ];
        for tier in upstream {
            assert!(
                MANAGED_MODEL_SLUGS.contains(&tier),
                "backend tier {tier} is not classified as managed"
            );
        }
        assert_eq!(
            MANAGED_MODEL_SLUGS.len(),
            upstream.len(),
            "MANAGED_MODEL_SLUGS has an entry with no matching backend tier constant"
        );
    }

    #[test]
    fn byok_and_local_models_are_not_managed() {
        // The exact models from #5016 / #5127 (OpenRouter + a self-hosted
        // OpenAI-compatible gateway) and other common BYOK / local shapes.
        for model in [
            "minimax/minimax-m3",
            "anthropic/claude-sonnet-4-20250514",
            "openai/gpt-4o",
            "llama3:8b",
            "ollama:gemma3:1b-it-qat",
            "lmstudio:qwen2.5-coder",
            "",
        ] {
            assert_eq!(
                route_for_model(model),
                CostRoute::Byok,
                "{model} must not bill as managed"
            );
        }
    }

    #[test]
    fn normalizes_case_whitespace_and_prefixes() {
        assert_eq!(route_for_model("  Chat-V1 "), CostRoute::Managed);
        assert_eq!(route_for_model("hint:chat-v1"), CostRoute::Managed);
        assert_eq!(
            route_for_model("openhuman/reasoning-v1"),
            CostRoute::Managed
        );
    }

    #[test]
    fn a_byok_model_merely_containing_a_tier_name_is_not_managed() {
        // Substring matching would misclassify these and silently re-introduce
        // the phantom limit for the user.
        for model in ["vendor/chat-v1-turbo", "my-chat-v1", "chat-v1x"] {
            assert_eq!(route_for_model(model), CostRoute::Byok, "{model}");
        }
    }

    #[test]
    fn only_managed_counts_toward_budget() {
        assert!(CostRoute::Managed.counts_toward_budget());
        assert!(!CostRoute::Byok.counts_toward_budget());
    }
}
