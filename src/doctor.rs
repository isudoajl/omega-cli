//! Health diagnostics for OMEGA installations.
//!
//! Checks the current project for a complete, healthy OMEGA deployment
//! and reports pass/warn/fail status for each component.

use std::path::Path;
use std::process::Command;

use console::style;

use crate::assets;
use crate::version::binary_version;

/// Status of an individual health check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

/// A single diagnostic check result.
pub struct Check {
    pub name: String,
    pub status: CheckStatus,
    pub detail: String,
}

/// Overall installation health, derived from individual checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverallHealth {
    Healthy,
    Degraded,
    Broken,
}

/// Complete diagnostic report for a target project.
pub struct DiagnosticReport {
    pub checks: Vec<Check>,
    pub overall: OverallHealth,
}

impl Check {
    fn pass(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Pass,
            detail: detail.into(),
        }
    }
    fn warn(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Warn,
            detail: detail.into(),
        }
    }
    fn fail(name: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Fail,
            detail: detail.into(),
        }
    }
}

impl DiagnosticReport {
    /// Print a formatted, colored report to stdout.
    pub fn print(&self) {
        println!();
        println!(
            "{}",
            style("OMEGA Doctor -- Installation Health Check").bold()
        );
        println!("{}", style("=".repeat(48)).dim());
        println!();

        for check in &self.checks {
            let indicator = match check.status {
                CheckStatus::Pass => style("  [PASS]").green().bold(),
                CheckStatus::Warn => style("  [WARN]").yellow().bold(),
                CheckStatus::Fail => style("  [FAIL]").red().bold(),
            };
            println!("{} {}", indicator, check.name);
            if !check.detail.is_empty() {
                println!("         {}", style(&check.detail).dim());
            }
        }

        println!();
        println!("{}", style("-".repeat(48)).dim());

        let (pass, warn, fail) = self.counts();
        println!(
            "  {} passed, {} warnings, {} failures",
            style(pass).green(),
            style(warn).yellow(),
            style(fail).red(),
        );
        match self.overall {
            OverallHealth::Healthy => {
                println!("  Overall: {}", style("Healthy").green().bold());
            }
            OverallHealth::Degraded => {
                println!("  Overall: {}", style("Degraded").yellow().bold());
                println!("  Run {} to fix warnings.", style("omg update").cyan());
            }
            OverallHealth::Broken => {
                println!("  Overall: {}", style("Broken").red().bold());
                println!("  Run {} to deploy OMEGA.", style("omg init").cyan());
            }
        }
        println!();
    }

    fn counts(&self) -> (usize, usize, usize) {
        let (mut p, mut w, mut f) = (0, 0, 0);
        for c in &self.checks {
            match c.status {
                CheckStatus::Pass => p += 1,
                CheckStatus::Warn => w += 1,
                CheckStatus::Fail => f += 1,
            }
        }
        (p, w, f)
    }
}

/// Run all diagnostic checks against `target_dir` and return a report.
pub fn run_diagnostics(target_dir: &Path) -> DiagnosticReport {
    let checks = vec![
        check_git_repo(target_dir),
        check_claude_dir(target_dir),
        check_asset_group(
            target_dir,
            "Core agents",
            ".claude/agents",
            assets::core_agents(),
        ),
        check_asset_group(
            target_dir,
            "Core commands",
            ".claude/commands",
            assets::core_commands(),
        ),
        check_hooks(target_dir),
        check_settings_json(target_dir),
        check_memory_db(target_dir),
        check_claude_md(target_dir),
        check_file_exists(target_dir, "specs/SPECS.md"),
        check_file_exists(target_dir, "docs/DOCS.md"),
        check_sqlite3_cli(),
        check_version_stamp(target_dir),
    ];
    let overall = derive_overall(&checks);
    DiagnosticReport { checks, overall }
}

fn derive_overall(checks: &[Check]) -> OverallHealth {
    if checks.iter().any(|c| c.status == CheckStatus::Fail) {
        OverallHealth::Broken
    } else if checks.iter().any(|c| c.status == CheckStatus::Warn) {
        OverallHealth::Degraded
    } else {
        OverallHealth::Healthy
    }
}

// ---------------------------------------------------------------------------
// Individual checks
// ---------------------------------------------------------------------------

fn check_git_repo(dir: &Path) -> Check {
    if dir.join(".git").exists() {
        Check::pass("Git repository", "")
    } else {
        Check::fail("Git repository", "Not inside a git repository")
    }
}

fn check_claude_dir(dir: &Path) -> Check {
    if dir.join(".claude").is_dir() {
        Check::pass(".claude/ directory", "")
    } else {
        Check::fail(".claude/ directory", "Missing -- run omg init")
    }
}

/// Generic check for a group of deployed assets (agents or commands).
fn check_asset_group(
    target_dir: &Path,
    group_name: &str,
    rel_dir: &str,
    expected: &[assets::Asset],
) -> Check {
    let dir = target_dir.join(rel_dir);
    let total = expected.len();

    if !dir.is_dir() {
        return Check::fail(
            format!("{} ({} expected)", group_name, total),
            format!("{} directory missing", rel_dir),
        );
    }

    let mut missing: Vec<&str> = Vec::new();
    let mut outdated: Vec<&str> = Vec::new();
    let mut ok = 0usize;

    for asset in expected {
        let path = dir.join(asset.name);
        match std::fs::read_to_string(&path) {
            Ok(content) if content == asset.content => ok += 1,
            Ok(_) => outdated.push(asset.name),
            Err(_) => missing.push(asset.name),
        }
    }

    let label = format!("{} ({}/{})", group_name, ok, total);
    if !missing.is_empty() {
        Check::fail(label, format!("Missing: {}", missing.join(", ")))
    } else if !outdated.is_empty() {
        Check::warn(label, format!("Outdated: {}", outdated.join(", ")))
    } else {
        Check::pass(label, "")
    }
}

/// Check that all 5 hooks are deployed and executable.
fn check_hooks(target_dir: &Path) -> Check {
    let hooks_dir = target_dir.join(".claude/hooks");
    let expected = assets::core_hooks();
    let total = expected.len();

    if !hooks_dir.is_dir() {
        return Check::fail(
            format!("Hooks ({} expected)", total),
            ".claude/hooks/ directory missing",
        );
    }

    let mut missing: Vec<&str> = Vec::new();
    let mut not_exec: Vec<&str> = Vec::new();
    let mut outdated: Vec<&str> = Vec::new();
    let mut ok = 0usize;

    for asset in expected {
        let path = hooks_dir.join(asset.name);
        if !path.exists() {
            missing.push(asset.name);
        } else if !is_executable(&path) {
            not_exec.push(asset.name);
        } else {
            match std::fs::read_to_string(&path) {
                Ok(c) if c == asset.content => ok += 1,
                Ok(_) => outdated.push(asset.name),
                Err(_) => missing.push(asset.name),
            }
        }
    }

    let label = format!("Hooks ({}/{})", ok, total);
    if !missing.is_empty() {
        Check::fail(label, format!("Missing: {}", missing.join(", ")))
    } else if !not_exec.is_empty() || !outdated.is_empty() {
        let mut issues = Vec::new();
        if !not_exec.is_empty() {
            issues.push(format!("Not executable: {}", not_exec.join(", ")));
        }
        if !outdated.is_empty() {
            issues.push(format!("Outdated: {}", outdated.join(", ")));
        }
        Check::warn(label, issues.join("; "))
    } else {
        Check::pass(label, "")
    }
}

fn check_settings_json(target_dir: &Path) -> Check {
    let path = target_dir.join(".claude/settings.json");
    if !path.exists() {
        return Check::fail("settings.json", ".claude/settings.json missing");
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(val) if val.get("hooks").is_some() => Check::pass("settings.json", ""),
            Ok(_) => Check::warn("settings.json", "No hooks key configured"),
            Err(e) => Check::warn("settings.json", format!("Malformed JSON: {}", e)),
        },
        Err(e) => Check::fail("settings.json", format!("Cannot read: {}", e)),
    }
}

fn check_memory_db(target_dir: &Path) -> Check {
    let db_path = target_dir.join(".claude/memory.db");
    if !db_path.exists() {
        return Check::fail("memory.db", ".claude/memory.db missing");
    }
    let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY;
    match rusqlite::Connection::open_with_flags(&db_path, flags) {
        Ok(conn) => {
            let sql = "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'";
            match conn.query_row(sql, [], |row| row.get::<_, i64>(0)) {
                Ok(n) if n >= 13 => Check::pass("memory.db", format!("{} tables found", n)),
                Ok(n) if n > 0 => {
                    Check::warn("memory.db", format!("Only {} tables (expected 13+)", n))
                }
                Ok(_) => Check::fail("memory.db", "No tables found -- DB may be empty"),
                Err(e) => Check::fail("memory.db", format!("Query failed: {}", e)),
            }
        }
        Err(e) => Check::fail("memory.db", format!("Cannot open: {}", e)),
    }
}

fn check_claude_md(target_dir: &Path) -> Check {
    let path = target_dir.join("CLAUDE.md");
    if !path.exists() {
        return Check::fail("CLAUDE.md", "CLAUDE.md missing");
    }
    match std::fs::read_to_string(&path) {
        Ok(content) if content.contains("# OMEGA") => Check::pass("CLAUDE.md", ""),
        Ok(_) => Check::warn("CLAUDE.md", "Missing OMEGA workflow rules section"),
        Err(e) => Check::fail("CLAUDE.md", format!("Cannot read: {}", e)),
    }
}

/// Generic existence check for scaffold files (specs/SPECS.md, docs/DOCS.md).
fn check_file_exists(target_dir: &Path, rel_path: &str) -> Check {
    if target_dir.join(rel_path).exists() {
        Check::pass(rel_path, "")
    } else {
        Check::fail(rel_path, "Missing -- run omg init")
    }
}

fn check_sqlite3_cli() -> Check {
    let available = Command::new("which")
        .arg("sqlite3")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if available {
        Check::pass("sqlite3 CLI", "")
    } else {
        Check::fail(
            "sqlite3 CLI",
            "sqlite3 not found in PATH -- agents require it",
        )
    }
}

fn check_version_stamp(target_dir: &Path) -> Check {
    let path = target_dir.join(".claude/.omg-version");
    if !path.exists() {
        return Check::fail("Version tracking", ".claude/.omg-version missing");
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(val) => {
                let deployed = val
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let current = binary_version();
                if deployed == current {
                    Check::pass("Version tracking", format!("v{}", current))
                } else {
                    Check::warn(
                        "Version tracking",
                        format!(
                            "Deployed v{}, binary v{} -- run omg update",
                            deployed, current
                        ),
                    )
                }
            }
            Err(e) => Check::warn("Version tracking", format!("Malformed version file: {}", e)),
        },
        Err(e) => Check::fail("Version tracking", format!("Cannot read: {}", e)),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.exists()
}
