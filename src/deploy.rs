//! Deploy engine for OMEGA CLI.
//!
//! Orchestrates the full deployment pipeline: writing agents, commands, hooks,
//! scaffolding, query files, CLAUDE.md injection, settings configuration,
//! database initialization, and version stamping to the target project.

use std::fs;
use std::path::{Path, PathBuf};

use console::style;

use crate::assets;
use crate::claude_md;
use crate::db;
use crate::settings;
use crate::version;

#[derive(thiserror::Error, Debug)]
pub enum DeployError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Deploy error: {0}")]
    #[allow(dead_code)]
    Other(String),
}

/// Summary of what the deploy engine did (or would do in dry-run mode).
#[derive(Debug, Default)]
pub struct DeployReport {
    pub new_files: Vec<String>,
    pub updated_files: Vec<String>,
    pub unchanged_files: Vec<String>,
    pub errors: Vec<String>,
}

impl DeployReport {
    /// Merge another report into this one.
    #[allow(dead_code)]
    fn merge(&mut self, other: DeployReport) {
        self.new_files.extend(other.new_files);
        self.updated_files.extend(other.updated_files);
        self.unchanged_files.extend(other.unchanged_files);
        self.errors.extend(other.errors);
    }

    /// Print a human-readable summary to stdout with colored output.
    pub fn print_summary(&self, verbose: bool) {
        for f in &self.new_files {
            println!("  {} {}", style("+").green().bold(), style(f).green());
        }
        for f in &self.updated_files {
            println!("  {} {}", style("~").yellow().bold(), style(f).yellow());
        }
        if verbose {
            for f in &self.unchanged_files {
                println!("  {} {}", style("=").dim(), style(f).dim());
            }
        }
        for e in &self.errors {
            println!("  {} {}", style("!").red().bold(), style(e).red());
        }
        println!();
        let total = self.new_files.len() + self.updated_files.len() + self.unchanged_files.len();
        println!(
            "  {} new, {} updated, {} unchanged ({} total)",
            style(self.new_files.len()).green().bold(),
            style(self.updated_files.len()).yellow().bold(),
            style(self.unchanged_files.len()).dim(),
            total,
        );
        if !self.errors.is_empty() {
            println!(
                "  {} {}",
                style(format!("{} error(s)", self.errors.len()))
                    .red()
                    .bold(),
                style("-- see above").red(),
            );
        }
    }
}

/// Options controlling deployment behavior.
pub struct DeployOptions {
    /// Extensions to install. Empty = core only, `["all"]` = every extension.
    pub extensions: Vec<String>,
    /// If true, skip SQLite database initialization.
    pub skip_db: bool,
    /// If true, show unchanged files in the report (used by print_summary).
    #[allow(dead_code)]
    pub verbose: bool,
    /// If true, don't write anything -- just report what would happen.
    pub dry_run: bool,
    /// If true, overwrite files even when content matches.
    pub force: bool,
}

/// Orchestrates the full OMEGA deployment to a target project directory.
pub struct DeployEngine {
    target_dir: PathBuf,
    options: DeployOptions,
}

impl DeployEngine {
    pub fn new(target_dir: PathBuf, options: DeployOptions) -> Self {
        Self {
            target_dir,
            options,
        }
    }

    /// Run the full deployment pipeline and return a report of actions taken.
    pub fn deploy(&self) -> Result<DeployReport, DeployError> {
        let mut report = DeployReport::default();
        self.deploy_agents(&mut report)?;
        self.deploy_commands(&mut report)?;
        self.deploy_hooks(&mut report)?;
        self.deploy_scaffolding(&mut report)?;
        self.deploy_query_files(&mut report)?;
        self.inject_workflow_rules(&mut report)?;
        self.configure_hooks_settings(&mut report)?;
        self.initialize_db(&mut report)?;
        self.write_version_stamp(&mut report)?;
        Ok(report)
    }

    /// Deploy core agent files plus agents from selected extensions.
    fn deploy_agents(&self, report: &mut DeployReport) -> Result<(), DeployError> {
        let dir = self.target_dir.join(".claude/agents");
        self.ensure_dir(&dir)?;
        for asset in assets::core_agents() {
            self.write_if_changed(&dir.join(asset.name), asset.content, report)?;
        }
        for ext in self.resolve_extensions() {
            for asset in ext.agents {
                self.write_if_changed(&dir.join(asset.name), asset.content, report)?;
            }
        }
        Ok(())
    }

    /// Deploy core command files plus commands from selected extensions.
    fn deploy_commands(&self, report: &mut DeployReport) -> Result<(), DeployError> {
        let dir = self.target_dir.join(".claude/commands");
        self.ensure_dir(&dir)?;
        for asset in assets::core_commands() {
            self.write_if_changed(&dir.join(asset.name), asset.content, report)?;
        }
        for ext in self.resolve_extensions() {
            for asset in ext.commands {
                self.write_if_changed(&dir.join(asset.name), asset.content, report)?;
            }
        }
        Ok(())
    }

    /// Deploy hook scripts and set executable permissions.
    fn deploy_hooks(&self, report: &mut DeployReport) -> Result<(), DeployError> {
        let dir = self.target_dir.join(".claude/hooks");
        self.ensure_dir(&dir)?;
        for asset in assets::core_hooks() {
            let dest = dir.join(asset.name);
            self.write_if_changed(&dest, asset.content, report)?;
            if !self.options.dry_run && dest.exists() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&dest, fs::Permissions::from_mode(0o755))?;
                }
            }
        }
        Ok(())
    }

    /// Create specs/, docs/ directories and SPECS.md/DOCS.md if missing.
    fn deploy_scaffolding(&self, report: &mut DeployReport) -> Result<(), DeployError> {
        let specs_dir = self.target_dir.join("specs");
        let docs_dir = self.target_dir.join("docs");
        self.ensure_dir(&specs_dir)?;
        self.ensure_dir(&docs_dir)?;

        let specs_md = specs_dir.join("SPECS.md");
        if !specs_md.exists() {
            self.write_if_changed(&specs_md, assets::scaffold_specs_md(), report)?;
        }
        let docs_md = docs_dir.join("DOCS.md");
        if !docs_md.exists() {
            self.write_if_changed(&docs_md, assets::scaffold_docs_md(), report)?;
        }
        Ok(())
    }

    /// Deploy SQL query reference files to `.claude/db-queries/`.
    fn deploy_query_files(&self, report: &mut DeployReport) -> Result<(), DeployError> {
        let dir = self.target_dir.join(".claude/db-queries");
        self.ensure_dir(&dir)?;
        for asset in assets::query_files() {
            self.write_if_changed(&dir.join(asset.name), asset.content, report)?;
        }
        Ok(())
    }

    /// Inject workflow rules into the target project's CLAUDE.md.
    fn inject_workflow_rules(&self, report: &mut DeployReport) -> Result<(), DeployError> {
        if self.options.dry_run {
            report.new_files.push("CLAUDE.md (workflow rules)".into());
            return Ok(());
        }
        match claude_md::inject_workflow_rules(&self.target_dir) {
            Ok(result) => {
                let label = "CLAUDE.md".to_string();
                match result {
                    claude_md::ClaudeMdResult::Created => report.new_files.push(label),
                    claude_md::ClaudeMdResult::Appended | claude_md::ClaudeMdResult::Updated => {
                        report.updated_files.push(label)
                    }
                    claude_md::ClaudeMdResult::Unchanged => report.unchanged_files.push(label),
                }
                Ok(())
            }
            Err(e) => {
                report
                    .errors
                    .push(format!("CLAUDE.md injection failed: {e}"));
                Ok(())
            }
        }
    }

    /// Configure hooks in `.claude/settings.json`.
    fn configure_hooks_settings(&self, report: &mut DeployReport) -> Result<(), DeployError> {
        if self.options.dry_run {
            report
                .new_files
                .push(".claude/settings.json (hooks)".into());
            return Ok(());
        }
        match settings::configure_hooks(&self.target_dir) {
            Ok(result) => {
                let label = ".claude/settings.json".to_string();
                match result {
                    settings::SettingsResult::Created => report.new_files.push(label),
                    settings::SettingsResult::Updated => report.updated_files.push(label),
                    settings::SettingsResult::Unchanged => report.unchanged_files.push(label),
                }
                Ok(())
            }
            Err(e) => {
                report
                    .errors
                    .push(format!("settings.json configuration failed: {e}"));
                Ok(())
            }
        }
    }

    /// Initialize the SQLite institutional memory database (unless --no-db).
    fn initialize_db(&self, report: &mut DeployReport) -> Result<(), DeployError> {
        if self.options.skip_db {
            return Ok(());
        }
        if self.options.dry_run {
            report.new_files.push(".claude/memory.db (SQLite)".into());
            return Ok(());
        }
        match db::initialize_db(&self.target_dir) {
            Ok(result) => {
                let label = ".claude/memory.db".to_string();
                match result {
                    db::DbResult::Created { .. } => report.new_files.push(label),
                    db::DbResult::Migrated { .. } => report.updated_files.push(label),
                    db::DbResult::AlreadyCurrent { .. } => report.unchanged_files.push(label),
                }
                Ok(())
            }
            Err(e) => {
                report
                    .errors
                    .push(format!("Database initialization failed: {e}"));
                Ok(())
            }
        }
    }

    /// Write the `.claude/.omg-version` stamp file.
    fn write_version_stamp(&self, report: &mut DeployReport) -> Result<(), DeployError> {
        if self.options.dry_run {
            report.new_files.push(".claude/.omg-version".into());
            return Ok(());
        }
        match version::write_version_stamp(&self.target_dir, &self.options.extensions) {
            Ok(()) => {
                report.new_files.push(".claude/.omg-version".into());
                Ok(())
            }
            Err(e) => {
                report.errors.push(format!("Version stamp failed: {e}"));
                Ok(())
            }
        }
    }

    // -- Utility methods ------------------------------------------------------

    /// Idempotent file writer. Only writes when content differs (or force/new).
    fn write_if_changed(
        &self,
        path: &Path,
        content: &str,
        report: &mut DeployReport,
    ) -> Result<(), DeployError> {
        let label = self.relative_display_path(path);
        if path.exists() {
            let existing = fs::read_to_string(path)?;
            if existing == content && !self.options.force {
                report.unchanged_files.push(label);
                return Ok(());
            }
            if !self.options.dry_run {
                fs::write(path, content)?;
            }
            report.updated_files.push(label);
        } else {
            if !self.options.dry_run {
                if let Some(parent) = path.parent() {
                    self.ensure_dir(parent)?;
                }
                fs::write(path, content)?;
            }
            report.new_files.push(label);
        }
        Ok(())
    }

    /// Resolve `--ext` into concrete extension references.
    fn resolve_extensions(&self) -> Vec<&'static assets::Extension> {
        if self.options.extensions.is_empty() {
            return Vec::new();
        }
        if self.options.extensions.len() == 1 && self.options.extensions[0] == "all" {
            return assets::extensions().iter().collect();
        }
        self.options
            .extensions
            .iter()
            .filter_map(|name| assets::extension_by_name(name))
            .collect()
    }

    /// Create a directory (and parents) if it doesn't exist. Skips in dry-run.
    fn ensure_dir(&self, path: &Path) -> Result<(), DeployError> {
        if !path.exists() && !self.options.dry_run {
            fs::create_dir_all(path)?;
        }
        Ok(())
    }

    /// Produce a display-friendly path relative to the target directory.
    fn relative_display_path(&self, path: &Path) -> String {
        path.strip_prefix(&self.target_dir)
            .unwrap_or(path)
            .display()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> DeployOptions {
        DeployOptions {
            extensions: Vec::new(),
            skip_db: true,
            verbose: false,
            dry_run: false,
            force: false,
        }
    }

    #[test]
    fn deploy_report_merge() {
        let mut a = DeployReport::default();
        a.new_files.push("a.md".into());
        let mut b = DeployReport::default();
        b.updated_files.push("b.md".into());
        b.errors.push("oops".into());
        a.merge(b);
        assert_eq!(a.new_files.len(), 1);
        assert_eq!(a.updated_files.len(), 1);
        assert_eq!(a.errors.len(), 1);
    }

    #[test]
    fn write_if_changed_creates_new_file() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = DeployEngine::new(tmp.path().to_path_buf(), opts());
        let mut report = DeployReport::default();
        let file = tmp.path().join("test.md");
        engine
            .write_if_changed(&file, "hello", &mut report)
            .unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "hello");
        assert_eq!(report.new_files.len(), 1);
    }

    #[test]
    fn write_if_changed_skips_identical() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = DeployEngine::new(tmp.path().to_path_buf(), opts());
        let mut report = DeployReport::default();
        let file = tmp.path().join("test.md");
        fs::write(&file, "hello").unwrap();
        engine
            .write_if_changed(&file, "hello", &mut report)
            .unwrap();
        assert_eq!(report.unchanged_files.len(), 1);
        assert!(report.new_files.is_empty());
        assert!(report.updated_files.is_empty());
    }

    #[test]
    fn write_if_changed_updates_different() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = DeployEngine::new(tmp.path().to_path_buf(), opts());
        let mut report = DeployReport::default();
        let file = tmp.path().join("test.md");
        fs::write(&file, "old").unwrap();
        engine.write_if_changed(&file, "new", &mut report).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "new");
        assert_eq!(report.updated_files.len(), 1);
    }

    #[test]
    fn write_if_changed_dry_run_does_not_write() {
        let tmp = tempfile::tempdir().unwrap();
        let o = DeployOptions {
            dry_run: true,
            ..opts()
        };
        let engine = DeployEngine::new(tmp.path().to_path_buf(), o);
        let mut report = DeployReport::default();
        let file = tmp.path().join("test.md");
        engine
            .write_if_changed(&file, "hello", &mut report)
            .unwrap();
        assert!(!file.exists());
        assert_eq!(report.new_files.len(), 1);
    }

    #[test]
    fn write_if_changed_force_overwrites_identical() {
        let tmp = tempfile::tempdir().unwrap();
        let o = DeployOptions {
            force: true,
            ..opts()
        };
        let engine = DeployEngine::new(tmp.path().to_path_buf(), o);
        let mut report = DeployReport::default();
        let file = tmp.path().join("test.md");
        fs::write(&file, "hello").unwrap();
        engine
            .write_if_changed(&file, "hello", &mut report)
            .unwrap();
        assert_eq!(report.updated_files.len(), 1);
        assert!(report.unchanged_files.is_empty());
    }

    #[test]
    fn resolve_extensions_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = DeployEngine::new(tmp.path().to_path_buf(), opts());
        assert!(engine.resolve_extensions().is_empty());
    }

    #[test]
    fn resolve_extensions_all() {
        let tmp = tempfile::tempdir().unwrap();
        let o = DeployOptions {
            extensions: vec!["all".into()],
            ..opts()
        };
        let engine = DeployEngine::new(tmp.path().to_path_buf(), o);
        assert_eq!(
            engine.resolve_extensions().len(),
            assets::extensions().len()
        );
    }

    #[test]
    fn resolve_extensions_by_name() {
        let tmp = tempfile::tempdir().unwrap();
        let o = DeployOptions {
            extensions: vec!["blockchain".into()],
            ..opts()
        };
        let engine = DeployEngine::new(tmp.path().to_path_buf(), o);
        let resolved = engine.resolve_extensions();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "blockchain");
    }

    #[test]
    fn resolve_extensions_unknown_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        let o = DeployOptions {
            extensions: vec!["nonexistent".into()],
            ..opts()
        };
        let engine = DeployEngine::new(tmp.path().to_path_buf(), o);
        assert!(engine.resolve_extensions().is_empty());
    }

    #[test]
    fn relative_display_path_strips_prefix() {
        let engine = DeployEngine::new(PathBuf::from("/project"), opts());
        let result = engine.relative_display_path(Path::new("/project/.claude/agents/foo.md"));
        assert_eq!(result, ".claude/agents/foo.md");
    }

    #[test]
    fn ensure_dir_creates_nested() {
        let tmp = tempfile::tempdir().unwrap();
        let engine = DeployEngine::new(tmp.path().to_path_buf(), opts());
        let nested = tmp.path().join("a/b/c");
        engine.ensure_dir(&nested).unwrap();
        assert!(nested.exists());
    }

    #[test]
    fn ensure_dir_dry_run_skips() {
        let tmp = tempfile::tempdir().unwrap();
        let o = DeployOptions {
            dry_run: true,
            ..opts()
        };
        let engine = DeployEngine::new(tmp.path().to_path_buf(), o);
        let nested = tmp.path().join("should_not_exist");
        engine.ensure_dir(&nested).unwrap();
        assert!(!nested.exists());
    }
}
