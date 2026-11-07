//! Autonomous agent loop P1: goal -> explore -> act -> verify.
//!
//! Every step is persisted to eternal session memory, so "what did we do
//! last Tuesday?" is a `SessionRead`, not a reconstruction. The brain is
//! `craft-ai-core` (Ollama local by default, cloud when configured); when
//! the brain is unreachable the deterministic explorer still runs, and the
//! loop records that honestly instead of hallucinating tool calls.
//!
//! Brain protocol (kept deliberately tiny so small local models can follow):
//! respond with either `FINAL: <summary>` or one fenced JSON tool call:
//! ```json
//! {"tool": "glob|grep|read|index|shell|patch|plan", "args": {...}}
//! ```
//!
//! P1 upgrades: brain timeout (spawn_blocking + tokio timeout, so a slow
//! model can never wedge the loop), retry-on-garbage (max 2 reprompts,
//! budget-counted), verify-after-patch (auto test run), per-mode budgets
//! (plan defaults 5 read-only steps, build defaults 8).
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use codercraft_agent::{AgentMode, AgentSession};
use craft_ai_core::CraftAiClient;
use databasecraft_store::DatabaseStore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{permissions, patch, sessions, CoderCraftCore};

const MAX_STEPS_HARD: usize = 25;
const MAX_GARBAGE_REPROMPTS: u8 = 2;
const TOOL_RESULT_CHARS: usize = 4000;
const HISTORY_CHARS: usize = 6000;
const DEFAULT_TIMEOUT_S: u64 = 120;
const PLAN_BUDGET: usize = 5;
const BUILD_BUDGET: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StepRecord {
    step: usize,
    kind: String,
    target: String,
    decision: String,
    success: bool,
    summary: String,
}

/// Read-only in plan mode: the planner maps, the builder cuts.
#[must_use]
pub fn tool_allowed_in_mode(mode: &str, tool: &str) -> bool {
    if mode == "plan" {
        matches!(tool, "glob" | "grep" | "read" | "index" | "plan")
    } else {
        matches!(
            tool,
            "glob" | "grep" | "read" | "index" | "shell" | "patch" | "plan"
        )
    }
}

/// Default step budget per agent mode.
#[must_use]
pub fn default_budget_for_mode(mode: &str) -> usize {
    if mode == "plan" {
        PLAN_BUDGET
    } else {
        BUILD_BUDGET
    }
}

/// Detect the workspace test command from project markers.
/// Returns None when no known runner is present (then verify is skipped,
/// honestly, instead of guessing).
#[must_use]
pub fn detect_test_command(workspace: &str) -> Option<String> {
    let root = Path::new(workspace);
    if root.join("Cargo.toml").is_file() {
        Some("cargo test".to_string())
    } else if root.join("pytest.ini").is_file()
        || root.join("pyproject.toml").is_file()
        || root.join("setup.cfg").is_file()
        || root.join("tox.ini").is_file()
    {
        Some("python -m pytest -q".to_string())
    } else {
        None
    }
}

fn trunc(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        let mut end = n;
        while !s.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("…[truncated {} chars]", s.len() - end)
    }
}

fn resolve_in(workspace: &str, p: &str) -> String {
    let t = p.trim();
    if t.is_empty() {
        return workspace.to_string();
    }
    if Path::new(t).is_absolute() {
        return t.to_string();
    }
    Path::new(workspace).join(t).to_string_lossy().to_string()
}

/// Split the goal into searchable keywords (alphanumeric tokens, len >= 4).
fn keywords(goal: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for tok in goal.split(|c: char| !c.is_alphanumeric()) {
        let t = tok.to_lowercase();
        if t.len() >= 4 && !out.contains(&t) {
            out.push(t);
        }
        if out.len() >= 5 {
            break;
        }
    }
    out
}

fn system_prompt(workspace: &str, mode: &str) -> String {
    let readonly = if mode == "plan" {
        " You are in PLAN mode: read-only. Never emit shell or patch calls; map the territory and finish with FINAL."
    } else {
        ""
    };
    format!(
        "You are CoderCraft Builder, an autonomous coding agent. Workspace: {workspace}.{readonly}\n\
        You act through ONE tool call per reply. Reply with exactly one of:\n\
        - FINAL: <short summary of what was done and verification status>\n\
        - a single fenced json block {{\"tool\": ..., \"args\": ...}} and nothing else.\n\
        Tools:\n\
        - glob {{pattern}}: list files, e.g. {{\"tool\":\"glob\",\"args\":{{\"pattern\":\"**/*.py\"}}}}\n\
        - grep {{query}}: search file contents, e.g. {{\"tool\":\"grep\",\"args\":{{\"query\":\"health\"}}}}\n\
        - read {{path[, start_line, end_line]}}: read a file (paths may be workspace-relative)\n\
        - index {{}}: rescan workspace tree + instruction files\n\
        - shell {{command_line}}: run tests/linters only (destructive commands are refused)\n\
        - patch {{patch_text}}: multi-file patch using *** Add File:/*** Update File:/*** Delete File:/*** Move to: markers with full file content\n\
        - plan {{goal}}: produce an implementation plan\n\
        Rules: explore (glob/grep/read) before patching. Prefer small patches. \
        After patching, tests run automatically. Emit FINAL when done or blocked."
    )
}

/// Extract the first balanced `{...}` object containing a "tool" key.
#[must_use]
pub fn extract_tool_json(text: &str) -> Option<Value> {
    let bytes: Vec<char> = text.chars().collect();
    let mut starts: Vec<usize> = Vec::new();
    for (i, c) in bytes.iter().enumerate() {
        if *c == '{' {
            starts.push(i);
        }
    }
    for s in starts {
        let mut depth = 0;
        let mut in_str = false;
        let mut esc = false;
        let mut end: Option<usize> = None;
        let mut i = s;
        while i < bytes.len() {
            let c = bytes[i];
            if in_str {
                if esc {
                    esc = false;
                } else if c == '\\' {
                    esc = true;
                } else if c == '"' {
                    in_str = false;
                }
            } else if c == '"' {
                in_str = true;
            } else if c == '{' {
                depth += 1;
            } else if c == '}' {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            i += 1;
        }
        if let Some(e) = end {
            let candidate: String = bytes[s..=e].iter().collect();
            if candidate.contains("\"tool\"") {
                if let Ok(v) = serde_json::from_str::<Value>(&candidate) {
                    if v.get("tool").and_then(|t| t.as_str()).is_some() {
                        return Some(v);
                    }
                }
            }
        }
    }
    None
}

/// Brain call with a hard timeout. `craft-ai-core` shells out to curl with
/// no timeout of its own, so we isolate it on the blocking pool and stop
/// waiting after `timeout_s` — the stray curl finishes on its own and its
/// result is dropped. Returns None on timeout or transport error.
async fn brain_complete(
    client: CraftAiClient,
    system: String,
    user: String,
    timeout_s: u64,
) -> Option<String> {
    let handle = tokio::runtime::Handle::try_current().ok()?;
    let join = tokio::task::spawn_blocking(move || {
        handle.block_on(client.complete(&system, &user))
    });
    match tokio::time::timeout(Duration::from_secs(timeout_s.max(5)), join).await {
        Ok(Ok(Ok(reply))) => Some(reply),
        _ => None,
    }
}

/// Blocking closure with a hard timeout on the blocking pool.
async fn block_with_timeout<T>(timeout_s: u64, f: impl FnOnce() -> T + Send + 'static) -> Option<T>
where
    T: Send + 'static,
{
    let join = tokio::task::spawn_blocking(f);
    match tokio::time::timeout(Duration::from_secs(timeout_s.max(5)), join).await {
        Ok(Ok(v)) => Some(v),
        _ => None,
    }
}

#[allow(clippy::too_many_lines)]
fn execute_tool(workspace: &str, tool: &str, args: &Value) -> (bool, String) {
    let arg_str = |k: &str| {
        args.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    match tool {
        "glob" => {
            let pattern = arg_str("pattern");
            let pattern = if pattern.is_empty() {
                "**/*.rs,**/*.py,**/*.ts,**/*.tsx,**/*.go".to_string()
            } else {
                pattern
            };
            let files = patch::glob_files(workspace, &pattern);
            let shown: Vec<&str> = files.iter().take(20).map(String::as_str).collect();
            (
                true,
                format!("{} files; first: {}", files.len(), shown.join(", ")),
            )
        }
        "grep" => {
            let query = arg_str("query");
            if query.is_empty() {
                return (false, "grep needs a query".to_string());
            }
            let hits = CoderCraftCore::grep_search(workspace, &query);
            let shown: Vec<String> = hits
                .iter()
                .take(10)
                .map(|(p, line, text)| format!("{p}:{line}: {}", trunc(text.trim(), 160)))
                .collect();
            (
                true,
                format!("{} matches; {}", hits.len(), shown.join(" | ")),
            )
        }
        "read" => {
            let path = arg_str("path");
            if path.is_empty() {
                return (false, "read needs a path".to_string());
            }
            let full = resolve_in(workspace, &path);
            let start = args.get("start_line").and_then(serde_json::Value::as_u64).map(|n| n as usize);
            let end = args.get("end_line").and_then(serde_json::Value::as_u64).map(|n| n as usize);
            match CoderCraftCore::read_file_range(&full, start, end) {
                Ok(content) => (true, format!("{full}:\n{}", trunc(&content, TOOL_RESULT_CHARS))),
                Err(e) => (false, e),
            }
        }
        "index" => {
            let tree = CoderCraftCore::scan_tree(Path::new(workspace), 0, 3);
            let inst = CoderCraftCore::discover_instructions(workspace);
            (
                true,
                format!(
                    "indexed: {} top-level entries; agents_md={} claude_md={} cursorrules={}",
                    tree.children.len(),
                    inst.agents_md.is_some(),
                    inst.claude_md.is_some(),
                    inst.cursor_rules.is_some()
                ),
            )
        }
        "shell" => {
            let cmd_line = arg_str("command_line");
            if cmd_line.is_empty() {
                return (false, "shell needs command_line".to_string());
            }
            let (code, stdout, stderr) =
                CoderCraftCore::execute_shell(&cmd_line, Some(workspace));
            (
                code == 0,
                format!(
                    "exit={code} stdout={} stderr={}",
                    trunc(stdout.trim(), 2000),
                    trunc(stderr.trim(), 2000)
                ),
            )
        }
        "patch" => {
            let patch_text = arg_str("patch_text");
            if patch_text.is_empty() {
                return (false, "patch needs patch_text".to_string());
            }
            match patch::apply_patch_text(workspace, &patch_text) {
                Ok(files) => {
                    let list: Vec<String> =
                        files.iter().map(|f| format!("{} {}", f.action, f.path)).collect();
                    (true, format!("applied: {}", list.join(", ")))
                }
                Err(e) => (false, e),
            }
        }
        "plan" => {
            let goal = arg_str("goal");
            let mut sess = AgentSession::new("plan-once", AgentMode::Plan, workspace);
            let plan = sess.generate_step_plan(if goal.is_empty() { workspace } else { &goal });
            (true, trunc(&plan, TOOL_RESULT_CHARS))
        }
        other => (false, format!("unknown tool '{other}'")),
    }
}

fn permission_class(tool: &str) -> &'static str {
    match tool {
        "glob" | "grep" | "read" | "index" | "plan" => "read",
        "shell" => "shell",
        "patch" => "patch",
        _ => "unknown",
    }
}

fn append_step_turn(
    store: &DatabaseStore,
    session_id: &str,
    step: &StepRecord,
    tool_result: &str,
) {
    let content = format!(
        "step {} [{}] {} success={} decision={}\n{}",
        step.step,
        step.kind,
        step.target,
        step.success,
        step.decision,
        trunc(tool_result, 1500)
    );
    let _ = sessions::append_turn(store, session_id, "assistant", &content, "{}");
}

/// Run the autonomous loop. Returns a JSON Value (never panics, never errors
/// at the transport level: failures are reported inside the payload).
/// All inputs for one autonomous run. `brain: None` loads the default client
/// (`~/.craft/ai_config.json`); tests inject a client pointed at a dead
/// endpoint for a fast deterministic fallback path.
pub struct RunRequest {
    pub workspace_dir: String,
    pub goal: String,
    pub session_id: Option<String>,
    pub max_steps: Option<usize>,
    pub mode: String,
    pub timeout_s: u64,
    pub brain: Option<CraftAiClient>,
}

pub async fn run_goal(store: &DatabaseStore, req: RunRequest) -> Value {
    let RunRequest {
        workspace_dir,
        goal,
        session_id,
        max_steps,
        mode,
        timeout_s,
        brain,
    } = req;
    let workspace_dir = workspace_dir.as_str();
    let goal = goal.as_str();
    let session_id = session_id.as_deref();
    if goal.trim().is_empty() {
        return json!({ "success": false, "error": "empty goal" });
    }
    if let Err(e) = sessions::ensure_schema(store) {
        return json!({ "success": false, "error": format!("sessions schema: {e}") });
    }
    let mode = if mode.trim().is_empty() {
        "build".to_string()
    } else {
        mode.trim().to_lowercase()
    };
    let mode = if mode == "plan" { "plan".to_string() } else { "build".to_string() };
    let budget = max_steps
        .unwrap_or_else(|| default_budget_for_mode(&mode))
        .clamp(1, MAX_STEPS_HARD);
    let timeout_s = timeout_s.clamp(10, 600);

    let title: String = goal.chars().take(80).collect();
    // Always upsert: INSERT OR IGNORE makes this safe for fresh and resumed ids.
    let requested = match session_id {
        Some(s) if !s.trim().is_empty() => s.trim(),
        _ => "",
    };
    let sid = match sessions::create_session(store, requested, workspace_dir, &mode, &title) {
        Ok(id) => id,
        Err(e) => {
            return json!({ "success": false, "error": format!("create session: {e}") });
        }
    };
    if sessions::append_turn(store, &sid, "user", goal, "{}").is_err() {
        return json!({ "success": false, "error": "cannot write to session store", "session_id": sid });
    }

    let mut transcript = AgentSession::new(&sid, AgentMode::Build, workspace_dir);
    // Injectable brain: production passes None (loads ~/.craft/ai_config.json),
    // tests pass a client pointed at a dead endpoint for a fast deterministic
    // fallback path with no lingering child processes.
    let brain = brain.unwrap_or_default();
    let mut steps: Vec<StepRecord> = Vec::new();
    let mut history: Vec<String> = vec![format!("GOAL: {goal}")];
    let mut done = false;
    let mut blocked: Option<String> = None;
    let mut garbage_streak: u8 = 0;

    // Phase 0: deterministic explorer (real work, no LLM needed).
    {
        let tree = CoderCraftCore::scan_tree(Path::new(workspace_dir), 0, 3);
        let inst = CoderCraftCore::discover_instructions(workspace_dir);
        let kws = keywords(goal);
        let mut grep_total = 0;
        let mut grep_show: Vec<String> = Vec::new();
        for kw in kws.iter().take(3) {
            let hits = CoderCraftCore::grep_search(workspace_dir, kw);
            grep_total += hits.len();
            for (p, line, text) in hits.iter().take(3) {
                grep_show.push(format!("{p}:{line}: {}", trunc(text.trim(), 120)));
            }
        }
        let code_files = patch::glob_files(
            workspace_dir,
            "**/*.rs,**/*.py,**/*.ts,**/*.tsx,**/*.go",
        );
        let summary = format!(
            "explore: {} top-level entries, {} code files, {} keyword hits{} agents_md={} claude_md={}",
            tree.children.len(),
            code_files.len(),
            grep_total,
            if grep_show.is_empty() {
                String::new()
            } else {
                format!(" [{}]", grep_show.join(" | "))
            },
            inst.agents_md.is_some(),
            inst.claude_md.is_some()
        );
        let step = StepRecord {
            step: 0,
            kind: "explore".to_string(),
            target: workspace_dir.to_string(),
            decision: "allow".to_string(),
            success: true,
            summary: trunc(&summary, 1500),
        };
        append_step_turn(store, &sid, &step, &summary);
        transcript.add_assistant_message(&summary, vec![]);
        history.push(summary.clone());
        steps.push(step);
    }

    let mut i: usize = 0;
    while i < budget {
        i += 1;
        let hist_tail: String = history.join("\n---\n");
        let hist_tail = if hist_tail.len() > HISTORY_CHARS {
            let start = hist_tail.len() - HISTORY_CHARS;
            let mut s = start;
            while !hist_tail.is_char_boundary(s) {
                s += 1;
            }
            hist_tail[s..].to_string()
        } else {
            hist_tail
        };
        let user_msg = format!(
            "Step {i}/{budget} (mode {mode}). History so far:\n{hist_tail}\nNext: emit ONE tool call or FINAL."
        );
        let reply = match brain_complete(
            brain.clone(),
            system_prompt(workspace_dir, &mode),
            user_msg,
            timeout_s,
        )
        .await
        {
            Some(r) => r,
            None => {
                let step = StepRecord {
                    step: i,
                    kind: "brain-timeout".to_string(),
                    target: format!("no reply within {timeout_s}s"),
                    decision: "allow".to_string(),
                    success: false,
                    summary: "brain unreachable or timed out; ending loop".to_string(),
                };
                append_step_turn(store, &sid, &step, &step.summary);
                steps.push(step);
                blocked = Some(format!("brain timeout after {timeout_s}s"));
                break;
            }
        };

        // Garbage protocol: neither FINAL nor a tool call. Reprompt (bounded),
        // then stop honestly instead of pretending the run completed.
        if !reply.trim_start().starts_with("FINAL:") && extract_tool_json(&reply).is_none() {
            garbage_streak += 1;
            if garbage_streak > MAX_GARBAGE_REPROMPTS {
                let step = StepRecord {
                    step: i,
                    kind: "brain-incoherent".to_string(),
                    target: String::new(),
                    decision: "allow".to_string(),
                    success: false,
                    summary: format!(
                        "brain sent {garbage_streak} non-protocol replies; ending loop. last reply: {}",
                        trunc(reply.trim(), 500)
                    ),
                };
                append_step_turn(store, &sid, &step, &reply);
                steps.push(step);
                blocked = Some("brain incoherent: no FINAL and no tool call".to_string());
                break;
            }
            let step = StepRecord {
                step: i,
                kind: "reprompt".to_string(),
                target: String::new(),
                decision: "allow".to_string(),
                success: false,
                summary: "reply matched neither FINAL nor tool JSON; reprompting".to_string(),
            };
            append_step_turn(store, &sid, &step, &reply);
            history.push(
                "SYSTEM: your last reply matched neither FINAL: nor a tool JSON block. Reply again with exactly one of the two.".to_string(),
            );
            steps.push(step);
            continue;
        }
        garbage_streak = 0;

        let call = extract_tool_json(&reply);
        let (tool_name, tool_args) = match call {
            None => {
                let summary = reply
                    .trim_start()
                    .strip_prefix("FINAL:")
                    .unwrap_or(&reply)
                    .trim()
                    .to_string();
                let step = StepRecord {
                    step: i,
                    kind: "final".to_string(),
                    target: String::new(),
                    decision: "allow".to_string(),
                    success: true,
                    summary: trunc(if summary.is_empty() { &reply } else { &summary }, 1500),
                };
                append_step_turn(store, &sid, &step, &reply);
                transcript.add_assistant_message(&reply, vec![]);
                steps.push(step);
                done = true;
                break;
            }
            Some(v) => {
                let name = v
                    .get("tool")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                let args = v.get("args").cloned().unwrap_or_else(|| json!({}));
                (name, args)
            }
        };

        if !tool_allowed_in_mode(&mode, &tool_name) {
            let step = StepRecord {
                step: i,
                kind: tool_name.clone(),
                target: String::new(),
                decision: "deny".to_string(),
                success: false,
                summary: format!("{mode} mode is read-only; '{tool_name}' refused"),
            };
            append_step_turn(store, &sid, &step, &step.summary);
            steps.push(step);
            blocked = Some(format!("{mode} mode refused mutating tool '{tool_name}'"));
            break;
        }

        let target_for_gate = match tool_name.as_str() {
            "shell" => tool_args
                .get("command_line")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "patch" => workspace_dir,
            "read" | "glob" | "grep" => tool_args
                .get("path")
                .or_else(|| tool_args.get("pattern"))
                .or_else(|| tool_args.get("query"))
                .and_then(|v| v.as_str())
                .unwrap_or(workspace_dir),
            _ => workspace_dir,
        };
        let verdict = permissions::check_tool(permission_class(&tool_name), target_for_gate);
        if verdict.decision == "deny" {
            let step = StepRecord {
                step: i,
                kind: tool_name.clone(),
                target: target_for_gate.to_string(),
                decision: verdict.decision.clone(),
                success: false,
                summary: format!("refused: {}", verdict.reason),
            };
            append_step_turn(store, &sid, &step, &step.summary);
            steps.push(step);
            blocked = Some(verdict.reason.clone());
            break;
        }

        let (ok, result) = execute_tool(workspace_dir, &tool_name, &tool_args);
        let step = StepRecord {
            step: i,
            kind: tool_name.clone(),
            target: target_for_gate.to_string(),
            decision: verdict.decision.clone(),
            success: ok,
            summary: trunc(&result, 1500),
        };
        append_step_turn(store, &sid, &step, &result);
        transcript.add_assistant_message(&format!("[{}] {}", tool_name, trunc(&result, 800)), vec![]);
        history.push(format!("[{}] {}", tool_name, trunc(&result, 1200)));
        steps.push(step);

        // Verify-after-patch: a patch that applies cleanly earns an automatic
        // test run (same timeout family), logged as its own step.
        if tool_name == "patch" && ok {
            i += 1;
            let verify_cmd = detect_test_command(workspace_dir);
            let verify_label = verify_cmd.clone().unwrap_or_else(|| "none".to_string());
            let (vok, vresult) = match verify_cmd {
                None => (
                    true,
                    "verify skipped: no Cargo.toml / pytest markers found".to_string(),
                ),
                Some(cmd) => {
                    let cmd_owned = cmd.clone();
                    let ws_owned = workspace_dir.to_string();
                    let vto = timeout_s.saturating_mul(2);
                    match block_with_timeout(vto, move || {
                        CoderCraftCore::execute_shell(&cmd_owned, Some(&ws_owned))
                    })
                    .await
                    {
                        None => (
                            false,
                            format!("verify timed out after {vto}s: {cmd}"),
                        ),
                        Some((code, stdout, stderr)) => (
                            code == 0,
                            format!(
                                "verify {cmd}: exit={code} stdout={} stderr={}",
                                trunc(stdout.trim(), 1500),
                                trunc(stderr.trim(), 1500)
                            ),
                        ),
                    }
                }
            };
            let vstep = StepRecord {
                step: i,
                kind: "verify".to_string(),
                target: verify_label,
                decision: "allow".to_string(),
                success: vok,
                summary: trunc(&vresult, 1500),
            };
            append_step_turn(store, &sid, &vstep, &vresult);
            history.push(format!("[verify] {}", trunc(&vresult, 800)));
            steps.push(vstep);
            if i >= budget {
                break;
            }
        }
    }

    if !done && blocked.is_none() {
        let note = format!("step budget exhausted ({budget}) without FINAL");
        let _ = sessions::append_turn(store, &sid, "assistant", &note, "{}");
        blocked = Some(note);
    }

    json!({
        "success": true,
        "session_id": sid,
        "workspace": workspace_dir,
        "goal": goal,
        "mode": mode,
        "done": done,
        "blocked": blocked,
        "step_count": steps.len(),
        "steps": steps
    })
}

/// Workspace DB path helper shared by CLI/MCP binaries.
#[must_use]
pub fn eternal_db_path() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .ok()
        .or_else(|| std::env::var("HOME").ok())
        .unwrap_or_else(|| ".".to_string());
    PathBuf::from(home).join(".craft").join("codercraft_sessions.db")
}

/// Re-exported default so CLI/MCP agree on one number.
#[must_use]
pub fn default_timeout_s() -> u64 {
    DEFAULT_TIMEOUT_S
}
