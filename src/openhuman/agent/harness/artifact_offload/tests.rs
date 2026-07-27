//! Tests for the artifact-offload convention (#3883).
//!
//! Covers the happy path (oversized result lands in `outputs/`, parent gets a
//! path + abstract), the fallback path (offload refused, inline payload
//! survives for the summarizer/truncation backstop), and the fail-closed path
//! hardening that keeps offload inside `action_dir` and out of `workspace_dir`.

use std::path::PathBuf;
use std::sync::Arc;

use super::*;
use crate::openhuman::security::{AutonomyLevel, SecurityPolicy};
use crate::openhuman::tools::traits::Tool;
use crate::openhuman::tools::FileReadTool;
use serde_json::json;

/// Policy with disjoint action/workspace roots, the shipped default layout
/// (`~/OpenHuman/projects` vs `~/.openhuman/users/<id>/workspace`).
fn policy_with(action_dir: PathBuf, workspace_dir: PathBuf) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        action_dir,
        workspace_dir,
        ..SecurityPolicy::default()
    })
}

fn offload_for(action_dir: &std::path::Path, workspace_dir: &std::path::Path) -> ArtifactOffload {
    ArtifactOffload::new(
        action_dir.to_path_buf(),
        Some(policy_with(
            action_dir.to_path_buf(),
            workspace_dir.to_path_buf(),
        )),
        "researcher",
        "sub-1234",
    )
}

// ── Convention directories ──────────────────────────────────────────────────

#[test]
fn kinds_map_to_the_documented_directories() {
    assert_eq!(ArtifactKind::Output.subdir(), OUTPUTS_DIR);
    assert_eq!(ArtifactKind::Output.subdir(), "outputs");
    assert_eq!(ArtifactKind::Scratch.subdir(), SCRATCH_DIR);
    assert_eq!(ArtifactKind::Scratch.subdir(), "workspace");
    assert_eq!(ArtifactKind::Output.as_str(), "output");
    assert_eq!(ArtifactKind::Scratch.as_str(), "scratch");
}

#[test]
fn prompt_contract_names_both_directories_and_the_file_write_step() {
    let rendered = render_artifact_offload_contract();
    assert!(rendered.starts_with(ARTIFACT_OFFLOAD_HEADING));
    assert!(rendered.contains("`outputs/`"));
    assert!(rendered.contains("`workspace/`"));
    assert!(rendered.contains("file_write"));
    assert!(rendered.contains("file_read"));
    // Byte-stable: the sub-agent system prompt is prefix-cached.
    assert_eq!(rendered, render_artifact_offload_contract());
}

// ── Path hardening ──────────────────────────────────────────────────────────

#[test]
fn resolves_under_the_convention_directory() {
    let action = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let policy = policy_with(action.path().to_path_buf(), workspace.path().to_path_buf());

    let resolved = resolve_artifact_path(
        action.path(),
        Some(&*policy),
        ArtifactKind::Output,
        "researcher/report.md",
    )
    .expect("a plain relative name resolves");

    assert_eq!(
        resolved,
        action
            .path()
            .join("outputs")
            .join("researcher")
            .join("report.md")
    );
    assert_eq!(
        relative_to_action_dir(action.path(), &resolved),
        "outputs/researcher/report.md"
    );
}

#[test]
fn scratch_kind_resolves_under_action_dir_workspace_not_the_core_workspace() {
    let action = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let policy = policy_with(action.path().to_path_buf(), workspace.path().to_path_buf());

    let resolved = resolve_artifact_path(
        action.path(),
        Some(&*policy),
        ArtifactKind::Scratch,
        "notes.txt",
    )
    .expect("scratch resolves");

    assert!(resolved.starts_with(action.path().join("workspace")));
    assert!(
        !resolved.starts_with(workspace.path()),
        "action_dir/workspace must never be the core workspace_dir"
    );
}

#[test]
fn rejects_parent_traversal() {
    let action = tempfile::tempdir().unwrap();
    let err = resolve_artifact_path(
        action.path(),
        None,
        ArtifactKind::Output,
        "../../etc/passwd",
    )
    .expect_err("`..` traversal must be refused");
    assert!(matches!(err, OffloadError::PathEscape { .. }), "{err}");
}

#[test]
fn rejects_absolute_paths() {
    let action = tempfile::tempdir().unwrap();
    let absolute = if cfg!(windows) {
        "C:\\Windows\\System32\\drivers\\etc\\hosts"
    } else {
        "/etc/passwd"
    };
    let err = resolve_artifact_path(action.path(), None, ArtifactKind::Output, absolute)
        .expect_err("absolute paths must be refused");
    assert!(matches!(err, OffloadError::AbsolutePath { .. }), "{err}");
}

#[test]
fn rejects_empty_and_whitespace_names() {
    let action = tempfile::tempdir().unwrap();
    for name in ["", "   ", "\n\t"] {
        let err = resolve_artifact_path(action.path(), None, ArtifactKind::Output, name)
            .expect_err("empty names must be refused");
        assert!(matches!(err, OffloadError::EmptyName), "{err}");
    }
}

#[test]
fn accepts_leading_current_dir_segments() {
    let action = tempfile::tempdir().unwrap();
    let resolved =
        resolve_artifact_path(action.path(), None, ArtifactKind::Output, "./report.md").unwrap();
    assert_eq!(resolved, action.path().join("outputs").join("report.md"));
}

#[test]
fn rejects_targets_inside_workspace_dir_fail_closed() {
    // action_dir configured INSIDE workspace_dir: every offload target lands in
    // the core's internal state root, so every offload must be refused rather
    // than quietly writing there.
    let workspace = tempfile::tempdir().unwrap();
    let action = workspace.path().join("projects");
    let policy = policy_with(action.clone(), workspace.path().to_path_buf());

    let err = resolve_artifact_path(&action, Some(&*policy), ArtifactKind::Output, "report.md")
        .expect_err("a target under workspace_dir must be refused");
    assert!(matches!(err, OffloadError::WorkspaceTarget { .. }), "{err}");
}

#[test]
fn rejects_workspace_internal_state_paths() {
    // `memory/` is one of the internal state dirs `is_workspace_internal_path`
    // fences off. Pointing action_dir at workspace_dir itself makes
    // `outputs/..`-free names still land on internal state once the caller
    // names one, so the more specific error wins.
    let workspace = tempfile::tempdir().unwrap();
    let action = workspace.path().to_path_buf();
    let policy = policy_with(action.clone(), workspace.path().to_path_buf());

    let err = resolve_artifact_path(&action, Some(&*policy), ArtifactKind::Output, "report.md")
        .expect_err("workspace-rooted action_dir must be refused");
    // Either fail-closed variant is acceptable; both refuse the write.
    assert!(
        matches!(
            err,
            OffloadError::WorkspaceTarget { .. } | OffloadError::WorkspaceInternal { .. }
        ),
        "{err}"
    );
}

#[test]
fn workspace_internal_dir_is_refused_by_the_policy_check() {
    let workspace = tempfile::tempdir().unwrap();
    let policy = policy_with(
        workspace.path().to_path_buf(),
        workspace.path().to_path_buf(),
    );
    // `<workspace>/memory` is workspace-internal; resolve with a kind whose
    // subdir IS `memory` is impossible, so assert the policy predicate the
    // resolver delegates to directly on the path it would build.
    assert!(policy.is_workspace_internal_path(&workspace.path().join("memory")));
    assert!(!policy.is_workspace_internal_path(&workspace.path().join("outputs")));
}

#[test]
fn sanitize_component_strips_separators_and_never_returns_empty() {
    assert_eq!(sanitize_component("researcher"), "researcher");
    assert_eq!(sanitize_component("sub-12/34"), "sub-12_34");
    assert_eq!(sanitize_component("../../etc"), "______etc");
    assert_eq!(sanitize_component(""), "unknown");
    assert_eq!(sanitize_component("///"), "___");
    assert!(sanitize_component(&"x".repeat(500)).chars().count() <= 80);
}

#[test]
fn relative_to_action_dir_falls_back_to_display_for_outside_paths() {
    let action = PathBuf::from("/tmp/action");
    let outside = PathBuf::from("/var/other/file.md");
    assert_eq!(
        relative_to_action_dir(&action, &outside),
        outside.to_string_lossy()
    );
}

// ── Threshold + abstract ────────────────────────────────────────────────────

#[test]
fn should_offload_respects_threshold_and_the_zero_disable() {
    assert!(should_offload(100, 50));
    assert!(!should_offload(50, 50), "exactly at threshold stays inline");
    assert!(!should_offload(10, 50));
    assert!(
        !should_offload(usize::MAX, 0),
        "zero threshold disables offload"
    );
}

#[test]
fn build_abstract_returns_short_content_unchanged() {
    assert_eq!(build_abstract("  short answer  ", 100), "short answer");
}

#[test]
fn build_abstract_cuts_at_a_line_boundary_when_one_is_available() {
    let content = format!("{}\n{}", "a".repeat(60), "b".repeat(200));
    let out = build_abstract(&content, 100);
    assert!(out.ends_with("..."));
    assert!(!out.contains('b'), "should stop at the line break: {out}");
}

#[test]
fn build_abstract_cuts_at_a_word_boundary_when_there_is_no_line_break() {
    let content = format!("{} {}", "word ".repeat(20), "tail".repeat(50));
    let out = build_abstract(&content, 60);
    assert!(out.ends_with("..."));
    assert!(out.chars().count() <= 64);
}

#[test]
fn build_abstract_handles_a_zero_budget_and_boundary_free_text() {
    assert_eq!(build_abstract("anything", 0), "");
    let out = build_abstract(&"x".repeat(500), 40);
    assert!(out.ends_with("..."));
    assert!(out.chars().count() <= 44);
}

#[test]
fn build_abstract_never_splits_a_multibyte_character() {
    let content = "é".repeat(400);
    let out = build_abstract(&content, 50);
    assert!(out.ends_with("..."));
    assert!(out.chars().all(|c| c == 'é' || c == '.'));
}

// ── Pointer render + parse ──────────────────────────────────────────────────

fn sample_artifact(redacted: bool) -> OffloadedArtifact {
    OffloadedArtifact {
        kind: ArtifactKind::Output,
        relative_path: "outputs/researcher/sub-1234-result.md".to_string(),
        absolute_path: PathBuf::from("/tmp/action/outputs/researcher/sub-1234-result.md"),
        stored_bytes: 4096,
        original_bytes: 4096,
        redacted,
    }
}

#[test]
fn pointer_carries_path_size_and_a_file_read_call() {
    let rendered = render_artifact_pointer(&sample_artifact(false), "two-line abstract");
    assert!(rendered.starts_with(ARTIFACT_POINTER_PREFIX));
    assert!(rendered.contains("kind=output"));
    assert!(rendered.contains("path=outputs/researcher/sub-1234-result.md"));
    assert!(rendered.contains("bytes=4096"));
    assert!(rendered
        .contains(r#"read_with: file_read {"path":"outputs/researcher/sub-1234-result.md"}"#));
    assert!(rendered.contains("[abstract]\ntwo-line abstract"));
    assert!(!rendered.contains("redaction was applied"));
}

#[test]
fn pointer_discloses_redaction_when_it_happened() {
    let rendered = render_artifact_pointer(&sample_artifact(true), "abstract");
    assert!(rendered.contains("Credential/PII redaction was applied"));
}

#[test]
fn extract_artifact_paths_reads_pointers_out_of_a_handoff() {
    let rendered = render_artifact_pointer(&sample_artifact(false), "abstract");
    assert_eq!(
        extract_artifact_paths(&rendered),
        vec!["outputs/researcher/sub-1234-result.md".to_string()]
    );
}

#[test]
fn extract_artifact_paths_dedupes_and_keeps_encounter_order() {
    let text = "[artifact] kind=output path=outputs/a.md bytes=1\n\
                prose in between\n\
                  [artifact] kind=scratch path=workspace/b.md bytes=2\n\
                [artifact] kind=output path=outputs/a.md bytes=1\n";
    assert_eq!(
        extract_artifact_paths(text),
        vec!["outputs/a.md".to_string(), "workspace/b.md".to_string()]
    );
}

#[test]
fn extract_artifact_paths_ignores_non_pointer_and_malformed_lines() {
    let text = "ordinary answer text\n\
                [artifact] kind=output bytes=1\n\
                [artifact] kind=output path= bytes=1\n\
                the word [artifact] appearing mid-sentence path=nope\n";
    assert!(extract_artifact_paths(text).is_empty());
    assert!(extract_artifact_paths("").is_empty());
}

#[test]
fn note_artifact_handoff_reports_how_many_paths_crossed() {
    let paths = vec!["outputs/a.md".to_string(), "outputs/b.md".to_string()];
    assert_eq!(note_artifact_handoff("researcher", "sub-1", &paths), 2);
    assert_eq!(note_artifact_handoff("researcher", "sub-1", &[]), 0);
}

// ── Write path ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn write_persists_under_outputs_and_reports_the_relative_path() {
    let action = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let offload = offload_for(action.path(), workspace.path());

    let artifact = offload
        .write(ArtifactKind::Output, "researcher/report.md", "full body")
        .await
        .expect("write succeeds");

    assert_eq!(artifact.relative_path, "outputs/researcher/report.md");
    assert_eq!(artifact.kind, ArtifactKind::Output);
    assert!(!artifact.redacted);
    assert_eq!(
        tokio::fs::read_to_string(&artifact.absolute_path)
            .await
            .unwrap(),
        "full body"
    );
}

#[tokio::test]
async fn write_redacts_credentials_before_they_reach_disk() {
    let action = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let offload = offload_for(action.path(), workspace.path());

    let body = "findings ghp_abcdefghijklmnopqrstuvwxyz123456";
    let artifact = offload
        .write(ArtifactKind::Output, "leaky.md", body)
        .await
        .unwrap();

    assert!(artifact.redacted, "the token must be scrubbed");
    let stored = tokio::fs::read_to_string(&artifact.absolute_path)
        .await
        .unwrap();
    assert!(!stored.contains("ghp_abcdefghijklmnopqrstuvwxyz123456"));
}

#[tokio::test]
async fn write_refuses_a_traversal_target_without_touching_disk() {
    let action = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let offload = offload_for(action.path(), workspace.path());

    let err = offload
        .write(ArtifactKind::Output, "../escaped.md", "body")
        .await
        .expect_err("traversal must be refused");

    assert!(matches!(err, OffloadError::PathEscape { .. }), "{err}");
    assert!(!action.path().join("escaped.md").exists());
}

#[tokio::test]
async fn default_result_name_sanitizes_both_identifiers() {
    let action = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let offload = ArtifactOffload::new(
        action.path().to_path_buf(),
        Some(policy_with(
            action.path().to_path_buf(),
            workspace.path().to_path_buf(),
        )),
        "team/researcher",
        "sub/../1",
    );

    assert_eq!(
        offload.default_result_name(),
        "team_researcher/sub____1-result.md"
    );
    assert_eq!(offload.action_dir(), action.path());
    assert!(offload
        .resolve(ArtifactKind::Output, &offload.default_result_name())
        .is_ok());
}

// ── End-to-end offload ──────────────────────────────────────────────────────

#[tokio::test]
async fn oversized_result_is_offloaded_and_the_parent_gets_a_path_plus_abstract() {
    let action = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let offload = offload_for(action.path(), workspace.path());

    let body = format!("HEADLINE FINDING\n{}", "detail line\n".repeat(2_000));
    let (handed_to_parent, artifact) =
        offload_oversized_result(body.clone(), &offload, DEFAULT_OFFLOAD_THRESHOLD_BYTES).await;

    let artifact = artifact.expect("an oversized result must be offloaded");
    assert_eq!(
        artifact.relative_path,
        "outputs/researcher/sub-1234-result.md"
    );
    assert!(
        handed_to_parent.len() < body.len(),
        "the pointer must be smaller than the payload it replaces"
    );
    assert!(handed_to_parent.starts_with(ARTIFACT_POINTER_PREFIX));
    assert!(
        handed_to_parent.contains("HEADLINE FINDING"),
        "abstract keeps the lede"
    );
    assert_eq!(
        extract_artifact_paths(&handed_to_parent),
        vec![artifact.relative_path.clone()]
    );

    // The parent can recover full fidelity with an ordinary file_read scoped to
    // the action dir — that is the whole point of the convention.
    let policy = policy_with(action.path().to_path_buf(), workspace.path().to_path_buf());
    let read = FileReadTool::new(policy)
        .execute(json!({ "path": artifact.relative_path }))
        .await
        .unwrap();
    assert!(!read.is_error, "{}", read.output());
    assert!(read.output().contains("HEADLINE FINDING"));
}

#[tokio::test]
async fn small_result_stays_inline() {
    let action = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let offload = offload_for(action.path(), workspace.path());

    let (out, artifact) = offload_oversized_result(
        "short answer".to_string(),
        &offload,
        DEFAULT_OFFLOAD_THRESHOLD_BYTES,
    )
    .await;

    assert_eq!(out, "short answer");
    assert!(artifact.is_none());
    assert!(!action.path().join("outputs").exists(), "no file written");
}

#[tokio::test]
async fn offload_failure_keeps_the_inline_payload_for_the_summarizer_fallback() {
    // action_dir inside workspace_dir: every target is refused fail-closed, so
    // the caller must get its payload back untouched rather than losing it.
    let workspace = tempfile::tempdir().unwrap();
    let action = workspace.path().join("projects");
    let offload = ArtifactOffload::new(
        action.clone(),
        Some(policy_with(action, workspace.path().to_path_buf())),
        "researcher",
        "sub-1234",
    );

    let body = "y".repeat(DEFAULT_OFFLOAD_THRESHOLD_BYTES + 1);
    let (out, artifact) =
        offload_oversized_result(body.clone(), &offload, DEFAULT_OFFLOAD_THRESHOLD_BYTES).await;

    assert_eq!(
        out, body,
        "the inline payload must survive a refused offload"
    );
    assert!(artifact.is_none());
}

#[tokio::test]
async fn offload_is_disabled_by_a_zero_threshold() {
    let action = tempfile::tempdir().unwrap();
    let workspace = tempfile::tempdir().unwrap();
    let offload = offload_for(action.path(), workspace.path());

    let body = "z".repeat(100_000);
    let (out, artifact) = offload_oversized_result(body.clone(), &offload, 0).await;

    assert_eq!(out, body);
    assert!(artifact.is_none());
}

#[tokio::test]
async fn offload_without_a_policy_still_enforces_containment() {
    let action = tempfile::tempdir().unwrap();
    let offload = ArtifactOffload::new(action.path().to_path_buf(), None, "planner", "sub-9");

    let artifact = offload
        .write(ArtifactKind::Scratch, "plan.md", "scratch body")
        .await
        .expect("no policy means no workspace checks, containment still applies");
    assert_eq!(artifact.relative_path, "workspace/plan.md");

    let err = offload
        .write(ArtifactKind::Scratch, "../../outside.md", "body")
        .await
        .expect_err("traversal is refused with or without a policy");
    assert!(matches!(err, OffloadError::PathEscape { .. }), "{err}");
}
