//! CLAUDE.md injector module.
//!
//! Handles creating, updating, or appending workflow rules to a target
//! project's CLAUDE.md file.  Preserves user content above the separator
//! and replaces only the workflow-rules section below it.

use std::fs;
use std::path::Path;

use crate::assets;

// ---------------------------------------------------------------------------
// Marker constants
// ---------------------------------------------------------------------------

/// Primary marker that begins the OMEGA workflow-rules section.
const PRIMARY_MARKER: &str = "# OMEGA \u{03A9}";

/// Legacy marker from the older "Quality Workflow" era.
const LEGACY_MARKER: &str = "# Claude Code Quality Workflow";

/// Horizontal-rule separator placed immediately before the marker.
const SEPARATOR: &str = "---";

// ---------------------------------------------------------------------------
// Result type
// ---------------------------------------------------------------------------

/// Describes what happened when injecting workflow rules.
#[derive(Debug, PartialEq, Eq)]
pub enum ClaudeMdResult {
    /// A brand-new CLAUDE.md was created.
    Created,
    /// Workflow rules were appended to an existing CLAUDE.md that had no marker.
    Appended,
    /// Existing workflow rules section was replaced with a newer version.
    Updated,
    /// The existing rules already matched the source -- no write performed.
    Unchanged,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Inject OMEGA workflow rules into the target project's CLAUDE.md.
///
/// Replicates the exact behaviour of the original `setup.sh` CLAUDE.md
/// section:
///
/// 1. **No CLAUDE.md** -- create one with header, placeholder, separator,
///    and the full workflow rules.
/// 2. **CLAUDE.md exists, no marker** -- append separator + rules.
/// 3. **CLAUDE.md exists, has primary marker** -- compare; replace if
///    different, otherwise return `Unchanged`.
/// 4. **CLAUDE.md exists, has legacy marker** -- always replace (upgrade).
pub fn inject_workflow_rules(target_dir: &Path) -> Result<ClaudeMdResult, std::io::Error> {
    let workflow_rules = assets::workflow_rules();
    let claude_md_path = target_dir.join("CLAUDE.md");

    if !claude_md_path.exists() {
        return create_new(&claude_md_path, workflow_rules);
    }

    let content = fs::read_to_string(&claude_md_path)?;

    // Determine which marker (if any) is present.
    if let Some(pos) = find_marker_pos(&content, PRIMARY_MARKER) {
        handle_existing_marker(&claude_md_path, &content, pos, workflow_rules, false)
    } else if let Some(pos) = find_marker_pos(&content, LEGACY_MARKER) {
        handle_existing_marker(&claude_md_path, &content, pos, workflow_rules, true)
    } else {
        append_rules(&claude_md_path, &content, workflow_rules)
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Create a brand-new CLAUDE.md with header + placeholder + rules.
fn create_new(path: &Path, workflow_rules: &str) -> Result<ClaudeMdResult, std::io::Error> {
    let content = format!(
        "# CLAUDE.md\n\
         \n\
         This file provides guidance to Claude Code (powered by OMEGA) \
         when working with code in this repository.\n\
         \n\
         ## Project-Specific Rules\n\
         \n\
         _(Add your project-specific rules here.)_\n\
         \n\
         ---\n\
         \n\
         {workflow_rules}"
    );
    fs::write(path, content)?;
    Ok(ClaudeMdResult::Created)
}

/// Append workflow rules to an existing CLAUDE.md that has no marker.
fn append_rules(
    path: &Path,
    existing: &str,
    workflow_rules: &str,
) -> Result<ClaudeMdResult, std::io::Error> {
    let mut out = String::with_capacity(existing.len() + workflow_rules.len() + 8);
    out.push_str(existing);
    // Ensure we start on a fresh line.
    if !existing.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("\n---\n\n");
    out.push_str(workflow_rules);
    fs::write(path, out)?;
    Ok(ClaudeMdResult::Appended)
}

/// Handle the case where a marker (primary or legacy) already exists.
///
/// When `is_legacy` is true we always rewrite (upgrade path).
fn handle_existing_marker(
    path: &Path,
    content: &str,
    marker_byte_pos: usize,
    workflow_rules: &str,
    is_legacy: bool,
) -> Result<ClaudeMdResult, std::io::Error> {
    // Extract existing rules (from marker line to EOF).
    let existing_rules = &content[marker_byte_pos..];

    // For the primary marker, check if an update is even needed.
    if !is_legacy && existing_rules == workflow_rules {
        return Ok(ClaudeMdResult::Unchanged);
    }

    // Build the "user section" -- everything above the marker, minus the
    // separator line and an optional blank line immediately before it.
    let user_section = strip_trailing_separator(&content[..marker_byte_pos]);

    let mut out = String::with_capacity(user_section.len() + workflow_rules.len() + 8);
    out.push_str(user_section);
    // Ensure we start on a fresh line.
    if !user_section.is_empty() && !user_section.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("\n---\n\n");
    out.push_str(workflow_rules);
    fs::write(path, out)?;
    Ok(ClaudeMdResult::Updated)
}

/// Find the byte position of the first line that starts with `marker`.
///
/// The marker must appear at the very start of a line (after a newline or
/// at position 0).
fn find_marker_pos(content: &str, marker: &str) -> Option<usize> {
    // Check if the file starts with the marker.
    if content.starts_with(marker) {
        return Some(0);
    }
    // Search for `\n<marker>` -- the marker at the start of any line.
    let needle = format!("\n{marker}");
    content.find(&needle).map(|pos| pos + 1)
}

/// Walk backwards from the cut-point to remove the `---` separator line and
/// an optional blank line before it, matching `setup.sh` behaviour.
///
/// Given text ending with `...content\n\n---\n\n`, returns everything up to
/// `...content\n`.
fn strip_trailing_separator(text: &str) -> &str {
    let trimmed = text.trim_end_matches('\n');

    // Check if the last non-empty line is exactly `---`.
    if let Some(last_nl) = trimmed.rfind('\n') {
        let last_line = &trimmed[last_nl + 1..];
        if last_line.trim() == SEPARATOR {
            let before_sep = &trimmed[..last_nl];
            // Check if there is a trailing blank line before the separator.
            let before_trimmed = before_sep.trim_end_matches('\n');
            if before_trimmed.len() < before_sep.len() {
                // At least one blank line existed -- remove it too.
                return if before_trimmed.is_empty() {
                    ""
                } else {
                    &text[..before_trimmed.len() + 1]
                };
            }
            // No blank line before `---`; just strip the separator.
            return if before_sep.is_empty() {
                ""
            } else {
                &text[..last_nl + 1]
            };
        }
    } else if trimmed.trim() == SEPARATOR {
        // The entire text (sans trailing newlines) is just `---`.
        return "";
    }

    // No separator found -- return as-is.
    text
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        TempDir::new().unwrap()
    }

    fn read_claude_md(dir: &Path) -> String {
        fs::read_to_string(dir.join("CLAUDE.md")).unwrap()
    }

    // -- find_marker_pos -----------------------------------------------------

    #[test]
    fn find_marker_at_start() {
        let text = "# OMEGA \u{03A9}\nsome stuff";
        assert_eq!(find_marker_pos(text, PRIMARY_MARKER), Some(0));
    }

    #[test]
    fn find_marker_on_later_line() {
        let text = "hello\n# OMEGA \u{03A9}\nmore";
        assert_eq!(find_marker_pos(text, PRIMARY_MARKER), Some(6));
    }

    #[test]
    fn find_marker_not_present() {
        assert_eq!(find_marker_pos("no marker", PRIMARY_MARKER), None);
    }

    #[test]
    fn find_marker_not_mid_line() {
        // Should NOT match if the marker text appears but not at line start.
        let text = "prefix # OMEGA \u{03A9}\nmore";
        assert_eq!(find_marker_pos(text, PRIMARY_MARKER), None);
    }

    #[test]
    fn find_legacy_marker() {
        let text = "stuff\n# Claude Code Quality Workflow\nmore";
        assert_eq!(find_marker_pos(text, LEGACY_MARKER), Some(6));
    }

    // -- strip_trailing_separator --------------------------------------------

    #[test]
    fn strip_separator_with_blank_line() {
        assert_eq!(strip_trailing_separator("hello\n\n---\n\n"), "hello\n");
    }

    #[test]
    fn strip_separator_without_blank_line() {
        assert_eq!(strip_trailing_separator("hello\n---\n"), "hello\n");
    }

    #[test]
    fn strip_separator_only() {
        assert_eq!(strip_trailing_separator("---\n"), "");
    }

    #[test]
    fn strip_no_separator() {
        assert_eq!(strip_trailing_separator("hello\nworld\n"), "hello\nworld\n");
    }

    // -- inject_workflow_rules (integration) ----------------------------------

    #[test]
    fn creates_new_claude_md() {
        let dir = tmp();
        let result = inject_workflow_rules(dir.path()).unwrap();
        assert_eq!(result, ClaudeMdResult::Created);

        let content = read_claude_md(dir.path());
        assert!(content.starts_with("# CLAUDE.md"));
        assert!(content.contains("powered by OMEGA"));
        assert!(content.contains("## Project-Specific Rules"));
        assert!(content.contains(PRIMARY_MARKER));
    }

    #[test]
    fn appends_to_existing_without_marker() {
        let dir = tmp();
        fs::write(
            dir.path().join("CLAUDE.md"),
            "# My Project\n\nSome rules.\n",
        )
        .unwrap();

        let result = inject_workflow_rules(dir.path()).unwrap();
        assert_eq!(result, ClaudeMdResult::Appended);

        let content = read_claude_md(dir.path());
        assert!(content.starts_with("# My Project"));
        assert!(content.contains("---"));
        assert!(content.contains(PRIMARY_MARKER));
    }

    #[test]
    fn unchanged_when_rules_match() {
        let dir = tmp();
        inject_workflow_rules(dir.path()).unwrap();
        let result = inject_workflow_rules(dir.path()).unwrap();
        assert_eq!(result, ClaudeMdResult::Unchanged);
    }

    #[test]
    fn updates_when_rules_differ() {
        let dir = tmp();
        let claude = dir.path().join("CLAUDE.md");
        let fake = format!(
            "# My Project\n\nCustom stuff.\n\n---\n\n{PRIMARY_MARKER}\n\nOld rules here.\n"
        );
        fs::write(&claude, fake).unwrap();

        let result = inject_workflow_rules(dir.path()).unwrap();
        assert_eq!(result, ClaudeMdResult::Updated);

        let content = read_claude_md(dir.path());
        assert!(content.starts_with("# My Project"));
        assert!(content.contains("Custom stuff."));
        assert!(content.contains(PRIMARY_MARKER));
        assert!(!content.contains("Old rules here."));
    }

    #[test]
    fn upgrades_legacy_marker() {
        let dir = tmp();
        let claude = dir.path().join("CLAUDE.md");
        let legacy = format!("# Project\n\n---\n\n{LEGACY_MARKER}\n\nLegacy rules.\n");
        fs::write(&claude, legacy).unwrap();

        let result = inject_workflow_rules(dir.path()).unwrap();
        assert_eq!(result, ClaudeMdResult::Updated);

        let content = read_claude_md(dir.path());
        assert!(content.contains(PRIMARY_MARKER));
        assert!(!content.contains(LEGACY_MARKER));
        assert!(!content.contains("Legacy rules."));
    }

    #[test]
    fn preserves_user_content_above_separator() {
        let dir = tmp();
        let claude = dir.path().join("CLAUDE.md");
        let user_content = "# My Project\n\n## Important\n\nDo not delete this.\n";
        let original = format!("{user_content}\n---\n\n{PRIMARY_MARKER}\n\nOld.\n");
        fs::write(&claude, original).unwrap();

        inject_workflow_rules(dir.path()).unwrap();
        let content = read_claude_md(dir.path());
        assert!(content.contains("Do not delete this."));
        assert!(content.contains("## Important"));
    }
}
