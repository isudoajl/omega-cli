//! Settings.json hook configuration.
//!
//! Creates or merges `.claude/settings.json` to register OMEGA hooks.
//! Preserves all non-hook keys when merging into an existing file.

use std::fs;
use std::path::Path;

use serde_json::Value;

// ---------------------------------------------------------------------------
// Result / Error types
// ---------------------------------------------------------------------------

/// Describes what happened when configuring hooks.
#[derive(Debug, PartialEq, Eq)]
pub enum SettingsResult {
    /// New settings.json was created (or recreated from malformed file).
    Created,
    /// The hooks section was updated in an existing settings.json.
    Updated,
    /// The hooks section was already identical -- no write performed.
    Unchanged,
}

/// Error type for settings operations.
#[derive(thiserror::Error, Debug)]
pub enum SettingsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Configure hooks in the target project's `.claude/settings.json`.
///
/// 1. Resolves the absolute project path via `canonicalize`.
/// 2. Builds the hooks JSON with absolute paths to `.claude/hooks/*.sh`.
/// 3. Creates or merges the settings file, preserving non-hook keys.
pub fn configure_hooks(target_dir: &Path) -> Result<SettingsResult, SettingsError> {
    let abs_path = fs::canonicalize(target_dir)?;
    let hooks = build_hooks_value(&abs_path);

    let claude_dir = target_dir.join(".claude");
    if !claude_dir.exists() {
        fs::create_dir_all(&claude_dir)?;
    }

    let settings_path = claude_dir.join("settings.json");

    if !settings_path.exists() {
        write_hooks_only(&settings_path, hooks)?;
        return Ok(SettingsResult::Created);
    }

    let raw = fs::read_to_string(&settings_path)?;
    let mut root: Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => {
            // Malformed JSON -- overwrite entirely.
            write_hooks_only(&settings_path, hooks)?;
            return Ok(SettingsResult::Created);
        }
    };

    // Compare existing hooks with the generated ones.
    if root.get("hooks") == Some(&hooks) {
        return Ok(SettingsResult::Unchanged);
    }

    // Merge: replace the hooks key, keep everything else.
    root.as_object_mut()
        .unwrap_or(&mut serde_json::Map::new())
        .insert("hooks".to_string(), hooks);

    write_json(&settings_path, &root)?;
    Ok(SettingsResult::Updated)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build the complete hooks JSON `Value` with absolute paths.
fn build_hooks_value(project_abs: &Path) -> Value {
    let hooks_dir = project_abs.join(".claude").join("hooks");
    let hp = hooks_dir.display().to_string();

    serde_json::json!({
        "UserPromptSubmit": [
            {
                "matcher": "",
                "hooks": [
                    {
                        "type": "command",
                        "command": format!("{hp}/briefing.sh"),
                        "timeout": 30000
                    }
                ]
            }
        ],
        "PreToolUse": [
            {
                "matcher": "Bash",
                "hooks": [
                    {
                        "type": "command",
                        "command": format!("{hp}/debrief-gate.sh"),
                        "timeout": 5000
                    }
                ]
            },
            {
                "matcher": "Write",
                "hooks": [
                    {
                        "type": "command",
                        "command": format!("{hp}/incremental-gate.sh"),
                        "timeout": 5000
                    }
                ]
            },
            {
                "matcher": "Edit",
                "hooks": [
                    {
                        "type": "command",
                        "command": format!("{hp}/incremental-gate.sh"),
                        "timeout": 5000
                    }
                ]
            }
        ],
        "PostToolUse": [
            {
                "matcher": "",
                "hooks": [
                    {
                        "type": "command",
                        "command": format!("{hp}/debrief-nudge.sh"),
                        "timeout": 5000
                    }
                ]
            }
        ],
        "Notification": [
            {
                "matcher": "",
                "hooks": [
                    {
                        "type": "command",
                        "command": format!("{hp}/session-close.sh"),
                        "timeout": 10000
                    }
                ]
            }
        ]
    })
}

/// Write a settings.json that contains only the hooks key.
fn write_hooks_only(path: &Path, hooks: Value) -> Result<(), SettingsError> {
    let mut root = serde_json::Map::new();
    root.insert("hooks".to_string(), hooks);
    write_json(path, &Value::Object(root))
}

/// Write a `Value` to disk as pretty-printed JSON with a trailing newline.
fn write_json(path: &Path, value: &Value) -> Result<(), SettingsError> {
    let mut json = serde_json::to_string_pretty(value)?;
    json.push('\n');
    fs::write(path, json)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Create a temp dir with the `.claude` subdirectory pre-created.
    fn tmp_with_claude() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".claude")).unwrap();
        dir
    }

    fn read_settings(dir: &Path) -> String {
        fs::read_to_string(dir.join(".claude/settings.json")).unwrap()
    }

    fn parse_settings(dir: &Path) -> Value {
        serde_json::from_str(&read_settings(dir)).unwrap()
    }

    #[test]
    fn creates_new_settings() {
        let dir = tmp_with_claude();
        let result = configure_hooks(dir.path()).unwrap();
        assert_eq!(result, SettingsResult::Created);

        let val = parse_settings(dir.path());
        assert!(val.get("hooks").is_some());
        let hooks = val.get("hooks").unwrap();
        assert!(hooks.get("UserPromptSubmit").is_some());
        assert!(hooks.get("PreToolUse").is_some());
        assert!(hooks.get("PostToolUse").is_some());
        assert!(hooks.get("Notification").is_some());
    }

    #[test]
    fn creates_claude_dir_if_missing() {
        let dir = TempDir::new().unwrap();
        // .claude does not exist yet.
        let result = configure_hooks(dir.path()).unwrap();
        assert_eq!(result, SettingsResult::Created);
        assert!(dir.path().join(".claude/settings.json").exists());
    }

    #[test]
    fn unchanged_on_second_call() {
        let dir = tmp_with_claude();
        configure_hooks(dir.path()).unwrap();
        let result = configure_hooks(dir.path()).unwrap();
        assert_eq!(result, SettingsResult::Unchanged);
    }

    #[test]
    fn preserves_non_hook_keys() {
        let dir = tmp_with_claude();
        let settings_path = dir.path().join(".claude/settings.json");
        let existing = serde_json::json!({
            "customKey": "preserve me",
            "hooks": { "old": true }
        });
        fs::write(
            &settings_path,
            serde_json::to_string_pretty(&existing).unwrap(),
        )
        .unwrap();

        let result = configure_hooks(dir.path()).unwrap();
        assert_eq!(result, SettingsResult::Updated);

        let val = parse_settings(dir.path());
        assert_eq!(val.get("customKey").unwrap(), "preserve me");
        assert!(val.get("hooks").unwrap().get("UserPromptSubmit").is_some());
    }

    #[test]
    fn overwrites_malformed_json() {
        let dir = tmp_with_claude();
        let settings_path = dir.path().join(".claude/settings.json");
        fs::write(&settings_path, "this is not json {{{").unwrap();

        let result = configure_hooks(dir.path()).unwrap();
        assert_eq!(result, SettingsResult::Created);

        let val = parse_settings(dir.path());
        assert!(val.get("hooks").is_some());
    }

    #[test]
    fn hooks_contain_absolute_paths() {
        let dir = tmp_with_claude();
        configure_hooks(dir.path()).unwrap();
        let raw = read_settings(dir.path());
        // The paths should be absolute (start with /).
        assert!(raw.contains("/briefing.sh"));
        assert!(raw.contains("/debrief-gate.sh"));
        assert!(raw.contains("/incremental-gate.sh"));
        assert!(raw.contains("/debrief-nudge.sh"));
        assert!(raw.contains("/session-close.sh"));
    }

    #[test]
    fn hooks_have_correct_structure() {
        let dir = tmp_with_claude();
        configure_hooks(dir.path()).unwrap();
        let val = parse_settings(dir.path());
        let hooks = val.get("hooks").unwrap();

        // UserPromptSubmit: 1 entry, matcher ""
        let ups = hooks.get("UserPromptSubmit").unwrap().as_array().unwrap();
        assert_eq!(ups.len(), 1);
        assert_eq!(ups[0].get("matcher").unwrap(), "");
        let inner = ups[0].get("hooks").unwrap().as_array().unwrap();
        assert_eq!(inner[0].get("type").unwrap(), "command");
        assert_eq!(inner[0].get("timeout").unwrap(), 30000);

        // PreToolUse: 3 entries (Bash, Write, Edit)
        let ptu = hooks.get("PreToolUse").unwrap().as_array().unwrap();
        assert_eq!(ptu.len(), 3);
        assert_eq!(ptu[0].get("matcher").unwrap(), "Bash");
        assert_eq!(ptu[1].get("matcher").unwrap(), "Write");
        assert_eq!(ptu[2].get("matcher").unwrap(), "Edit");

        // PostToolUse: 1 entry
        let post = hooks.get("PostToolUse").unwrap().as_array().unwrap();
        assert_eq!(post.len(), 1);

        // Notification: 1 entry
        let notif = hooks.get("Notification").unwrap().as_array().unwrap();
        assert_eq!(notif.len(), 1);
    }

    #[test]
    fn output_ends_with_newline() {
        let dir = tmp_with_claude();
        configure_hooks(dir.path()).unwrap();
        let raw = read_settings(dir.path());
        assert!(raw.ends_with('\n'));
    }
}
