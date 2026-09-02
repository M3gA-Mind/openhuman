use super::*;
use crate::openhuman::security::SecurityPolicy;
use tempfile::TempDir;
use tinyagents_harness::tool::ToolExecutionContext;

fn test_tool(dir: &std::path::Path) -> GitOperationsTool {
    let security = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        ..SecurityPolicy::default()
    });
    GitOperationsTool::new(security, dir.to_path_buf())
}

/// Suppress the developer's own system/global git config on a raw
/// `std::process::Command`, so a machine-local `init.templateDir` or similar
/// cannot write extra keys into a test repository's `.git/config` and make
/// these tests depend on ambient environment. Mirrors [`hardened_git`]'s two
/// env vars; the production code under test applies its own suppression when
/// it later reads this same config, so this only affects setup.
fn hermetic(cmd: &mut std::process::Command) -> &mut std::process::Command {
    cmd.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", NULL_CONFIG_PATH)
}

/// Initialise a git repo at `path` and fail the test if `git init`
/// itself didn't succeed (so we don't misread later assertion failures
/// as product bugs when the real problem is a missing/broken git).
fn init_git_repo(path: &std::path::Path) {
    let output = hermetic(
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(path),
    )
    .output()
    .expect("failed to spawn `git init`");
    assert!(
        output.status.success(),
        "`git init` failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Extract the error text from a Result<ToolResult> — whether the
/// failure came through `Err(anyhow::Error)` or `Ok(ToolResult::error)`.
fn error_text(result: &anyhow::Result<ToolResult>) -> String {
    match result {
        Ok(r) => {
            assert!(r.is_error, "expected a tool-error ToolResult");
            r.output().to_string()
        }
        Err(e) => e.to_string(),
    }
}

/// Write a `core.fsmonitor` hook into `dir`'s repository config that creates a
/// marker file when git runs it, and return the marker's path.
///
/// Runs the hook once up front and asserts the marker appears, then removes
/// it — so a later absent marker means the hardening refused the hook, not
/// that the hook itself was silently broken (e.g. by `{:?}`-escaping a path
/// the shell would quote differently than Rust's `Debug` does).
#[cfg(unix)]
fn plant_fsmonitor_hook(dir: &std::path::Path) -> std::path::PathBuf {
    let hook = dir.join("hook.sh");
    let marker = dir.join("COMMAND_RAN");
    std::fs::write(
        &hook,
        format!("#!/bin/sh\ntouch {:?}\nexit 1\n", marker.to_string_lossy()),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();

    std::process::Command::new(&hook).status().unwrap();
    assert!(marker.exists(), "the planted hook does not run at all");
    std::fs::remove_file(&marker).unwrap();

    // Written with `git config` rather than by appending to the file:
    // appending only lands in `[core]` while `[core]` happens to be the last
    // section, which is true of a fresh `git init` and is not a property
    // worth depending on.
    let ok = hermetic(
        std::process::Command::new("git")
            .args(["config", "core.fsmonitor"])
            .arg(&hook)
            .current_dir(dir),
    )
    .status()
    .unwrap()
    .success();
    assert!(ok, "failed to plant the hook in the repository config");
    marker
}

/// Set a repository config key with `git config`, asserting it took.
fn set_config(dir: &std::path::Path, key: &str, value: &str) {
    let ok = hermetic(
        std::process::Command::new("git")
            .args(["config", key, value])
            .current_dir(dir),
    )
    .status()
    .unwrap()
    .success();
    assert!(ok, "failed to set {key} in the test workspace");
}

#[path = "git_operations_tests_part_01_tests.rs"]
mod part_01_tests;
#[path = "git_operations_tests_part_02_tests.rs"]
mod part_02_tests;
