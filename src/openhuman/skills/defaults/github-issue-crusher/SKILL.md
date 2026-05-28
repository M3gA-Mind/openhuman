# GitHub Issue Crusher

Fix the **single** GitHub issue named in the inputs, end to end, then open a **draft** pull request via the **fork workflow** (issue on upstream `{repo}`, fix pushed to a fork, cross-repo draft PR back). This is an **autonomous** run — work until the draft PR is open or you hit a real blocker, then stop.

## This is a FOCUSED task — do NOT explore
The repo and issue are **given**. Do **not**:
- search for repositories or users (`*_SEARCH_*`), browse, or "discover" anything — you already know the repo and the issue;
- create gists, send email, or use **any non-GitHub integration** (Gmail, Drive, etc.);
- create repositories or touch anything outside fixing `#{issue}`.

Go straight: read the issue → ensure fork → clone → pin git identity → **codegraph_search to locate** → edit → verify → commit + push → open the **draft** PR.

## Tools — name the delegate, do not improvise
You only have two delegation tools in this skill. Pick the right one for each step — naming the tool *literally*:

- **`delegate_to_integrations_agent`** with `toolkit: "github"` — for **reads** of the issue body / comments / repo metadata. Pass the exact Composio action + arguments. Do **not** use it to commit, push, or open PRs (the raw `blob → tree → commit → ref` GitHub API is fragile and the worker will churn).
- **`delegate_run_code`** — for **everything that touches a file on disk OR runs a shell command**: clone, navigate with `codegraph_search`, `edit` / `apply_patch` / `file_write`, run tests, `git`, `gh`. This is the `code_executor` agent and it is the **only** worker with `edit`, `apply_patch`, `file_write`, `shell`, `git_operations` on its tool surface. **Do NOT** route file edits to `tools_agent` or `spawn_worker_thread` — those workers don't have edit tools and will silently stall in read-mode. Every iteration that needs a file changed MUST be `delegate_run_code`.

**One identity end to end**: commit author == push credential == PR opener (the authed GitHub account). Pin the commit identity in the clone (step 4) — otherwise commits show "Unverified".

## codegraph_search is MANDATORY for locating code
Every "find where to edit" brief MUST start with `codegraph_search`. `codegraph_search` auto-indexes the repo on its first call (~30–90s build on a fresh repo — this is normal, not a hang; subsequent calls are millisecond-cheap). Only after `codegraph_search` returns may the worker call `grep` / `glob` / `file_read` / `shell` to refine. **A locate brief that calls `grep` before `codegraph_search` is malformed — rewrite it before sending.**

Why it's mandatory even for "obvious" string searches: codegraph returns ranked structural+semantic hits across the whole repo with one call; a blind grep returns every occurrence and forces the worker to re-rank by hand. Even for literal-string queries (i18n keys, error strings, config names) codegraph is strictly better signal-per-call than grep — and we want every run to exercise it so we discover where it actually falls short.

## How to delegate (this is how it scales)
You are the orchestrator: you hold this plan and hand each worker a **complete, scoped brief** — never a vague one. Every brief MUST state: the repo (`{repo}`), the issue (`#{issue}`), the exact subtask + the specific files, the constraints (*"do not search or explore — act only on this"*), and the **literal tool calls** the worker should make (`codegraph_search`, `edit`, `apply_patch`, `shell`, `git_operations`, …) in the order they should be called.

A worker should never have to guess, search, or explore. If a brief would require that, you haven't scoped it enough — rewrite it.

## The two repos
- **Upstream** = `{repo}` — where `#{issue}` lives and where the draft PR is opened (base = `{pr_base}`, or the upstream's default branch).
- **Fork** = `{fork}` if given, otherwise the existing fork of `{repo}` under the **authed GitHub account** (`gh api user --jq .login`). If no fork exists, create one: `gh repo fork {repo} --remote=false --clone=false`. Call its owner `<fork-owner>`.

## Steps

> Step 1 uses `delegate_to_integrations_agent`. **Steps 2–9 all use `delegate_run_code`** with scoped briefs — every brief names the exact tool calls so the worker has no room to drift into read-only exploration.

1. **Read the issue** — `delegate_to_integrations_agent { toolkit: "github" }`. Brief: fetch issue `#{issue}` from `{repo}` (title, body, comments). One call. Identify the exact files/changes it asks for.

2. **Ensure the fork** — `delegate_run_code`. Brief: run `gh api user --jq .login` to obtain `<fork-owner>`. Check whether `<fork-owner>/<repo-name>` exists with `gh repo view <fork-owner>/<repo-name>`. If not, run `gh repo fork {repo} --remote=false --clone=false`. Report `<fork-owner>` back.

3. **Clone upstream** — `delegate_run_code`. Brief: run `git clone https://github.com/{repo} <local-dir>` and report the path back. **Do NOT call `codegraph_index` here** — `codegraph_search` in step 5 auto-indexes on its first call, so a separate index call is redundant and wastes a turn.

4. **Pin the LOCAL git identity in the clone** — `delegate_run_code`. Brief: run these exact `shell` commands inside `<local-dir>`. Never `--global`; never clobber the host's global config:
   ```
   git -C <local-dir> config user.name  "$(gh api user --jq .login)"
   git -C <local-dir> config user.email "$(gh api user --jq '"\(.id)+\(.login)@users.noreply.github.com"')"
   ```
   This pins the commit author to the authed GitHub account so commits stay **verified**.

5. **Locate the edit site (codegraph-first, NO exceptions)** — `delegate_run_code`. The brief MUST be exactly this shape:

   > "In `<local-dir>`, your **FIRST tool call MUST be `codegraph_search`** with `query=<concrete symbol, identifier, literal string, or short phrase from the issue>`. Do **NOT** call `grep`, `glob`, `file_read`, or `shell` before `codegraph_search` returns. codegraph auto-indexes on first call (expect ~30–90s on a fresh clone — this is the index build, **not a hang**; do not retry, do not switch to grep). After `codegraph_search` returns, inspect the result:
   >  - If `coverage` is `full` and the top hits look relevant: call `file_read` on the top 3 hits and report back files + lines to change.
   >  - If `coverage` is `partial`: call `grep` **scoped to the directories codegraph already returned**, then `file_read` the refined hits.
   >  - If and only if `coverage` is `none` or hits are zero, fall back to a blind `grep` / `glob` over the tree.
   > **Locate only — do NOT edit in this brief.** Report back: (a) the `codegraph_search` query you used, (b) the top results + coverage, (c) the exact files + lines that must change."

6. **Apply the fix** — `delegate_run_code`. Brief: "In `<local-dir>`, apply these edits — list each file by path with the before/after: `<file1>`: change `<X>` → `<Y>`. `<file2>`: …. Use **`edit`** (single-line / small change) or **`apply_patch`** (multi-file or multi-line change) for **existing** files; use `file_write` ONLY for brand-new files; never use `shell` redirection (`>`) for edits. After each file, call `shell` `git -C <local-dir> diff <file>` and confirm the diff matches. Stop after the listed files — do not edit anything else."

7. **Verify** — `delegate_run_code`. Brief: "Run only the test/lint commands that apply to the changed files (e.g. `pnpm i18n:check` for i18n parity). Do **not** build the whole project or run the full test suite. If no test applies (pure docs / string-only), say so explicitly and skip."

8. **Commit + push to the fork** — `delegate_run_code`. Single brief, exact `shell` commands:
   ```
   git -C <local-dir> checkout -b fix/{issue}-<short-slug>
   git -C <local-dir> add <only-the-changed-files>     # never git add -A
   git -C <local-dir> commit -m "<type>(scope): <short description> (#{issue})"
   gh repo set-default {repo}                          # so subsequent gh calls target upstream
   git -C <local-dir> push -u "https://github.com/<fork-owner>/<repo-name>" fix/{issue}-<short-slug>
   ```

9. **Open the DRAFT cross-repo PR** — `delegate_run_code`. Brief: run this exact `shell` command:
   ```
   gh pr create -R {repo} --draft \
       --head "<fork-owner>:fix/{issue}-<short-slug>" \
       --base "{pr_base}" \
       --title "<type>(scope): <short description> (#{issue})" \
       --body "Closes #{issue}\n\n## Root cause\n<one paragraph>\n\n## Fix\n<one paragraph>\n\n## Verified\n<what you ran>"
   ```
   **Always `--draft`.** Non-negotiable for autonomous runs — CI runs and a human reviews before promotion to ready. Do not open as ready-to-merge. Report the PR URL back.

## Rules
- **Routing:** reads → `delegate_to_integrations_agent { toolkit: "github" }`; **every** step that touches a file or runs a shell command → `delegate_run_code`. **Never** delegate file edits to `tools_agent` / `spawn_worker_thread` — those workers don't have `edit`/`apply_patch`/`file_write` and will stall in read-mode.
- **codegraph_search is mandatory** (step 5): every locate brief MUST call `codegraph_search` as its first tool call; `grep` / `glob` / `file_read` / `shell` are refinement or fallback only. A brief that omits `codegraph_search` is malformed — rewrite it.
- **No redundant `codegraph_index` call**: step 3 must NOT call `codegraph_index` — search in step 5 auto-indexes on first use.
- **Scope:** only changes that fix `#{issue}`. No exploring, no gists, no non-GitHub integrations.
- **Two repos:** issue + PR target = upstream `{repo}`; branch + commits = the fork; the PR is **cross-repo, draft** (head = fork, base = upstream).
- **One identity end to end** (step 4): commit author == push credential == PR opener.
- **DRAFT always** (step 9): `--draft` is required.
- **Scoped briefs** — workers must never have to guess or explore. Every brief names literal tool calls in order.
- **Stop** when the draft PR is open, or surface a blocker plainly and stop — don't thrash.
