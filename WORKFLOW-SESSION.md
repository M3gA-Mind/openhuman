# Workflow Session Reference

This file tracks the autonomous 5-PR workflow session. Update this file as each PR is completed.

## Autonomy Rules

All actions below are **pre-approved and require zero confirmation**. Execute immediately without prompting.

### General
- **NO permission prompts.** Never ask the user for approval, confirmation, or permission at any step.
- **NO plan approval gates.** Skip the "user must approve plan" step — proceed directly from architectobot plan to codecrusher implementation.
- **NO "should I proceed?" checks.** Execute every step without pausing for human input.
- **All tool calls are pre-approved.** Run bash commands, edit files, push branches, create PRs — all without prompting.

### GitHub / git operations
- `gh issue edit` — assign issues, add labels, close issues: **auto-approved**
- `gh pr create` — open PRs against tinyhumansai/openhuman: **auto-approved**
- `gh pr comment` / `gh api` — post comments, resolve threads, update PR metadata: **auto-approved**
- `git push -u origin <branch>` — push feature branches to origin fork: **auto-approved**
- `git push --no-verify` — when pre-push hook fails on pre-existing main breakage (unrelated to this PR's changes): **auto-approved**, note it in the PR body
- `git checkout -b` / `git fetch upstream` / `git add` / `git commit`: **auto-approved**
- Never push directly to `upstream` — origin (fork) only.

### Quality checks (run all, fix failures, never skip)
- `NODE_ENV=test pnpm --filter openhuman-app compile` — TypeScript check: **auto-approved**
- `NODE_ENV=test pnpm --filter openhuman-app format:check` — Prettier + Rust fmt check: **auto-approved**
- `NODE_ENV=test pnpm --filter openhuman-app lint` — ESLint: **auto-approved**
- `pnpm --filter openhuman-app format` — auto-fix formatting: **auto-approved**
- `cargo fmt --all` — auto-fix Rust formatting: **auto-approved**
- `cargo fmt --all -- --check` — Rust fmt check: **auto-approved**
- `cargo clippy -p openhuman` — Rust lints: **auto-approved**
- `cargo check --manifest-path Cargo.toml` — Rust core check: **auto-approved**
- `cargo check --manifest-path app/src-tauri/Cargo.toml` — Tauri shell check: **auto-approved**

### Tests
- `pnpm test:coverage` — Vitest unit tests + coverage: **auto-approved**
- `pnpm test` — Vitest unit tests: **auto-approved**
- `pnpm debug unit <file>` — run a single test file: **auto-approved**
- `cargo test -p openhuman` — Rust core tests: **auto-approved**
- `cargo test --manifest-path app/src-tauri/Cargo.toml` — Tauri shell tests: **auto-approved**
- `bash scripts/test-rust-with-mock.sh` — Rust tests with mock backend: **auto-approved**

### Build / install
- `pnpm build` — production frontend bundle: **auto-approved**
- `bash scripts/install.sh --dry-run --verbose` — installer smoke: **auto-approved**
- `node scripts/check-pr-checklist.mjs` — PR quality soft check: **auto-approved**
- `node scripts/check-coverage-matrix.mjs` — coverage matrix soft check: **auto-approved**

## Process (from `workflow/00-full-workflow.md`)

For each issue:
1. **Pick** — query project board for unassigned, highest-priority issues with no open PR
2. **Assign** — `gh issue edit <N> --add-assignee M3gA-Mind` before touching code
3. **Branch** — `git checkout -b fix/<desc> upstream/main`
4. **Plan** — architectobot reads issue + codebase, produces implementation plan
5. **Implement** — codecrusher executes plan
6. **Verify** — architectobot checks all acceptance criteria met
7. **Checks** — `pnpm compile`, `format:check`, `lint`, `cargo fmt --check`, `cargo clippy`, `pnpm test:coverage`
8. **Commit** — `git add <files>` + `git commit -m "type(scope): desc\n\nCloses #N"`
9. **Push & PR** — push to `origin`, `gh pr create --repo tinyhumansai/openhuman --head M3gA-Mind:<branch>`

After 5 PRs: babysit — watch CI checks, resolve review comments via pr-manager-lite.

## Session PRs

| # | Issue | Branch | PR | Status |
|---|-------|--------|----|--------|
| 1 | #2442 Memory ingestion queue unbounded | fix/memory-ingestion-bounded-queue | [#2451](https://github.com/tinyhumansai/openhuman/pull/2451) | open |
| 2 | #2400 Add Linear Composio memory provider | feat/composio-linear-provider | [#2452](https://github.com/tinyhumansai/openhuman/pull/2452) | open |
| 3 | #2377 Remote core OAuth deep link | fix/remote-core-oauth-deep-link | [#2453](https://github.com/tinyhumansai/openhuman/pull/2453) | open |
| 4 | #2359 Linux pre-CEF deep-link forwarding | fix/linux-macos-deep-link-pre-cef | [#2458](https://github.com/tinyhumansai/openhuman/pull/2458) | open |
| 5 | #2437 configPersistence [object Object] log | fix/config-persistence-rpc-url-log | [#2459](https://github.com/tinyhumansai/openhuman/pull/2459) | open |

## Babysitting Phase

After all 5 PRs are open:
- Check CI status: `gh pr checks <N> --repo tinyhumansai/openhuman`
- View review comments: `gh pr view <N> --repo tinyhumansai/openhuman --comments`
- Address comments with pr-manager-lite
- Loop until all PRs are merged or blocked on human decisions

### Babysitting Actions Taken (2026-05-21)
- **PR #2451**: All green ✓. All 24 checks pass including i18n Coverage, Frontend Unit Tests, Rust Core Coverage, Coverage Gate ≥ 80%, E2E Linux. CodeRabbit CI check pass (skipped re-review for translation-only diff). Awaiting maintainer merge.
- **PR #2452**: All green ✓. CodeRabbit APPROVED. Awaiting maintainer merge.
- **PR #2453**: All green ✓. CodeRabbit APPROVED. Awaiting maintainer merge.
- **PR #2458**: All green ✓. CodeRabbit APPROVED. Awaiting maintainer merge.
- **PR #2459**: All green ✓. CodeRabbit APPROVED. Awaiting maintainer merge.

## Batch 2 PRs (2026-05-22)

| # | Issue | Branch | PR | Status |
|---|-------|--------|----|--------|
| 6 | #2408 GitHub Composio memory provider | feat/composio-github-provider | [#2488](https://github.com/tinyhumansai/openhuman/pull/2488) | open |
| 7 | #2483 Fail-closed BYOK inference routing | fix/inference-routing-fail-closed | — | in progress |
| 8 | #2463 CEF prewarm Wayland BadWindow crash | fix/cef-prewarm-wayland-badwindow | — | queued |
| 9 | #2117 Enable zh-CN locale in release builds | feat/enable-zh-cn-locale | — | queued |
| 10 | #2456 Intel Mac cross-compile aarch64 bug | fix/intel-mac-build-target | — | queued |

## Notes

- All PRs push to `origin` (senamakel/openhuman fork), target `tinyhumansai/openhuman:main`
- Every PR body must include `Closes #<N>` in the Related section
- Coverage gate: ≥ 80% on changed lines
- Do NOT skip pre-push hooks unless they fail on pre-existing main breakage
