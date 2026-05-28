# PR Review Shepherd

Drive a single open GitHub PR all the way to **ready-for-merge** — CI green, every actionable reviewer/bot comment addressed, approvals in. This is autonomous Phase-6 work: iterate the **check → fix → push → re-check** loop until both gates close, or surface a real blocker and stop.

## When this skill is "done"
Both must hold:
1. **CI green** — every required check on PR `#{pr}` is `success` (or explicitly waived by a maintainer in the thread).
2. **All actionable comments resolved** — every comment from a human reviewer or bot (CodeRabbit, Codecov, etc.) is either (a) addressed by a follow-up commit AND replied to on the thread, or (b) intentionally deferred with a one-line reason replied on the thread.

Also stop if the PR is **merged** (success) or **closed without merge** (note the reason and report).

## Steps

1. **Snapshot the PR state** for `#{pr}` on `{repo}`:
   ```
   gh pr view {pr} -R {repo} \
       --json title,headRefName,headRepositoryOwner,baseRefName,isDraft,mergeable,mergeStateStatus,reviews,statusCheckRollup,url,state
   gh api repos/{repo}/pulls/{pr}/comments              # inline review comments
   gh pr view {pr} -R {repo} --comments                 # top-level comments + review summaries
   gh pr checks {pr} -R {repo}                          # CI check rollup
   ```
   Derive `<fork-owner>` from `headRepositoryOwner.login` (or use `{fork}` if provided). Note the head branch name as `<branch>`. Record: failing-check ids, unresolved comment threads (with their body + author + path/line if inline), approval count, merge state, PR state (`OPEN` / `MERGED` / `CLOSED`).

2. **Check terminal conditions first.**
   - PR `state` is `MERGED` → report `"merged: <url>"` and stop.
   - PR `state` is `CLOSED` (not merged) → report `"closed: <one-line reason from the latest comment>"` and stop.
   - All required checks `success` AND zero unresolved actionable threads AND at least one approval → report `"ready for merge: <url>"` and stop.
   - Otherwise → continue.

3. **Clone the fork branch fresh** to a unique local directory (skip this if the directory from a prior round in this same run already exists and is on the right HEAD):
   ```
   git clone --branch <branch> https://github.com/<fork-owner>/<repo-name> /tmp/<repo-name>-pr{pr}-<rand>
   ```
   Pin the local git identity in the clone so any new commits are verified under the authed account:
   ```
   git -C <dir> config user.name  "$(gh api user --jq .login)"
   git -C <dir> config user.email "$(gh api user --jq '"\(.id)+\(.login)@users.noreply.github.com"')"
   ```

4. **Address each signal in turn.** Process every open item before pushing — group changes into one push per round:

   - **CI check failed** — fetch the log: `gh run view <run-id> --log-failed -R {repo}`. Read the failure, locate the cause (start with `codegraph_search` on the failing test name or error string), apply the minimal fix, run the targeted test locally to confirm green (`cargo test -p <crate> <name>` / `pnpm test <pattern>` etc.), commit with a message that names the failing check:
     ```
     git -C <dir> add <only-the-fixed-files>
     git -C <dir> commit -m "fix(<scope>): <one line> (CI: <check-name>)"
     ```
     Do **not** bypass with `--no-verify` unless the failure is verifiably unrelated to this PR.

   - **Reviewer asks for a code change (actionable, human or bot)** — make the edit, commit referencing the comment: `git commit -m "address review: <one-line> (#{pr} review)"`. The reply on the thread happens after the push in step 6.

   - **Bot comment (CodeRabbit / Codecov / etc.)** — treat as actionable by default. If clearly a false positive, plan a thread reply (in step 6) with a one-line reason instead of a spurious code change.

   - **Reviewer requests deferral / accepts a known limitation** — plan a thread reply acknowledging, file a follow-up issue if appropriate, and persist it as "deferred" in the round summary.

5. **Push the round's fixes** to the fork in one push:
   ```
   git -C <dir> push --force-with-lease "https://github.com/<fork-owner>/<repo-name>" <branch>
   ```
   Use `--force-with-lease` (never plain `--force`) so a concurrent push from someone else aborts the push instead of clobbering. If `--force-with-lease` refuses because the remote moved, re-run step 1 (the remote diverged — handle the new commits before pushing).

6. **Reply to every addressed comment** by id so reviewers know it's been handled — even when the fix is obvious from the diff:
   - **Inline review comment** (file:line, has `id` from step 1):
     ```
     gh api -X POST repos/{repo}/pulls/{pr}/comments/<comment-id>/replies \
         --field body="Fixed in <short-sha>. <one-line description>"
     ```
   - **Top-level review or general thread**: `gh pr comment {pr} -R {repo} -b "<reply>"`.
   - **Deferred / disagreed**: reply with the one-line reason instead of a code change.

7. **Wait for CI to re-run on the new commits** before declaring the round done:
   ```
   gh pr checks {pr} -R {repo} --watch
   ```
   This blocks until all checks reach a terminal state. Do **not** spin-poll in a shell loop.

8. **Re-loop to step 1.** If `{max_rounds}` rounds (default 5) have run without both gates closing, exit with `"blocked after N rounds — surfacing for human review"` plus the still-failing checks and still-open comment ids.

## Rules
- **Scope:** only fixes for *this PR's* review feedback or CI failures. No unrelated refactors, no scope creep, no other issues.
- **`--force-with-lease`, never `--force`.** Preserve anyone else's pushes.
- **Don't bypass CI** with `--no-verify` unless the failure is verifiably unrelated to this PR AND that's been justified in the round summary.
- **Reply to every actionable signal** — addressed-and-pushed comments still need a thread reply so the reviewer knows.
- **CI green ≠ done.** Comments still matter; both gates must close.
- **Approvals don't auto-merge.** Note the approval and keep monitoring until the PR is actually merged or closed.
- **Don't push to upstream.** Pushes go to the fork only.
- **Stop** when both gates close, the PR is merged/closed, the round cap is hit, or you've identified a blocker that needs a human — report status plainly either way.
