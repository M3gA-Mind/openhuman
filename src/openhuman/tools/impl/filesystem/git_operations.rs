use crate::openhuman::security::{AutonomyLevel, CommandClass, GateDecision, SecurityPolicy};
use crate::openhuman::tools::traits::{Tool, ToolCallOptions, ToolResult};
use async_trait::async_trait;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tinytools::ToolRunContext;

/// Git operations tool for structured repository management.
/// Provides safe, parsed git operations with JSON output.
pub struct GitOperationsTool {
    security: Arc<SecurityPolicy>,
    action_dir: PathBuf,
}

impl GitOperationsTool {
    pub fn new(security: Arc<SecurityPolicy>, action_dir: PathBuf) -> Self {
        Self {
            security,
            action_dir,
        }
    }

    /// Resolve the working directory for git operations.
    ///
    /// Returns the per-worker git-worktree checkout when the tinyagents harness
    /// threaded a [`WorkspaceDescriptor`] into this call's
    /// [`ToolExecutionContext`] — an edit-capable worker running with
    /// `isolation = "worktree"`, whose isolated worktree root is carried on the
    /// run context (`RunContext::with_workspace`) and surfaced per tool call via
    /// `ToolExecutionContext::from_run_context`. Otherwise falls back to the
    /// tool's configured `action_dir`, which preserves the non-isolated
    /// behaviour exactly. See #3376, #4249 (08.5).
    fn effective_action_dir_for_context(&self, context: Option<&dyn ToolRunContext>) -> PathBuf {
        if let Some(workspace) = context.and_then(|ctx| ctx.workspace()) {
            tracing::debug!(
                workspace_root = %workspace.root.display(),
                policy_id = %workspace.policy_id,
                "[git_operations] using TinyAgents workspace descriptor as action dir"
            );
            return workspace.root.clone();
        }
        self.action_dir.clone()
    }

    /// Sanitize git arguments to prevent injection attacks
    fn sanitize_git_args(&self, args: &str) -> anyhow::Result<Vec<String>> {
        let mut result = Vec::new();
        for arg in args.split_whitespace() {
            // Block dangerous git options that could lead to command injection
            let arg_lower = arg.to_lowercase();
            if arg_lower.starts_with("--exec=")
                || arg_lower.starts_with("--upload-pack=")
                || arg_lower.starts_with("--receive-pack=")
                || arg_lower.starts_with("--pager=")
                || arg_lower.starts_with("--editor=")
                || arg_lower == "--no-verify"
                || arg_lower.contains("$(")
                || arg_lower.contains('`')
                || arg.contains('|')
                || arg.contains(';')
                || arg.contains('>')
            {
                anyhow::bail!("Blocked potentially dangerous git argument: {arg}");
            }
            // Block `-c` config injection (exact match or `-c=...` prefix).
            // This must not false-positive on `--cached` or `-cached`.
            if arg_lower == "-c" || arg_lower.starts_with("-c=") {
                anyhow::bail!("Blocked potentially dangerous git argument: {arg}");
            }
            result.push(arg.to_string());
        }
        Ok(result)
    }

    /// Check if an operation requires write access
    fn requires_write_access(&self, operation: &str) -> bool {
        matches!(
            operation,
            "commit" | "add" | "checkout" | "stash" | "reset" | "revert"
        )
    }

    /// Check if an operation is read-only
    fn is_read_only(&self, operation: &str) -> bool {
        matches!(
            operation,
            "status" | "diff" | "log" | "show" | "branch" | "rev-parse"
        )
    }

    async fn run_git_command_in(&self, cwd: &Path, args: &[&str]) -> anyhow::Result<String> {
        if let Some(key) = first_disallowed_repo_config_key(cwd).await? {
            tracing::debug!(
                "[git_operations] refusing to run git: dir={}, disallowed_config_key={key}",
                cwd.display()
            );
            anyhow::bail!("{}", untrusted_repo_config_refusal(cwd, &key))
        }

        let output = hardened_git(cwd).args(args).output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Git command failed: {stderr}");
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn git_status(&self, cwd: &Path, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let output = self
            .run_git_command_in(cwd, &["status", "--porcelain=2", "--branch"])
            .await?;

        // Parse git status output into structured format
        let mut result = serde_json::Map::new();
        let mut branch = String::new();
        let mut staged = Vec::new();
        let mut unstaged = Vec::new();
        let mut untracked = Vec::new();

        for line in output.lines() {
            if line.starts_with("# branch.head ") {
                branch = line.trim_start_matches("# branch.head ").to_string();
            } else if let Some(rest) = line.strip_prefix("1 ") {
                // Ordinary changed entry
                let mut parts = rest.splitn(3, ' ');
                if let (Some(staging), Some(path)) = (parts.next(), parts.next()) {
                    if !staging.is_empty() {
                        let status_char = staging.chars().next().unwrap_or(' ');
                        if status_char != '.' && status_char != ' ' {
                            staged.push(json!({"path": path, "status": status_char}));
                        }
                        let status_char = staging.chars().nth(1).unwrap_or(' ');
                        if status_char != '.' && status_char != ' ' {
                            unstaged.push(json!({"path": path, "status": status_char}));
                        }
                    }
                }
            } else if let Some(rest) = line.strip_prefix("? ") {
                untracked.push(rest.to_string());
            }
        }

        result.insert("branch".to_string(), json!(branch));
        result.insert("staged".to_string(), json!(staged));
        result.insert("unstaged".to_string(), json!(unstaged));
        result.insert("untracked".to_string(), json!(untracked));
        result.insert(
            "clean".to_string(),
            json!(staged.is_empty() && unstaged.is_empty() && untracked.is_empty()),
        );

        let mut tr = ToolResult::success(serde_json::to_string_pretty(&result).unwrap_or_default());
        tr.markdown_formatted = Some(render_status_markdown(&result));
        Ok(tr)
    }

    async fn git_diff(&self, cwd: &Path, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let files = args.get("files").and_then(|v| v.as_str()).unwrap_or(".");
        let cached = args
            .get("cached")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Validate files argument against injection patterns
        self.sanitize_git_args(files)?;

        let mut git_args = vec!["diff", "--unified=3"];
        if cached {
            git_args.push("--cached");
        }
        git_args.push("--");
        git_args.push(files);

        let output = self.run_git_command_in(cwd, &git_args).await?;

        // Parse diff into structured hunks
        let mut result = serde_json::Map::new();
        let mut hunks = Vec::new();
        let mut current_file = String::new();
        let mut current_hunk = serde_json::Map::new();
        let mut lines = Vec::new();

        for line in output.lines() {
            if line.starts_with("diff --git ") {
                if !lines.is_empty() {
                    current_hunk.insert("lines".to_string(), json!(lines));
                    if !current_hunk.is_empty() {
                        hunks.push(serde_json::Value::Object(current_hunk.clone()));
                    }
                    lines = Vec::new();
                    current_hunk = serde_json::Map::new();
                }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    current_file = parts[3].trim_start_matches("b/").to_string();
                    current_hunk.insert("file".to_string(), json!(current_file));
                }
            } else if line.starts_with("@@ ") {
                if !lines.is_empty() {
                    current_hunk.insert("lines".to_string(), json!(lines));
                    if !current_hunk.is_empty() {
                        hunks.push(serde_json::Value::Object(current_hunk.clone()));
                    }
                    lines = Vec::new();
                    current_hunk = serde_json::Map::new();
                    current_hunk.insert("file".to_string(), json!(current_file));
                }
                current_hunk.insert("header".to_string(), json!(line));
            } else if !line.is_empty() {
                lines.push(json!({
                    "text": line,
                    "type": if line.starts_with('+') { "add" }
                           else if line.starts_with('-') { "delete" }
                           else { "context" }
                }));
            }
        }

        if !lines.is_empty() {
            current_hunk.insert("lines".to_string(), json!(lines));
            if !current_hunk.is_empty() {
                hunks.push(serde_json::Value::Object(current_hunk));
            }
        }

        result.insert("hunks".to_string(), json!(hunks));
        result.insert("file_count".to_string(), json!(hunks.len()));

        Ok(ToolResult::success(
            serde_json::to_string_pretty(&result).unwrap_or_default(),
        ))
    }

    async fn git_log(&self, cwd: &Path, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let limit_raw = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10);
        let limit = usize::try_from(limit_raw).unwrap_or(usize::MAX).min(1000);
        let limit_str = limit.to_string();

        let output = self
            .run_git_command_in(
                cwd,
                &[
                    "log",
                    &format!("-{limit_str}"),
                    "--pretty=format:%H|%an|%ae|%ad|%s",
                    "--date=iso",
                ],
            )
            .await?;

        let mut commits = Vec::new();

        for line in output.lines() {
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() >= 5 {
                commits.push(json!({
                    "hash": parts[0],
                    "author": parts[1],
                    "email": parts[2],
                    "date": parts[3],
                    "message": parts[4]
                }));
            }
        }

        let mut tr = ToolResult::success(
            serde_json::to_string_pretty(&json!({ "commits": commits })).unwrap_or_default(),
        );
        tr.markdown_formatted = Some(render_log_markdown(&commits));
        Ok(tr)
    }

    async fn git_branch(&self, cwd: &Path, _args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let output = self
            .run_git_command_in(cwd, &["branch", "--format=%(refname:short)|%(HEAD)"])
            .await?;

        let mut branches = Vec::new();
        let mut current = String::new();

        for line in output.lines() {
            if let Some((name, head)) = line.split_once('|') {
                let is_current = head == "*";
                if is_current {
                    current = name.to_string();
                }
                branches.push(json!({
                    "name": name,
                    "current": is_current
                }));
            }
        }

        let mut tr = ToolResult::success(
            serde_json::to_string_pretty(&json!({
                "current": current,
                "branches": branches
            }))
            .unwrap_or_default(),
        );
        tr.markdown_formatted = Some(render_branch_markdown(&current, &branches));
        Ok(tr)
    }

    fn truncate_commit_message(message: &str) -> String {
        if message.chars().count() > 2000 {
            format!("{}...", message.chars().take(1997).collect::<String>())
        } else {
            message.to_string()
        }
    }

    async fn git_commit(&self, cwd: &Path, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'message' parameter"))?;

        // Sanitize commit message
        let sanitized = message
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect::<Vec<_>>()
            .join("\n");

        if sanitized.is_empty() {
            anyhow::bail!("Commit message cannot be empty");
        }

        // Limit message length
        let message = Self::truncate_commit_message(&sanitized);

        let output = self
            .run_git_command_in(cwd, &["commit", "-m", &message])
            .await;

        match output {
            Ok(_) => Ok(ToolResult::success(format!("Committed: {message}"))),
            Err(e) => Ok(ToolResult::error(format!("Commit failed: {e}"))),
        }
    }

    async fn git_add(&self, cwd: &Path, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let paths = args
            .get("paths")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'paths' parameter"))?;

        // Validate paths against injection patterns
        self.sanitize_git_args(paths)?;

        let output = self.run_git_command_in(cwd, &["add", "--", paths]).await;

        match output {
            Ok(_) => Ok(ToolResult::success(format!("Staged: {paths}"))),
            Err(e) => Ok(ToolResult::error(format!("Add failed: {e}"))),
        }
    }

    async fn git_checkout(
        &self,
        cwd: &Path,
        args: serde_json::Value,
    ) -> anyhow::Result<ToolResult> {
        let branch = args
            .get("branch")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'branch' parameter"))?;

        // Sanitize branch name
        let sanitized = self.sanitize_git_args(branch)?;

        if sanitized.is_empty() || sanitized.len() > 1 {
            anyhow::bail!("Invalid branch specification");
        }

        let branch_name = &sanitized[0];

        // Block dangerous branch names
        if branch_name.contains('@') || branch_name.contains('^') || branch_name.contains('~') {
            anyhow::bail!("Branch name contains invalid characters");
        }

        let output = self
            .run_git_command_in(cwd, &["checkout", branch_name])
            .await;

        match output {
            Ok(_) => Ok(ToolResult::success(format!(
                "Switched to branch: {branch_name}"
            ))),
            Err(e) => Ok(ToolResult::error(format!("Checkout failed: {e}"))),
        }
    }

    async fn git_stash(&self, cwd: &Path, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("push");

        let output = match action {
            "push" | "save" => {
                self.run_git_command_in(cwd, &["stash", "push", "-m", "auto-stash"])
                    .await
            }
            "pop" => self.run_git_command_in(cwd, &["stash", "pop"]).await,
            "list" => self.run_git_command_in(cwd, &["stash", "list"]).await,
            "drop" => {
                let index_raw = args.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                let index = i32::try_from(index_raw)
                    .map_err(|_| anyhow::anyhow!("stash index too large: {index_raw}"))?;
                self.run_git_command_in(cwd, &["stash", "drop", &format!("stash@{{{index}}}")])
                    .await
            }
            _ => anyhow::bail!("Unknown stash action: {action}. Use: push, pop, list, drop"),
        };

        match output {
            Ok(out) => Ok(ToolResult::success(out)),
            Err(e) => Ok(ToolResult::error(format!("Stash {action} failed: {e}"))),
        }
    }
}

/// Repository config keys this tool will run `git` under.
///
/// This is an allowlist, and the direction is the whole point. Several git
/// config keys name a command that git then executes — `core.fsmonitor` is
/// run by `git status` (used by the `status` operation here), `core.sshCommand`
/// by anything that reaches a remote, `diff.external` by `diff`, `core.pager`
/// and `core.editor` by the commands that use them, and the `filter.*.process`
/// / `*.clean` / `*.smudge` and `diff.*.textconv` families by content
/// operations. Enumerating *those* and clearing them is the obvious fix and it
/// is the wrong shape: the list is only correct until git adds a key, and a
/// denylist that has gone stale reads as protection while providing none.
///
/// An allowlist ages the other way. A key nobody here has heard of is refused,
/// so a new git release makes this tool fail closed and loud rather than
/// silently regain the hole.
///
/// The entries are `section.key`, lowercased, with any subsection elided —
/// `remote.origin.url` is checked as `remote.url`.
const ALLOWED_REPO_CONFIG: &[&str] = &[
    // What `git init` and `git clone` write, and nothing else.
    //
    // `core.worktree` is deliberately absent, unlike the read-only sibling
    // this list started from. It redirects the working-tree root Git
    // operates against — including a linked-worktree-shaped redirect this
    // tool does not otherwise offer — and this tool runs *write* operations
    // (`checkout`, `add`, `commit`, `stash`) whose target directory is
    // supposed to be `action_dir`/the resolved workspace, never something a
    // repository's own config gets to name. Worktree isolation is already
    // handled through `WorkspaceDescriptor` (`effective_action_dir_for_context`),
    // a trusted, in-process mechanism — nothing here needs the config key.
    "core.repositoryformatversion",
    "core.filemode",
    "core.bare",
    "core.logallrefupdates",
    "core.ignorecase",
    "core.precomposeunicode",
    "core.symlinks",
    "remote.url",
    "remote.fetch",
    "remote.pushurl",
    "remote.mirror",
    "branch.remote",
    "branch.merge",
    "branch.rebase",
    "submodule.active",
    "submodule.url",
    "user.name",
    "user.email",
    "pull.rebase",
    "push.default",
    "init.defaultbranch",
    // Inert settings ordinary repositories carry that a first-draft allowlist
    // would refuse, making the tool useless on a large class of real
    // workspaces. Each is a value git *interprets*; none names a program git
    // runs.
    "core.autocrlf",
    "core.eol",
    "core.untrackedcache",
    "core.longpaths",
    "core.fscache",
    "core.hidedotfiles",
    "core.sparsecheckout",
    "core.sparsecheckoutcone",
    "commit.gpgsign",
    "tag.gpgsign",
    "remote.tagopt",
    "remote.prune",
    "remote.partialclonefilter",
    "remote.promisor",
    "branch.vscodemerge",
    "gc.auto",
    "fetch.prune",
    // SHA-256 repositories and worktree-scoped config. Only these two
    // `extensions.*` keys — the namespace as a whole is where git puts
    // repository-format switches, and a blanket allow would admit whatever it
    // adds next.
    "extensions.objectformat",
    "extensions.worktreeconfig",
    // `filter.<driver>.required` is a boolean. The driver's actual programs —
    // `clean`, `smudge`, `process` — are NOT here and must not be; see the LFS
    // note on `NEUTRALISED_CONFIG`.
    "filter.required",
    "lfs.repositoryformatversion",
];

/// Command-valued keys cleared on the command line as a second layer.
///
/// Command-line `-c` outranks every config file, so this genuinely
/// neutralises these keys even when a repository sets them — and it does so
/// at the moment `git` actually runs, not at the moment
/// [`first_disallowed_repo_config_key`] happened to inspect the config a
/// moment earlier. That distinction matters: the inspection and the real
/// command are two separate `git` invocations, so a key set in the gap
/// between them (a second concurrent writer, or a worktree-scoped write) is
/// invisible to the inspection but still reaches here — where it is
/// neutralised regardless of timing. It is a denylist and therefore cannot be
/// the *only* guarantee — [`ALLOWED_REPO_CONFIG`] is the one that fails
/// closed on a key nobody anticipated — but this layer is what actually holds
/// under a race, not the inspection step.
///
/// `credential.helper` is command-valued — a value beginning `!` is run as a
/// shell command — so it belongs nowhere near [`ALLOWED_REPO_CONFIG`] despite
/// reading like a mere preference; it is refused there instead, since none of
/// this tool's operations reach a remote and so have no legitimate use for it
/// to preserve.
///
/// A `git lfs install` clone is refused, deliberately: `filter.lfs.clean`,
/// `.smudge` and `.process` each name a program, so an LFS working copy
/// cannot be read by this tool. That is the fail-closed answer and it is the
/// intended one. Only `filter.<driver>.required`, a boolean, is allowed.
///
/// `core.hooksPath` and `commit.gpgSign` are handled separately, in
/// [`hardened_git`] — see there for why.
const NEUTRALISED_CONFIG: &[&str] = &[
    "core.fsmonitor=",
    "core.sshCommand=",
    "core.pager=cat",
    "core.editor=false",
    "diff.external=",
    "sequence.editor=false",
    "uploadpack.packObjectsHook=",
];

/// A path git will read as an empty config file.
///
/// `GIT_CONFIG_GLOBAL` must name something readable-and-empty rather than be
/// unset — unsetting it lets git fall back to `~/.gitconfig`, which is the
/// thing being suppressed. `/dev/null` is not a path on Windows; `NUL` is.
const NULL_CONFIG_PATH: &str = if cfg!(windows) { "NUL" } else { "/dev/null" };

/// Close the system/global config files and the command-valued `GIT_*` env
/// vars on a `git` invocation, without touching anything about how it reads
/// the repository's own config.
///
/// `GIT_CONFIG_NOSYSTEM` and `GIT_CONFIG_GLOBAL` close the system and global
/// config files. Note what they do *not* close: the repository's own local
/// and worktree-scoped config, which is what an agent-writable workspace
/// actually lets an attacker author. That is handled by
/// [`first_disallowed_repo_config_key`] — a separate step, precisely because
/// [`hardened_git`]'s `-c` layer below must not be present while that step is
/// reading what the repository itself set (see its own doc comment).
fn suppress_ambient_git_config(cmd: &mut tokio::process::Command) -> &mut tokio::process::Command {
    cmd.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", NULL_CONFIG_PATH)
        // `git` consults these before it reads any config file.
        .env_remove("GIT_EXTERNAL_DIFF")
        .env_remove("GIT_PAGER")
        .env_remove("GIT_EDITOR")
        .env_remove("GIT_SSH")
        .env_remove("GIT_SSH_COMMAND")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env("GIT_TERMINAL_PROMPT", "0")
}

/// Build the `git` invocation actually used to run a requested operation.
///
/// Layers [`suppress_ambient_git_config`] under the [`NEUTRALISED_CONFIG`]
/// `-c` overrides, which outrank every config file the repository itself
/// could set — including one set in the gap between
/// [`first_disallowed_repo_config_key`]'s inspection and this invocation.
///
/// Two more `-c` overrides are added here rather than in the static
/// [`NEUTRALISED_CONFIG`] list, because their safe value is not a fixed
/// literal:
///
/// - `core.hooksPath=` pointed at [`NULL_CONFIG_PATH`]. `commit` and
///   `checkout` are the two operations this tool exposes that run hooks
///   (`pre-commit`/`commit-msg`/`post-commit`, `post-checkout`), and a
///   repository-writable `core.hooksPath` naming a directory with an
///   executable `pre-commit` in it is exactly the shape of the worktree-scoped
///   bypass this hardening exists to close — verified directly: pointing
///   `core.hooksPath` at a directory containing a `pre-commit` that touches a
///   marker file, then running with `-c core.hooksPath=<null path>`, leaves
///   the marker untouched. A previous version of this comment claimed there
///   was no portable value that meant "nowhere"; there is — the same one
///   [`suppress_ambient_git_config`] already uses for `GIT_CONFIG_GLOBAL`,
///   since a location with nothing at it is exactly what git needs it to be.
/// - `commit.gpgSign=false`. `commit.gpgsign` is on [`ALLOWED_REPO_CONFIG`] as
///   an ordinary boolean, but a repository could still set it to force every
///   commit through this tool to be GPG-signed — with whatever real signing
///   key the *host* happens to have configured, silently attributing a
///   cryptographic signature to a commit the operator did not ask this tool to
///   sign. Overriding it here removes that decision from the repository
///   entirely, the same way `core.editor=false` removes commit message
///   editing from it in [`NEUTRALISED_CONFIG`] above.
fn hardened_git(dir: &Path) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new("git");
    suppress_ambient_git_config(&mut cmd).current_dir(dir);
    for kv in NEUTRALISED_CONFIG {
        cmd.arg("-c").arg(kv);
    }
    cmd.arg("-c")
        .arg(format!("core.hooksPath={NULL_CONFIG_PATH}"))
        .arg("-c")
        .arg("commit.gpgSign=false");
    cmd
}

/// Normalise a `git config --list` key to the `section.key` form
/// [`ALLOWED_REPO_CONFIG`] uses, dropping any subsection.
///
/// `remote.origin.url` → `remote.url`; `core.filemode` → `core.filemode`. A
/// subsection may itself contain dots (`includeIf.gitdir:~/x.y/.path`), so the
/// first and last components are the reliable ones.
fn normalise_config_key(key: &str) -> String {
    let key = key.to_ascii_lowercase();
    match (key.find('.'), key.rfind('.')) {
        (Some(first), Some(last)) if first != last => {
            format!("{}.{}", &key[..first], &key[last + 1..])
        }
        _ => key,
    }
}

/// The refusal reported when a repository's own config sets a key this tool
/// will not run under.
///
/// Shared by the two sites that refuse — [`GitOperationsTool::run_git_command_in`],
/// at the moment it would spawn `git`, and `execute_in_context`, which refuses
/// before its worktree probe — so the two can never drift into describing the
/// same condition differently.
fn untrusted_repo_config_refusal(dir: &Path, key: &str) -> String {
    format!(
        "refusing to run git in {}: its repository config sets `{key}`, which is \
         not on the allowlist of configuration this tool will run under. \
         Several git config keys name a command git then executes, and this \
         directory is agent-writable, so unrecognised configuration is treated \
         as untrusted rather than honoured.",
        dir.display()
    )
}

/// Returns the first repository config key at `dir` that is not on
/// [`ALLOWED_REPO_CONFIG`], or `None` if every key it sets is recognised.
///
/// Deliberately **not** `--local`: when `extensions.worktreeConfig` is set —
/// itself on [`ALLOWED_REPO_CONFIG`] as an ordinary, non-command-valued
/// setting — git additionally reads `config.worktree`, a second file `--local`
/// does not cover. A `core.hooksPath` set with `git config --worktree ...`
/// lands there, is invisible to `--local`, and is exactly the kind of key this
/// check exists to catch: `commit` runs the hook it names. Bare `--list`
/// returns the same merged view `git` itself consults — local *and*
/// worktree-scoped — with system and global excluded by
/// [`suppress_ambient_git_config`] instead of by a location flag. Verified
/// directly: `git config --worktree core.hooksPath ...` followed by
/// `--local --null` omits it; the same followed by a bare `--list --null`
/// (system/global suppressed) reports it.
///
/// This step must run through [`suppress_ambient_git_config`] alone, **not**
/// [`hardened_git`]: the latter's `-c` layer would inject the very keys this
/// check inspects for (`core.fsmonitor=`, `core.pager=cat`, …), none of which
/// are themselves on [`ALLOWED_REPO_CONFIG`], and every invocation would
/// refuse itself. Reading the config this way also does not consult
/// `core.fsmonitor` or spawn a pager when its output is captured, so this
/// inspection step does not have the property it is checking for.
///
/// A non-zero exit here is treated as a refusal, not as "nothing to
/// distrust": `git config --list` failing means the config could not be read,
/// not that there is none. Proceeding to run the real command against config
/// this step never actually inspected would defeat the point of inspecting it
/// first. `execute_in_context` calls this before it establishes whether `dir`
/// is a work tree at all, so "not a repository" also surfaces here as a
/// refusal rather than as an empty config — which is the correct direction to
/// fail in.
async fn first_disallowed_repo_config_key(dir: &Path) -> anyhow::Result<Option<String>> {
    let mut cmd = tokio::process::Command::new("git");
    suppress_ambient_git_config(&mut cmd).current_dir(dir);
    let output = cmd.args(["config", "--list", "--null"]).output().await?;

    if !output.status.success() {
        anyhow::bail!(
            "refusing to run git in {}: could not inspect its repository config \
             ({}), so whether it is safe to run under could not be determined",
            dir.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let listing = String::from_utf8_lossy(&output.stdout);
    for entry in listing.split('\0').filter(|e| !e.is_empty()) {
        // `--null` separates entries with NUL and key from value with LF.
        let key = entry.split('\n').next().unwrap_or(entry);
        if !ALLOWED_REPO_CONFIG.contains(&normalise_config_key(key).as_str()) {
            return Ok(Some(key.to_string()));
        }
    }
    Ok(None)
}

#[async_trait]
impl Tool for GitOperationsTool {
    fn name(&self) -> &str {
        "git_operations"
    }

    fn description(&self) -> &str {
        "Perform structured Git operations (status, diff, log, branch, commit, add, checkout, stash). Provides parsed JSON output and integrates with security policy for autonomy controls."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "operation": {
                    "type": "string",
                    "enum": ["status", "diff", "log", "branch", "commit", "add", "checkout", "stash"],
                    "description": "Git operation to perform"
                },
                "message": {
                    "type": "string",
                    "description": "Commit message (for 'commit' operation)"
                },
                "paths": {
                    "type": "string",
                    "description": "File paths to stage (for 'add' operation)"
                },
                "branch": {
                    "type": "string",
                    "description": "Branch name (for 'checkout' operation)"
                },
                "files": {
                    "type": "string",
                    "description": "File or path to diff (for 'diff' operation, default: '.')"
                },
                "cached": {
                    "type": "boolean",
                    "description": "Show staged changes (for 'diff' operation)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Number of log entries (for 'log' operation, default: 10)"
                },
                "action": {
                    "type": "string",
                    "enum": ["push", "pop", "list", "drop"],
                    "description": "Stash action (for 'stash' operation)"
                },
                "index": {
                    "type": "integer",
                    "description": "Stash index (for 'stash' with 'drop' action)"
                }
            },
            "required": ["operation"]
        })
    }

    fn supports_markdown(&self) -> bool {
        true
    }

    /// Write git operations (commit/add/checkout/stash/…) route through the
    /// human approval gate in ask-before-edit; read operations (status/diff/
    /// log/branch) never prompt. In Full, writes run; read-only is blocked in
    /// `execute` via the existing `can_act()` / autonomy check.
    fn external_effect_with_args(&self, args: &serde_json::Value) -> bool {
        let operation = args.get("operation").and_then(|v| v.as_str()).unwrap_or("");
        self.requires_write_access(operation)
            && self.security.gate_decision(CommandClass::Write) == GateDecision::Prompt
    }

    async fn execute_with_options(
        &self,
        args: serde_json::Value,
        _options: ToolCallOptions,
    ) -> anyhow::Result<ToolResult> {
        // git_operations always populates `markdown_formatted` for the
        // structured sub-operations (status/diff/log/branch). The harness
        // picks it up when `prefer_markdown` is on; the JSON content
        // block is preserved for callers that want the raw structure.
        self.execute_in_context(args, None).await
    }

    async fn execute_with_context(
        &self,
        args: serde_json::Value,
        _options: ToolCallOptions,
        context: Option<&dyn ToolRunContext>,
    ) -> anyhow::Result<ToolResult> {
        self.execute_in_context(args, context).await
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        self.execute_in_context(args, None).await
    }
}

impl GitOperationsTool {
    async fn execute_in_context(
        &self,
        args: serde_json::Value,
        context: Option<&dyn ToolRunContext>,
    ) -> anyhow::Result<ToolResult> {
        let operation = match args.get("operation").and_then(|v| v.as_str()) {
            Some(op) => op,
            None => {
                return Ok(ToolResult::error("Missing 'operation' parameter"));
            }
        };

        let effective_dir = self.effective_action_dir_for_context(context);

        // Repository-config refusal comes first, and the ordering is the point.
        // The worktree probe below is itself a `run_git_command_in` call, which
        // refuses when the config names a command git would execute — and its
        // `is_ok_and` flattened that refusal into `false`, so the operation
        // answered "Not in a git repository": untrue of a directory that plainly
        // is one, and it discarded the key that caused the refusal. Both
        // orderings return before anything runs under the untrusted config, so
        // what this restores is the diagnostic, not the guard.
        if let Some(key) = first_disallowed_repo_config_key(&effective_dir).await? {
            tracing::debug!(
                "[git_operations] refusing operation: dir={}, operation={operation}, \
                 disallowed_config_key={key}",
                effective_dir.display()
            );
            return Ok(ToolResult::error(untrusted_repo_config_refusal(
                &effective_dir,
                &key,
            )));
        }

        // Validate the repository instead of trusting the presence of a `.git`
        // path. This handles linked-worktree gitfiles and rejects malformed
        // ancestor markers that Git itself cannot open.
        let is_worktree = self
            .run_git_command_in(&effective_dir, &["rev-parse", "--is-inside-work-tree"])
            .await
            .is_ok_and(|output| output.trim() == "true");
        if !is_worktree {
            return Ok(ToolResult::error("Not in a git repository"));
        }

        // Check autonomy level for write operations
        if self.requires_write_access(operation) {
            if !self.security.can_act() {
                return Ok(ToolResult::error(
                    "[policy-blocked] Action blocked: git write operations require higher autonomy level",
                ));
            }

            match self.security.autonomy {
                AutonomyLevel::ReadOnly => {
                    return Ok(ToolResult::error(
                        "[policy-blocked] Action blocked: read-only mode",
                    ));
                }
                AutonomyLevel::Supervised | AutonomyLevel::Full => {}
            }
        }

        // Record action for rate limiting
        if !self.security.record_action() {
            return Ok(ToolResult::error("Action blocked: rate limit exceeded"));
        }

        // Execute the requested operation
        match operation {
            "status" => self.git_status(&effective_dir, args).await,
            "diff" => self.git_diff(&effective_dir, args).await,
            "log" => self.git_log(&effective_dir, args).await,
            "branch" => self.git_branch(&effective_dir, args).await,
            "commit" => self.git_commit(&effective_dir, args).await,
            "add" => self.git_add(&effective_dir, args).await,
            "checkout" => self.git_checkout(&effective_dir, args).await,
            "stash" => self.git_stash(&effective_dir, args).await,
            _ => Ok(ToolResult::error(format!("Unknown operation: {operation}"))),
        }
    }
}

fn render_status_markdown(result: &serde_json::Map<String, serde_json::Value>) -> String {
    let mut out = String::new();
    if let Some(branch) = result.get("branch").and_then(|v| v.as_str()) {
        out.push_str(&format!("**branch**: `{branch}`\n"));
    }
    let clean = result
        .get("clean")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if clean {
        out.push_str("_Working tree clean._\n");
        return out;
    }
    let push_section = |out: &mut String, label: &str, items: Option<&Vec<serde_json::Value>>| {
        if let Some(items) = items {
            if !items.is_empty() {
                out.push_str(&format!("\n**{label}** ({})\n", items.len()));
                for it in items {
                    if let (Some(p), Some(s)) = (
                        it.get("path").and_then(|v| v.as_str()),
                        it.get("status").and_then(|v| v.as_str()),
                    ) {
                        out.push_str(&format!("- `{s}` {p}\n"));
                    }
                }
            }
        }
    };
    push_section(
        &mut out,
        "staged",
        result.get("staged").and_then(|v| v.as_array()),
    );
    push_section(
        &mut out,
        "unstaged",
        result.get("unstaged").and_then(|v| v.as_array()),
    );
    if let Some(items) = result.get("untracked").and_then(|v| v.as_array()) {
        if !items.is_empty() {
            out.push_str(&format!("\n**untracked** ({})\n", items.len()));
            for it in items {
                if let Some(p) = it.as_str() {
                    out.push_str(&format!("- {p}\n"));
                }
            }
        }
    }
    out
}

fn render_log_markdown(commits: &[serde_json::Value]) -> String {
    if commits.is_empty() {
        return "_No commits._".to_string();
    }
    let mut out = format!("# Commits ({})\n", commits.len());
    for c in commits {
        let hash = c.get("hash").and_then(|v| v.as_str()).unwrap_or("");
        let short = hash.get(..hash.len().min(8)).unwrap_or(hash);
        let author = c.get("author").and_then(|v| v.as_str()).unwrap_or("");
        let date = c.get("date").and_then(|v| v.as_str()).unwrap_or("");
        let msg = c.get("message").and_then(|v| v.as_str()).unwrap_or("");
        out.push_str(&format!("- `{short}` {msg} _(by {author}, {date})_\n"));
    }
    out
}

fn render_branch_markdown(current: &str, branches: &[serde_json::Value]) -> String {
    let mut out = format!("**current**: `{current}`\n\n## Branches\n");
    for b in branches {
        let name = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let cur = b.get("current").and_then(|v| v.as_bool()).unwrap_or(false);
        if cur {
            out.push_str(&format!("- **{name}** ← current\n"));
        } else {
            out.push_str(&format!("- {name}\n"));
        }
    }
    out
}

#[cfg(test)]
#[path = "git_operations_tests.rs"]
mod tests;
