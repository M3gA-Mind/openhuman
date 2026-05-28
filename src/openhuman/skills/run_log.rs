//! Per-run streaming logs for `skills_run`.
//!
//! Each run writes a human-readable trace to
//! `<workspace>/skills/.runs/<skill>_<UTC-ts>_<run>.log`: a header (skill,
//! inputs, task prompt), one line per agent step (tool calls + results,
//! sub-agent lifecycle, iteration boundaries) streamed live off the agent's
//! [`AgentProgress`] channel, then a footer (status, duration, final output).
//!
//! `.runs` is a sibling of the runtime skill *definitions* (`<workspace>/
//! skills/<id>/`) so run logs never collide with a skill-id directory.

use std::path::{Path, PathBuf};

use serde_json::Value;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::Receiver;

use crate::openhuman::agent::progress::AgentProgress;

/// `<workspace>/skills/.runs`.
pub fn runs_dir(workspace: &Path) -> PathBuf {
    workspace.join("skills").join(".runs")
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn short(s: &str) -> &str {
    s.get(..8).unwrap_or(s)
}

/// `<runs_dir>/<skill>_<UTC ts>_<short run id>.log`.
pub fn run_log_path(workspace: &Path, skill_id: &str, run_id: &str) -> PathBuf {
    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    runs_dir(workspace).join(format!(
        "{}_{}_{}.log",
        sanitize(skill_id),
        ts,
        sanitize(short(run_id))
    ))
}

async fn append(path: &Path, line: &str) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        tokio::fs::create_dir_all(dir).await.ok();
    }
    let mut f = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    f.write_all(line.as_bytes()).await?;
    if !line.ends_with('\n') {
        f.write_all(b"\n").await?;
    }
    f.flush().await
}

fn truncate(s: &str, n: usize) -> String {
    let s = s.replace('\n', " ");
    if s.chars().count() > n {
        format!("{}…", s.chars().take(n).collect::<String>())
    } else {
        s
    }
}

/// Write the run header (skill, inputs, the resolved task prompt).
pub async fn write_header(
    path: &Path,
    skill_id: &str,
    run_id: &str,
    inputs: &Value,
    task_prompt: &str,
) -> std::io::Result<()> {
    let header = format!(
        "==== skill_run: {skill} ====\n\
         run_id : {run}\n\
         started: {start} UTC\n\
         inputs : {inputs}\n\n\
         --- task prompt ---\n{prompt}\n\n\
         --- steps ---",
        skill = skill_id,
        run = run_id,
        start = chrono::Utc::now().to_rfc3339(),
        inputs = serde_json::to_string(inputs).unwrap_or_default(),
        prompt = task_prompt,
    );
    append(path, &header).await
}

/// One log line for a step, or `None` for events too noisy to log per-event
/// (token / argument deltas, cost ticks — the final text lands in the footer).
pub fn format_event(ev: &AgentProgress) -> Option<String> {
    let line = match ev {
        AgentProgress::TurnStarted => "turn started".to_string(),
        AgentProgress::IterationStarted {
            iteration,
            max_iterations,
        } => format!("· iteration {iteration}/{max_iterations}"),
        AgentProgress::ToolCallStarted {
            tool_name,
            arguments,
            iteration,
            ..
        } => format!(
            "[it {iteration}] tool {tool_name}({})",
            truncate(&arguments.to_string(), 200)
        ),
        AgentProgress::ToolCallCompleted {
            tool_name,
            success,
            output_chars,
            elapsed_ms,
            ..
        } => format!(
            "        ↳ {tool_name} {} ({output_chars} chars, {elapsed_ms} ms)",
            if *success { "ok" } else { "FAILED" }
        ),
        AgentProgress::SubagentSpawned {
            agent_id,
            task_id,
            prompt_chars,
            ..
        } => format!(
            "  ⮑ spawned subagent {agent_id} [{}] ({prompt_chars}-char prompt)",
            short(task_id)
        ),
        AgentProgress::SubagentToolCallStarted {
            agent_id,
            tool_name,
            ..
        } => format!("    [{agent_id}] tool {tool_name}"),
        AgentProgress::SubagentToolCallCompleted {
            agent_id,
            tool_name,
            success,
            elapsed_ms,
            ..
        } => format!(
            "    [{agent_id}] ↳ {tool_name} {} ({elapsed_ms} ms)",
            if *success { "ok" } else { "FAILED" }
        ),
        AgentProgress::SubagentCompleted {
            agent_id,
            elapsed_ms,
            iterations,
            ..
        } => format!("  ⮑ subagent {agent_id} done ({iterations} turns, {elapsed_ms} ms)"),
        AgentProgress::SubagentFailed {
            agent_id, error, ..
        } => format!("  ⮑ subagent {agent_id} FAILED: {}", truncate(error, 200)),
        AgentProgress::TurnCompleted { iterations } => {
            format!("turn completed ({iterations} iterations)")
        }
        // Noisy / non-step events — skipped (the final text is in the footer).
        AgentProgress::TextDelta { .. }
        | AgentProgress::ThinkingDelta { .. }
        | AgentProgress::ToolCallArgsDelta { .. }
        | AgentProgress::TurnCostUpdated { .. }
        | AgentProgress::TaskBoardUpdated { .. }
        | AgentProgress::SubagentIterationStarted { .. } => return None,
    };
    Some(format!(
        "{}  {}",
        chrono::Utc::now().format("%H:%M:%S%.3f"),
        line
    ))
}

/// Drain the progress channel to the log until the agent drops its sender.
pub async fn drain_to_log(mut rx: Receiver<AgentProgress>, path: PathBuf) {
    while let Some(ev) = rx.recv().await {
        if let Some(line) = format_event(&ev) {
            let _ = append(&path, &line).await;
        }
    }
}

/// Detect the degenerate "model emitted the same paragraph many times in one
/// generation" final-response failure mode we keep seeing on autonomous runs
/// (e.g. `"Now I understand the structure..." × 23`, `"Good, the repo is
/// cloned. Let me narrow down..." × 8`). When this fires we don't want the
/// autonomous-skill path to mark the run `DONE` and have callers treat the
/// degenerate text as a real result — we want it surfaced as `DEGENERATE` with
/// the offending line attached, so the caller can retry / fail loud.
///
/// Splits on line boundaries (each repeat we've observed lands on its own
/// line or paragraph), trims, counts non-trivial lines (`>= min_len` chars),
/// and returns the most-repeated line if its count reaches `min_count`.
pub fn detect_repeated_line(
    text: &str,
    min_len: usize,
    min_count: usize,
) -> Option<(String, usize)> {
    use std::collections::HashMap;
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for line in text.lines() {
        let t = line.trim();
        if t.len() >= min_len {
            *counts.entry(t).or_insert(0) += 1;
        }
    }
    counts
        .into_iter()
        .filter(|(_, c)| *c >= min_count)
        .max_by_key(|(_, c)| *c)
        .map(|(line, count)| (line.to_string(), count))
}

/// One run extracted from a `.runs/<skill>_<utc>_<run>.log` file. Built by
/// [`scan_runs`] for the `openhuman.skills_recent_runs` RPC + the Skills
/// Runner panel's "Recent runs" section. Status is `RUNNING` until the
/// footer block (`--- result ---` + `status: …` + `duration: … ms`) lands.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct ScannedRun {
    pub run_id: String,
    pub skill_id: String,
    /// Header `started:` timestamp (RFC3339); empty if header was malformed.
    pub started: String,
    /// `"DONE"` / `"DEGENERATE"` / `"FAILED"` / `"RUNNING"` (running ⇔ no footer yet).
    pub status: String,
    /// Footer `duration: <ms> ms`, parsed; `None` while running.
    pub duration_ms: Option<u64>,
    /// Footer `finished:` timestamp; `None` while running.
    pub finished: Option<String>,
    /// Absolute path to the streaming log file — what the FE shows for
    /// "view full log" or future tail-streaming.
    pub log_path: String,
}

/// Scan `<workspace>/skills/.runs/` for run-log files, parse their header +
/// footer, and return a vec sorted by `started` *descending* (most-recent
/// first). When `skill_id` is `Some(_)`, only entries whose header
/// `skill_id` matches are returned. `limit` caps the result (post-filter,
/// post-sort) so the panel can render a short list cheaply. Malformed
/// files are skipped silently — never blocks the response.
pub fn scan_runs(workspace: &Path, skill_id: Option<&str>, limit: usize) -> Vec<ScannedRun> {
    let dir = runs_dir(workspace);
    let mut runs: Vec<ScannedRun> = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return runs;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with(".log") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let mut sid = String::new();
        let mut rid = String::new();
        let mut started = String::new();
        let mut status = String::from("RUNNING");
        let mut duration_ms: Option<u64> = None;
        let mut finished: Option<String> = None;
        let mut seen_result = false;
        for line in text.lines() {
            // Header
            if let Some(rest) = line.strip_prefix("==== skill_run:") {
                sid = rest
                    .trim()
                    .trim_end_matches('=')
                    .trim()
                    .to_string();
            } else if let Some(rest) = line.strip_prefix("run_id ") {
                rid = rest.trim_start_matches(':').trim().to_string();
            } else if let Some(rest) = line.strip_prefix("started:") {
                started = rest.trim().to_string();
            }
            // Footer (only fields that appear AFTER `--- result ---`)
            if line.starts_with("--- result ---") {
                seen_result = true;
                continue;
            }
            if seen_result {
                if let Some(rest) = line.strip_prefix("status ") {
                    status = rest.trim_start_matches(':').trim().to_string();
                } else if let Some(rest) = line.strip_prefix("duration:") {
                    // Format: "<n> ms"
                    let trimmed = rest.trim();
                    let num = trimmed.trim_end_matches(" ms").trim();
                    if let Ok(n) = num.parse::<u64>() {
                        duration_ms = Some(n);
                    }
                } else if let Some(rest) = line.strip_prefix("finished:") {
                    finished = Some(rest.trim().trim_end_matches(" UTC").trim().to_string());
                }
            }
        }
        if sid.is_empty() || rid.is_empty() {
            // Malformed header — skip rather than show a half-row.
            continue;
        }
        if let Some(want) = skill_id {
            if sid != want {
                continue;
            }
        }
        runs.push(ScannedRun {
            run_id: rid,
            skill_id: sid,
            started,
            status,
            duration_ms,
            finished,
            log_path: path.to_string_lossy().into_owned(),
        });
    }
    // Sort most-recent first by `started` (RFC3339 sorts lexicographically).
    runs.sort_by(|a, b| b.started.cmp(&a.started));
    runs.truncate(limit);
    runs
}

/// Final footer: status, duration, and the agent's final output text.
pub async fn write_footer(
    path: &Path,
    status: &str,
    elapsed_ms: u64,
    output: &str,
) -> std::io::Result<()> {
    let footer = format!(
        "\n--- result ---\n\
         status  : {status}\n\
         duration: {elapsed_ms} ms\n\
         finished: {fin} UTC\n\n{output}\n",
        fin = chrono::Utc::now().to_rfc3339(),
    );
    append(path, &footer).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_repeated_line_catches_real_failure_modes() {
        // The exact text shapes we observed in run adcd2dfd (×23) and
        // dffae55d (×8). With defaults (min_len=30, min_count=4) both must
        // trip and the worst offender is returned.
        let adcd = std::iter::repeat(
            "Now I understand the structure. The keys need to go into the chunk files.",
        )
        .take(23)
        .collect::<Vec<_>>()
        .join("\n");
        let (line, n) = detect_repeated_line(&adcd, 30, 4).expect("must trip");
        assert_eq!(n, 23);
        assert!(line.contains("Now I understand the structure"));

        let dffae = std::iter::repeat("Good, the repo is cloned. Let me narrow down the search.")
            .take(8)
            .collect::<Vec<_>>()
            .join("\n");
        let (_, n2) = detect_repeated_line(&dffae, 30, 4).expect("must trip");
        assert_eq!(n2, 8);
    }

    #[test]
    fn scan_runs_parses_header_footer_and_status() {
        // Mirror the on-disk layout: <workspace>/skills/.runs/<file>.log
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let runs = runs_dir(tmp.path());
        std::fs::create_dir_all(&runs).unwrap();

        // (a) finished run — full footer
        let done = "==== skill_run: github-issue-crusher ====\n\
                    run_id : aaaaaaaa-1111-2222-3333-444444444444\n\
                    started: 2026-05-28T07:51:13.604134255+00:00 UTC\n\
                    inputs : {}\n\n\
                    --- task prompt ---\nfoo\n\
                    --- steps ---\nstep 1\n\
                    --- result ---\n\
                    status  : DONE\n\
                    duration: 617236 ms\n\
                    finished: 2026-05-28T08:01:30.944918997+00:00 UTC\n\n\
                    body...\n";
        std::fs::write(runs.join("github-issue-crusher_20260528T075113Z_aaaaaaaa.log"), done)
            .unwrap();

        // (b) still-running — no footer yet
        let running = "==== skill_run: pr-review-shepherd ====\n\
                       run_id : bbbbbbbb-1111-2222-3333-444444444444\n\
                       started: 2026-05-28T09:00:00.000000000+00:00 UTC\n\
                       inputs : {}\n\n\
                       --- task prompt ---\nfoo\n\
                       --- steps ---\nstep 1\n";
        std::fs::write(runs.join("pr-review-shepherd_20260528T090000Z_bbbbbbbb.log"), running)
            .unwrap();

        let all = scan_runs(tmp.path(), None, 10);
        assert_eq!(all.len(), 2, "both runs visible");
        // Newest first — (b) started later than (a).
        assert_eq!(all[0].run_id, "bbbbbbbb-1111-2222-3333-444444444444");
        assert_eq!(all[0].status, "RUNNING");
        assert_eq!(all[0].duration_ms, None);
        assert_eq!(all[1].status, "DONE");
        assert_eq!(all[1].duration_ms, Some(617236));
        assert!(all[1].finished.as_deref().unwrap().starts_with("2026-05-28T08:01:30"));

        // Filter by skill_id
        let only_pr = scan_runs(tmp.path(), Some("pr-review-shepherd"), 10);
        assert_eq!(only_pr.len(), 1);
        assert_eq!(only_pr[0].skill_id, "pr-review-shepherd");

        // Limit caps the result post-sort
        let one = scan_runs(tmp.path(), None, 1);
        assert_eq!(one.len(), 1);
        assert_eq!(one[0].run_id, "bbbbbbbb-1111-2222-3333-444444444444");
    }

    #[test]
    fn scan_runs_skips_malformed_files() {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let runs = runs_dir(tmp.path());
        std::fs::create_dir_all(&runs).unwrap();
        // Empty header — no `==== skill_run: ` line ⇒ skip silently.
        std::fs::write(runs.join("garbage_x_y.log"), "hi i'm not a run log\n").unwrap();
        let scanned = scan_runs(tmp.path(), None, 10);
        assert!(scanned.is_empty(), "malformed files must be skipped");
    }

    #[test]
    fn detect_repeated_line_does_not_false_positive_on_legitimate_output() {
        // Normal prose with each sentence on its own line and no repeats
        // should not trip. Also short lines (`OK`, `Done`) under min_len
        // must be ignored even when repeated, so a verbose log of "OK"
        // markers doesn't look like degeneracy.
        let prose = "First, I read the issue and identified the failing test.\n\
                     Then I edited src/foo.rs to add a None-guard around the dereference.\n\
                     Finally I ran cargo test -p foo and confirmed the fix.\n\
                     OK\nOK\nOK\nOK\nOK\nOK\nOK\nOK";
        assert!(detect_repeated_line(prose, 30, 4).is_none());
    }

    #[test]
    fn log_path_is_under_runs_and_sanitised() {
        let p = run_log_path(Path::new("/ws"), "github/issue crusher", "abcdef12-3456");
        let s = p.to_string_lossy();
        assert!(s.contains("/ws/skills/.runs/"));
        assert!(s.contains("github-issue-crusher_"));
        assert!(s.ends_with("_abcdef12.log"), "got {s}");
    }

    #[test]
    fn noisy_events_are_skipped_steps_are_kept() {
        assert!(format_event(&AgentProgress::TextDelta {
            delta: "hi".into(),
            iteration: 1
        })
        .is_none());
        let line = format_event(&AgentProgress::ToolCallStarted {
            call_id: "c1".into(),
            tool_name: "codegraph_search".into(),
            arguments: serde_json::json!({"query": "x"}),
            iteration: 2,
        })
        .expect("tool call logged");
        assert!(line.contains("codegraph_search"));
        assert!(line.contains("it 2"));
    }
}
