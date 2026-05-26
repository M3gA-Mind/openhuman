//! Skill registry types: a **skill** is an [`AgentDefinition`] plus declared
//! `[[inputs]]`. The agent fields (`id`, `system_prompt`, `tools`,
//! `max_iterations`, `sandbox_mode`, …) are flattened in from the same
//! `skill.toml`, so a skill is just a runnable agent that also advertises the
//! inputs it needs. Schema lives here; values are supplied at `skill_run` time
//! and rendered into the prompt (see [`render_inputs_block`]).
//!
//! This keeps [`AgentDefinition`] untouched (no widespread struct-literal
//! churn) — inputs ride at the skill layer via `#[serde(flatten)]`.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::openhuman::agent::harness::definition::{AgentDefinition, PromptSource};

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

/// Load the skill registry: compile-time builtins (no declared inputs) plus
/// runtime skills under `<workspace>/skills/<id>/{skill.toml, SKILL.md}`. A
/// skill's `SKILL.md`, when present, becomes its inline system prompt. A bad
/// `skill.toml` is skipped with a warning, not fatal.
pub fn load_skills(workspace_dir: &Path) -> Vec<SkillDefinition> {
    let mut skills: Vec<SkillDefinition> = Vec::new();

    if let Ok(builtins) = crate::openhuman::agent::agents::load_builtins() {
        for definition in builtins {
            skills.push(SkillDefinition { definition, inputs: Vec::new() });
        }
    }

    let dir = workspace_dir.join("skills");
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let sd = entry.path();
            if !sd.is_dir() {
                continue;
            }
            let toml_path = sd.join("skill.toml");
            let Ok(toml_str) = std::fs::read_to_string(&toml_path) else {
                continue;
            };
            let mut skill: SkillDefinition = match toml::from_str(&toml_str) {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("[skills] skipping {}: {e}", toml_path.display());
                    continue;
                }
            };
            if let Ok(md) = std::fs::read_to_string(sd.join("SKILL.md")) {
                skill.definition.system_prompt = PromptSource::Inline(md);
            }
            skills.push(skill);
        }
    }
    skills
}

/// Look up one skill by id across the registry.
pub fn get_skill(workspace_dir: &Path, id: &str) -> Option<SkillDefinition> {
    load_skills(workspace_dir)
        .into_iter()
        .find(|s| s.definition.id == id)
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

    #[test]
    fn load_skills_reads_runtime_skill_prompt_and_inputs() {
        let tmp = tempfile::TempDir::new().unwrap();
        let sd = tmp.path().join("skills").join("github-issue-crusher");
        std::fs::create_dir_all(&sd).unwrap();
        std::fs::write(
            sd.join("skill.toml"),
            "id = \"github-issue-crusher\"\nwhen_to_use = \"fix a github issue\"\n\
             [[inputs]]\nname = \"repo\"\ndescription = \"owner/name\"\nrequired = true\n\
             [[inputs]]\nname = \"issue\"\ndescription = \"issue #\"\nrequired = true\ntype = \"integer\"\n",
        )
        .unwrap();
        std::fs::write(sd.join("SKILL.md"), "# Issue Crusher\nFix it.").unwrap();

        let skills = load_skills(tmp.path());
        let s = skills
            .iter()
            .find(|s| s.definition.id == "github-issue-crusher")
            .expect("runtime skill loaded");
        assert_eq!(s.inputs.len(), 2);
        assert_eq!(s.inputs[1].kind.as_deref(), Some("integer"));
        match &s.definition.system_prompt {
            PromptSource::Inline(p) => assert!(p.contains("Fix it.")),
            other => panic!("expected inline prompt, got {other:?}"),
        }
    }
}
