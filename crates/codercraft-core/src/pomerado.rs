//! Pomerado website-to-MCP integration support.
//!
//! Pomerado mints deterministic MCP integrations from a natural-language task
//! (`mint` -> `get_job` -> `provide_input` / `cancel_job`). Each integration is
//! emitted as source under `<root>/<name>/` with `pomerado.json`,
//! `deployment.json`, and `mcp.json`, then served as its own stdio MCP server.
//!
//! This module is intentionally side-effect free: it validates requests,
//! builds the exact `pomerado-mcp` / `npx` command lines to run, probes the
//! host environment, and maps an emitted integration to a
//! [`crate::McpServerEntry`]. Execution stays in `codercraft_shell`
//! (permission-gated), registration stays in `dispatch`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Mint authority. `read` only looks, `write` may change the site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PomeradoAuthority {
    Read,
    Write,
}

impl PomeradoAuthority {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
        }
    }

    #[must_use]
    pub fn parse_authority(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "read" => Some(Self::Read),
            "write" => Some(Self::Write),
            _ => None,
        }
    }
}

/// Validated mint request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PomeradoMintRequest {
    pub name: String,
    pub url: String,
    pub task: String,
    pub authority: PomeradoAuthority,
    pub root: String,
    pub example_input: Option<serde_json::Value>,
    pub authentication_origins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PomeradoEnvProbe {
    pub os_supported: bool,
    pub os_name: String,
    pub node_found: bool,
    pub node_version: Option<String>,
    pub node_ok: bool,
    pub openai_key_present: bool,
    pub root_absolute: bool,
    pub hint: String,
}

/// Pomerado integration names: lowercase start, then lowercase/digits/underscore.
#[must_use]
pub fn is_valid_integration_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => (),
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

#[must_use]
pub fn is_valid_http_url(url: &str) -> bool {
    url.starts_with("https://") || url.starts_with("http://")
}

pub fn validate_mint(req: &PomeradoMintRequest) -> Result<(), String> {
    if !is_valid_integration_name(&req.name) {
        return Err("invalid name: start lowercase, only a-z 0-9 _, max 64 chars".to_string());
    }
    if !is_valid_http_url(&req.url) {
        return Err("invalid url: must start with http:// or https://".to_string());
    }
    if req.task.trim().is_empty() {
        return Err("invalid task: must be non-empty".to_string());
    }
    if req.task.len() > 4000 {
        return Err("invalid task: max 4000 chars".to_string());
    }
    let root = Path::new(&req.root);
    if !root.is_absolute() {
        return Err("invalid root: must be an absolute path".to_string());
    }
    if req.name.contains("..")
        || req.url.contains('\"')
        || req.root.contains('\"')
        || req.task.contains('\0')
    {
        return Err("invalid request: illegal characters".to_string());
    }
    for origin in &req.authentication_origins {
        if !is_valid_http_url(origin) {
            return Err(format!("invalid authentication origin: {origin}"));
        }
    }
    Ok(())
}

/// Quote one shell token with double quotes (Windows + POSIX safe enough for
/// generated commands shown to the user; execution goes via shell tool).
fn q(token: &str) -> String {
    format!("\"{}\"", token.replace('\"', "'"))
}

#[must_use]
pub fn integration_dir(root: &str, name: &str) -> PathBuf {
    Path::new(root).join(name)
}

#[must_use]
pub fn mint_command(req: &PomeradoMintRequest) -> String {
    // Served entry per Pomerado docs: `pomerado-mcp mint --root <abs>`.
    // npx form keeps zero global install; global installs can swap npx for pomerado-mcp.
    let mut parts = vec![
        "npx".to_string(),
        "-y".to_string(),
        "-p".to_string(),
        "pomerado".to_string(),
        "pomerado-mcp".to_string(),
        "mint".to_string(),
        "--root".to_string(),
        q(&req.root),
        "--name".to_string(),
        q(&req.name),
        "--url".to_string(),
        q(&req.url),
        "--task".to_string(),
        q(&req.task),
        "--authority".to_string(),
        req.authority.as_str().to_string(),
    ];
    for origin in &req.authentication_origins {
        parts.push("--authentication-origins".to_string());
        parts.push(q(origin));
    }
    if let Some(example) = &req.example_input {
        parts.push("--example-input".to_string());
        parts.push(q(&example.to_string()));
    }
    parts.join(" ")
}

#[must_use]
pub fn get_job_command(job_id: &str) -> String {
    format!(
        "npx -y -p pomerado pomerado-mcp get-job --job-id {}",
        q(job_id)
    )
}

#[must_use]
pub fn provide_input_command(job_id: &str, input: &serde_json::Value) -> String {
    format!(
        "npx -y -p pomerado pomerado-mcp provide-input --job-id {} --input {}",
        q(job_id),
        q(&input.to_string())
    )
}

#[must_use]
pub fn cancel_job_command(job_id: &str) -> String {
    format!(
        "npx -y -p pomerado pomerado-mcp cancel-job --job-id {}",
        q(job_id)
    )
}

#[must_use]
pub fn run_integration_command(root: &str, name: &str, input: &serde_json::Value) -> String {
    let dir = integration_dir(root, name);
    let dir_s = dir.to_string_lossy().to_string();
    format!(
        "npx -y -p pomerado pomerado-mcp run --dir {} --input {}",
        q(&dir_s),
        q(&input.to_string())
    )
}

/// Map an emitted integration to a CoderCraft MCP server entry.
/// Prefers the integration's own `mcp.json` when parsable, else falls back
/// to the canonical `node mcp.mjs` launcher documented by Pomerado.
pub fn integration_to_server_entry(
    root: &str,
    name: &str,
    mcp_json_text: Option<&str>,
) -> crate::McpServerEntry {
    let dir = integration_dir(root, name);
    let dir_s = dir.to_string_lossy().to_string();
    if let Some(text) = mcp_json_text {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(text) {
            let cmd = v
                .get("command")
                .and_then(|c| c.as_str())
                .unwrap_or("node")
                .to_string();
            let args = v
                .get("args")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_else(|| vec![format!("{dir_s}/mcp.mjs")]);
            return crate::McpServerEntry {
                name: format!("pomerado-{name}"),
                command: cmd,
                args,
                env: HashMap::new(),
                is_enabled: true,
            };
        }
    }
    crate::McpServerEntry {
        name: format!("pomerado-{name}"),
        command: "node".to_string(),
        args: vec![format!("{dir_s}/mcp.mjs")],
        is_enabled: true,
        env: HashMap::new(),
    }
}

/// Probe host readiness without executing anything.
/// `node_version_output` is the stdout of `node --version` when known.
#[must_use]
pub fn probe_environment(
    node_version_output: Option<&str>,
    openai_key_present: bool,
    root: &str,
) -> PomeradoEnvProbe {
    let os_name = std::env::consts::OS.to_string();
    // Pomerado docs list macOS + Linux. Windows hosts use WSL2 / container.
    let os_supported = os_name == "macos" || os_name == "linux";
    let (node_found, node_version, node_ok) = match node_version_output {
        None => (false, None, false),
        Some(raw) => {
            let v = raw.trim().trim_start_matches('v').to_string();
            let ok = is_node_ok(&v);
            (true, Some(v), ok)
        }
    };
    let root_absolute = Path::new(root).is_absolute();
    let hint = if !os_supported {
        "Windows host: run Pomerado minter in WSL2 or a Linux container; keep integrations under a shared absolute --root".to_string()
    } else if !node_found || !node_ok {
        "install Node 24.21+ and `npx -y -p pomerado playwright install chromium` (Linux: add --with-deps)".to_string()
    } else if !openai_key_present {
        "set OPENAI_API_KEY in the minter environment (runs are keyless; minting is not)"
            .to_string()
    } else if !root_absolute {
        "pass an absolute --root; every MCP client starts from its own cwd".to_string()
    } else {
        "ready: mint with authority read unless the task changes the site".to_string()
    };
    PomeradoEnvProbe {
        os_supported,
        os_name,
        node_found,
        node_version,
        node_ok,
        openai_key_present,
        root_absolute,
        hint,
    }
}

fn is_node_ok(version: &str) -> bool {
    // Accept Node 24.21+ on the 24 line, or any later major.
    let mut parts = version.split('.');
    let major: u32 = parts.next().unwrap_or("0").parse().unwrap_or(0);
    let minor: u32 = parts
        .next()
        .unwrap_or("0")
        .trim_matches(|c: char| !c.is_ascii_digit())
        .parse()
        .unwrap_or(0);
    if major > 24 {
        return true;
    }
    if major < 24 {
        return false;
    }
    minor >= 21
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_req() -> PomeradoMintRequest {
        PomeradoMintRequest {
            name: "example_reader".to_string(),
            url: "https://example.com".to_string(),
            task: "Read the main heading. Keep it read-only.".to_string(),
            authority: PomeradoAuthority::Read,
            root: if cfg!(windows) {
                "D:\\pomerado-integrations".to_string()
            } else {
                "/tmp/pomerado-integrations".to_string()
            },
            example_input: None,
            authentication_origins: vec![],
        }
    }

    #[test]
    fn valid_names_pass() {
        assert!(is_valid_integration_name("example_reader"));
        assert!(is_valid_integration_name("a1_b2"));
    }

    #[test]
    fn invalid_names_fail() {
        assert!(!is_valid_integration_name(""));
        assert!(!is_valid_integration_name("Example"));
        assert!(!is_valid_integration_name("1abc"));
        assert!(!is_valid_integration_name("has-dash"));
        assert!(!is_valid_integration_name("has space"));
    }

    #[test]
    fn mint_validation_ok() {
        assert!(validate_mint(&sample_req()).is_ok());
    }

    #[test]
    fn mint_rejects_relative_root() {
        let mut r = sample_req();
        r.root = "relative/path".to_string();
        assert!(validate_mint(&r).is_err());
    }

    #[test]
    fn mint_command_contains_authority() {
        let cmd = mint_command(&sample_req());
        assert!(cmd.contains("--authority"));
        assert!(cmd.contains("read"));
        assert!(cmd.contains("example_reader"));
    }

    #[test]
    fn node_gate() {
        assert!(is_node_ok("24.21.0"));
        assert!(is_node_ok("25.0.0"));
        assert!(!is_node_ok("22.14.0"));
        assert!(!is_node_ok("24.20.9"));
    }

    #[test]
    fn probe_windows_hint() {
        let p = probe_environment(Some("v24.21.0"), true, &sample_req().root);
        assert!(p.node_ok);
        if cfg!(windows) {
            assert!(!p.os_supported);
        }
    }
}
