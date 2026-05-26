//! Skill registry types: a **skill** is an [`AgentDefinition`] plus declared
//! `[[inputs]]`. The agent fields (`id`, `system_prompt`, `tools`,
//! `max_iterations`, `sandbox_mode`, …) are flattened in from the same
//! `skill.toml`, so a skill is just a runnable agent that also advertises the
//! inputs it needs. Schema lives here; values are supplied at `skill_run` time
//! and rendered into the prompt (see [`render_inputs_block`]).
//!
//! This keeps [`AgentDefinition`] untouched (no widespread struct-literal
//! churn) — inputs ride at the skill layer via `#[serde(flatten)]`.

use serde::{Deserialize, Serialize};

use crate::openhuman::agent::harness::definition::AgentDefinition;

/// One declared input — a parameter the skill needs, with a human description.
/// `required` inputs must be supplied at run time; `kind` is an optional type
/// hint (`"string"`, `"integer"`, …) for the UI / validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
}

/// A skill = an agent definition + its declared inputs (parsed from `skill.toml`).
#[derive(Debug, Clone, Deserialize)]
pub struct SkillDefinition {
    #[serde(flatten)]
    pub definition: AgentDefinition,
    #[serde(default)]
    pub inputs: Vec<SkillInput>,
}

/// Names of `required` inputs that are absent or null in `provided`. Empty ⇒ OK.
pub fn missing_required_inputs(defs: &[SkillInput], provided: &serde_json::Value) -> Vec<String> {
    defs.iter()
        .filter(|d| d.required)
        .filter(|d| provided.get(&d.name).map(|v| v.is_null()).unwrap_or(true))
        .map(|d| d.name.clone())
        .collect()
}

/// Render the resolved inputs as an `## Inputs` prompt block injected alongside
/// the skill's `SKILL.md`. Empty string when the skill declares no inputs.
pub fn render_inputs_block(defs: &[SkillInput], provided: &serde_json::Value) -> String {
    if defs.is_empty() {
        return String::new();
    }
    let mut lines = vec!["## Inputs".to_string()];
    for d in defs {
        let shown = match provided.get(&d.name) {
            None | Some(serde_json::Value::Null) => "(not provided)".to_string(),
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
        };
        lines.push(format!("- **{}**: {}", d.name, shown));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn defs() -> Vec<SkillInput> {
        vec![
            SkillInput { name: "repo".into(), description: "owner/name".into(), required: true, kind: None },
            SkillInput { name: "issue".into(), description: "issue #".into(), required: true, kind: Some("integer".into()) },
            SkillInput { name: "pr_base".into(), description: "base branch".into(), required: false, kind: None },
        ]
    }

    #[test]
    fn missing_required_is_detected() {
        assert_eq!(missing_required_inputs(&defs(), &json!({"repo": "acme/web"})), vec!["issue".to_string()]);
        assert!(missing_required_inputs(&defs(), &json!({"repo": "acme/web", "issue": 42})).is_empty());
        // null counts as missing
        assert_eq!(missing_required_inputs(&defs(), &json!({"repo": "acme/web", "issue": null})), vec!["issue".to_string()]);
    }

    #[test]
    fn renders_inputs_block_with_values_and_gaps() {
        let b = render_inputs_block(&defs(), &json!({"repo": "acme/web", "issue": 42}));
        assert!(b.starts_with("## Inputs"));
        assert!(b.contains("**repo**: acme/web"));
        assert!(b.contains("**issue**: 42"));
        assert!(b.contains("**pr_base**: (not provided)"));
        assert!(render_inputs_block(&[], &json!({})).is_empty());
    }

    #[test]
    fn skill_input_parses_type_alias() {
        let i: SkillInput = serde_json::from_value(json!({
            "name": "issue", "description": "issue #", "required": true, "type": "integer"
        })).unwrap();
        assert_eq!(i.kind.as_deref(), Some("integer"));
        assert!(i.required);
    }
}
