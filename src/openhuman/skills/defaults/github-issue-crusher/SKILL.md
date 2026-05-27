# GitHub Issue Crusher

Fix the **single** GitHub issue named in the inputs, end to end, then open a pull request — handling the **fork workflow**: the issue lives on the upstream repo `{repo}`, you push your fix to a **fork**, and you open a **cross-repo PR** back to `{repo}`. Stay strictly scoped to this one issue — do not pick up unrelated work.

## The two repos
- **Upstream** = `{repo}` — where issue `#{issue}` lives and where the PR is opened (base = `{pr_base}`, or the upstream's default branch).
- **Fork** = `{fork}` if provided, otherwise a fork under the **connected GitHub account** — where your fix branch is pushed.
- You act as the **connected GitHub identity**. **Commit through the GitHub API** — assume you have *no* local `git push` credentials for the fork. Never block on `git push`.

## Steps

1. **Read the issue.** Fetch issue `#{issue}` in `{repo}` (title, body, comments) via the GitHub tool. Note the connected login — it namespaces the PR head.
2. **Ensure the fork.** If `{fork}` is set, use it. Otherwise fork `{repo}` under the connected account (create the fork if it doesn't exist) and use that. Call its owner `<fork-owner>`.
3. **Get the code locally.** Clone the **upstream** `{repo}` to a worktree at `{pr_base}` (or its default branch). Start `codegraph_index` on the worktree (background — don't wait).
4. **Locate the cause.** Call `codegraph_search` with the issue's key symbols / error strings. **Respect the `coverage` flag** — if it's not `full`, treat hits as hints and also use `grep`/`lsp`; re-search as coverage grows. Open the top candidates and confirm the exact edit site.
5. **Fix.** Make the **minimal** change locally. Re-`read` / `git diff` instead of trusting memory.
6. **Verify.** Run the relevant tests + linter locally; iterate until green.
7. **Push to the fork via the API.** Create a fix branch `fix/{issue}-<short-slug>` on the **fork** (a ref off the base commit). Apply your changed files (from `git diff`) onto that branch **through the GitHub API** — for a multi-file change prefer a single commit (blob → tree → commit → update-ref); for one or two files, create-or-update file contents is fine. **Do not `git push`.**
8. **Open the cross-repo PR.** Open a PR **against `{repo}`** with **head = `<fork-owner>:fix/{issue}-<short-slug>`** and **base = `{pr_base}`** (or the upstream default). The body must include `Closes #{issue}`, a short root-cause + fix summary, and how you verified.

## Rules

- **Scope:** only changes that fix `#{issue}`.
- **Two repos:** the issue + PR target are the upstream `{repo}`; the branch + commits live on the **fork**; the PR is **cross-repo** (head = fork, base = upstream).
- **API commits only:** the host has no fork push credentials — push the diff via the GitHub API as the connected identity; never block on `git push`.
- **Source of truth** is the filesystem + `git` + `codegraph` — re-read / re-search rather than relying on recall; recover progress with `git diff`.
- **codegraph is an accelerant, not a gate:** if it's cold or unavailable, fall back to `grep`/`lsp` — never block on indexing.
- You are the **orchestrator**: delegate narrow, well-scoped subtasks to subagents when it helps, but keep ownership of the single end goal.
- **Stop** when the PR is open, or surface a blocker plainly and stop — don't thrash.
