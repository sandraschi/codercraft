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
#[command(about = "CoderCraft: Sovereign Agentic IDE & Universal CLI", version = APP_VERSION)]
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
    /// Index workspace AST symbols, call-graph, and vector embeddings
    Index {
        #[arg(default_value = ".")]
        workspace: String,
    },
    /// Synthesize autonomous code refactoring proposal
    Refactor {
        intent: String,
        #[arg(long, default_value = "src/main.rs")]
        target: String,
    },
    /// Execute workspace automated test suite
    Test {
        #[arg(long)]
        filter: Option<String>,
    },
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
        Some(Commands::Index { workspace }) => {
            let res = rt.block_on(dispatch(Command::IndexWorkspace { workspace_dir: workspace }, &store));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        Some(Commands::Refactor { intent, target }) => {
            let res = rt.block_on(dispatch(Command::AgentRefactor { intent, target_file: target }, &store));
            println!("{}", serde_json::to_string_pretty(&res["proposal"]).unwrap_or_default());
        }
        Some(Commands::Test { filter }) => {
            let res = rt.block_on(dispatch(Command::RunTestSuite { test_filter: filter }, &store));
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        }
        None => {
            println!("CoderCraft CLI v{APP_VERSION} on Port {CONTROL_PORT}");
            println!("Use --help to view available commands or --mcp to start JSON-RPC server.");
        }
    }
}

fn run_mcp_stdio(rt: &Runtime, store: &DatabaseStore) {
    eprintln!("[codercraft-mcp] Starting MCP stdio loop on port {CONTROL_PORT}");
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

        let resp = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "codercraft-mcp", "version": APP_VERSION }
                }
            }),
            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": [
                        { "name": "code_status", "description": "Check CoderCraft engine status", "inputSchema": { "type": "object", "properties": {} } },
                        {
                            "name": "code_open_file",
                            "description": "Open and inspect source file buffer",
                            "inputSchema": {
                                "type": "object",
                                "properties": { "path": { "type": "string" } },
                                "required": ["path"]
                            }
                        },
                        {
                            "name": "code_refactor",
                            "description": "Synthesize autonomous AST code refactoring proposal",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "intent": { "type": "string" },
                                    "target": { "type": "string" }
                                },
                                "required": ["intent", "target"]
                            }
                        },
                        { "name": "code_run_tests", "description": "Run workspace verification test suite", "inputSchema": { "type": "object", "properties": {} } }
                    ]
                }
            }),
            "tools/call" => {
                let params = req.get("params").cloned().unwrap_or(Value::Null);
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));

                let tool_res = match tool_name {
                    "code_status" => rt.block_on(dispatch(Command::Status, store)),
                    "code_open_file" => {
                        let path = args["path"].as_str().unwrap_or("src/main.rs").to_string();
                        rt.block_on(dispatch(Command::OpenFile { path }, store))
                    }
                    "code_refactor" => {
                        let intent = args["intent"].as_str().unwrap_or("Refactor").to_string();
                        let target = args["target"].as_str().unwrap_or("src/main.rs").to_string();
                        rt.block_on(dispatch(Command::AgentRefactor { intent, target_file: target }, store))
                    }
                    "code_run_tests" => rt.block_on(dispatch(Command::RunTestSuite { test_filter: None }, store)),
                    unknown => json!({ "success": false, "error": format!("Unknown tool '{unknown}'") })
                };

                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": { "content": [{ "type": "text", "text": tool_res.to_string() }] }
                })
            }
            _ => json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": "Method not found" } })
        };

        let _ = writeln!(stdout, "{}", resp.to_string());
        let _ = stdout.flush();
    }
}
