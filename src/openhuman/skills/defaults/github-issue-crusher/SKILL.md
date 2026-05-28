# GitHub Issue Crusher

Fix the **single** GitHub issue named in the inputs, end to end, then open a **DRAFT** pull request via the **fork workflow** — issue on upstream `{repo}`, fix pushed to a fork, cross-repo draft PR back to `{repo}`. Stay strictly in scope; this is autonomous, so work until the draft PR is open or you hit a real blocker, then stop.

## The two repos
- **Upstream** = `{repo}` — where `#{issue}` lives and where the draft PR is opened (base = `{pr_base}`, or the upstream's default branch).
- **Fork** = `{fork}` if provided, otherwise the existing fork of `{repo}` under the authed GitHub account (`gh api user --jq .login`). If no fork exists, create one: `gh repo fork {repo} --remote=false --clone=false`. Call its owner `<fork-owner>`.

## Steps

1. **Read the issue.** Fetch issue `#{issue}` in `{repo}` (title, body, comments) via the GitHub integration. Identify the exact files/changes it asks for.

2. **Ensure the fork.** Obtain `<fork-owner>` from `gh api user --jq .login`. Create the fork under that account if it doesn't already exist.

3. **Clone fresh.** Clone `{repo}` to a unique local directory (e.g. `/tmp/<repo-name>-{issue}-<rand>`). If the directory already exists from a previous run, remove it first so the clone starts clean.

4. **Pin the local git identity** in the clone so commits are verified under the authed account:
   ```
   git -C <dir> config user.name  "$(gh api user --jq .login)"
   git -C <dir> config user.email "$(gh api user --jq '"\(.id)+\(.login)@users.noreply.github.com"')"
   ```
   Never `--global`; never clobber the host's global config.

5. **Locate the cause.** Start with `codegraph_search` on the issue's key symbols / error strings / literal phrases — it auto-indexes on first call (~30–90s on a fresh clone, this is normal not a hang). Inspect the result:
   - `coverage: full` → read the top hits and confirm the exact edit site.
   - `coverage: partial` → refine with `grep` scoped to the directories codegraph returned.
   - `coverage: none` or zero hits → fall back to a blind `grep` / `glob`.

6. **Apply the minimal fix.** Edit only the files identified in step 5. Re-read each file or `git diff` to confirm the change matches the intent — never trust memory.

7. **Verify.** Run the test/lint commands that apply to the changed files (e.g. `pnpm i18n:check` for i18n, `cargo test -p <crate>` for Rust, `pnpm test <pattern>` for TS). Skip if the change is pure docs / strings.

8. **Branch, commit, push to the fork** via local git:
   ```
   git -C <dir> checkout -b fix/{issue}-<short-slug>
   git -C <dir> add <only-the-changed-files>          # never git add -A
   git -C <dir> commit -m "<type>(scope): <short description> (#{issue})"
   git -C <dir> push -u "https://github.com/<fork-owner>/<repo-name>" fix/{issue}-<short-slug>
   ```

9. **Open the DRAFT cross-repo PR:**
   ```
   gh pr create -R {repo} --draft \
       --head "<fork-owner>:fix/{issue}-<short-slug>" \
       --base "{pr_base}" \
       --title "<type>(scope): <short description> (#{issue})" \
       --body "Closes #{issue}\n\n## Root cause\n<one paragraph>\n\n## Fix\n<one paragraph>\n\n## Verified\n<what you ran>"
   ```
   `--draft` is non-negotiable for autonomous runs — CI runs and a human reviews before promotion to ready.

## Rules
- **Scope:** only changes that fix `#{issue}`. No unrelated cleanup, no other issues.
- **Source of truth** is the filesystem + `git` + `codegraph` — re-read / re-search rather than relying on recall.
- **codegraph_search first** for every locate step (it auto-indexes); `grep` / `glob` are refinement or fallback only.
- **DRAFT always** — never open a PR as ready-to-merge from an autonomous run.
- **Stop** when the draft PR is open or surface a real blocker and stop — don't thrash.
