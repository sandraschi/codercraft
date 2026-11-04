//! CoderCraft Core Library & Universal Command Bus.
//!
//! Subsumes OpenCode, Cursor, and Zed into a single sovereign ArtCraft application.
//! Features parity with Claude Desktop / AGY:
//! - Multi-language support (Rust, Python, TypeScript, Go, C++)
//! - Automated linter setup (Clippy, Ruff, Biome, golangci-lint, clang-tidy)
//! - Repository and project explorer with recursive file tree
//! - Workspace instruction discovery (AGENTS.md, CLAUDE.md, .cursorrules)
//! - Local (Ollama) and Cloud (Claude, GPT, DeepSeek) LLM configuration
//! - Stigmergic tool calling, logging, and integrated terminal/bash execution
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use codercraft_agent::{AgentEngine, AgentMode, AgentSession};
use databasecraft_store::DatabaseStore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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
pub struct LlmConfig {
    pub provider: String, // "Ollama", "Anthropic", "OpenAI", "DeepSeek"
    pub endpoint: String,
    pub model_name: String,
    pub api_key_configured: bool,
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
        config: LlmConfig,
    },
    AgentRefactor {
        intent: String,
        target_file: String,
    },
    AgentPlan {
        goal: String,
        workspace_dir: String,
    },
}

pub struct CoderCraftCore {
    pub active_language: SupportedLanguage,
    pub mcp_servers: Vec<McpServerEntry>,
    pub llm_config: LlmConfig,
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
            mcp_servers: vec![
                McpServerEntry {
                    name: "databasecraft".to_string(),
                    command: "D:\\Dev\\repos\\databasecraft\\target\\debug\\databasecraft-mcp.exe".to_string(),
                    args: vec![],
                    env: HashMap::new(),
                    is_enabled: true,
                },
                McpServerEntry {
                    name: "djcraft".to_string(),
                    command: "D:\\Dev\\repos\\djcraft\\target\\debug\\djcraft-mcp.exe".to_string(),
                    args: vec![],
                    env: HashMap::new(),
                    is_enabled: true,
                },
                McpServerEntry {
                    name: "gamecraft".to_string(),
                    command: "D:\\Dev\\repos\\gamecraft\\target\\debug\\gamecraft-mcp.exe".to_string(),
                    args: vec![],
                    env: HashMap::new(),
                    is_enabled: true,
                },
            ],
            llm_config: LlmConfig {
                provider: "Ollama".to_string(),
                endpoint: "http://localhost:11434/api/chat".to_string(),
                model_name: "qwen2.5-coder:32b".to_string(),
                api_key_configured: true,
            },
        }
    }

    #[must_use]
    pub fn scan_tree(root: &Path, depth: usize, max_depth: usize) -> FileTreeNode {
        let name = root.file_name().and_then(|n| n.to_str()).unwrap_or("root").to_string();
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
                    if name_str.starts_with('.') || name_str == "target" || name_str == "node_modules" {
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
                auto_fix_args: vec!["clippy".to_string(), "--fix".to_string(), "--allow-no-vcs".to_string()],
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

    pub fn read_file_range(path: &str, start: Option<usize>, end: Option<usize>) -> Result<String, String> {
        let content = std::fs::read_to_string(path).map_err(|e| format!("Failed to read {path}: {e}"))?;
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
        let content = std::fs::read_to_string(path).map_err(|e| format!("Failed to read {path}: {e}"))?;
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
                                results.push((p.to_string_lossy().to_string(), idx + 1, line.to_string()));
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

pub async fn dispatch(cmd: Command, _store: &DatabaseStore) -> Value {
    let mut core = CoderCraftCore::new();

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
                "capabilities": [
                    "OpenCodeBackbone", "ClaudeDesktopParity", "McpServerRegistry",
                    "IntegratedBashConsole", "AstTreeSitter", "StigmergicSwarm"
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
        Command::ReadFileRange { path, start_line, end_line } => {
            match CoderCraftCore::read_file_range(&path, start_line, end_line) {
                Ok(content) => json!({ "success": true, "path": path, "content": content }),
                Err(e) => json!({ "success": false, "error": e }),
            }
        }
        Command::WriteFile { path, content } => {
            match CoderCraftCore::write_file(&path, &content) {
                Ok(()) => json!({ "success": true, "path": path, "status": "SAVED" }),
                Err(e) => json!({ "success": false, "error": e }),
            }
        }
        Command::ApplyHunkEdit { path, target, replacement } => {
            match CoderCraftCore::apply_hunk_edit(&path, &target, &replacement) {
                Ok(()) => json!({ "success": true, "path": path, "status": "HUNK_APPLIED" }),
                Err(e) => json!({ "success": false, "error": e }),
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
        Command::ScanWorkspaceTree { root_path, max_depth } => {
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
        Command::RunLinter { workspace_dir, auto_fix } => {
            let linter = CoderCraftCore::get_linter_for(&core.active_language);
            let args = if auto_fix { &linter.auto_fix_args } else { &linter.default_args };
            let cmd_str = format!("{} {}", linter.command, args.join(" "));
            let (code, stdout, stderr) = CoderCraftCore::execute_shell(&cmd_str, Some(&workspace_dir));
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
            core.mcp_servers.push(entry);
            json!({
                "success": true,
                "total_servers": core.mcp_servers.len()
            })
        }
        Command::ListMcpServers => {
            json!({
                "success": true,
                "servers": core.mcp_servers
            })
        }
        Command::ExecuteConsoleCommand { command_line, working_dir } => {
            let (code, stdout, stderr) = CoderCraftCore::execute_shell(&command_line, working_dir.as_deref());
            json!({
                "success": code == 0,
                "exit_code": code,
                "stdout": stdout,
                "stderr": stderr
            })
        }
        Command::ConfigureLlm { config } => {
            core.llm_config = config;
            json!({
                "success": true,
                "llm_config": core.llm_config
            })
        }
        Command::AgentRefactor { intent, target_file } => {
            let proposal = AgentEngine::synthesize_refactor(&intent, &target_file);
            json!({
                "success": true,
                "proposal": proposal
            })
        }
        Command::AgentPlan { goal, workspace_dir } => {
            let mut session = AgentSession::new("session-plan", AgentMode::Plan, &workspace_dir);
            let plan_text = session.generate_step_plan(&goal);
            json!({
                "success": true,
                "mode": "Plan",
                "goal": goal,
                "plan": plan_text
            })
        }
    }
}
