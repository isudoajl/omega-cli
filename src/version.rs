//! Version tracking for OMEGA CLI deployments.
//!
//! Manages the `.claude/.omg-version` stamp that records which version
//! of `omg` deployed assets to a project, when, and with which extensions.

use std::fs;
use std::path::Path;

use chrono::Utc;
use serde::{Deserialize, Serialize};

/// Persistent version stamp written to `.claude/.omg-version`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct VersionStamp {
    /// The `omg` binary version that performed the deployment.
    pub version: String,
    /// ISO-8601 timestamp of the deployment.
    pub deployed_at: String,
    /// List of extension names that were deployed.
    pub extensions: Vec<String>,
}

/// Returns the compiled-in version of the `omg` binary from `Cargo.toml`.
pub fn binary_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Reads the deployed version stamp from a target project.
///
/// Returns `None` if the stamp file is missing or contains invalid JSON.
#[allow(dead_code)]
pub fn deployed_version(target_dir: &Path) -> Option<VersionStamp> {
    let stamp_path = target_dir.join(".claude/.omg-version");
    let content = fs::read_to_string(stamp_path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Writes a version stamp to `.claude/.omg-version` in the target project.
///
/// Records the current binary version, current UTC timestamp, and which
/// extensions were deployed.
pub fn write_version_stamp(target_dir: &Path, extensions: &[String]) -> Result<(), std::io::Error> {
    let claude_dir = target_dir.join(".claude");
    fs::create_dir_all(&claude_dir)?;

    let stamp = VersionStamp {
        version: binary_version().to_string(),
        deployed_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        extensions: extensions.to_vec(),
    };

    let json = serde_json::to_string_pretty(&stamp).map_err(std::io::Error::other)?;

    fs::write(target_dir.join(".claude/.omg-version"), json)?;

    Ok(())
}

/// Prints version information to stdout.
///
/// If `json` is true, outputs a JSON object with version, build target,
/// and other metadata. Otherwise outputs a human-readable summary.
pub fn print_version(json: bool) {
    let version = binary_version();

    if json {
        let info = serde_json::json!({
            "version": version,
            "target": option_env!("TARGET").unwrap_or("unknown"),
        });
        println!("{}", serde_json::to_string_pretty(&info).unwrap());
    } else {
        println!("omg {} (OMEGA CLI)", version);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_dir(label: &str) -> PathBuf {
        let id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "omg-ver-test-{}-{}-{}",
            std::process::id(),
            label,
            id,
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_binary_version() {
        let ver = binary_version();
        assert!(!ver.is_empty());
        assert_eq!(ver, "0.1.0");
    }

    #[test]
    fn test_deployed_version_missing() {
        let dir = temp_dir("missing");
        assert!(deployed_version(&dir).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_and_read_version_stamp() {
        let dir = temp_dir("write-read");
        let exts = vec!["blockchain".to_string()];
        write_version_stamp(&dir, &exts).unwrap();

        let stamp = deployed_version(&dir).unwrap();
        assert_eq!(stamp.version, "0.1.0");
        assert_eq!(stamp.extensions, vec!["blockchain".to_string()]);
        assert!(!stamp.deployed_at.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_write_version_stamp_empty_extensions() {
        let dir = temp_dir("empty-exts");
        write_version_stamp(&dir, &[]).unwrap();

        let stamp = deployed_version(&dir).unwrap();
        assert!(stamp.extensions.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_deployed_version_invalid_json() {
        let dir = temp_dir("invalid-json");
        let claude_dir = dir.join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(claude_dir.join(".omg-version"), "not valid json").unwrap();

        assert!(deployed_version(&dir).is_none());
        let _ = fs::remove_dir_all(&dir);
    }
}
