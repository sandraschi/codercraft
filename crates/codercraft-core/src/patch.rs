//! Multi-file apply_patch + gitignore-aware glob (OpenCode-parity P0).
//!
//! Patch format (one or more sections, same markers as OpenCode):
//! ```text
//! *** Add File: src/new.rs
//! <full content>
//! *** Update File: src/old.rs
//! <full new content>
//! *** Delete File: src/gone.rs
//! *** Move to: src/renamed.rs        (renames previous section's path)
//! ```
//! For P0 Update carries the full new file content (no hunk math), which is
//! exactly what an LLM emits most reliably. Surgical single-hunk edits stay
//! on `ApplyHunkEdit`.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppliedFile {
    pub action: String,
    pub path: String,
}

#[derive(Debug, Clone)]
struct Section {
    action: String,
    path: String,
    move_to: Option<String>,
    body: String,
}

fn parse_patch(patch_text: &str) -> Result<Vec<Section>, String> {
    let mut sections: Vec<Section> = Vec::new();
    let mut current: Option<Section> = None;
    for line in patch_text.lines() {
        if let Some(rest) = line.strip_prefix("*** Add File:") {
            if let Some(prev) = current.take() {
                sections.push(prev);
            }
            current = Some(Section {
                action: "add".to_string(),
                path: rest.trim().to_string(),
                move_to: None,
                body: String::new(),
            });
        } else if let Some(rest) = line.strip_prefix("*** Update File:") {
            if let Some(prev) = current.take() {
                sections.push(prev);
            }
            current = Some(Section {
                action: "update".to_string(),
                path: rest.trim().to_string(),
                move_to: None,
                body: String::new(),
            });
        } else if let Some(rest) = line.strip_prefix("*** Delete File:") {
            if let Some(prev) = current.take() {
                sections.push(prev);
            }
            current = Some(Section {
                action: "delete".to_string(),
                path: rest.trim().to_string(),
                move_to: None,
                body: String::new(),
            });
        } else if let Some(rest) = line.strip_prefix("*** Move to:") {
            match current.as_mut() {
                Some(sec) => {
                    sec.move_to = Some(rest.trim().to_string());
                }
                None => {
                    return Err("*** Move to: without a preceding file section".to_string());
                }
            }
        } else if current.is_some() {
            if let Some(sec) = current.as_mut() {
                sec.body.push_str(line);
                sec.body.push('\n');
            }
        } else if line.trim().is_empty() {
            continue;
        } else {
            return Err(format!(
                "patch must start each section with a *** marker; got: '{line}'"
            ));
        }
    }
    if let Some(prev) = current.take() {
        sections.push(prev);
    }
    if sections.is_empty() {
        return Err("empty patch: no *** sections found".to_string());
    }
    Ok(sections)
}

/// Join workspace + rel and jail inside workspace (no `..` escapes,
// no absolute paths escaping the root).
fn jailed_path(workspace_dir: &str, rel: &str) -> Result<PathBuf, String> {
    let t = rel.trim().replace('\\', "/");
    if t.is_empty() {
        return Err("empty file path in patch".to_string());
    }
    let p = Path::new(&t);
    if p.is_absolute() {
        return Err(format!("absolute path not allowed in patch: '{rel}'"));
    }
    for comp in p.components() {
        if matches!(comp, Component::ParentDir) {
            return Err(format!("path escapes workspace: '{rel}'"));
        }
    }
    Ok(Path::new(workspace_dir).join(p))
}

pub fn apply_patch_text(
    workspace_dir: &str,
    patch_text: &str,
) -> Result<Vec<AppliedFile>, String> {
    let sections = parse_patch(patch_text)?;
    let mut applied = Vec::new();
    for sec in &sections {
        if let Some(dest) = &sec.move_to {
            let from = jailed_path(workspace_dir, &sec.path)?;
            let to = jailed_path(workspace_dir, dest)?;
            if !from.exists() {
                return Err(format!("move source not found: '{}'", sec.path));
            }
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("cannot create dir for '{dest}': {e}"))?;
            }
            // If the section also carries a body, write it to source first
            // (rename-with-content), else plain rename.
            if sec.action == "update" && !sec.body.trim().is_empty() {
                std::fs::write(&from, &sec.body)
                    .map_err(|e| format!("cannot write '{}': {e}", sec.path))?;
            }
            std::fs::rename(&from, &to)
                .map_err(|e| format!("cannot move '{}' -> '{dest}': {e}", sec.path))?;
            applied.push(AppliedFile {
                action: "move".to_string(),
                path: dest.clone(),
            });
            continue;
        }
        match sec.action.as_str() {
            "add" | "update" => {
                let target = jailed_path(workspace_dir, &sec.path)?;
                if sec.action == "add" && target.exists() {
                    return Err(format!("add target already exists: '{}'", sec.path));
                }
                if sec.action == "update" && !target.exists() {
                    return Err(format!("update target not found: '{}'", sec.path));
                }
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| format!("cannot create dir for '{}': {e}", sec.path))?;
                }
                std::fs::write(&target, &sec.body)
                    .map_err(|e| format!("cannot write '{}': {e}", sec.path))?;
                applied.push(AppliedFile {
                    action: sec.action.clone(),
                    path: sec.path.clone(),
                });
            }
            "delete" => {
                let target = jailed_path(workspace_dir, &sec.path)?;
                if !target.exists() {
                    return Err(format!("delete target not found: '{}'", sec.path));
                }
                std::fs::remove_file(&target)
                    .map_err(|e| format!("cannot delete '{}': {e}", sec.path))?;
                applied.push(AppliedFile {
                    action: "delete".to_string(),
                    path: sec.path.clone(),
                });
            }
            other => {
                return Err(format!("unknown patch action: '{other}'"));
            }
        }
    }
    Ok(applied)
}

// ---------------------------------------------------------------------------
// gitignore-aware glob (no new deps: hand-rolled *, **, ? matcher)
// ---------------------------------------------------------------------------

fn load_ignore(root: &Path) -> Vec<String> {
    let mut patterns: Vec<String> = Vec::new();
    let gi = root.join(".gitignore");
    if let Ok(text) = std::fs::read_to_string(gi) {
        for line in text.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') || t.starts_with('!') {
                continue;
            }
            let p = t.trim_end_matches('/').replace('\\', "/");
            if !p.is_empty() {
                patterns.push(p);
            }
        }
    }
    patterns
}

fn ignored(rel: &str, is_dir: bool, patterns: &[String]) -> bool {
    // Always-pruned build dirs unless the pattern explicitly wants them.
    const ALWAYS: &[&str] = &[".git/", "target/", "node_modules/", "dist/", "build/"];
    for a in ALWAYS {
        let dir_prefix = a.trim_end_matches('/');
        if rel == dir_prefix || rel.starts_with(*a) {
            return true;
        }
        let _ = is_dir;
    }
    for pat in patterns {
        if pat.contains('*') {
            // Suffix globs like *.log / *.bak
            if let Some(suf) = pat.strip_prefix('*') {
                if rel.ends_with(suf) {
                    return true;
                }
            }
        } else if rel == pat.as_str() || rel.starts_with(&format!("{pat}/")) {
            return true;
        } else if let Some(name) = rel.rsplit('/').next() {
            if name == pat {
                return true;
            }
        }
    }
    false
}

fn match_parts(pat: &[&str], path: &[&str]) -> bool {
    if pat.is_empty() {
        return path.is_empty();
    }
    if pat[0] == "**" {
        // ** matches zero or more segments.
        let mut i = 0;
        while i <= path.len() {
            if match_parts(&pat[1..], &path[i..]) {
                return true;
            }
            i += 1;
        }
        return false;
    }
    if path.is_empty() {
        return false;
    }
    if segment_match(pat[0], path[0]) {
        return match_parts(&pat[1..], &path[1..]);
    }
    false
}

fn segment_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let mut pi = 0;
    let mut ti = 0;
    let mut star: Option<usize> = None;
    let mut star_ti = 0;
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            star_ti = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            star_ti += 1;
            ti = star_ti;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

fn glob_match(pattern: &str, rel: &str) -> bool {
    let pat_parts: Vec<&str> = pattern.split('/').collect();
    let path_parts: Vec<&str> = rel.split('/').collect();
    match_parts(&pat_parts, &path_parts)
}

fn walk(
    dir: &Path,
    rel_prefix: &str,
    patterns: &[String],
    ignore: &[String],
    out: &mut Vec<String>,
    cap: usize,
) {
    if out.len() >= cap {
        return;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        if out.len() >= cap {
            return;
        }
        let p = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') && name != ".gitignore" {
            // Skip dotfiles/dirs (.git handled by ALWAYS, keep .gitignore readable)
            if p.is_dir() {
                continue;
            }
        }
        let rel = if rel_prefix.is_empty() {
            name.clone()
        } else {
            format!("{rel_prefix}/{name}")
        };
        let is_dir = p.is_dir();
        if ignored(&rel, is_dir, ignore) {
            continue;
        }
        if is_dir {
            walk(&p, &rel, patterns, ignore, out, cap);
        } else {
            for pat in patterns {
                if glob_match(pat, &rel) {
                    out.push(p.to_string_lossy().to_string());
                    break;
                }
            }
        }
    }
}

/// Glob files under root honoring .gitignore. `pattern` may be comma- or
/// `|`-separated for multiple patterns. Caps at 500 hits.
pub fn glob_files(root: &str, pattern: &str) -> Vec<String> {
    let root_p = Path::new(root);
    let pats: Vec<String> = pattern
        .split([',', '|'])
        .map(|s| s.trim().replace('\\', "/"))
        .map(|s| s.trim_start_matches("./").to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if pats.is_empty() || !root_p.is_dir() {
        return Vec::new();
    }
    let ignore = load_ignore(root_p);
    let mut out = Vec::new();
    walk(root_p, "", &pats, &ignore, &mut out, 500);
    out.sort();
    out
}
