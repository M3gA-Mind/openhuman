# GitHub Issue Crusher

Fix the **single** GitHub issue named in the inputs, end to end, then open a pull request. Stay strictly scoped to this one issue — do not pick up unrelated work.

## Steps

1. **Read the issue.** Fetch issue `#{issue}` in `{repo}` (title, body, comments) via the GitHub tool.
2. **Get the code locally.** Ensure `{repo}` is checked out (clone if needed); create a fix branch `fix/{issue}-<short-slug>`. Start `codegraph_index` on the worktree (background — don't wait).
3. **Locate the cause.** Call `codegraph_search` with the issue's key symbols / error strings. **Respect the `coverage` flag** — if it's not `full`, treat the hits as hints and also use `grep`/`lsp`; re-search as coverage grows. Open the top candidates and confirm the exact edit site.
4. **Fix.** Make the **minimal** change. Re-`read` / `git diff` instead of trusting memory.
5. **Verify.** Run the relevant tests + linter; iterate until green.
6. **Open the PR.** Commit, push the fix branch, and open a PR against `{pr_base}` (or the repo's default branch) via the GitHub tool. The body must include `Closes #{issue}`, a short root-cause + fix summary, and how it was verified.

## Rules

- **Scope:** only changes that fix `#{issue}`.
- **Source of truth** is the filesystem + `git` + `codegraph` — re-read / re-search rather than relying on recall; recover progress with `git diff`.
- **codegraph is an accelerant, not a gate:** if it's cold or unavailable, fall back to `grep`/`lsp` — never block on indexing.
- You are the **orchestrator**: delegate narrow, well-scoped subtasks to subagents when it helps, but keep ownership of the single end goal.
- **Stop** when the PR is open, or surface a blocker plainly and stop — don't thrash.
