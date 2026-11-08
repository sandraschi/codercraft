//! CoderCraft MCP Server & Universal CLI.
//!
//! Follows the MCP 2024-11-05 standard: stdout is 100% JSON-RPC, diagnostics go to stderr.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use clap::{Parser, Subcommand};
use codercraft_core::{dispatch, Command, APP_VERSION, CONTROL_PORT};
use databasecraft_store::DatabaseStore;
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use tokio::runtime::Runtime;

#[derive(Parser)]
#[command(name = "codercraft-cli")]
#[command(about = "CoderCraft: Local-First Agentic IDE & Universal CLI", version = APP_VERSION)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    /// Run as MCP 2024-11-05 stdio server
    #[arg(long)]
    mcp: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Check CoderCraft engine status and active subsystems
    Status,
    /// Open and inspect source file buffer
    Open { path: String },
    /// Read lines from a file
    Read {
        path: String,
        #[arg(long)]
        start: Option<usize>,
        #[arg(long)]
        end: Option<usize>,
    },
    /// Grep text pattern across workspace files
    Grep {
        query: String,
        #[arg(default_value = ".")]
        workspace: String,
    },
    /// Extract AST symbols (functions, structs, enums) from a file
    Symbols { path: String },
    /// Index workspace AST symbols, instructions, and file tree
    Index {
        #[arg(default_value = ".")]
        workspace: String,
    },
    /// Generate architectural implementation plan (Plan Mode)
    Plan {
        goal: String,
        #[arg(default_value = ".")]
        workspace: String,
    },
    /// Synthesize autonomous code refactoring proposal (Build Mode)
    Refactor {
        intent: String,
        #[arg(long, default_value = "src/main.rs")]
        target: String,
    },
    /// Run active linter (clippy, ruff, biome)
    Lint {
        #[arg(default_value = ".")]
        workspace: String,
        #[arg(long)]
        fix: bool,
    },
    /// Execute workspace automated test suite
    Test {
        #[arg(long)]
        filter: Option<String>,
    },
    /// List all fleet Craft apps in depot
    Depot,
    /// Actively probe all fleet Craft apps ports
    Probe,
    /// Launch a Craft fleet app by name
    Launch { project: String },
    /// Inspect Stigmergic Agent Swarm status
    Swarm,
    /// Detailed help and architectural overview
    HelpOverview,
    /// Compare CoderCraft with Claude Desktop, OpenCode, and AGY
    Compare,
    /// Apply multi-file patch (OpenCode apply_patch markers)
    ApplyPatch {
        #[arg(default_value = ".")]
        workspace: String,
        #[arg(long)]
        patch_file: Option<String>,
    },
    /// Gitignore-aware file glob
    Glob {
        pattern: String,
        #[arg(default_value = ".")]
        workspace: String,
    },
    /// Check permission verdict for a tool call
    CheckPerm { tool: String, target: String },
    /// Create eternal-memory session
    SessionNew {
        workspace: String,
        #[arg(default_value = "build")]
        mode: String,
        #[arg(default_value = "codercraft session")]
        title: String,
        #[arg(long)]
        id: Option<String>,
    },
    /// Append a turn to a session
    SessionAppend {
        session: String,
        role: String,
        content: String,
    },
    /// List sessions (eternal memory)
    SessionList {
        #[arg(long)]
        workspace: Option<String>,
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        until: Option<String>,
    },
    /// Full-text search across all session turns
    SessionSearch { query: String },
    /// Recall what was discussed in a calendar month (e.g. last September)
    SessionRecall { year: i64, month: u32 },
    /// Read one session transcript
    SessionRead { session: String },
    /// Run autonomous goal loop (plan -> act -> verify, logged to eternal memory)
    Run {
        goal: String,
        #[arg(default_value = ".")]
        workspace: String,
        #[arg(long)]
        session: Option<String>,
        #[arg(long, default_value_t = 0)]
        steps: usize,
        #[arg(long, default_value = "build")]
        mode: String,
        #[arg(long, default_value_t = 0)]
        timeout: u64,
    },
    /// LLM provider management: status, list, use, set-key, evict, test.
    /// Keys passed on the command line land in shell history: prefer the
    /// Settings tab or MCP for secrets.
    Llm {
        #[arg(default_value = "status")]
        action: String,
        #[arg(long)]
        provider: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        key: Option<String>,
    },
}

fn open_eternal_store() -> DatabaseStore {
    let db_path = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map(|h| {
            std::path::PathBuf::from(h)
                .join(".craft")
                .join("codercraft_sessions.db")
        });
    if let Ok(path) = db_path {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(store) = DatabaseStore::open(&path) {
            eprintln!(
                "[codercraft-mcp] eternal session memory: {}",
                path.to_string_lossy()
            );
            return store;
        }
        eprintln!("[codercraft-mcp] disk store unavailable, falling back to memory");
    }
    DatabaseStore::open_in_memory().unwrap_or_else(|e| {
        eprintln!("Failed to open in-memory store: {e}");
        std::process::exit(1);
    })
}

fn main() {
    let cli = Cli::parse();
    let rt = Runtime::new().unwrap_or_else(|e| {
        eprintln!("Failed to create tokio runtime: {e}");
        std::process::exit(1);
    });

    let store = open_eternal_store();

    if cli.mcp {
        run_mcp_stdio(&rt, &store);
        return;
    }

    match cli.command {
        Some(Commands::Status) => {
            let res = rt.block_on(dispatch(Command::Status, &store));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Open { path }) => {
            let res = rt.block_on(dispatch(Command::OpenFile { path }, &store));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Read { path, start, end }) => {
            let res = rt.block_on(dispatch(
                Command::ReadFileRange {
                    path,
                    start_line: start,
                    end_line: end,
                },
                &store,
            ));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Grep { query, workspace }) => {
            let res = rt.block_on(dispatch(
                Command::GrepSearch {
                    root: workspace,
                    query,
                },
                &store,
            ));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Symbols { path }) => {
            let res = rt.block_on(dispatch(Command::ExtractSymbols { path }, &store));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Index { workspace }) => {
            let res = rt.block_on(dispatch(
                Command::IndexWorkspace {
                    workspace_dir: workspace,
                },
                &store,
            ));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Plan { goal, workspace }) => {
            let res = rt.block_on(dispatch(
                Command::AgentPlan {
                    goal,
                    workspace_dir: workspace,
                },
                &store,
            ));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Refactor { intent, target }) => {
            let res = rt.block_on(dispatch(
                Command::AgentRefactor {
                    intent,
                    target_file: target,
                },
                &store,
            ));
            println!(
                "{}",
                serde_json::to_string_pretty(&res["proposal"]).unwrap_or_default()
            );
        }
        Some(Commands::Lint { workspace, fix }) => {
            let res = rt.block_on(dispatch(
                Command::RunLinter {
                    workspace_dir: workspace,
                    auto_fix: fix,
                },
                &store,
            ));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Test { filter }) => {
            let res = rt.block_on(dispatch(
                Command::RunTestSuite {
                    test_filter: filter,
                },
                &store,
            ));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Depot) => {
            let res = rt.block_on(dispatch(Command::ListFleetProjects, &store));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Probe) => {
            let res = rt.block_on(dispatch(Command::ProbeFleetProjects, &store));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Launch { project }) => {
            let res = rt.block_on(dispatch(
                Command::LaunchFleetProject {
                    folder_name: project,
                },
                &store,
            ));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Swarm) => {
            let res = rt.block_on(dispatch(Command::GetSwarmState, &store));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::HelpOverview) => {
            let res = rt.block_on(dispatch(Command::GetHelp, &store));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Compare) => {
            let res = rt.block_on(dispatch(Command::GetComparison, &store));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::ApplyPatch {
            workspace,
            patch_file,
        }) => {
            let patch_text = match patch_file {
                Some(f) => std::fs::read_to_string(&f).unwrap_or_else(|e| {
                    eprintln!("Cannot read patch file {f}: {e}");
                    std::process::exit(1);
                }),
                None => {
                    use std::io::Read as _;
                    let mut buf = String::new();
                    std::io::stdin().read_to_string(&mut buf).unwrap_or(0);
                    buf
                }
            };
            let res = rt.block_on(dispatch(
                Command::ApplyPatch {
                    workspace_dir: workspace,
                    patch_text,
                },
                &store,
            ));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Glob { pattern, workspace }) => {
            let res = rt.block_on(dispatch(
                Command::GlobSearch {
                    root: workspace,
                    pattern,
                },
                &store,
            ));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::CheckPerm { tool, target }) => {
            let res = rt.block_on(dispatch(Command::CheckPermission { tool, target }, &store));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::SessionNew {
            workspace,
            mode,
            title,
            id,
        }) => {
            let res = rt.block_on(dispatch(
                Command::SessionCreate {
                    session_id: id,
                    workspace_dir: workspace,
                    mode,
                    title,
                },
                &store,
            ));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::SessionAppend {
            session,
            role,
            content,
        }) => {
            let res = rt.block_on(dispatch(
                Command::SessionAppend {
                    session_id: session,
                    role,
                    content,
                    tool_calls_json: None,
                },
                &store,
            ));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::SessionList {
            workspace,
            since,
            until,
        }) => {
            let res = rt.block_on(dispatch(
                Command::SessionList {
                    workspace_filter: workspace,
                    since,
                    until,
                    limit: Some(100),
                },
                &store,
            ));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::SessionSearch { query }) => {
            let res = rt.block_on(dispatch(
                Command::SessionSearch {
                    query,
                    top_k: Some(20),
                },
                &store,
            ));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::SessionRecall { year, month }) => {
            let res = rt.block_on(dispatch(
                Command::SessionRecall {
                    year,
                    month,
                    limit: Some(100),
                },
                &store,
            ));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::SessionRead { session }) => {
            let res = rt.block_on(dispatch(
                Command::SessionRead {
                    session_id: session,
                    limit: Some(200),
                },
                &store,
            ));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Run {
            goal,
            workspace,
            session,
            steps,
            mode,
            timeout,
        }) => {
            let res = rt.block_on(dispatch(
                Command::AgentRun {
                    goal,
                    workspace_dir: workspace,
                    session_id: session,
                    max_steps: if steps == 0 { None } else { Some(steps) },
                    mode: Some(mode),
                    timeout_s: if timeout == 0 { None } else { Some(timeout) },
                },
                &store,
            ));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Llm {
            action,
            provider,
            model,
            key,
        }) => {
            let res = match action.as_str() {
                "status" => rt.block_on(dispatch(Command::LlmStatus, &store)),
                "list" => rt.block_on(dispatch(Command::LlmListModels { provider }, &store)),
                "use" => {
                    let p = provider.unwrap_or_default();
                    let m = model.unwrap_or_default();
                    rt.block_on(dispatch(
                        Command::LlmSelect {
                            provider: p,
                            model: m,
                        },
                        &store,
                    ))
                }
                "set-key" => {
                    let p = provider.unwrap_or_default();
                    let k = key.unwrap_or_default();
                    rt.block_on(dispatch(
                        Command::LlmSetKey {
                            provider: p,
                            api_key: k,
                        },
                        &store,
                    ))
                }
                "evict" => rt.block_on(dispatch(Command::LlmEvict { keep: model }, &store)),
                "test" => rt.block_on(dispatch(Command::LlmTest { prompt: model }, &store)),
                other => serde_json::json!({
                    "success": false,
                    "error": format!("unknown llm action '{other}' (status|list|use|set-key|evict|test)")
                }),
            };
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        None => {
            let help_res = rt.block_on(dispatch(Command::GetHelp, &store));
            println!(
                "CoderCraft Local-First Agentic IDE & Studio v{APP_VERSION} (Port {CONTROL_PORT})"
            );
            println!(
                "{}",
                serde_json::to_string_pretty(&help_res).unwrap_or_default()
            );
            println!("\nUse 'codercraft-cli compare for Claude/OpenCode/AGY matrix, or --mcp for FastMCP server.");
        }
    }
    // One-shot CLI: exit immediately. Dropping the Tokio runtime would block
    // until abandoned blocking-pool threads (e.g. a curl still waiting on a
    // slow model after a brain timeout) drain. The DB writes above are all
    // synchronous, so nothing is lost by skipping destructors here.
    // The MCP stdio loop above never reaches this line while serving.
    std::process::exit(0);
}

fn run_mcp_stdio(rt: &Runtime, store: &DatabaseStore) {
    eprintln!("[codercraft-mcp] Starting MCP stdio loop on port {CONTROL_PORT}");
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let raw = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }

        let parsed: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": Value::Null,
                    "error": { "code": -32700, "message": format!("Parse error: {e}") }
                });
                let _ = writeln!(stdout, "{}", err_resp);
                let _ = stdout.flush();
                continue;
            }
        };

        let id = parsed.get("id").cloned().unwrap_or(Value::Null);
        let method = parsed.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = parsed.get("params").cloned().unwrap_or_else(|| json!({}));

        let response = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": {
                        "name": "codercraft",
                        "version": APP_VERSION
                    }
                }
            }),
            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": [
                        {
                            "name": "codercraft_status",
                            "description": "Check CoderCraft engine status, active language, and connected MCP servers.",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "codercraft_open_file",
                            "description": "Open and inspect source file buffer metadata.",
                            "inputSchema": {
                                "type": "object",
                                "properties": { "path": { "type": "string" } },
                                "required": ["path"]
                            }
                        },
                        {
                            "name": "codercraft_read_file",
                            "description": "Read line range from file.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string" },
                                    "start_line": { "type": "integer" },
                                    "end_line": { "type": "integer" }
                                },
                                "required": ["path"]
                            }
                        },
                        {
                            "name": "codercraft_write_file",
                            "description": "Write or overwrite content to file.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string" },
                                    "content": { "type": "string" }
                                },
                                "required": ["path", "content"]
                            }
                        },
                        {
                            "name": "codercraft_apply_edit",
                            "description": "Replace exact target block with replacement string in file.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string" },
                                    "target": { "type": "string" },
                                    "replacement": { "type": "string" }
                                },
                                "required": ["path", "target", "replacement"]
                            }
                        },
                        {
                            "name": "codercraft_grep",
                            "description": "Search workspace files for matching text.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "root": { "type": "string" },
                                    "query": { "type": "string" }
                                },
                                "required": ["root", "query"]
                            }
                        },
                        {
                            "name": "codercraft_extract_symbols",
                            "description": "Extract AST function, struct, and enum declarations from file.",
                            "inputSchema": {
                                "type": "object",
                                "properties": { "path": { "type": "string" } },
                                "required": ["path"]
                            }
                        },
                        {
                            "name": "codercraft_index_workspace",
                            "description": "Index workspace directory, instruction files, and tree.",
                            "inputSchema": {
                                "type": "object",
                                "properties": { "workspace_dir": { "type": "string" } },
                                "required": ["workspace_dir"]
                            }
                        },
                        {
                            "name": "codercraft_plan",
                            "description": "Generate multi-step architectural implementation plan (Plan Mode).",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "goal": { "type": "string" },
                                    "workspace_dir": { "type": "string" }
                                },
                                "required": ["goal", "workspace_dir"]
                            }
                        },
                        {
                            "name": "codercraft_refactor",
                            "description": "Synthesize autonomous AST code refactoring proposal (Build Mode).",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "intent": { "type": "string" },
                                    "target_file": { "type": "string" }
                                },
                                "required": ["intent", "target_file"]
                            }
                        },
                        {
                            "name": "codercraft_lint",
                            "description": "Execute active linter on workspace.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "workspace_dir": { "type": "string" },
                                    "auto_fix": { "type": "boolean" }
                                },
                                "required": ["workspace_dir"]
                            }
                        },
                        {
                            "name": "codercraft_shell",
                            "description": "Execute shell command in workspace sandbox.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "command_line": { "type": "string" },
                                    "working_dir": { "type": "string" }
                                },
                                "required": ["command_line"]
                            }
                        },
                        {
                            "name": "codercraft_fleet_depot",
                            "description": "List all 12 Craft fleet apps, ports, and execution states.",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "codercraft_probe_depot",
                            "description": "Actively probes TCP ports for all 12 Craft fleet apps and returns latencies.",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "codercraft_fleet_launch",
                            "description": "Launch a Craft fleet app by name.",
                            "inputSchema": {
                                "type": "object",
                                "properties": { "project": { "type": "string" } },
                                "required": ["project"]
                            }
                        },
                        {
                            "name": "codercraft_swarm_status",
                            "description": "Inspect Stigmergic Agent Swarm supervisor workers and task ledger.",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "codercraft_help",
                            "description": "Get comprehensive CoderCraft help, feature summary, and capabilities.",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "codercraft_compare",
                            "description": "Detailed architectural comparison matrix: CoderCraft vs Claude Desktop vs OpenCode vs AGY.",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "codercraft_apply_patch",
                            "description": "Apply multi-file patch with OpenCode markers (*** Add File / *** Update File / *** Delete File / *** Move to).",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "workspace_dir": { "type": "string" },
                                    "patch_text": { "type": "string" }
                                },
                                "required": ["workspace_dir", "patch_text"]
                            }
                        },
                        {
                            "name": "codercraft_glob",
                            "description": "Gitignore-aware file glob (supports **, *, ?).",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "root": { "type": "string" },
                                    "pattern": { "type": "string" }
                                },
                                "required": ["root", "pattern"]
                            }
                        },
                        {
                            "name": "codercraft_check_permission",
                            "description": "Check allow/ask/deny verdict for a mutating tool call before running it.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "tool": { "type": "string" },
                                    "target": { "type": "string" }
                                },
                                "required": ["tool", "target"]
                            }
                        },
                        {
                            "name": "codercraft_session_create",
                            "description": "Create an eternal-memory chat session.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "workspace_dir": { "type": "string" },
                                    "mode": { "type": "string" },
                                    "title": { "type": "string" },
                                    "session_id": { "type": "string" }
                                },
                                "required": ["workspace_dir"]
                            }
                        },
                        {
                            "name": "codercraft_session_append",
                            "description": "Append a turn (user/assistant) to an eternal-memory session.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "session_id": { "type": "string" },
                                    "role": { "type": "string" },
                                    "content": { "type": "string" }
                                },
                                "required": ["session_id", "role", "content"]
                            }
                        },
                        {
                            "name": "codercraft_session_list",
                            "description": "List eternal-memory sessions with optional workspace/date filters.",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "codercraft_session_search",
                            "description": "Full-text search across every session turn (eternal memory elicitation).",
                            "inputSchema": {
                                "type": "object",
                                "properties": { "query": { "type": "string" } },
                                "required": ["query"]
                            }
                        },
                        {
                            "name": "codercraft_session_recall",
                            "description": "Recall what was discussed in a calendar month (year + 1-12 month).",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "year": { "type": "integer" },
                                    "month": { "type": "integer" }
                                },
                                "required": ["year", "month"]
                            }
                        },
                        {
                            "name": "codercraft_session_read",
                            "description": "Read one session transcript.",
                            "inputSchema": {
                                "type": "object",
                                "properties": { "session_id": { "type": "string" } },
                                "required": ["session_id"]
                            }
                        },
                        {
                            "name": "codercraft_run",
                            "description": "Run autonomous goal loop (explore -> act -> verify), every step logged to eternal memory. Returns session_id plus step ledger.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "goal": { "type": "string" },
                                    "workspace_dir": { "type": "string" },
                                    "session_id": { "type": "string" },
                                    "max_steps": { "type": "integer" },
                                    "mode": { "type": "string" },
                                    "timeout_s": { "type": "integer" }
                                },
                                "required": ["goal", "workspace_dir"]
                            }
                        },
                        {
                            "name": "codercraft_llm_status",
                            "description": "LLM provider status: file config (keys redacted) plus reachability probe of the configured provider.",
                            "inputSchema": { "type": "object", "properties": {} }
                        },
                        {
                            "name": "codercraft_llm_list_models",
                            "description": "List models for a provider (live list; curated only for Anthropic with a key).",
                            "inputSchema": {
                                "type": "object",
                                "properties": { "provider": { "type": "string" } }
                            }
                        },
                        {
                            "name": "codercraft_llm_select",
                            "description": "Select provider + model, persist it, and switch VRAM (Ollama evicts others and warms the pick).",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "provider": { "type": "string" },
                                    "model": { "type": "string" }
                                },
                                "required": ["provider", "model"]
                            }
                        },
                        {
                            "name": "codercraft_llm_set_key",
                            "description": "Store a cloud provider API key in ~/.craft/ai_config.json. Never echoed back.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "provider": { "type": "string" },
                                    "api_key": { "type": "string" }
                                },
                                "required": ["provider", "api_key"]
                            }
                        },
                        {
                            "name": "codercraft_llm_evict",
                            "description": "Evict Ollama residents; keep one warm when given.",
                            "inputSchema": {
                                "type": "object",
                                "properties": { "keep": { "type": "string" } }
                            }
                        },
                        {
                            "name": "codercraft_llm_test",
                            "description": "Probe the configured brain with a short completion (90s cap).",
                            "inputSchema": {
                                "type": "object",
                                "properties": { "prompt": { "type": "string" } }
                            }
                        }
                    ]
                }
            }),
            "tools/call" => {
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));

                let tool_output = match name {
                    "codercraft_status" => rt.block_on(dispatch(Command::Status, store)),
                    "codercraft_open_file" => {
                        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
                        rt.block_on(dispatch(
                            Command::OpenFile {
                                path: path.to_string(),
                            },
                            store,
                        ))
                    }
                    "codercraft_read_file" => {
                        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
                        let start = args
                            .get("start_line")
                            .and_then(|s| s.as_u64())
                            .map(|s| s as usize);
                        let end = args
                            .get("end_line")
                            .and_then(|e| e.as_u64())
                            .map(|e| e as usize);
                        rt.block_on(dispatch(
                            Command::ReadFileRange {
                                path: path.to_string(),
                                start_line: start,
                                end_line: end,
                            },
                            store,
                        ))
                    }
                    "codercraft_write_file" => {
                        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
                        let content = args.get("content").and_then(|c| c.as_str()).unwrap_or("");
                        rt.block_on(dispatch(
                            Command::WriteFile {
                                path: path.to_string(),
                                content: content.to_string(),
                            },
                            store,
                        ))
                    }
                    "codercraft_apply_edit" => {
                        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
                        let target = args.get("target").and_then(|t| t.as_str()).unwrap_or("");
                        let replacement = args
                            .get("replacement")
                            .and_then(|r| r.as_str())
                            .unwrap_or("");
                        rt.block_on(dispatch(
                            Command::ApplyHunkEdit {
                                path: path.to_string(),
                                target: target.to_string(),
                                replacement: replacement.to_string(),
                            },
                            store,
                        ))
                    }
                    "codercraft_grep" => {
                        let root = args.get("root").and_then(|r| r.as_str()).unwrap_or(".");
                        let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
                        rt.block_on(dispatch(
                            Command::GrepSearch {
                                root: root.to_string(),
                                query: query.to_string(),
                            },
                            store,
                        ))
                    }
                    "codercraft_extract_symbols" => {
                        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
                        rt.block_on(dispatch(
                            Command::ExtractSymbols {
                                path: path.to_string(),
                            },
                            store,
                        ))
                    }
                    "codercraft_index_workspace" => {
                        let dir = args
                            .get("workspace_dir")
                            .and_then(|d| d.as_str())
                            .unwrap_or(".");
                        rt.block_on(dispatch(
                            Command::IndexWorkspace {
                                workspace_dir: dir.to_string(),
                            },
                            store,
                        ))
                    }
                    "codercraft_plan" => {
                        let goal = args.get("goal").and_then(|g| g.as_str()).unwrap_or("");
                        let dir = args
                            .get("workspace_dir")
                            .and_then(|d| d.as_str())
                            .unwrap_or(".");
                        rt.block_on(dispatch(
                            Command::AgentPlan {
                                goal: goal.to_string(),
                                workspace_dir: dir.to_string(),
                            },
                            store,
                        ))
                    }
                    "codercraft_refactor" => {
                        let intent = args.get("intent").and_then(|i| i.as_str()).unwrap_or("");
                        let target = args
                            .get("target_file")
                            .and_then(|t| t.as_str())
                            .unwrap_or("src/main.rs");
                        rt.block_on(dispatch(
                            Command::AgentRefactor {
                                intent: intent.to_string(),
                                target_file: target.to_string(),
                            },
                            store,
                        ))
                    }
                    "codercraft_lint" => {
                        let dir = args
                            .get("workspace_dir")
                            .and_then(|d| d.as_str())
                            .unwrap_or(".");
                        let fix = args
                            .get("auto_fix")
                            .and_then(|f| f.as_bool())
                            .unwrap_or(false);
                        rt.block_on(dispatch(
                            Command::RunLinter {
                                workspace_dir: dir.to_string(),
                                auto_fix: fix,
                            },
                            store,
                        ))
                    }
                    "codercraft_shell" => {
                        let cmd_str = args
                            .get("command_line")
                            .and_then(|c| c.as_str())
                            .unwrap_or("");
                        let wd = args
                            .get("working_dir")
                            .and_then(|w| w.as_str())
                            .map(|w| w.to_string());
                        rt.block_on(dispatch(
                            Command::ExecuteConsoleCommand {
                                command_line: cmd_str.to_string(),
                                working_dir: wd,
                            },
                            store,
                        ))
                    }
                    "codercraft_fleet_depot" => {
                        rt.block_on(dispatch(Command::ListFleetProjects, store))
                    }
                    "codercraft_probe_depot" => {
                        rt.block_on(dispatch(Command::ProbeFleetProjects, store))
                    }
                    "codercraft_fleet_launch" => {
                        let project = args.get("project").and_then(|p| p.as_str()).unwrap_or("");
                        rt.block_on(dispatch(
                            Command::LaunchFleetProject {
                                folder_name: project.to_string(),
                            },
                            store,
                        ))
                    }
                    "codercraft_swarm_status" => {
                        rt.block_on(dispatch(Command::GetSwarmState, store))
                    }
                    "codercraft_help" => rt.block_on(dispatch(Command::GetHelp, store)),
                    "codercraft_compare" => rt.block_on(dispatch(Command::GetComparison, store)),
                    "codercraft_apply_patch" => {
                        let dir = args
                            .get("workspace_dir")
                            .and_then(|d| d.as_str())
                            .unwrap_or(".");
                        let patch = args
                            .get("patch_text")
                            .and_then(|p| p.as_str())
                            .unwrap_or("");
                        rt.block_on(dispatch(
                            Command::ApplyPatch {
                                workspace_dir: dir.to_string(),
                                patch_text: patch.to_string(),
                            },
                            store,
                        ))
                    }
                    "codercraft_glob" => {
                        let root = args.get("root").and_then(|r| r.as_str()).unwrap_or(".");
                        let pattern = args
                            .get("pattern")
                            .and_then(|p| p.as_str())
                            .unwrap_or("**/*.rs");
                        rt.block_on(dispatch(
                            Command::GlobSearch {
                                root: root.to_string(),
                                pattern: pattern.to_string(),
                            },
                            store,
                        ))
                    }
                    "codercraft_check_permission" => {
                        let tool = args.get("tool").and_then(|t| t.as_str()).unwrap_or("");
                        let target = args.get("target").and_then(|t| t.as_str()).unwrap_or("");
                        rt.block_on(dispatch(
                            Command::CheckPermission {
                                tool: tool.to_string(),
                                target: target.to_string(),
                            },
                            store,
                        ))
                    }
                    "codercraft_session_create" => {
                        let dir = args
                            .get("workspace_dir")
                            .and_then(|d| d.as_str())
                            .unwrap_or(".");
                        let mode = args.get("mode").and_then(|m| m.as_str()).unwrap_or("build");
                        let title = args
                            .get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or("codercraft session");
                        let sid = args
                            .get("session_id")
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string());
                        rt.block_on(dispatch(
                            Command::SessionCreate {
                                session_id: sid,
                                workspace_dir: dir.to_string(),
                                mode: mode.to_string(),
                                title: title.to_string(),
                            },
                            store,
                        ))
                    }
                    "codercraft_session_append" => {
                        let sid = args
                            .get("session_id")
                            .and_then(|s| s.as_str())
                            .unwrap_or("");
                        let role = args.get("role").and_then(|r| r.as_str()).unwrap_or("user");
                        let content = args.get("content").and_then(|c| c.as_str()).unwrap_or("");
                        rt.block_on(dispatch(
                            Command::SessionAppend {
                                session_id: sid.to_string(),
                                role: role.to_string(),
                                content: content.to_string(),
                                tool_calls_json: None,
                            },
                            store,
                        ))
                    }
                    "codercraft_session_list" => {
                        let ws = args
                            .get("workspace_filter")
                            .and_then(|w| w.as_str())
                            .map(|w| w.to_string());
                        let since = args
                            .get("since")
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string());
                        let until = args
                            .get("until")
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string());
                        rt.block_on(dispatch(
                            Command::SessionList {
                                workspace_filter: ws,
                                since,
                                until,
                                limit: Some(100),
                            },
                            store,
                        ))
                    }
                    "codercraft_session_search" => {
                        let q = args.get("query").and_then(|x| x.as_str()).unwrap_or("");
                        rt.block_on(dispatch(
                            Command::SessionSearch {
                                query: q.to_string(),
                                top_k: Some(20),
                            },
                            store,
                        ))
                    }
                    "codercraft_session_recall" => {
                        let year = args.get("year").and_then(|y| y.as_i64()).unwrap_or(2026);
                        let month = args.get("month").and_then(|m| m.as_u64()).unwrap_or(1) as u32;
                        rt.block_on(dispatch(
                            Command::SessionRecall {
                                year,
                                month,
                                limit: Some(100),
                            },
                            store,
                        ))
                    }
                    "codercraft_session_read" => {
                        let sid = args
                            .get("session_id")
                            .and_then(|s| s.as_str())
                            .unwrap_or("");
                        rt.block_on(dispatch(
                            Command::SessionRead {
                                session_id: sid.to_string(),
                                limit: Some(200),
                            },
                            store,
                        ))
                    }
                    "codercraft_run" => {
                        let goal = args.get("goal").and_then(|g| g.as_str()).unwrap_or("");
                        let dir = args
                            .get("workspace_dir")
                            .and_then(|d| d.as_str())
                            .unwrap_or(".");
                        let sid = args
                            .get("session_id")
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string());
                        let steps = args
                            .get("max_steps")
                            .and_then(|n| n.as_u64())
                            .map(|n| n as usize);
                        let mode = args
                            .get("mode")
                            .and_then(|m| m.as_str())
                            .map(|m| m.to_string());
                        let timeout = args.get("timeout_s").and_then(|n| n.as_u64());
                        rt.block_on(dispatch(
                            Command::AgentRun {
                                goal: goal.to_string(),
                                workspace_dir: dir.to_string(),
                                session_id: sid,
                                max_steps: steps,
                                mode,
                                timeout_s: timeout,
                            },
                            store,
                        ))
                    }
                    "codercraft_llm_status" => rt.block_on(dispatch(Command::LlmStatus, store)),
                    "codercraft_llm_list_models" => {
                        let provider = args
                            .get("provider")
                            .and_then(|p| p.as_str())
                            .map(|p| p.to_string());
                        rt.block_on(dispatch(Command::LlmListModels { provider }, store))
                    }
                    "codercraft_llm_select" => {
                        let provider = args.get("provider").and_then(|p| p.as_str()).unwrap_or("");
                        let model = args.get("model").and_then(|m| m.as_str()).unwrap_or("");
                        rt.block_on(dispatch(
                            Command::LlmSelect {
                                provider: provider.to_string(),
                                model: model.to_string(),
                            },
                            store,
                        ))
                    }
                    "codercraft_llm_set_key" => {
                        let provider = args.get("provider").and_then(|p| p.as_str()).unwrap_or("");
                        let key = args.get("api_key").and_then(|k| k.as_str()).unwrap_or("");
                        rt.block_on(dispatch(
                            Command::LlmSetKey {
                                provider: provider.to_string(),
                                api_key: key.to_string(),
                            },
                            store,
                        ))
                    }
                    "codercraft_llm_evict" => {
                        let keep = args
                            .get("keep")
                            .and_then(|k| k.as_str())
                            .map(|k| k.to_string());
                        rt.block_on(dispatch(Command::LlmEvict { keep }, store))
                    }
                    "codercraft_llm_test" => {
                        let prompt = args
                            .get("prompt")
                            .and_then(|p| p.as_str())
                            .map(|p| p.to_string());
                        rt.block_on(dispatch(Command::LlmTest { prompt }, store))
                    }
                    _ => json!({ "error": format!("Unknown tool: {name}") }),
                };

                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": serde_json::to_string_pretty(&tool_output).unwrap_or_default()
                            }
                        ]
                    }
                })
            }
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Method not found: {method}") }
            }),
        };

        let _ = writeln!(stdout, "{}", response);
        let _ = stdout.flush();
    }
}
