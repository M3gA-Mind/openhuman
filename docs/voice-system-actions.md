# Voice → System Action Feature Tracker

**GitHub Issue:** [#3148](https://github.com/tinyhumansai/openhuman/issues/3148)  
**Branch:** `feat/voice-always-on`  
**PR:** [#3168](https://github.com/tinyhumansai/openhuman/pull/3168)  
**Started:** 2026-06-02  

---

## Goal

Enable the app to continuously listen to the user, understand spoken commands, and perform system actions on the laptop — e.g., saying *"open my Music player"* causes the app to open it, without any hotkey press or manual send.

---

## Companion Feature (Separate PR)

**Notch Live Activity Indicator** — [PR #3166](https://github.com/tinyhumansai/openhuman/pull/3166)  
A transparent NSPanel pill at the top of the primary screen (the macOS notch area) that shows live voice/agent status. Built alongside this feature; will light up automatically once always-on listening is implemented.

---

## Phase 1 — Quick Wins ✅ Complete

> Low-effort changes that make the existing hotkey-triggered dictation flow work end-to-end without manual sends or approval prompts.

---

### Change 1.1 — Auto-send after transcription

**Status:** ✅ Done  
**Commit:** `7269f4373`

**Problem:** After speaking via the dictation hotkey, the transcript appeared in the chat composer but the user had to press Enter manually to send it.

**Fix:**
- `app/src/hooks/useDictationHotkey.ts` — added `autoSend: true` to the `dictation://insert-text` event detail
- `app/src/pages/Conversations.tsx` — `onDictationInsert` now checks the flag; when set, calls `handleSendMessage(text)` directly instead of inserting into the textarea. Added `handleSendMessageRef` (updated every render) so the mount-time effect can access the latest send function

**Result:** Press hotkey → speak → message auto-sends to agent. No Enter key needed.

---

### Change 1.2 — Shell allowlist for app-launching

**Status:** ✅ Done  
**Commit:** `7269f4373`

**Problem:** `open -a Music` classified as `Write` → triggers approval prompt in Supervised mode.

**Fix:**
- `src/openhuman/security/policy_command.rs` — added `"open"` (macOS) and `"xdg-open"` (Linux) to `READ_ONLY_BASES`. These are OS-native app launchers that don't modify the workspace, so `Read` classification is correct.

**Result:** Agent can run `open -a Music` in Supervised mode without approval prompt.

---

### Change 1.3 — Shell tool description fix

**Status:** ✅ Done  
**Commit:** `ec8f5be2e`

**Problem:** Shell tool description said *"Execute a shell command in the workspace directory"* — the LLM was reasoning that it could only run workspace commands, not launch apps.

**Fix:**
- `src/openhuman/tools/impl/system/shell.rs` — updated description to explicitly mention system actions and app launching examples

**Result:** Agent now understands the shell tool can perform system actions, not just workspace file operations.

---

### Change 1.4 — Dedicated `launch_app` tool

**Status:** ✅ Done  
**Commit:** `802fbca76`

**Problem:** Using the `shell` tool for app launching requires loosening `workspace_only` and expanding `allowed_commands` — a security regression. The `shell` tool also couldn't be used because the orchestrator's strict `named` tool list excluded it.

**Fix (production approach):**
- `src/openhuman/tools/impl/system/launch_app.rs` — **new tool** with `PermissionLevel::ReadOnly` (never triggers approval gate)
  - macOS: `open -a "<app_name>"` via `tokio::process::Command`
  - Linux: `gtk-launch`, fallback `xdg-open`  
  - Windows: `Start-Process` via PowerShell
  - Input validation: rejects paths, metacharacters, empty names
  - Unit tests: name, permission, schema, validation, error cases
- `src/openhuman/tools/impl/system/mod.rs` — registered module + pub use
- `src/openhuman/tools/ops.rs` — added `LaunchAppTool` to `all_tools_with_runtime`
- `src/openhuman/tools/user_filter.rs` — added `"launch_app"` family, `default_enabled = true`
- `app/src/utils/toolDefinitions.ts` — added to frontend tool catalog (Settings → Agent Access toggle)

**Result:** Agent has a purpose-built, always-allow tool for launching apps. No shell exposure, no path security concerns.

---

### Change 1.5 — Orchestrator agent tool scope

**Status:** ✅ Done  
**Commit:** `7d04fc4bc`

**Problem:** Even though `launch_app` was registered, it was invisible to the agent. The orchestrator (`src/openhuman/agent_registry/agents/orchestrator/agent.toml`) has a strict `named = [...]` allowlist. `launch_app` was not in it, so it was filtered out. Confirmed via logs: `visible=24, names=[...no launch_app...]`.

**Fix:**
- `src/openhuman/agent_registry/agents/orchestrator/agent.toml` — added `"launch_app"` to the `[tools] named` list, alongside `"current_time"` (same pattern: direct answer without delegation)

**Confirmed working via logs:**
```
visible=25, names=[..., launch_app, ...]
[launch_app] ▶ execute called  app_name="Music"
[launch_app] macOS: running `open -a "Music"`
[launch_app] macOS: `open -a` exit=exit status: 0  stderr=
[launch_app] ✓ launch succeeded  msg="Opened 'Music'."
```

**Result:** Saying "open my Music app" now opens Music directly. No approval prompt, no delegation, no refusal.

---

### Change 1.6 — SOUL.md capability hint

**Status:** ✅ Done  
**Commit:** `cdd3bb4a4`

**Problem:** Even with the tool available, the agent was refusing ("I can't open apps on your device") because its training overrides the function-calling schema.

**Fix:**
- `src/openhuman/agent/prompts/SOUL.md` — added explicit *"What you can do on the user's machine"* section listing `launch_app`, `shell`, `file_read`/`file_write` with the instruction: *"Never say 'I can't open apps' when you have a tool to do it. Use the tool."*

**Result:** Agent now knows it has these capabilities and is instructed to use them.

---

### Change 1.7 — Diagnostic logging

**Status:** ✅ Done  
**Commit:** `cdd3bb4a4`

**Added logging to:**
- `src/openhuman/tools/impl/system/launch_app.rs` — logs every step: `▶ execute`, validation pass/fail, platform dispatch, `open -a` exit code + stderr, fallback result
- `src/openhuman/agent/harness/session/builder.rs` — logs the **full list** of visible tool names at session build time (previously only logged count)

**Result:** Can now confirm at a glance whether `launch_app` is in the tool list and trace every step of its execution.

---

---

### Change 1.8 — Computer control (mouse + keyboard) — ⚠️ REVERTED

**Status:** ❌ Reverted (commits `50ca434b7` add, `bi0rd96sa` revert)

**Problem:** Agent could open apps but couldn't interact with their UI.

**What was tried:** Enabled the existing `mouse` + `keyboard` tools (enigo / `CGEventPost`), wired into the orchestrator, user filter, and frontend catalog.

**Why reverted:** `CGEventPost` injects synthetic events to the **currently focused window**. When the focused window was OpenHuman's own CEF renderer (the chat UI), a Space keypress crashed the app — `EXC_BREAKPOINT / SIGTRAP` in `CFRelease → NSKeyValueWillChangeWithPerThreadPendingNotifications → -[NSApplication stop:]`. CEF can't handle arbitrary key injection. Confirmed via crash report `OpenHuman-2026-06-02-035139.ips`.

**Replaced by:** Change 1.9 (`ax_interact`) — AXUIElement targets elements directly by label with no synthetic events and no CEF crash risk.

---

### Change 1.9 — AXUIElement app UI interaction (`ax_interact`)

**Status:** ✅ Done  
**Commits:** `4f9ca1cad` (feature), `2c32b59c9` (exact-match fix), `betuerj11`/test commits

**Problem:** Need to interact with desktop app UIs reliably, without the CEF crash from synthetic events.

**Fix — uses the macOS Accessibility API (AXUIElement) instead of CGEventPost:**
- `src/openhuman/accessibility/helper.rs` — extended the unified Swift helper with three commands:
  - `ax_list` → walk the AX tree, return interactive elements (buttons, fields, cells)
  - `ax_press` → `AXUIElementPerformAction(kAXPressAction)` by label, **exact match preferred over contains** (so "Play" beats "Playlist")
  - `ax_set_value` → `AXUIElementSetAttributeValue(kAXValueAttribute)` by label
- `src/openhuman/accessibility/ax_interact.rs` (new) — Rust wrappers `ax_list_elements`, `ax_press_element`, `ax_set_field_value`
- `src/openhuman/tools/impl/computer/ax_interact.rs` (new) — `AxInteractTool` with actions `list` / `press` / `set_value`, `PermissionLevel::ReadOnly`
- `src/openhuman/accessibility/ax_interact_tests.rs` (new) — integration tests (open Music → search AC/DC → find row → press)
- Wired into `tools/ops.rs`, `tools/user_filter.rs`, `toolDefinitions.ts` (App UI Control), `orchestrator/agent.toml`, `SOUL.md`

**Why it's better than mouse/keyboard:**

| | mouse/keyboard (reverted) | ax_interact |
|---|---|---|
| Mechanism | `CGEventPost` synthetic events | `AXUIElementPerformAction` direct API |
| CEF crash risk | Yes | None |
| Coordinates | Required (needs screenshot) | None — finds by label |
| Works when app unfocused | No | Yes |

**Verified working:** Direct AX test against Music listed 256 elements including `Bollywood Hits`, `Play`, etc.; pressing `Bollywood Hits` then `Play` both returned `exact=true` and acted correctly.

---

### Change 1.10 — Multi-step UI workflow guidance

**Status:** ✅ Done

**Problem:** When asked to "play Highway to Hell by AC/DC", the agent ran: launch → list → press Library → press Songs → press "Show Filter Field" → set_value "Highway to Hell" → **press "Play"**. The final press hit the **global playback bar Play button** (plays last queue item), not the specific song row. Result: app navigated correctly but the wrong/no track played.

**Fix:**
- `src/openhuman/agent/prompts/SOUL.md` — added explicit multi-step workflow:
  1. `list` → discover elements
  2. `set_value` → type in filter/search
  3. `list` **again** → see filtered results
  4. `press` the **specific item** (song row), not the generic Play button
- Added Apple Music guidance: use `shell` to open `music://music.apple.com/search?term=...`, then `ax_interact list` to see song rows as AXCells, then press the specific row. More reliable than the Library filter field.

**Result:** Agent is directed to select the specific item before pressing playback, instead of pressing the global Play button after filtering.

---

### Change 1.11 — Apple Music two-step play (navigate then play)

**Status:** ✅ Done

**Problem:** When asked to "play Highway to Hell by AC/DC", the agent navigated to the right screen but **nothing played**. Pressing a search-result row in Apple Music only *selects/navigates* — it does not start playback. The agent then pressed the global transport Play button, but nothing was queued.

**Investigation (empirical AX probing against live Music):**
- Every "Highway to Hell" element (AXCell, AXGroup, AXButton) exposes only the `AXPress` action — which selects/navigates, never plays.
- Double `AXPress`, a real CGEvent double-click on the Top-Results card, and AX-select + Return key **all left player state `stopped`**.
- **Working sequence found:** AXPress the search-result card to **navigate into the song's detail page**, then AXPress the **Play button on that detail page** → `player state: playing` ✅

**Fix:**
- `src/openhuman/agent/prompts/SOUL.md` — replaced the Apple Music guidance with the exact 5-step sequence: URL-scheme search → list → press song row (navigates in) → list detail page → press detail-page Play. Explicitly warns that pressing a search result only navigates, and the second Play press is mandatory.
- `src/openhuman/accessibility/ax_interact_tests.rs` — `test_full_flow_search_and_play_acdc` now asserts real playback via `osascript ... get player state` == "playing" (not just element presence). **Passes.**

**Verified:**
```
[step 4] navigate into song: Ok("Pressed 'Highway to Hell' in 'Music'.")
[step 5] press detail Play: Ok("Pressed 'Play' in 'Music'.")
[step 6] player state: playing
test ... ok
```

---

### Change 1.12 — One-shot `play_music` tool (root-cause fix)

**Status:** ✅ Done

**Problem:** Even after change 1.11, the agent still used the broken filter-field approach and didn't play. Transcript analysis (`~/.openhuman/users/<id>/workspace/session_raw/*.jsonl`) revealed two real root causes:

1. **The orchestrator has no `shell` tool.** Change 1.11 put the play guidance in `SOUL.md` — but the orchestrator runs with `omit_identity = true` and **never sees SOUL.md**. Change 1.11b moved it to the `ax_interact` description, which told the agent to "use the shell tool to open `music://...`" — but the orchestrator can't run shell (it delegates). The agent wrapped the command in a `prompt` arg to a delegation tool; it never executed, and it fell back to the filter approach.
2. **Cross-chat memory contamination.** The user message was prefixed with `[Cross-chat context — historical]` containing prior filter-approach "Progress Checkpoint" steps, biasing the agent back to the wrong method.

**Fix — stop relying on the LLM to orchestrate a fragile multi-step flow with a tool it lacks. Encapsulate the whole proven sequence in native Rust:**
- `src/openhuman/accessibility/ax_interact.rs` — `play_apple_music(query)`: open search URL → AX-find + press song cell (navigate) → press detail-page Play → verify `player state == playing`
- `src/openhuman/tools/impl/computer/play_music.rs` (new) — `PlayMusicTool`, single call `play_music{query}`, `PermissionLevel::ReadOnly`, runs the blocking flow via `spawn_blocking`
- Registered in `ops.rs`, `user_filter.rs`, `orchestrator/agent.toml`, `toolDefinitions.ts`

**Result:** Agent calls `play_music{query:'Highway to Hell AC/DC'}` **once**; Rust does search→navigate→play→verify deterministically. No shell dependency, no multi-step LLM orchestration, no filter-field fallback. Unit tests pass; the underlying flow is verified by `test_full_flow_search_and_play_acdc`.

**Key learning:** The orchestrator (chat agent) only reads **tool descriptions + agent.toml** — NOT SOUL.md (omit_identity=true). Behavior guidance for the chat agent must live in tool descriptions or be encapsulated in the tool itself.

---

### Change 1.13 — Generic any-app tool + filtered list (remove play_music)

**Status:** ✅ Done

**Problem:** "Play Numb by Linkin Park" still failed, and the agent **hallucinated**. Transcript (`session_raw/*.jsonl`) showed:
1. `play_music` hit a 4s timing race — results hadn't rendered, so it returned "No matching song found. Top result cells: [empty]".
2. The agent fell back to `ax_interact list`, which dumped **273 elements**. The tool result was **truncated mid-list**, so the model reasoned over a partial view and hallucinated a wrong result ("Numb - Single by Marshmello").

**Feedback:** A music-specific tool is the wrong abstraction. Build a generic tool that interacts with **any** app.

**Fix:**
- **Removed** `play_music` tool + `play_apple_music` helper and all registrations.
- **`ax_interact` is now a robust generic any-app tool:**
  - `ax_list_elements_filtered(app, filter)` — Rust-side label filter so `list` returns only relevant elements (fixes the truncation→hallucination root cause).
  - `list` action takes a new `filter` param; output capped at 60 elements with a "narrow your filter" hint; empty-match returns a "UI may still be loading" hint instead of failing hard.
  - Description rewritten to be app-agnostic and document the general **navigate-then-activate** pattern (pressing a list row/search result selects/opens it; press the action button afterward) — no hardcoded Apple Music steps.

**Key learning:** Dumping a full AX tree (hundreds of elements) overflows the tool-result budget; the truncated view makes the model hallucinate. Always filter list results to keep them small and accurate.

---

## Phase 2 — Always-On Listening ⏳ Not Started

> Continuous microphone listening without requiring a hotkey press.

**Planned files:**
- `src/openhuman/voice/always_on.rs` (new) — dedicated tokio task holding the mic open, running VAD, emitting utterances to the STT pipeline
- `src/openhuman/config/schema/voice_server.rs` — add `always_on_enabled: bool` config flag
- Privacy hook: pause always-on when screen is locked

**Acceptance criteria:**
- [ ] User can speak without pressing any hotkey
- [ ] VAD detects end of utterance and sends to agent
- [ ] Toggle in Settings → Voice

---

## Phase 3 — Wake-Word + Fast Routing ⏳ Not Started

> Activate only on a trigger phrase; route simple commands locally without a full LLM turn.

**Planned files:**
- `src/openhuman/inference/voice/wake_word.rs` (new) — lightweight always-on model (Porcupine or custom ONNX)
- `src/openhuman/voice/command_router.rs` (new) — intent→tool mapping for high-confidence commands, LLM fallback for ambiguous input

**Acceptance criteria:**
- [ ] Wake-word detection runs fully on-device
- [ ] Latency from end-of-utterance to action start ≤ 500ms for local-routed commands

---

## Phase 4 — Polish ⏳ Not Started

> Voice confirmation loop, UI indicator, computer control onboarding.

**Planned:**
- TTS confirmation before executing sensitive actions ("Opening Music — confirm?")
- Always-on status indicator (notch pill from PR #3166 will handle this automatically)
- Computer control (`mouse`/`keyboard` tools) toggle in Settings onboarding

---

## Summary

| Phase | Item | Status |
|---|---|---|
| 1 | Auto-send after transcription | ✅ Done |
| 1 | Shell allowlist for `open`/`xdg-open` | ✅ Done |
| 1 | Shell tool description clarification | ✅ Done |
| 1 | Dedicated `launch_app` tool | ✅ Done |
| 1 | Orchestrator tool scope | ✅ Done |
| 1 | SOUL.md capability hint | ✅ Done |
| 1 | Diagnostic logging | ✅ Done |
| 1 | Computer control (mouse/keyboard) | ❌ Reverted (CEF crash) |
| 1 | AXUIElement app UI interaction (`ax_interact`) | ✅ Done |
| 1 | Multi-step UI workflow guidance | ✅ Done |
| 1 | Apple Music two-step play (navigate→play) | ✅ Done (playback verified) |
| 2 | Always-on microphone loop | ⏳ Not started |
| 2 | `always_on_enabled` config flag | ⏳ Not started |
| 2 | Privacy hook (screen lock pause) | ⏳ Not started |
| 3 | Wake-word detection | ⏳ Not started |
| 3 | Local command router | ⏳ Not started |
| 4 | Voice confirmation loop | ⏳ Not started |
| 4 | Always-on UI indicator | ✅ Done (notch PR #3166) |
