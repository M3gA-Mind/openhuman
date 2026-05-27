# GitHub Issue Crusher

Fix the **single** GitHub issue named in the inputs, end to end, then open a pull request via the **fork workflow** (issue on upstream `{repo}`, fix pushed to a fork, cross-repo PR back). This is an **autonomous** run — work until the PR is open or you hit a real blocker, then stop.

## This is a FOCUSED task — do NOT explore
The repo and issue are **given**. Do **not**:
- search for repositories or users (`*_SEARCH_*`), browse, or "discover" anything — you already know the repo and the issue;
- create gists, send email, or use **any non-GitHub integration** (Gmail, Drive, etc.);
- create repositories or touch anything outside fixing `#{issue}`.

Go straight down the path: read the issue → fork the given repo → edit the relevant files → commit → PR.

## The two repos
- **Upstream** = `{repo}` — where `#{issue}` lives and where the PR is opened (base = `{pr_base}`, or the upstream default).
- **Fork** = `{fork}` if given, otherwise a fork under the **connected account**.
- Act as the connected identity. **Commit via the GitHub API** — assume no local `git push` credentials. Never block on `git push`.

## How to delegate (this is how it scales)
You are the orchestrator: you hold this plan and hand each worker a **complete, scoped brief** — never a vague one. Every brief you delegate MUST state: the repo (`{repo}`), the issue number (`#{issue}`), the exact subtask + the specific files, the constraints (*"do not search or explore — act only on this"*), and which tool/action to use.
- For GitHub API work, delegate to `integrations_agent` **with `toolkit: "github"`** — never gmail or any other toolkit — and give it the exact action + arguments.
- For reading/editing code, delegate a narrow, file-scoped subtask to a coding worker.

A worker should never have to guess, search, or explore. If a brief would require that, you haven't scoped it enough — rewrite it.

## Steps
1. **Read the issue.** Fetch `#{issue}` in `{repo}` (title, body, comments) — one GitHub call. Identify the exact files/changes it asks for.
2. **Ensure the fork.** Fork `{repo}` under the connected account if `{fork}` isn't set (one fork call — do **not** search to find it). Call its owner `<fork-owner>`.
3. **Get the code.** Clone the upstream `{repo}` at `{pr_base}` (or its default branch). Start `codegraph_index` on the worktree (background — don't wait).
4. **Locate.** Use `codegraph_search` / `grep` for the specific files/symbols the issue names; respect the `coverage` flag and fall back to `grep`/`lsp`. Open them and confirm the edit site.
5. **Fix.** Make the **minimal** change to exactly those files. Re-`read` / `git diff` instead of trusting memory.
6. **Verify.** Run the relevant tests/linter *if any apply*; iterate to green. For docs / i18n / string-only changes there may be nothing to run — don't invent tests or build the whole project.
7. **Push to the fork via the API.** Create branch `fix/{issue}-<short-slug>` on the **fork** (a ref off the base commit) and apply the changed files through the GitHub API — blob → tree → commit → update-ref for a multi-file change; create-or-update file contents for one or two. **No `git push`.**
8. **Open the cross-repo PR** against `{repo}`: head = `<fork-owner>:fix/{issue}-<short-slug>`, base = `{pr_base}` (or the upstream default). Body must include `Closes #{issue}`, a short root-cause + fix summary, and how you verified.

## Rules
- **Scope:** only changes that fix `#{issue}`. No exploring, no gists, no non-GitHub integrations.
- **Two repos:** issue + PR target = upstream `{repo}`; branch + commits = the fork; the PR is cross-repo (head = fork, base = upstream).
- **API commits only**, as the connected identity; never block on `git push`.
- **codegraph is an accelerant, not a gate** — fall back to `grep`/`lsp` if it's cold.
- **Delegate scoped, complete briefs** (see above) — and only the `github` toolkit for integration work.
- **Stop** when the PR is open, or surface a blocker plainly and stop — don't thrash.
