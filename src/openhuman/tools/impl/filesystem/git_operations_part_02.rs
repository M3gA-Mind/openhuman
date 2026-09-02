
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
/// distrust": by the time this runs, the caller (`execute_in_context`) has
/// already confirmed `dir` — or one of its parents — contains a `.git`, so
/// `git config --list` failing means the config could not be read, not that
/// there is none. Proceeding to run the real command against config this step
/// never actually inspected would defeat the point of inspecting it first.
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
