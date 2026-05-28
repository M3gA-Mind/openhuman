# GitHub Issue Crusher

Fix the **single** GitHub issue named in the inputs, end to end, then open a **draft** pull request via the **fork workflow** (issue on upstream `{repo}`, fix pushed to a fork, cross-repo draft PR back). This is an **autonomous** run — work until the draft PR is open or you hit a real blocker, then stop.

## This is a FOCUSED task — do NOT explore
The repo and issue are **given**. Do **not**:
- search for repositories or users (`*_SEARCH_*`), browse, or "discover" anything — you already know the repo and the issue;
- create gists, send email, or use **any non-GitHub integration** (Gmail, Drive, etc.);
- create repositories or touch anything outside fixing `#{issue}`.

Go straight: read the issue → ensure fork → clone → pin git identity → edit → verify → commit + push → open the **draft** PR.

## Identity & transport — local `git` + `gh` for writes, Composio for reads
- **Reads** (issue body, comments, repo metadata): use Composio with `toolkit: "github"`.
- **Writes** (clone, branch, commit, push, open PR): use **local `git` + `gh`** via shell. The host already has `gh` authed and `git` configured for the user's GitHub account — use them. **Do NOT use Composio to commit or push** (the raw `blob → tree → commit → ref` API is fragile and you'll churn).
- **One identity end to end**: the commit author, the push credential, and the PR opener must all be the same GitHub account. Pin the commit identity in the clone (step 4) — otherwise commits show "Unverified" and provenance is muddled.

## The two repos
- **Upstream** = `{repo}` — where `#{issue}` lives and where the draft PR is opened (base = `{pr_base}`, or the upstream's default branch).
- **Fork** = `{fork}` if given, otherwise the existing fork of `{repo}` under the **authed GitHub account** (`gh api user --jq .login`). If no fork exists, create one: `gh repo fork {repo} --remote=false --clone=false`. Call its owner `<fork-owner>`.

## How to delegate (this is how it scales)
You are the orchestrator: you hold this plan and hand each worker a **complete, scoped brief** — never a vague one. Every brief MUST state: the repo (`{repo}`), the issue (`#{issue}`), the exact subtask + the specific files, the constraints (*"do not search or explore — act only on this"*), and which tool/command to use.
- For GitHub **reads**, delegate to `integrations_agent` with `toolkit: "github"` — give it the exact action + arguments.
- For **clone / edit / commit / push / PR**, delegate to a coding worker that uses `git` and `gh` via shell — never Composio for writes.

A worker should never have to guess, search, or explore. If a brief would require that, you haven't scoped it enough — rewrite it.

## Steps

1. **Read the issue.** Fetch `#{issue}` in `{repo}` (title, body, comments) via Composio (`toolkit: github`) — one read. Identify the exact files/changes it asks for.
2. **Ensure the fork** under the authed account (`gh api user --jq .login` → `<fork-owner>`). If it doesn't exist: `gh repo fork {repo} --remote=false --clone=false`.
3. **Clone upstream and start indexing.**
   ```
   git clone https://github.com/{repo} <local-dir>
   ```
   Then start `codegraph_index` on `<local-dir>` (background — don't wait).
4. **Pin the LOCAL git identity in the clone** — never `--global`, never clobber the host's global config:
   ```
   git -C <local-dir> config user.name  "$(gh api user --jq .login)"
   git -C <local-dir> config user.email "$(gh api user --jq '"\(.id)+\(.login)@users.noreply.github.com"')"
   ```
   This pins the commit author to the authed GitHub account so commits stay **verified** and the PR provenance reads cleanly.
5. **Locate the edit site.** Use `codegraph_search` first (it auto-indexes); fall back to `grep`/`glob`/`lsp` to refine or when coverage isn't `full`. Open the top candidates and confirm the exact lines to change.
6. **Fix.** Make the **minimal** change to exactly those files. Re-`read` / `git diff` instead of trusting memory.
7. **Verify.** Run the relevant tests/linter *if any apply*; iterate to green. For docs / i18n / string-only changes there may be nothing to run — don't invent tests or build the whole project.
8. **Commit + push to the fork via local git.**
   ```
   git -C <local-dir> checkout -b fix/{issue}-<short-slug>
   git -C <local-dir> add <only-the-changed-files>     # never git add -A
   git -C <local-dir> commit -m "<type>(scope): <short description> (#{issue})"
   gh repo set-default {repo}                          # so subsequent gh calls target upstream
   git -C <local-dir> push -u "https://github.com/<fork-owner>/<repo-name>" fix/{issue}-<short-slug>
   ```
9. **Open the DRAFT cross-repo PR via `gh`:**
   ```
   gh pr create -R {repo} --draft \
       --head "<fork-owner>:fix/{issue}-<short-slug>" \
       --base "{pr_base}" \
       --title "<type>(scope): <short description> (#{issue})" \
       --body "Closes #{issue}\n\n## Root cause\n<one paragraph>\n\n## Fix\n<one paragraph>\n\n## Verified\n<what you ran>"
   ```
   **Always `--draft`.** This is non-negotiable for autonomous runs — CI runs and a human reviews before the PR is promoted to ready. Do not open as ready-to-merge.

## Rules
- **Scope:** only changes that fix `#{issue}`. No exploring, no gists, no non-GitHub integrations.
- **Two repos:** issue + PR target = upstream `{repo}`; branch + commits = the fork; the PR is **cross-repo, draft** (head = fork, base = upstream).
- **Writes via local `git` + `gh`** (not Composio). Composio is read-only for the GitHub surface in this skill.
- **One identity end to end** (step 4): commit author == push credential == PR opener.
- **DRAFT always** (step 9): `--draft` is required.
- **codegraph is an accelerant, not a gate** — fall back to `grep`/`lsp` if it's cold.
- **Delegate scoped, complete briefs** — workers must never have to guess or explore.
- **Stop** when the draft PR is open, or surface a blocker plainly and stop — don't thrash.
