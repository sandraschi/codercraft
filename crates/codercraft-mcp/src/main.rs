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
    Open {
        path: String,
    },
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
    Symbols {
        path: String,
    },
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
    Launch {
        project: String,
    },
    /// Inspect Stigmergic Agent Swarm status
    Swarm,
    /// Detailed help and architectural overview
    HelpOverview,
    /// Compare CoderCraft with Claude Desktop, OpenCode, and AGY
    Compare,
}

fn main() {
    let cli = Cli::parse();
    let rt = Runtime::new().unwrap_or_else(|e| {
        eprintln!("Failed to create tokio runtime: {e}");
        std::process::exit(1);
    });

    let store = DatabaseStore::open_in_memory().unwrap_or_else(|e| {
        eprintln!("Failed to open in-memory store: {e}");
        std::process::exit(1);
    });

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
            let res = rt.block_on(dispatch(Command::ReadFileRange { path, start_line: start, end_line: end }, &store));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Grep { query, workspace }) => {
            let res = rt.block_on(dispatch(Command::GrepSearch { root: workspace, query }, &store));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Symbols { path }) => {
            let res = rt.block_on(dispatch(Command::ExtractSymbols { path }, &store));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Index { workspace }) => {
            let res = rt.block_on(dispatch(Command::IndexWorkspace { workspace_dir: workspace }, &store));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Plan { goal, workspace }) => {
            let res = rt.block_on(dispatch(Command::AgentPlan { goal, workspace_dir: workspace }, &store));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Refactor { intent, target }) => {
            let res = rt.block_on(dispatch(Command::AgentRefactor { intent, target_file: target }, &store));
            println!("{}", serde_json::to_string_pretty(&res["proposal"]).unwrap_or_default());
        }
        Some(Commands::Lint { workspace, fix }) => {
            let res = rt.block_on(dispatch(Command::RunLinter { workspace_dir: workspace, auto_fix: fix }, &store));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Test { filter }) => {
            let res = rt.block_on(dispatch(Command::RunTestSuite { test_filter: filter }, &store));
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
            let res = rt.block_on(dispatch(Command::LaunchFleetProject { folder_name: project }, &store));
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
        None => {
            let help_res = rt.block_on(dispatch(Command::GetHelp, &store));
            println!("CoderCraft Local-First Agentic IDE & Studio v{APP_VERSION} (Port {CONTROL_PORT})");
            println!("{}", serde_json::to_string_pretty(&help_res).unwrap_or_default());
            println!("\nUse 'codercraft-cli compare' for Claude/OpenCode/AGY matrix, or --mcp for FastMCP server.");
        }
    }
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
                        }
                    ]
                }
            }),
            "tools/call" => {
                let name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));

                let tool_output = match name {
                    "codercraft_status" => rt.block_on(dispatch(Command::Status, store)),
                    "codercraft_open_file" => {
                        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
                        rt.block_on(dispatch(Command::OpenFile { path: path.to_string() }, store))
                    }
                    "codercraft_read_file" => {
                        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
                        let start = args.get("start_line").and_then(|s| s.as_u64()).map(|s| s as usize);
                        let end = args.get("end_line").and_then(|e| e.as_u64()).map(|e| e as usize);
                        rt.block_on(dispatch(Command::ReadFileRange { path: path.to_string(), start_line: start, end_line: end }, store))
                    }
                    "codercraft_write_file" => {
                        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
                        let content = args.get("content").and_then(|c| c.as_str()).unwrap_or("");
                        rt.block_on(dispatch(Command::WriteFile { path: path.to_string(), content: content.to_string() }, store))
                    }
                    "codercraft_apply_edit" => {
                        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
                        let target = args.get("target").and_then(|t| t.as_str()).unwrap_or("");
                        let replacement = args.get("replacement").and_then(|r| r.as_str()).unwrap_or("");
                        rt.block_on(dispatch(Command::ApplyHunkEdit {
                            path: path.to_string(),
                            target: target.to_string(),
                            replacement: replacement.to_string(),
                        }, store))
                    }
                    "codercraft_grep" => {
                        let root = args.get("root").and_then(|r| r.as_str()).unwrap_or(".");
                        let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("");
                        rt.block_on(dispatch(Command::GrepSearch { root: root.to_string(), query: query.to_string() }, store))
                    }
                    "codercraft_extract_symbols" => {
                        let path = args.get("path").and_then(|p| p.as_str()).unwrap_or("");
                        rt.block_on(dispatch(Command::ExtractSymbols { path: path.to_string() }, store))
                    }
                    "codercraft_index_workspace" => {
                        let dir = args.get("workspace_dir").and_then(|d| d.as_str()).unwrap_or(".");
                        rt.block_on(dispatch(Command::IndexWorkspace { workspace_dir: dir.to_string() }, store))
                    }
                    "codercraft_plan" => {
                        let goal = args.get("goal").and_then(|g| g.as_str()).unwrap_or("");
                        let dir = args.get("workspace_dir").and_then(|d| d.as_str()).unwrap_or(".");
                        rt.block_on(dispatch(Command::AgentPlan { goal: goal.to_string(), workspace_dir: dir.to_string() }, store))
                    }
                    "codercraft_refactor" => {
                        let intent = args.get("intent").and_then(|i| i.as_str()).unwrap_or("");
                        let target = args.get("target_file").and_then(|t| t.as_str()).unwrap_or("src/main.rs");
                        rt.block_on(dispatch(Command::AgentRefactor { intent: intent.to_string(), target_file: target.to_string() }, store))
                    }
                    "codercraft_lint" => {
                        let dir = args.get("workspace_dir").and_then(|d| d.as_str()).unwrap_or(".");
                        let fix = args.get("auto_fix").and_then(|f| f.as_bool()).unwrap_or(false);
                        rt.block_on(dispatch(Command::RunLinter { workspace_dir: dir.to_string(), auto_fix: fix }, store))
                    }
                    "codercraft_shell" => {
                        let cmd_str = args.get("command_line").and_then(|c| c.as_str()).unwrap_or("");
                        let wd = args.get("working_dir").and_then(|w| w.as_str()).map(|w| w.to_string());
                        rt.block_on(dispatch(Command::ExecuteConsoleCommand { command_line: cmd_str.to_string(), working_dir: wd }, store))
                    }
                    "codercraft_fleet_depot" => rt.block_on(dispatch(Command::ListFleetProjects, store)),
                    "codercraft_probe_depot" => rt.block_on(dispatch(Command::ProbeFleetProjects, store)),
                    "codercraft_fleet_launch" => {
                        let project = args.get("project").and_then(|p| p.as_str()).unwrap_or("");
                        rt.block_on(dispatch(Command::LaunchFleetProject { folder_name: project.to_string() }, store))
                    }
                    "codercraft_swarm_status" => rt.block_on(dispatch(Command::GetSwarmState, store)),
                    "codercraft_help" => rt.block_on(dispatch(Command::GetHelp, store)),
                    "codercraft_compare" => rt.block_on(dispatch(Command::GetComparison, store)),
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
