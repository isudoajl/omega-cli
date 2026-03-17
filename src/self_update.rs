//! Binary self-update for the `omg` CLI.
//!
//! Checks GitHub releases for newer versions, downloads the appropriate
//! platform binary, verifies its SHA-256 checksum, and replaces the
//! running binary in-place.

use std::env::consts::{ARCH, OS};

use console::style;
use sha2::{Digest, Sha256};

use crate::version::binary_version;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Information about an available update.
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub download_url: String,
    pub checksum_sha256: String,
}

/// Errors that can occur during the self-update process.
#[derive(thiserror::Error, Debug)]
pub enum UpdateError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("No update available (current: v{0})")]
    #[allow(dead_code)]
    NoUpdate(String),

    #[error("{0}")]
    Other(String),
}

// ---------------------------------------------------------------------------
// GitHub API response types
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
    body: Option<String>,
}

#[derive(serde::Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Check whether a newer version is available on GitHub.
///
/// Returns `Ok(Some(info))` if a newer version exists, `Ok(None)` if the
/// binary is already up-to-date, or an error if the check fails.
pub async fn check_for_update() -> Result<Option<UpdateInfo>, UpdateError> {
    let current = binary_version();

    let client = reqwest::Client::builder()
        .user_agent(format!("omg/{}", current))
        .build()?;

    let release: GitHubRelease = client
        .get("https://api.github.com/repos/isudoajl/omega-cli/releases/latest")
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await?
        .error_for_status()
        .map_err(|e| UpdateError::Other(format!("GitHub API error: {}", e)))?
        .json()
        .await?;

    let latest = release.tag_name.trim_start_matches('v');

    if !is_newer(current, latest) {
        return Ok(None);
    }

    // Determine the platform-specific binary name.
    let target_triple = platform_triple()
        .ok_or_else(|| UpdateError::Other(format!("Unsupported platform: {} {}", OS, ARCH)))?;

    let binary_name = format!("omg-{}", target_triple);

    // Find the binary asset in the release.
    let binary_asset = release
        .assets
        .iter()
        .find(|a| a.name == binary_name)
        .ok_or_else(|| {
            UpdateError::Other(format!(
                "No binary for {} in release v{}",
                target_triple, latest
            ))
        })?;

    // Look for a checksum file (e.g., omg-aarch64-apple-darwin.sha256).
    let checksum_name = format!("{}.sha256", binary_name);
    let checksum =
        if let Some(checksum_asset) = release.assets.iter().find(|a| a.name == checksum_name) {
            // Download the checksum file content.
            let body = client
                .get(&checksum_asset.browser_download_url)
                .send()
                .await?
                .error_for_status()
                .map_err(|e| UpdateError::Other(format!("Failed to download checksum: {}", e)))?
                .text()
                .await?;
            // Checksum file format: "<hex>  <filename>" or just "<hex>"
            body.split_whitespace().next().unwrap_or("").to_string()
        } else {
            // No checksum file available -- allow update but warn.
            String::new()
        };

    let _notes = release.body.unwrap_or_default();

    Ok(Some(UpdateInfo {
        current_version: current.to_string(),
        latest_version: latest.to_string(),
        download_url: binary_asset.browser_download_url.clone(),
        checksum_sha256: checksum,
    }))
}

/// Download, verify, and install the update.
pub async fn perform_update(info: &UpdateInfo) -> Result<(), UpdateError> {
    println!("  Downloading v{} ...", style(&info.latest_version).cyan());

    let client = reqwest::Client::builder()
        .user_agent(format!("omg/{}", info.current_version))
        .build()?;

    let bytes = client
        .get(&info.download_url)
        .send()
        .await?
        .error_for_status()
        .map_err(|e| UpdateError::Other(format!("Download failed: {}", e)))?
        .bytes()
        .await?;

    // Verify SHA-256 checksum if one was provided.
    if !info.checksum_sha256.is_empty() {
        println!("  Verifying checksum ...");
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let actual = format!("{:x}", hasher.finalize());

        if actual != info.checksum_sha256 {
            return Err(UpdateError::ChecksumMismatch {
                expected: info.checksum_sha256.clone(),
                actual,
            });
        }
    }

    // Write to a temporary file and replace the running binary.
    println!("  Replacing binary ...");
    let tmp = std::env::temp_dir().join("omg-update-tmp");
    std::fs::write(&tmp, &bytes)?;

    // Ensure the downloaded binary is executable on Unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&tmp)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&tmp, perms)?;
    }

    self_replace::self_replace(&tmp).map_err(|e| {
        UpdateError::Other(format!(
            "Failed to replace binary: {}. Try: sudo omg self-update",
            e
        ))
    })?;

    // Clean up the temp file (best effort).
    let _ = std::fs::remove_file(&tmp);

    println!(
        "  {} Updated from v{} to v{}",
        style("Done!").green().bold(),
        info.current_version,
        info.latest_version,
    );

    Ok(())
}

/// Main entry point for the self-update command.
///
/// If `check_only` is true, just report whether an update is available.
/// Otherwise, download and install it.
pub async fn run(check_only: bool) -> Result<(), UpdateError> {
    let current = binary_version();
    println!(
        "  Current version: {}",
        style(format!("v{}", current)).cyan()
    );
    println!("  Checking for updates ...");

    match check_for_update().await {
        Ok(Some(info)) => {
            println!(
                "  {} v{} available",
                style("Update found:").green().bold(),
                info.latest_version,
            );

            if check_only {
                println!("  Run {} to install.", style("omg self-update").cyan());
                Ok(())
            } else {
                perform_update(&info).await
            }
        }
        Ok(None) => {
            println!(
                "  {} v{} is the latest version.",
                style("Up to date!").green().bold(),
                current,
            );
            Ok(())
        }
        Err(e) => {
            // Network errors during check are reported but not fatal.
            if check_only {
                eprintln!("  {} {}", style("Cannot check for updates:").yellow(), e);
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compare two semver strings. Returns true if `latest` is strictly newer
/// than `current`.
fn is_newer(current: &str, latest: &str) -> bool {
    let parse = |s: &str| -> (u64, u64, u64) {
        let parts: Vec<u64> = s.split('.').filter_map(|p| p.parse().ok()).collect();
        (
            parts.first().copied().unwrap_or(0),
            parts.get(1).copied().unwrap_or(0),
            parts.get(2).copied().unwrap_or(0),
        )
    };

    let c = parse(current);
    let l = parse(latest);

    l > c
}

/// Return the Rust target triple for the current platform, or None if
/// the platform is not supported.
fn platform_triple() -> Option<&'static str> {
    match (OS, ARCH) {
        ("macos", "aarch64") => Some("aarch64-apple-darwin"),
        ("macos", "x86_64") => Some("x86_64-apple-darwin"),
        ("linux", "x86_64") => Some("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Some("aarch64-unknown-linux-gnu"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_newer_basic() {
        assert!(is_newer("0.1.0", "0.2.0"));
        assert!(is_newer("0.1.0", "1.0.0"));
        assert!(is_newer("0.1.0", "0.1.1"));
        assert!(!is_newer("0.2.0", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("1.0.0", "0.9.9"));
    }

    #[test]
    fn test_is_newer_edge_cases() {
        assert!(is_newer("0.0.0", "0.0.1"));
        assert!(!is_newer("0.0.1", "0.0.0"));
        assert!(is_newer("0.9.9", "1.0.0"));
        assert!(is_newer("1.9.9", "2.0.0"));
    }

    #[test]
    fn test_platform_triple_returns_some() {
        // At least the current platform should be supported in CI/dev.
        let triple = platform_triple();
        // We can't assert Some on all CI platforms, but on macOS/Linux it should work.
        if matches!((OS, ARCH), ("macos" | "linux", "x86_64" | "aarch64")) {
            assert!(triple.is_some());
        }
    }
}
