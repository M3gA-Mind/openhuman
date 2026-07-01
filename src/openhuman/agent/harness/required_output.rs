//! Required structured-output validation & repair (issue #4117).
//!
//! Some agent contracts mandate a JSON block — e.g. a `thoughts` block like
//! `{"thoughts": "…", "next_action": "…"}` — on **every** turn, because
//! downstream parsing/routing depends on it. Models frequently omit the block
//! entirely, leaving those consumers with nothing.
//!
//! This module supplies the pure primitives the turn engine uses to guarantee a
//! well-formed block on every accepted turn:
//!
//! * [`output_satisfies_contract`] — validate presence + shape of the block,
//! * [`repair_instruction`] — the corrective re-prompt that asks the model to
//!   re-emit its reply with the block, and
//! * [`synthesize_block`] — a minimal, schema-valid block used as a deterministic
//!   fallback when the re-prompt also omits it.
//!
//! The orchestration that ties these together (validate → re-prompt → synthesize)
//! lives on the session in `session::turn::session_io` so it can drive the extra
//! provider call; keeping the logic here pure keeps it unit-testable without a
//! provider.

use crate::openhuman::config::RequiredOutputContract;

/// Whether `text` satisfies `contract`: it contains a JSON object carrying every
/// required key with a non-null value. An inert contract (no non-blank keys) is
/// treated as always satisfied so enforcement is a no-op.
pub(crate) fn output_satisfies_contract(text: &str, contract: &RequiredOutputContract) -> bool {
    if !contract.is_active() {
        return true;
    }
    find_required_block(text, contract).is_some()
}

/// The first JSON object in `text` that carries every required key with a
/// non-null value, or `None`. Reuses the harness JSON extractor so fenced,
/// inline, and whole-object replies are all recognised.
pub(crate) fn find_required_block(
    text: &str,
    contract: &RequiredOutputContract,
) -> Option<serde_json::Value> {
    let keys = contract.all_keys();
    if keys.is_empty() {
        return None;
    }
    for value in super::parse::extract_json_values(text) {
        if let Some(obj) = value.as_object() {
            let has_all = keys
                .iter()
                .all(|key| obj.get(key).is_some_and(|v| !v.is_null()));
            if has_all {
                return Some(value);
            }
        }
    }
    None
}

/// A minimal, schema-valid block synthesised when the model omits the block and
/// a corrective re-prompt fails to recover it. Every required key maps to an
/// empty string so downstream parsing always has a well-formed object to
/// consume. Returns `"{}"` only for an inert contract (which enforcement never
/// reaches).
pub(crate) fn synthesize_block(contract: &RequiredOutputContract) -> String {
    let mut obj = serde_json::Map::new();
    for key in contract.all_keys() {
        obj.insert(key, serde_json::Value::String(String::new()));
    }
    serde_json::to_string(&serde_json::Value::Object(obj)).unwrap_or_else(|_| "{}".to_string())
}

/// The corrective instruction that re-prompts the model to re-emit its reply
/// with the required block. Mirrors the iteration-cap checkpoint re-prompt: a
/// self-contained user turn appended after the omitting reply.
pub(crate) fn repair_instruction(contract: &RequiredOutputContract) -> String {
    let keys = contract.all_keys().join("\", \"");
    format!(
        "Your previous reply omitted the required JSON `{}` block that every turn must include. \
Reply again with the same answer, but this time emit a single valid JSON object containing the \
keys \"{}\" — all present and non-null. Do not call any tools.",
        contract.block_key, keys
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thoughts_contract() -> RequiredOutputContract {
        RequiredOutputContract {
            block_key: "thoughts".into(),
            required_keys: vec!["next_action".into()],
        }
    }

    #[test]
    fn present_well_formed_block_satisfies_contract() {
        let contract = thoughts_contract();
        let text = "Sure! {\"thoughts\": \"planning\", \"next_action\": \"call tool\"}";
        assert!(output_satisfies_contract(text, &contract));
        assert!(find_required_block(text, &contract).is_some());
    }

    #[test]
    fn prose_only_reply_fails_validation() {
        let contract = thoughts_contract();
        assert!(!output_satisfies_contract(
            "Sure, I'll handle that.",
            &contract
        ));
    }

    #[test]
    fn block_missing_a_required_sibling_key_fails() {
        let contract = thoughts_contract();
        // Has `thoughts` but not `next_action`.
        let text = "{\"thoughts\": \"planning\"}";
        assert!(!output_satisfies_contract(text, &contract));
    }

    #[test]
    fn null_valued_required_key_fails() {
        let contract = RequiredOutputContract::new("thoughts");
        assert!(!output_satisfies_contract(
            "{\"thoughts\": null}",
            &contract
        ));
    }

    #[test]
    fn synthesized_block_satisfies_its_own_contract() {
        let contract = thoughts_contract();
        let synthesized = synthesize_block(&contract);
        assert!(
            output_satisfies_contract(&synthesized, &contract),
            "synthesized fallback must satisfy the contract it was built from: {synthesized}"
        );
    }

    #[test]
    fn inert_contract_is_always_satisfied() {
        let contract = RequiredOutputContract::default();
        assert!(!contract.is_active());
        assert!(output_satisfies_contract("no block here", &contract));
        assert!(find_required_block("no block here", &contract).is_none());
    }

    #[test]
    fn repair_instruction_names_every_required_key() {
        let contract = thoughts_contract();
        let instruction = repair_instruction(&contract);
        assert!(instruction.contains("thoughts"));
        assert!(instruction.contains("next_action"));
    }
}
