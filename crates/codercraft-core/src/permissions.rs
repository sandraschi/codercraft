//! CoderCraft permission gate (OpenCode-parity P0).
//!
//! Every mutating dispatch goes through `check` first. Decisions:
//! `allow` (run), `ask` (run but flag for human review in non-interactive
//! CLI/MCP), `deny` (refuse). Mirrors OpenCode's `permission` semantics at
//! a fraction of the surface: exact tool + granular shell patterns.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    Allow,
    Ask,
    Deny,
}

impl Decision {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Decision::Allow => "allow",
            Decision::Ask => "ask",
            Decision::Deny => "deny",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionVerdict {
    pub decision: String,
    pub reason: String,
}

fn verdict(d: Decision, reason: &str) -> PermissionVerdict {
    PermissionVerdict {
        decision: d.as_str().to_string(),
        reason: reason.to_string(),
    }
}

fn lower(s: &str) -> String {
    s.to_lowercase()
}

/// Destructive shell patterns. Matched case-insensitively as substrings.
fn is_destructive_shell(cmd_line: &str) -> bool {
    let c = lower(cmd_line);
    const PATTERNS: &[&str] = &[
        "rm -rf /",
        "rm -rf ~",
        "rd /s /q c:\\",
        "format c:",
        "mkfs",
        ":(){:|:&};:",
        "del /f /s /q c:\\windows",
        "takeown /f c:\\windows",
    ];
    PATTERNS.iter().any(|p| c.contains(p))
}

/// Risky-but-legit patterns that need a human nod.
fn needs_review_shell(cmd_line: &str) -> bool {
    let c = lower(cmd_line);
    const PATTERNS: &[&str] = &[
        "git push --force",
        "git push -f",
        "chmod -r 777",
        "icacls",
        "curl.exe",
        "invoke-webrequest",
        "irm ",
    ];
    PATTERNS.iter().any(|p| c.contains(p))
}

fn is_env_path(path: &str) -> bool {
    let c = lower(path);
    c.ends_with(".env")
        || c.ends_with(".env.local")
        || c.contains(".env.")
        || c.contains("id_rsa")
        || c.contains("id_ed25519")
        || c.contains("secrets")
}

fn has_parent_escape(path: &str) -> bool {
    path.split(['/', '\\']).any(|seg| seg == "..")
}

/// Check a mutating operation described by (tool, path_or_command).
/// Read-only tools always allow; callers pass their own tool name.
pub fn check_tool(tool: &str, target: &str) -> PermissionVerdict {
    if has_parent_escape(target) {
        return verdict(
            Decision::Deny,
            "path escapes workspace (..); use a path inside the workspace",
        );
    }
    match tool {
        "read" | "grep" | "glob" | "index" | "plan" | "status" | "symbols" | "session"
        | "swarm" | "depot" => verdict(Decision::Allow, "read-only tool"),
        "shell" => {
            if is_destructive_shell(target) {
                verdict(Decision::Deny, "destructive shell pattern blocked")
            } else if needs_review_shell(target) {
                verdict(
                    Decision::Ask,
                    "risky shell pattern; review before running unattended",
                )
            } else {
                verdict(Decision::Allow, "shell command within policy")
            }
        }
        "write" | "edit" | "patch" => {
            if is_env_path(target) {
                verdict(
                    Decision::Ask,
                    "secrets-adjacent path (.env/keys); confirm before writing",
                )
            } else {
                verdict(Decision::Allow, "workspace write within policy")
            }
        }
        _ => verdict(Decision::Ask, "unknown tool; default to ask"),
    }
}
