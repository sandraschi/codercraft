//! CoderCraft Core Library & Universal Command Bus.
//!
//! Subsumes OpenCode, Cursor, and Zed into a single local-first ArtCraft application.
//! Features parity with Claude Desktop / AGY:
//! - Multi-language support (Rust, Python, TypeScript, Go, C++)
//! - Automated linter setup (Clippy, Ruff, Biome, golangci-lint, clang-tidy)
//! - Multi-Display Quad Layout management (Editor, Terminals, Supervisor, Preview)
//! - Fleet Project Depot with active TCP probe telemetry (Ports 14111..14131)
//! - AST symbol outline extraction and multi-buffer management
//! - Stigmergic Agent Swarm state management
//! - Workspace instruction discovery (AGENTS.md, CLAUDE.md, .cursorrules)
//! - Local (Ollama) and Cloud (Claude, GPT, DeepSeek) LLM configuration
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

pub mod autonomy;
pub mod i18n;
pub mod patch;
pub mod permissions;
pub mod pomerado;
pub mod sessions;

/// Minimal MCP 2024-11-05 stdio *client*: spawn any MCP server, run the
/// initialize handshake, then `tools/list` / `tools/call` in one shot.
///
/// Design notes:
/// - One-shot per call (spawn -> handshake -> request -> kill). No long-lived
///   child management, so the GUI/CLI can never leak a server process.
/// - Newline-delimited JSON-RPC, matching what `codercraft-cli --mcp` serves.
/// - All timeouts bounded; every failure is a `String`, never a panic.
pub mod mcp_client {
    use super::McpServerEntry;
    use serde_json::{json, Value};
    use std::path::PathBuf;
    use std::time::Duration;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    fn servers_file() -> PathBuf {
        let base = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        base.join(".craft").join("codercraft_mcp_servers.json")
    }

    fn default_servers() -> Vec<McpServerEntry> {
        vec![
            McpServerEntry {
                name: "databasecraft".to_string(),
                command: "D:\\Dev\\repos\\databasecraft\\target\\debug\\databasecraft-mcp.exe"
                    .to_string(),
                args: vec![],
                env: std::collections::HashMap::new(),
                is_enabled: true,
            },
            McpServerEntry {
                name: "djcraft".to_string(),
                command: "D:\\Dev\\repos\\djcraft\\target\\debug\\djcraft-mcp.exe".to_string(),
                args: vec![],
                env: std::collections::HashMap::new(),
                is_enabled: true,
            },
            McpServerEntry {
                name: "gamecraft".to_string(),
                command: "D:\\Dev\\repos\\gamecraft\\target\\debug\\gamecraft-mcp.exe"
                    .to_string(),
                args: vec![],
                env: std::collections::HashMap::new(),
                is_enabled: true,
            },
        ]
    }

    /// Load persisted registry; falls back to built-in defaults when the
    /// file is missing or corrupt (never an error to the caller).
    #[must_use]
    pub fn load_servers() -> Vec<McpServerEntry> {
        let path = servers_file();
        match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<Vec<McpServerEntry>>(&text) {
                Ok(list) => list,
                Err(_) => default_servers(),
            },
            Err(_) => default_servers(),
        }
    }

    pub fn save_servers(servers: &[McpServerEntry]) -> Result<(), String> {
        let path = servers_file();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create {}: {e}", parent.to_string_lossy()))?;
        }
        let text = serde_json::to_string_pretty(servers)
            .map_err(|e| format!("cannot encode registry: {e}"))?;
        std::fs::write(&path, text)
            .map_err(|e| format!("cannot write {}: {e}", path.to_string_lossy()))?;
        Ok(())
    }

    #[must_use]
    pub fn find_server(name: &str) -> Option<McpServerEntry> {
        load_servers()
            .into_iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
    }

    async fn read_line_timeout(
        reader: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
        timeout: Duration,
    ) -> Result<String, String> {
        tokio::time::timeout(timeout, reader.next_line())
            .await
            .map_err(|_| "timed out waiting for MCP server reply".to_string())?
            .map_err(|e| format!("failed reading MCP server stdout: {e}"))?
            .ok_or_else(|| "MCP server closed stdout (EOF)".to_string())
    }

    struct Session {
        child: tokio::process::Child,
        stdin: tokio::process::ChildStdin,
        lines: tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
        next_id: i64,
    }

    impl Session {
        async fn connect(entry: &McpServerEntry, timeout: Duration) -> Result<Self, String> {
            if !entry.is_enabled {
                return Err(format!("server '{}' is disabled", entry.name));
            }
            if entry.command.trim().is_empty() {
                return Err(format!("server '{}' has an empty command", entry.name));
            }
            let mut cmd = tokio::process::Command::new(&entry.command);
            cmd.args(&entry.args);
            for (k, v) in &entry.env {
                cmd.env(k, v);
            }
            cmd.stdin(std::process::Stdio::piped());
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::null());
            cmd.kill_on_drop(true);
            let mut child = cmd
                .spawn()
                .map_err(|e| format!("cannot spawn '{}': {e}", entry.command))?;
            let stdin = child
                .stdin
                .take()
                .ok_or_else(|| "server stdin unavailable".to_string())?;
            let stdout = child
                .stdout
                .take()
                .ok_or_else(|| "server stdout unavailable".to_string())?;
            let mut session = Self {
                child,
                stdin,
                lines: BufReader::new(stdout).lines(),
                next_id: 1,
            };
            // initialize handshake
            let init = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "codercraft", "version": super::APP_VERSION }
                }
            });
            session.next_id = 2;
            session.send(&init).await?;
            let reply = session.recv(timeout).await?;
            if reply.get("error").is_some() {
                return Err(format!(
                    "initialize rejected by '{}': {reply}",
                    entry.name
                ));
            }
            // notifications/initialized (no id, no reply expected)
            let notif = json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
            session.send(&notif).await?;
            Ok(session)
        }

        async fn send(&mut self, value: &Value) -> Result<(), String> {
            let mut text = serde_json::to_string(value)
                .map_err(|e| format!("cannot encode request: {e}"))?;
            text.push('\n');
            self.stdin
                .write_all(text.as_bytes())
                .await
                .map_err(|e| format!("failed writing to MCP server: {e}"))?;
            self.stdin
                .flush()
                .await
                .map_err(|e| format!("failed flushing MCP server stdin: {e}"))?;
            Ok(())
        }

        async fn recv(&mut self, timeout: Duration) -> Result<Value, String> {
            let line = read_line_timeout(&mut self.lines, timeout).await?;
            serde_json::from_str::<Value>(&line)
                .map_err(|e| format!("invalid JSON from MCP server: {e}"))
        }

        async fn request(
            &mut self,
            method: &str,
            params: Value,
            timeout: Duration,
        ) -> Result<Value, String> {
            let id = self.next_id;
            self.next_id += 1;
            self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))
                .await?;
            // Skip stray lines (progress notifications, or servers that
            // incorrectly reply to notifications) until the matching id.
            let deadline = std::time::Instant::now() + timeout;
            for _ in 0..100 {
                let remaining = deadline.saturating_duration_since(std::time::Instant::now());
                if remaining.is_zero() {
                    return Err(format!("timed out waiting for '{method}' reply"));
                }
                let line = read_line_timeout(&mut self.lines, remaining).await?;
                let reply: Value = serde_json::from_str::<Value>(&line)
                    .map_err(|e| format!("invalid JSON from MCP server: {e}"))?;
                let matches = reply
                    .get("id")
                    .and_then(|v| v.as_i64())
                    .is_some_and(|got| got == id);
                if !matches {
                    continue;
                }
                if let Some(err) = reply.get("error") {
                    return Err(format!("MCP error from '{method}': {err}"));
                }
                return reply
                    .get("result")
                    .cloned()
                    .ok_or_else(|| format!("MCP '{method}' reply had no result: {reply}"));
            }
            Err(format!("too many stray lines waiting for '{method}' reply"))
        }

        async fn shutdown(mut self) {
            let _ = self.child.kill().await;
        }
    }

    /// Spawn `entry`, handshake, return its `tools/list` result.
    pub async fn list_tools_live(entry: &McpServerEntry) -> Result<Value, String> {
        let timeout = Duration::from_secs(12);
        let mut session = Session::connect(entry, timeout).await?;
        let result = session
            .request("tools/list", json!({}), timeout)
            .await;
        session.shutdown().await;
        result
    }

    /// Spawn `entry`, handshake, run one `tools/call`, return its `result`.
    pub async fn call_tool_live(
        entry: &McpServerEntry,
        tool: &str,
        arguments: Value,
    ) -> Result<Value, String> {
        if tool.trim().is_empty() {
            return Err("empty tool name".to_string());
        }
        let timeout = Duration::from_secs(60);
        let mut session = Session::connect(entry, Duration::from_secs(12)).await?;
        let result = session
            .request(
                "tools/call",
                json!({"name": tool, "arguments": arguments}),
                timeout,
            )
            .await;
        session.shutdown().await;
        result
    }
}

use codercraft_agent::{AgentEngine, AgentMode, AgentSession, SwarmLedger};
use databasecraft_store::DatabaseStore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Single LLM truth for CoderCraft: the fleet-shared `craft-ai-core` config.
/// Every craft app must use this type instead of a per-app struct
/// (see `CRAFT_LLM_PROVIDER_STANDARD.md`). The file
/// `~/.craft/ai_config.json` is the persistence; environment wins per-field.
pub use craft_ai_core::{self, CraftAiConfig};

pub const CONTROL_PORT: u16 = 14128;
pub const APP_VERSION: &str = "0.2.0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SupportedLanguage {
    Rust,
    Python,
    TypeScript,
    Go,
    Cpp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinterConfig {
    pub language: SupportedLanguage,
    pub command: String,
    pub default_args: Vec<String>,
    pub auto_fix_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntry {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub is_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size_bytes: u64,
    pub children: Vec<FileTreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInstructions {
    pub agents_md: Option<String>,
    pub claude_md: Option<String>,
    pub cursor_rules: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstSymbol {
    pub name: String,
    pub kind: String, // "fn", "struct", "enum", "trait", "class", "impl"
    pub line_number: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenBuffer {
    pub path: String,
    pub file_name: String,
    pub content: String,
    pub is_dirty: bool,
    pub line_count: usize,
    pub language: SupportedLanguage,
}

impl OpenBuffer {
    #[must_use]
    pub fn new(path: &str, content: String) -> Self {
        let p = Path::new(path);
        let file_name = p
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("untitled")
            .to_string();
        let line_count = content.lines().count();
        let language = if path.ends_with(".rs") {
            SupportedLanguage::Rust
        } else if path.ends_with(".py") {
            SupportedLanguage::Python
        } else if path.ends_with(".ts") || path.ends_with(".js") || path.ends_with(".tsx") {
            SupportedLanguage::TypeScript
        } else if path.ends_with(".go") {
            SupportedLanguage::Go
        } else {
            SupportedLanguage::Cpp
        };

        Self {
            path: path.to_string(),
            file_name,
            content,
            is_dirty: false,
            line_count,
            language,
        }
    }

    pub fn save(&mut self) -> Result<(), String> {
        CoderCraftCore::write_file(&self.path, &self.content)?;
        self.is_dirty = false;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetProjectEntry {
    pub name: String,
    pub folder_name: String,
    pub port: u16,
    pub description: String,
    pub category: String,
    pub start_bat_path: String,
    pub is_running: bool,
    pub ping_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetDepot {
    pub projects: Vec<FleetProjectEntry>,
}

impl Default for FleetDepot {
    fn default() -> Self {
        Self::new()
    }
}

impl FleetDepot {
    #[must_use]
    pub fn new() -> Self {
        Self {
            projects: vec![
                FleetProjectEntry {
                    name: "codercraft".to_string(),
                    folder_name: "codercraft".to_string(),
                    port: 14128,
                    description: "Local-First Agentic IDE & Universal MCP Server".to_string(),
                    category: "IDE / DevTools".to_string(),
                    start_bat_path: "D:\\Dev\\repos\\codercraft\\start.bat".to_string(),
                    is_running: true,
                    ping_ms: Some(1),
                },
                FleetProjectEntry {
                    name: "databasecraft".to_string(),
                    folder_name: "databasecraft".to_string(),
                    port: 14113,
                    description: "Local Embedded SQLite + Vector RAG Storage Engine".to_string(),
                    category: "Data / Storage".to_string(),
                    start_bat_path: "D:\\Dev\\repos\\databasecraft\\start.bat".to_string(),
                    is_running: false,
                    ping_ms: None,
                },
                FleetProjectEntry {
                    name: "djcraft".to_string(),
                    folder_name: "djcraft".to_string(),
                    port: 14118,
                    description: "Universal DJ Controller, Stem Separation & DVS Studio"
                        .to_string(),
                    category: "Audio / Creative".to_string(),
                    start_bat_path: "D:\\Dev\\repos\\djcraft\\start.bat".to_string(),
                    is_running: false,
                    ping_ms: None,
                },
                FleetProjectEntry {
                    name: "modcraft".to_string(),
                    folder_name: "modcraft".to_string(),
                    port: 14120,
                    description: "Tracker, Synthesizer & Sound Design Audio Workstation"
                        .to_string(),
                    category: "Audio / Music".to_string(),
                    start_bat_path: "D:\\Dev\\repos\\modcraft\\start.bat".to_string(),
                    is_running: false,
                    ping_ms: None,
                },
                FleetProjectEntry {
                    name: "appcraft".to_string(),
                    folder_name: "appcraft".to_string(),
                    port: 14114,
                    description: "Cross-Platform Mobile & Desktop Local GUI Studio".to_string(),
                    category: "Applications".to_string(),
                    start_bat_path: "D:\\Dev\\repos\\appcraft\\start.bat".to_string(),
                    is_running: false,
                    ping_ms: None,
                },
                FleetProjectEntry {
                    name: "robotcraft".to_string(),
                    folder_name: "robotcraft".to_string(),
                    port: 14115,
                    description: "Autonomous Robotics Simulation, Kinematics & URDF CAD"
                        .to_string(),
                    category: "Robotics".to_string(),
                    start_bat_path: "D:\\Dev\\repos\\robotcraft\\start.bat".to_string(),
                    is_running: false,
                    ping_ms: None,
                },
                FleetProjectEntry {
                    name: "writercraft".to_string(),
                    folder_name: "writercraft".to_string(),
                    port: 14124,
                    description: "Local Distraction-Free Authoring & Research Suite".to_string(),
                    category: "Authoring".to_string(),
                    start_bat_path: "D:\\Dev\\repos\\writercraft\\start.bat".to_string(),
                    is_running: false,
                    ping_ms: None,
                },
                FleetProjectEntry {
                    name: "travelcraft".to_string(),
                    folder_name: "travelcraft".to_string(),
                    port: 14129,
                    description: "Autonomous Baedeker Itinerary & Offline GIS Navigator"
                        .to_string(),
                    category: "Geospatial".to_string(),
                    start_bat_path: "D:\\Dev\\repos\\travelcraft\\start.bat".to_string(),
                    is_running: false,
                    ping_ms: None,
                },
                FleetProjectEntry {
                    name: "makercraft".to_string(),
                    folder_name: "makercraft".to_string(),
                    port: 14130,
                    description: "Parametric CAD/CAM, CNC Toolpaths & 3D Slicing".to_string(),
                    category: "Hardware / CAD".to_string(),
                    start_bat_path: "D:\\Dev\\repos\\makercraft\\start.bat".to_string(),
                    is_running: false,
                    ping_ms: None,
                },
                FleetProjectEntry {
                    name: "sportscraft".to_string(),
                    folder_name: "sportscraft".to_string(),
                    port: 14131,
                    description: "Biometric Performance & Computer Vision Kinetic Coach"
                        .to_string(),
                    category: "Health / Kinetics".to_string(),
                    start_bat_path: "D:\\Dev\\repos\\sportscraft\\start.bat".to_string(),
                    is_running: false,
                    ping_ms: None,
                },
                FleetProjectEntry {
                    name: "moneycraft".to_string(),
                    folder_name: "moneycraft".to_string(),
                    port: 14116,
                    description: "Double-Entry Ledger & Autonomous Portfolio Analyzer".to_string(),
                    category: "Finance".to_string(),
                    start_bat_path: "D:\\Dev\\repos\\moneycraft\\start.bat".to_string(),
                    is_running: false,
                    ping_ms: None,
                },
                FleetProjectEntry {
                    name: "democraft-mcp".to_string(),
                    folder_name: "democraft-mcp".to_string(),
                    port: 14111,
                    description: "Fleet Showcase, Print Artworks & A2 Print Poster Hub".to_string(),
                    category: "Creative / Demo".to_string(),
                    start_bat_path: "D:\\Dev\\repos\\democraft-mcp\\start.bat".to_string(),
                    is_running: false,
                    ping_ms: None,
                },
            ],
        }
    }

    /// Actively probes ports for each fleet craft app via quick non-blocking TCP socket check.
    pub fn probe_ports(&mut self) {
        for project in &mut self.projects {
            let addr = SocketAddr::from(([127, 0, 0, 1], project.port));
            let start = Instant::now();
            if let Ok(_stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(15)) {
                project.is_running = true;
                project.ping_ms = Some(start.elapsed().as_millis().max(1) as u64);
            } else {
                // If it's codercraft itself, it's always running
                if project.name == "codercraft" {
                    project.is_running = true;
                    project.ping_ms = Some(1);
                } else {
                    project.is_running = false;
                    project.ping_ms = None;
                }
            }
        }
    }

    pub fn launch_project(folder_name: &str) -> (i32, String) {
        let bat_path = format!("D:\\Dev\\repos\\{folder_name}\\start.bat");
        if !Path::new(&bat_path).exists() {
            return (-1, format!("Start script not found: {bat_path}"));
        }

        let cmd = format!("Start-Process -FilePath '{bat_path}' -WindowStyle Minimized");
        let (code, stdout, stderr) = CoderCraftCore::execute_shell(&cmd, None);
        if code == 0 {
            (0, format!("Launched {folder_name} successfully"))
        } else {
            (code, format!("Failed to launch: {stderr} {stdout}"))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", content = "params")]
pub enum Command {
    Status,
    OpenFile {
        path: String,
    },
    ReadFileRange {
        path: String,
        start_line: Option<usize>,
        end_line: Option<usize>,
    },
    WriteFile {
        path: String,
        content: String,
    },
    ApplyHunkEdit {
        path: String,
        target: String,
        replacement: String,
    },
    GrepSearch {
        root: String,
        query: String,
    },
    ExtractSymbols {
        path: String,
    },
    IndexWorkspace {
        workspace_dir: String,
    },
    DiscoverInstructions {
        workspace_dir: String,
    },
    RunTestSuite {
        test_filter: Option<String>,
    },
    ScanWorkspaceTree {
        root_path: String,
        max_depth: Option<usize>,
    },
    SelectLanguage {
        language: SupportedLanguage,
    },
    RunLinter {
        workspace_dir: String,
        auto_fix: bool,
    },
    ConfigureMcpServer {
        entry: McpServerEntry,
    },
    ListMcpServers,
    ExecuteConsoleCommand {
        command_line: String,
        working_dir: Option<String>,
    },
    ConfigureLlm {
        provider: Option<String>,
        model: Option<String>,
        endpoint: Option<String>,
        api_key: Option<String>,
    },
    LlmStatus,
    LlmListModels {
        provider: Option<String>,
    },
    LlmSelect {
        provider: String,
        model: String,
    },
    LlmSetKey {
        provider: String,
        api_key: String,
    },
    LlmEvict {
        keep: Option<String>,
    },
    LlmTest {
        prompt: Option<String>,
    },
    AgentRefactor {
        intent: String,
        target_file: String,
    },
    AgentPlan {
        goal: String,
        workspace_dir: String,
    },
    ListFleetProjects,
    ProbeFleetProjects,
    LaunchFleetProject {
        folder_name: String,
    },
    GetSwarmState,
    StepSwarm,
    GetHelp,
    GetComparison,
    ApplyPatch {
        workspace_dir: String,
        patch_text: String,
    },
    GlobSearch {
        root: String,
        pattern: String,
    },
    CheckPermission {
        tool: String,
        target: String,
    },
    McpListTools {
        server: String,
    },
    McpCallTool {
        server: String,
        tool: String,
        args_json: Option<serde_json::Value>,
    },
    McpRemoveServer {
        name: String,
    },
    SessionCreate {
        session_id: Option<String>,
        workspace_dir: String,
        mode: String,
        title: String,
    },
    SessionAppend {
        session_id: String,
        role: String,
        content: String,
        tool_calls_json: Option<String>,
    },
    SessionList {
        workspace_filter: Option<String>,
        since: Option<String>,
        until: Option<String>,
        limit: Option<usize>,
    },
    SessionSearch {
        query: String,
        top_k: Option<usize>,
    },
    SessionRecall {
        year: i64,
        month: u32,
        limit: Option<usize>,
    },
    SessionRead {
        session_id: String,
        limit: Option<usize>,
    },
    AgentRun {
        goal: String,
        workspace_dir: String,
        session_id: Option<String>,
        max_steps: Option<usize>,
        mode: Option<String>,
        timeout_s: Option<u64>,
    },
    PomeradoStatus {
        root: String,
        node_version_output: Option<String>,
    },
    PomeradoMint {
        name: String,
        url: String,
        task: String,
        authority: String,
        root: String,
        example_input: Option<serde_json::Value>,
        authentication_origins: Option<Vec<String>>,
    },
    PomeradoJob {
        job_id: String,
    },
    PomeradoProvideInput {
        job_id: String,
        input: serde_json::Value,
    },
    PomeradoCancel {
        job_id: String,
    },
    PomeradoList {
        root: String,
    },
    PomeradoRegister {
        root: String,
        name: String,
        mcp_json_text: Option<String>,
    },
}

pub struct CoderCraftCore {
    pub active_language: SupportedLanguage,
    pub mcp_servers: Vec<McpServerEntry>,
    pub llm_config: CraftAiConfig,
    pub fleet_depot: FleetDepot,
    pub swarm_ledger: SwarmLedger,
}

impl Default for CoderCraftCore {
    fn default() -> Self {
        Self::new()
    }
}

impl CoderCraftCore {
    #[must_use]
    pub fn new() -> Self {
        Self {
            active_language: SupportedLanguage::Rust,
            mcp_servers: mcp_client::load_servers(),
            llm_config: CraftAiConfig::load(),
            fleet_depot: FleetDepot::new(),
            swarm_ledger: SwarmLedger::new(),
        }
    }

    #[must_use]
    pub fn extract_ast_symbols(source: &str, _lang: &SupportedLanguage) -> Vec<AstSymbol> {
        let mut symbols = Vec::new();
        for (idx, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            let line_number = idx + 1;

            if trimmed.starts_with("pub fn ")
                || trimmed.starts_with("fn ")
                || trimmed.starts_with("def ")
            {
                let name = trimmed
                    .split_whitespace()
                    .nth(if trimmed.starts_with("pub") { 2 } else { 1 })
                    .and_then(|s| s.split('(').next())
                    .unwrap_or("unknown_fn")
                    .to_string();
                symbols.push(AstSymbol {
                    name,
                    kind: "fn".to_string(),
                    line_number,
                });
            } else if trimmed.starts_with("pub struct ")
                || trimmed.starts_with("struct ")
                || trimmed.starts_with("class ")
            {
                let name = trimmed
                    .split_whitespace()
                    .nth(if trimmed.starts_with("pub") { 2 } else { 1 })
                    .and_then(|s| s.split('{').next())
                    .unwrap_or("unknown_struct")
                    .trim()
                    .to_string();
                symbols.push(AstSymbol {
                    name,
                    kind: "struct".to_string(),
                    line_number,
                });
            } else if trimmed.starts_with("pub enum ") || trimmed.starts_with("enum ") {
                let name = trimmed
                    .split_whitespace()
                    .nth(if trimmed.starts_with("pub") { 2 } else { 1 })
                    .and_then(|s| s.split('{').next())
                    .unwrap_or("unknown_enum")
                    .trim()
                    .to_string();
                symbols.push(AstSymbol {
                    name,
                    kind: "enum".to_string(),
                    line_number,
                });
            } else if trimmed.starts_with("pub trait ") || trimmed.starts_with("trait ") {
                let name = trimmed
                    .split_whitespace()
                    .nth(if trimmed.starts_with("pub") { 2 } else { 1 })
                    .and_then(|s| s.split('{').next())
                    .unwrap_or("unknown_trait")
                    .trim()
                    .to_string();
                symbols.push(AstSymbol {
                    name,
                    kind: "trait".to_string(),
                    line_number,
                });
            } else if trimmed.starts_with("impl ") {
                let name = trimmed
                    .trim_start_matches("impl ")
                    .split('{')
                    .next()
                    .unwrap_or("impl")
                    .trim()
                    .to_string();
                symbols.push(AstSymbol {
                    name,
                    kind: "impl".to_string(),
                    line_number,
                });
            }
        }
        symbols
    }

    #[must_use]
    pub fn scan_tree(root: &Path, depth: usize, max_depth: usize) -> FileTreeNode {
        let name = root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("root")
            .to_string();
        let path_str = root.to_string_lossy().to_string();

        if !root.is_dir() {
            let metadata = std::fs::metadata(root);
            let size = metadata.map(|m| m.len()).unwrap_or(0);
            return FileTreeNode {
                name,
                path: path_str,
                is_dir: false,
                size_bytes: size,
                children: vec![],
            };
        }

        let mut children = Vec::new();
        if depth < max_depth {
            if let Ok(entries) = std::fs::read_dir(root) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    let file_name = entry.file_name();
                    let name_str = file_name.to_string_lossy();
                    // Skip bulky folders
                    if name_str.starts_with('.')
                        || name_str == "target"
                        || name_str == "node_modules"
                    {
                        continue;
                    }
                    children.push(Self::scan_tree(&entry_path, depth + 1, max_depth));
                }
            }
        }

        FileTreeNode {
            name,
            path: path_str,
            is_dir: true,
            size_bytes: 0,
            children,
        }
    }

    #[must_use]
    pub fn get_linter_for(lang: &SupportedLanguage) -> LinterConfig {
        match lang {
            SupportedLanguage::Rust => LinterConfig {
                language: SupportedLanguage::Rust,
                command: "cargo".to_string(),
                default_args: vec!["clippy".to_string(), "--all-targets".to_string()],
                auto_fix_args: vec![
                    "clippy".to_string(),
                    "--fix".to_string(),
                    "--allow-no-vcs".to_string(),
                ],
            },
            SupportedLanguage::Python => LinterConfig {
                language: SupportedLanguage::Python,
                command: "ruff".to_string(),
                default_args: vec!["check".to_string(), ".".to_string()],
                auto_fix_args: vec!["check".to_string(), ".".to_string(), "--fix".to_string()],
            },
            SupportedLanguage::TypeScript => LinterConfig {
                language: SupportedLanguage::TypeScript,
                command: "biome".to_string(),
                default_args: vec!["check".to_string(), ".".to_string()],
                auto_fix_args: vec!["check".to_string(), ".".to_string(), "--write".to_string()],
            },
            SupportedLanguage::Go => LinterConfig {
                language: SupportedLanguage::Go,
                command: "golangci-lint".to_string(),
                default_args: vec!["run".to_string()],
                auto_fix_args: vec!["run".to_string(), "--fix".to_string()],
            },
            SupportedLanguage::Cpp => LinterConfig {
                language: SupportedLanguage::Cpp,
                command: "clang-tidy".to_string(),
                default_args: vec!["-p=build".to_string()],
                auto_fix_args: vec!["-p=build".to_string(), "--fix".to_string()],
            },
        }
    }

    #[must_use]
    pub fn discover_instructions(workspace_dir: &str) -> WorkspaceInstructions {
        let root = PathBuf::from(workspace_dir);
        let agents_path = root.join("AGENTS.md");
        let claude_path = root.join("CLAUDE.md");
        let cursor_path = root.join(".cursorrules");

        WorkspaceInstructions {
            agents_md: std::fs::read_to_string(agents_path).ok(),
            claude_md: std::fs::read_to_string(claude_path).ok(),
            cursor_rules: std::fs::read_to_string(cursor_path).ok(),
        }
    }

    pub fn execute_shell(cmd_line: &str, working_dir: Option<&str>) -> (i32, String, String) {
        let mut cmd = if cfg!(windows) {
            let mut c = std::process::Command::new("powershell.exe");
            c.args(["-NoProfile", "-NonInteractive", "-Command", cmd_line]);
            c
        } else {
            let mut c = std::process::Command::new("sh");
            c.args(["-c", cmd_line]);
            c
        };

        if let Some(wd) = working_dir {
            cmd.current_dir(wd);
        }

        match cmd.output() {
            Ok(out) => {
                let code = out.status.code().unwrap_or(-1);
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                (code, stdout, stderr)
            }
            Err(e) => (-1, String::new(), format!("Failed to spawn shell: {e}")),
        }
    }

    pub fn read_file_range(
        path: &str,
        start: Option<usize>,
        end: Option<usize>,
    ) -> Result<String, String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read {path}: {e}"))?;
        let lines: Vec<&str> = content.lines().collect();
        let s = start.unwrap_or(1).saturating_sub(1);
        let e = end.unwrap_or(lines.len()).min(lines.len());
        if s >= lines.len() {
            return Ok(String::new());
        }
        Ok(lines[s..e].join("\n"))
    }

    pub fn write_file(path: &str, content: &str) -> Result<(), String> {
        if let Some(parent) = Path::new(path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(path, content).map_err(|e| format!("Failed to write {path}: {e}"))
    }

    pub fn apply_hunk_edit(path: &str, target: &str, replacement: &str) -> Result<(), String> {
        let content =
            std::fs::read_to_string(path).map_err(|e| format!("Failed to read {path}: {e}"))?;
        if !content.contains(target) {
            return Err(format!("Target block not found in {path}"));
        }
        let updated = content.replace(target, replacement);
        std::fs::write(path, updated).map_err(|e| format!("Failed to write {path}: {e}"))
    }

    #[must_use]
    pub fn grep_search(root: &str, query: &str) -> Vec<(String, usize, String)> {
        let mut results = Vec::new();
        let path = Path::new(root);
        let _ = Self::grep_recursive(path, query, &mut results, 0, 50);
        results
    }

    fn grep_recursive(
        dir: &Path,
        query: &str,
        results: &mut Vec<(String, usize, String)>,
        depth: usize,
        max_matches: usize,
    ) -> std::io::Result<()> {
        if depth > 5 || results.len() >= max_matches {
            return Ok(());
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || name == "target" || name == "node_modules" {
                    continue;
                }
                if p.is_dir() {
                    let _ = Self::grep_recursive(&p, query, results, depth + 1, max_matches);
                } else if p.is_file() {
                    if let Ok(content) = std::fs::read_to_string(&p) {
                        for (idx, line) in content.lines().enumerate() {
                            if line.contains(query) {
                                results.push((
                                    p.to_string_lossy().to_string(),
                                    idx + 1,
                                    line.to_string(),
                                ));
                                if results.len() >= max_matches {
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

/// Redacted LLM view for API responses: everything except secret values.
/// Keys are presence flags only — a key must never round-trip through JSON.
fn redacted_llm_view(config: &CraftAiConfig) -> Value {
    let kind = craft_ai_core::providers::resolve_provider(&config.provider);
    let effective_endpoint = kind
        .map(|k| craft_ai_core::providers::default_endpoint(config, k))
        .unwrap_or_default();
    let present = |k: craft_ai_core::ProviderKind| {
        config
            .get_api_key(k)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    };
    json!({
        "provider": config.provider,
        "provider_known": kind.is_some(),
        "model": config.model,
        "ollama_endpoint": config.ollama_endpoint,
        "lmstudio_endpoint": config.lmstudio_endpoint,
        "endpoint_override": config.endpoint_override,
        "custom_endpoint": config.custom_endpoint,
        "effective_endpoint": effective_endpoint,
        "keys_present": {
            "openai": present(craft_ai_core::ProviderKind::OpenAi),
            "anthropic": present(craft_ai_core::ProviderKind::Anthropic),
            "deepseek": present(craft_ai_core::ProviderKind::DeepSeek),
            "meta": present(craft_ai_core::ProviderKind::Meta),
            "openrouter": present(craft_ai_core::ProviderKind::OpenRouter),
            "custom": present(craft_ai_core::ProviderKind::Custom),
        }
    })
}

/// Store an API key in the matching provider slot. Local providers have no
/// slot: storing a key for them is refused instead of silently dropped.
fn store_api_key(
    config: &mut CraftAiConfig,
    provider: &str,
    api_key: &str,
) -> Result<String, String> {
    let kind = craft_ai_core::providers::resolve_provider(provider)
        .ok_or_else(|| format!("unknown provider '{provider}'"))?;
    match kind {
        craft_ai_core::ProviderKind::Ollama | craft_ai_core::ProviderKind::LMStudio => {
            Err(format!(
                "{} needs no API key",
                craft_ai_core::providers::provider_id(kind)
            ))
        }
        craft_ai_core::ProviderKind::OpenAi => {
            config.openai_api_key = api_key.to_string();
            Ok("openai".to_string())
        }
        craft_ai_core::ProviderKind::Anthropic => {
            config.anthropic_api_key = api_key.to_string();
            Ok("anthropic".to_string())
        }
        craft_ai_core::ProviderKind::DeepSeek => {
            config.deepseek_api_key = api_key.to_string();
            Ok("deepseek".to_string())
        }
        craft_ai_core::ProviderKind::Meta => {
            config.meta_api_key = api_key.to_string();
            Ok("meta".to_string())
        }
        craft_ai_core::ProviderKind::OpenRouter => {
            config.openrouter_api_key = api_key.to_string();
            Ok("openrouter".to_string())
        }
        craft_ai_core::ProviderKind::Custom => {
            config.custom_api_key = api_key.to_string();
            Ok("custom".to_string())
        }
    }
}

pub async fn dispatch(cmd: Command, store: &DatabaseStore) -> Value {
    let mut core = CoderCraftCore::new();
    // Eternal memory tables exist from here on; a corrupt sessions.db must
    // never break the file tools, so schema errors are tolerated outside
    // the session commands themselves.
    let _ = sessions::ensure_schema(store);

    match cmd {
        Command::Status => {
            json!({
                "app": "codercraft",
                "version": APP_VERSION,
                "control_port": CONTROL_PORT,
                "status": "ready",
                "editor_engine": "Native_wgpu_60fps",
                "active_language": core.active_language,
                "mcp_servers_registered": core.mcp_servers.len(),
                "llm_provider": core.llm_config.provider,
                "llm_model": core.llm_config.model,
                "fleet_projects_indexed": core.fleet_depot.projects.len(),
                "swarm_workers_active": core.swarm_ledger.workers.len(),
                "capabilities": [
                    "OpenCodeBackbone", "ClaudeDesktopParity", "McpServerRegistry",
                    "IntegratedBashConsole", "AstTreeSitter", "StigmergicSwarm",
                    "QuadDisplayViewports", "FleetProjectDepot", "BezierNodeGraph",
                    "MultiBufferEditor", "AstSymbolOutline", "AutonomousGoalLoop",
                    "EternalSessionMemory", "PermissionGate", "MultiFilePatch",
                    "GitignoreAwareGlob"
                ],
                "success": true
            })
        }
        Command::OpenFile { path } => {
            let exists = Path::new(&path).exists();
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            json!({
                "success": exists,
                "path": path,
                "language": if path.ends_with(".rs") { "rust" } else if path.ends_with(".py") { "python" } else { "text" },
                "size_bytes": size,
                "status": if exists { "BUFFER_MAPPED" } else { "FILE_NOT_FOUND" }
            })
        }
        Command::ReadFileRange {
            path,
            start_line,
            end_line,
        } => match CoderCraftCore::read_file_range(&path, start_line, end_line) {
            Ok(content) => json!({ "success": true, "path": path, "content": content }),
            Err(e) => json!({ "success": false, "error": e }),
        },
        Command::WriteFile { path, content } => {
            let v = permissions::check_tool("write", &path);
            if v.decision == "deny" {
                json!({ "success": false, "permission": v.decision, "reason": v.reason, "path": path })
            } else {
                match CoderCraftCore::write_file(&path, &content) {
                    Ok(()) => {
                        json!({ "success": true, "path": path, "status": "SAVED", "permission": v.decision, "reason": v.reason })
                    }
                    Err(e) => json!({ "success": false, "error": e, "permission": v.decision }),
                }
            }
        }
        Command::ApplyHunkEdit {
            path,
            target,
            replacement,
        } => {
            let v = permissions::check_tool("edit", &path);
            if v.decision == "deny" {
                json!({ "success": false, "permission": v.decision, "reason": v.reason, "path": path })
            } else {
                match CoderCraftCore::apply_hunk_edit(&path, &target, &replacement) {
                    Ok(()) => {
                        json!({ "success": true, "path": path, "status": "HUNK_APPLIED", "permission": v.decision })
                    }
                    Err(e) => json!({ "success": false, "error": e, "permission": v.decision }),
                }
            }
        }
        Command::GrepSearch { root, query } => {
            let matches = CoderCraftCore::grep_search(&root, &query);
            json!({
                "success": true,
                "query": query,
                "match_count": matches.len(),
                "matches": matches.into_iter().map(|(p, line, text)| {
                    json!({ "path": p, "line": line, "text": text })
                }).collect::<Vec<_>>()
            })
        }
        Command::ExtractSymbols { path } => match std::fs::read_to_string(&path) {
            Ok(source) => {
                let symbols = CoderCraftCore::extract_ast_symbols(&source, &core.active_language);
                json!({ "success": true, "path": path, "symbols": symbols })
            }
            Err(e) => json!({ "success": false, "error": format!("Cannot read file: {e}") }),
        },
        Command::IndexWorkspace { workspace_dir } => {
            let tree = CoderCraftCore::scan_tree(Path::new(&workspace_dir), 0, 3);
            let instructions = CoderCraftCore::discover_instructions(&workspace_dir);
            json!({
                "success": true,
                "workspace": workspace_dir,
                "file_count": tree.children.len(),
                "instructions_found": {
                    "agents_md": instructions.agents_md.is_some(),
                    "claude_md": instructions.claude_md.is_some(),
                    "cursorrules": instructions.cursor_rules.is_some(),
                },
                "status": "INDEXED"
            })
        }
        Command::DiscoverInstructions { workspace_dir } => {
            let instructions = CoderCraftCore::discover_instructions(&workspace_dir);
            json!({
                "success": true,
                "workspace": workspace_dir,
                "instructions": instructions
            })
        }
        Command::ScanWorkspaceTree {
            root_path,
            max_depth,
        } => {
            let depth = max_depth.unwrap_or(2);
            let tree = CoderCraftCore::scan_tree(Path::new(&root_path), 0, depth);
            json!({
                "success": true,
                "tree": tree
            })
        }
        Command::SelectLanguage { language } => {
            core.active_language = language;
            json!({
                "success": true,
                "language": core.active_language
            })
        }
        Command::RunLinter {
            workspace_dir,
            auto_fix,
        } => {
            let linter = CoderCraftCore::get_linter_for(&core.active_language);
            let args = if auto_fix {
                &linter.auto_fix_args
            } else {
                &linter.default_args
            };
            let cmd_str = format!("{} {}", linter.command, args.join(" "));
            let (code, stdout, stderr) =
                CoderCraftCore::execute_shell(&cmd_str, Some(&workspace_dir));
            json!({
                "success": code == 0,
                "exit_code": code,
                "stdout": stdout,
                "stderr": stderr,
                "command": cmd_str,
                "linter": linter.command
            })
        }
        Command::RunTestSuite { test_filter } => {
            let filter_arg = test_filter.unwrap_or_default();
            let cmd_str = format!("cargo test {filter_arg}");
            let (code, stdout, stderr) = CoderCraftCore::execute_shell(&cmd_str, None);
            json!({
                "success": code == 0,
                "exit_code": code,
                "stdout": stdout,
                "stderr": stderr,
                "command": cmd_str
            })
        }
        Command::ConfigureMcpServer { entry } => {
            let mut servers = mcp_client::load_servers();
            if let Some(slot) = servers
                .iter_mut()
                .find(|s| s.name.eq_ignore_ascii_case(&entry.name))
            {
                *slot = entry.clone();
            } else {
                servers.push(entry.clone());
            }
            match mcp_client::save_servers(&servers) {
                Ok(()) => {
                    core.mcp_servers = servers.clone();
                    json!({
                        "success": true,
                        "server": entry,
                        "total_servers": servers.len()
                    })
                }
                Err(e) => json!({ "success": false, "error": e }),
            }
        }
        Command::ListMcpServers => {
            let servers = mcp_client::load_servers();
            core.mcp_servers = servers.clone();
            json!({
                "success": true,
                "servers": servers
            })
        }
        Command::McpRemoveServer { name } => {
            let mut servers = mcp_client::load_servers();
            let before = servers.len();
            servers.retain(|s| !s.name.eq_ignore_ascii_case(&name));
            if servers.len() == before {
                json!({ "success": false, "error": format!("unknown server '{name}'") })
            } else {
                match mcp_client::save_servers(&servers) {
                    Ok(()) => {
                        core.mcp_servers = servers.clone();
                        json!({ "success": true, "removed": name, "total_servers": servers.len() })
                    }
                    Err(e) => json!({ "success": false, "error": e }),
                }
            }
        }
        Command::McpListTools { server } => {
            match mcp_client::find_server(&server) {
                Some(entry) => match mcp_client::list_tools_live(&entry).await {
                    Ok(result) => json!({ "success": true, "server": entry.name, "result": result }),
                    Err(e) => json!({ "success": false, "server": entry.name, "error": e }),
                },
                None => json!({ "success": false, "error": format!("unknown server '{server}'") }),
            }
        }
        Command::McpCallTool {
            server,
            tool,
            args_json,
        } => {
            match mcp_client::find_server(&server) {
                Some(entry) => {
                    let args = args_json.unwrap_or(json!({}));
                    match mcp_client::call_tool_live(&entry, &tool, args).await {
                        Ok(result) => json!({ "success": true, "server": entry.name, "tool": tool, "result": result }),
                        Err(e) => json!({ "success": false, "server": entry.name, "tool": tool, "error": e }),
                    }
                }
                None => json!({ "success": false, "error": format!("unknown server '{server}'") }),
            }
        }
        Command::PomeradoStatus {
            root,
            node_version_output,
        } => {
            let key_present = std::env::var("OPENAI_API_KEY")
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false);
            let probe =
                pomerado::probe_environment(node_version_output.as_deref(), key_present, &root);
            json!({ "success": true, "probe": probe })
        }
        Command::PomeradoMint {
            name,
            url,
            task,
            authority,
            root,
            example_input,
            authentication_origins,
        } => {
            let auth = pomerado::PomeradoAuthority::parse_authority(&authority)
                .unwrap_or(pomerado::PomeradoAuthority::Read);
            let req = pomerado::PomeradoMintRequest {
                name: name.clone(),
                url: url.clone(),
                task: task.clone(),
                authority: auth,
                root: root.clone(),
                example_input: example_input.clone(),
                authentication_origins: authentication_origins.unwrap_or_default(),
            };
            match pomerado::validate_mint(&req) {
                Err(e) => json!({ "success": false, "error": e }),
                Ok(()) => {
                    let cmd_line = pomerado::mint_command(&req);
                    // Write authority mutates the target site: default to ask.
                    let tool = if auth == pomerado::PomeradoAuthority::Write {
                        "pomerado-write"
                    } else {
                        "shell"
                    };
                    let v = permissions::check_tool(tool, &cmd_line);
                    json!({
                        "success": v.decision != "deny",
                        "command_line": cmd_line,
                        "authority": auth.as_str(),
                        "integration_dir": pomerado::integration_dir(&root, &name).to_string_lossy().to_string(),
                        "permission": v.decision,
                        "permission_reason": v.reason,
                        "note": "run via codercraft_shell; poll with PomeradoJob; answer with PomeradoProvideInput"
                    })
                }
            }
        }
        Command::PomeradoJob { job_id } => {
            if job_id.trim().is_empty() {
                json!({ "success": false, "error": "empty job_id" })
            } else {
                json!({
                    "success": true,
                    "command_line": pomerado::get_job_command(&job_id),
                    "job_id": job_id
                })
            }
        }
        Command::PomeradoProvideInput { job_id, input } => {
            if job_id.trim().is_empty() {
                json!({ "success": false, "error": "empty job_id" })
            } else {
                json!({
                    "success": true,
                    "command_line": pomerado::provide_input_command(&job_id, &input),
                    "job_id": job_id
                })
            }
        }
        Command::PomeradoCancel { job_id } => {
            if job_id.trim().is_empty() {
                json!({ "success": false, "error": "empty job_id" })
            } else {
                json!({
                    "success": true,
                    "command_line": pomerado::cancel_job_command(&job_id),
                    "job_id": job_id
                })
            }
        }
        Command::PomeradoList { root } => {
            let base = std::path::Path::new(&root);
            if !base.is_absolute() {
                json!({ "success": false, "error": "root must be absolute" })
            } else {
                let mut names: Vec<String> = Vec::new();
                if let Ok(rd) = std::fs::read_dir(base) {
                    for e in rd.flatten() {
                        let p = e.path();
                        if p.is_dir() && p.join("pomerado.json").exists() {
                            if let Some(n) = p.file_name().and_then(|s| s.to_str()) {
                                names.push(n.to_string());
                            }
                        }
                    }
                }
                names.sort();
                json!({ "success": true, "root": root, "integrations": names })
            }
        }
        Command::PomeradoRegister {
            root,
            name,
            mcp_json_text,
        } => {
            let req = pomerado::PomeradoMintRequest {
                name: name.clone(),
                url: "https://example.com".to_string(),
                task: "register".to_string(),
                authority: pomerado::PomeradoAuthority::Read,
                root: root.clone(),
                example_input: None,
                authentication_origins: vec![],
            };
            // Reuse name+root validation only (url/task are placeholders here).
            if !pomerado::is_valid_integration_name(&req.name) {
                json!({ "success": false, "error": "invalid name" })
            } else if !std::path::Path::new(&req.root).is_absolute() {
                json!({ "success": false, "error": "root must be absolute" })
            } else {
                let entry =
                    pomerado::integration_to_server_entry(&root, &name, mcp_json_text.as_deref());
                let mut servers = mcp_client::load_servers();
                if let Some(slot) = servers
                    .iter_mut()
                    .find(|s| s.name.eq_ignore_ascii_case(&entry.name))
                {
                    *slot = entry.clone();
                } else {
                    servers.push(entry.clone());
                }
                match mcp_client::save_servers(&servers) {
                    Ok(()) => {
                        core.mcp_servers = servers.clone();
                        json!({ "success": true, "server": entry, "total_servers": servers.len() })
                    }
                    Err(e) => json!({ "success": false, "error": e }),
                }
            }
        }
        Command::ExecuteConsoleCommand {
            command_line,
            working_dir,
        } => {
            let v = permissions::check_tool("shell", &command_line);
            if v.decision == "deny" {
                json!({ "success": false, "permission": v.decision, "reason": v.reason })
            } else {
                let (code, stdout, stderr) =
                    CoderCraftCore::execute_shell(&command_line, working_dir.as_deref());
                json!({
                    "success": code == 0,
                    "exit_code": code,
                    "stdout": stdout,
                    "stderr": stderr,
                    "permission": v.decision,
                    "permission_reason": v.reason
                })
            }
        }
        Command::ConfigureLlm {
            provider,
            model,
            endpoint,
            api_key,
        } => {
            let mut cfg = CraftAiConfig::load();
            if let Some(p) = provider {
                if p.trim().is_empty() {
                    return json!({ "success": false, "error": "empty provider" });
                }
                if craft_ai_core::providers::resolve_provider(&p).is_none() {
                    return json!({ "success": false, "error": format!("unknown provider '{p}'") });
                }
                cfg.provider = p;
            }
            if let Some(m) = model {
                cfg.model = m;
            }
            if let Some(e) = endpoint {
                cfg.endpoint_override = e;
            }
            if let Some(k) = api_key {
                if k.trim().is_empty() {
                    return json!({ "success": false, "error": "empty api_key" });
                }
                let target = cfg.provider.clone();
                if let Err(e) = store_api_key(&mut cfg, &target, &k) {
                    return json!({ "success": false, "error": e });
                }
            }
            match cfg.save() {
                Ok(()) => {
                    core.llm_config = cfg.clone();
                    json!({ "success": true, "config": redacted_llm_view(&cfg) })
                }
                Err(e) => json!({ "success": false, "error": e.to_string() }),
            }
        }
        Command::LlmStatus => {
            let cfg = CraftAiConfig::load();
            let kind = craft_ai_core::providers::resolve_provider(&cfg.provider);
            let probe = kind.map(|k| craft_ai_core::providers::probe(&cfg, k));
            json!({
                "success": true,
                "config": redacted_llm_view(&cfg),
                "probe": probe,
            })
        }
        Command::LlmListModels { provider } => {
            let cfg = CraftAiConfig::load();
            let name = provider.as_deref().unwrap_or(&cfg.provider).to_string();
            let kind = match craft_ai_core::providers::resolve_provider(&name) {
                Some(k) => k,
                None => {
                    return json!({ "success": false, "error": format!("unknown provider '{name}'") })
                }
            };
            match craft_ai_core::providers::list_models(&cfg, kind) {
                Ok(models) => json!({
                    "success": true,
                    "provider": craft_ai_core::providers::provider_id(kind),
                    "count": models.len(),
                    "models": models,
                }),
                Err(e) => json!({ "success": false, "provider": name, "error": e.to_string() }),
            }
        }
        Command::LlmSelect { provider, model } => {
            let kind = match craft_ai_core::providers::resolve_provider(&provider) {
                Some(k) => k,
                None => {
                    return json!({ "success": false, "error": format!("unknown provider '{provider}'") })
                }
            };
            if model.trim().is_empty() {
                return json!({ "success": false, "error": "empty model (nothing auto-selected)" });
            }
            let mut cfg = CraftAiConfig::load();
            cfg.provider = craft_ai_core::providers::provider_id(kind).to_string();
            cfg.model = model.clone();
            if let Err(e) = cfg.save() {
                return json!({ "success": false, "error": e.to_string() });
            }
            core.llm_config = cfg.clone();
            // Saving a selection switches VRAM, not just config (Ollama only;
            // other providers are stateless from our side).
            let switch = if kind == craft_ai_core::ProviderKind::Ollama {
                Some(craft_ai_core::providers::switch_ollama_model(&cfg, &model))
            } else {
                None
            };
            json!({
                "success": true,
                "config": redacted_llm_view(&cfg),
                "switch": switch,
            })
        }
        Command::LlmSetKey { provider, api_key } => {
            let mut cfg = CraftAiConfig::load();
            if api_key.trim().is_empty() {
                return json!({ "success": false, "error": "empty api_key" });
            }
            match store_api_key(&mut cfg, &provider, &api_key) {
                Ok(slot) => match cfg.save() {
                    Ok(()) => {
                        core.llm_config = cfg.clone();
                        json!({ "success": true, "slot": slot, "config": redacted_llm_view(&cfg) })
                    }
                    Err(e) => json!({ "success": false, "error": e.to_string() }),
                },
                Err(e) => json!({ "success": false, "error": e }),
            }
        }
        Command::LlmEvict { keep } => {
            let cfg = CraftAiConfig::load();
            let report =
                craft_ai_core::providers::switch_ollama_model(&cfg, keep.as_deref().unwrap_or(""));
            json!({ "success": true, "evicted": report.evicted, "warmed": report.warmed, "engine": report.engine })
        }
        Command::LlmTest { prompt } => {
            let cfg = CraftAiConfig::load();
            let client = craft_ai_core::CraftAiClient::with_config(cfg);
            let probe_text = prompt.unwrap_or_else(|| "Reply with exactly: OK".to_string());
            let reply = tokio::time::timeout(
                std::time::Duration::from_secs(90),
                client.complete("You are a connectivity probe. Reply briefly.", &probe_text),
            )
            .await;
            match reply {
                Ok(Ok(text)) => {
                    let short: String = text.chars().take(300).collect();
                    json!({ "success": true, "reply": short })
                }
                Ok(Err(e)) => json!({ "success": false, "error": e.to_string() }),
                Err(_) => json!({ "success": false, "error": "probe timed out after 90s" }),
            }
        }
        Command::AgentRefactor {
            intent,
            target_file,
        } => {
            let proposal = AgentEngine::synthesize_refactor(&intent, &target_file);
            json!({
                "success": true,
                "proposal": proposal
            })
        }
        Command::AgentPlan {
            goal,
            workspace_dir,
        } => {
            let mut session = AgentSession::new("session-plan", AgentMode::Plan, &workspace_dir);
            let plan_text = session.generate_step_plan(&goal);
            json!({
                "success": true,
                "mode": "Plan",
                "goal": goal,
                "plan": plan_text
            })
        }
        Command::ListFleetProjects => {
            json!({
                "success": true,
                "total_projects": core.fleet_depot.projects.len(),
                "projects": core.fleet_depot.projects
            })
        }
        Command::ProbeFleetProjects => {
            core.fleet_depot.probe_ports();
            json!({
                "success": true,
                "total_projects": core.fleet_depot.projects.len(),
                "projects": core.fleet_depot.projects
            })
        }
        Command::LaunchFleetProject { folder_name } => {
            let (code, msg) = FleetDepot::launch_project(&folder_name);
            json!({
                "success": code == 0,
                "exit_code": code,
                "message": msg
            })
        }
        Command::GetSwarmState => {
            json!({
                "success": true,
                "workers": core.swarm_ledger.workers,
                "tasks": core.swarm_ledger.tasks,
                "total_tokens": core.swarm_ledger.total_swarm_tokens,
                "pass_rate": core.swarm_ledger.compiler_pass_rate
            })
        }
        Command::StepSwarm => {
            core.swarm_ledger.step_simulation();
            json!({
                "success": true,
                "tasks": core.swarm_ledger.tasks
            })
        }
        Command::ApplyPatch {
            workspace_dir,
            patch_text,
        } => {
            let v = permissions::check_tool("patch", &workspace_dir);
            if v.decision == "deny" {
                json!({ "success": false, "permission": v.decision, "reason": v.reason })
            } else {
                match patch::apply_patch_text(&workspace_dir, &patch_text) {
                    Ok(files) => {
                        json!({ "success": true, "workspace": workspace_dir, "applied": files, "permission": v.decision })
                    }
                    Err(e) => json!({ "success": false, "error": e, "permission": v.decision }),
                }
            }
        }
        Command::GlobSearch { root, pattern } => {
            let _ = permissions::check_tool("glob", &root);
            let files = patch::glob_files(&root, &pattern);
            json!({ "success": true, "root": root, "pattern": pattern, "match_count": files.len(), "files": files })
        }
        Command::CheckPermission { tool, target } => {
            let v = permissions::check_tool(&tool, &target);
            json!({ "success": true, "tool": tool, "target": target, "decision": v.decision, "reason": v.reason })
        }
        Command::SessionCreate {
            session_id,
            workspace_dir,
            mode,
            title,
        } => {
            match sessions::create_session(
                store,
                session_id.as_deref().unwrap_or(""),
                &workspace_dir,
                &mode,
                &title,
            ) {
                Ok(id) => {
                    json!({ "success": true, "session_id": id, "workspace": workspace_dir, "mode": mode, "title": title })
                }
                Err(e) => json!({ "success": false, "error": e.to_string() }),
            }
        }
        Command::SessionAppend {
            session_id,
            role,
            content,
            tool_calls_json,
        } => {
            match sessions::append_turn(
                store,
                &session_id,
                &role,
                &content,
                tool_calls_json.as_deref().unwrap_or("{}"),
            ) {
                Ok(turn_id) => {
                    json!({ "success": true, "session_id": session_id, "turn_id": turn_id })
                }
                Err(e) => json!({ "success": false, "error": e.to_string() }),
            }
        }
        Command::SessionList {
            workspace_filter,
            since,
            until,
            limit,
        } => {
            match sessions::list_sessions(
                store,
                workspace_filter.as_deref(),
                since.as_deref(),
                until.as_deref(),
                limit.unwrap_or(50),
            ) {
                Ok(items) => json!({ "success": true, "count": items.len(), "sessions": items }),
                Err(e) => json!({ "success": false, "error": e.to_string() }),
            }
        }
        Command::SessionSearch { query, top_k } => {
            match sessions::search(store, &query, top_k.unwrap_or(10)) {
                Ok(hits) => {
                    json!({ "success": true, "query": query, "count": hits.len(), "hits": hits })
                }
                Err(e) => json!({ "success": false, "error": e.to_string() }),
            }
        }
        Command::SessionRecall { year, month, limit } => {
            match sessions::recall_month(store, year, month, limit.unwrap_or(100)) {
                Ok(turns) => {
                    json!({ "success": true, "year": year, "month": month, "count": turns.len(), "turns": turns })
                }
                Err(e) => json!({ "success": false, "error": e.to_string() }),
            }
        }
        Command::SessionRead { session_id, limit } => {
            match sessions::read_session(store, &session_id, limit.unwrap_or(200)) {
                Ok(turns) => {
                    json!({ "success": true, "session_id": session_id, "count": turns.len(), "turns": turns })
                }
                Err(e) => json!({ "success": false, "error": e.to_string() }),
            }
        }
        Command::AgentRun {
            goal,
            workspace_dir,
            session_id,
            max_steps,
            mode,
            timeout_s,
        } => {
            autonomy::run_goal(
                store,
                autonomy::RunRequest {
                    workspace_dir,
                    goal,
                    session_id,
                    max_steps,
                    mode: mode.unwrap_or_else(|| "build".to_string()),
                    timeout_s: timeout_s.unwrap_or_else(autonomy::default_timeout_s),
                    brain: None,
                },
            )
            .await
        }
        Command::GetHelp => {
            json!({
                "success": true,
                "app": "codercraft",
                "version": APP_VERSION,
                "port": CONTROL_PORT,
                "summary": "CoderCraft is a 100% native Rust local-first agentic IDE & MCP server built with egui 0.37.",
                "capabilities": [
                    "Zero-Electron native binary (~35MB RAM, <30ms startup, 144 FPS)",
                    "Dual Plan/Build agent loop with instruction discovery (AGENTS.md, CLAUDE.md)",
                    "Interactive ComfyUI-style Bézier node graph swarm supervisor on Display 3",
                    "Multi-Display Quad Layout architecture (Displays 1, 2, 3, 4)",
                    "Fleet Project Depot with active non-blocking TCP socket probing for 12 Craft apps",
                    "Multi-Buffer Code Studio with AST symbol outline extraction and jump navigation",
                    "FastMCP 2024-11-05 JSON-RPC stdio server and client",
                    "Zero-panic containment (#![deny(clippy::unwrap_used)])"
                ]
            })
        }
        Command::GetComparison => {
            json!({
                "success": true,
                "matrix": {
                    "claude_desktop": {
                        "runtime": "Electron / Node.js (~800MB-2GB RAM)",
                        "startup_ms": 3000,
                        "multi_display": "Single window",
                        "mcp": "Client only",
                        "sovereignty": "Cloud-only (Anthropic API)",
                        "fleet_control": "None"
                    },
                    "opencode": {
                        "runtime": "Node.js / TypeScript / Ink (~600MB-1.5GB RAM)",
                        "startup_ms": 2000,
                        "multi_display": "Single window / terminal",
                        "mcp": "Client & Server (TypeScript)",
                        "sovereignty": "Cloud + local Ollama",
                        "fleet_control": "None"
                    },
                    "antigravity_agy": {
                        "runtime": "Python / Node CLI harness + Webview",
                        "startup_ms": 1500,
                        "multi_display": "Single workspace window",
                        "mcp": "Client & Server (Python/Node)",
                        "sovereignty": "Cloud-driven",
                        "fleet_control": "Cross-repo workspace navigation"
                    },
                    "codercraft": {
                        "runtime": "100% Native Rust (egui 0.37 + wgpu) (~35MB RAM)",
                        "startup_ms": 25,
                        "multi_display": "Native Quad-Display Viewports (Displays 1, 2, 3, 4)",
                        "mcp": "FastMCP 2024-11-05 JSON-RPC Server & Client (Rust)",
                        "sovereignty": "100% Sovereign Offline-First (craft-ai-core)",
                        "fleet_control": "Built-in Fleet Depot with live TCP socket probing (12 Craft apps)",
                        "swarm_graph": "Interactive Bézier Spaghetti Node Graph (ComfyUI / MaterialCraft style)"
                    }
                }
            })
        }
    }
}
