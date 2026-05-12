use std::fs;
use std::process::Command;

use anyhow::{anyhow, Context, Result};

const RELEASE_MANIFEST_PATH: &str = ".aiplus/agent-team/release-manifest.yaml";
const FINGERPRINT_PATH: &str = ".aiplus/agent-team/owner-key-fingerprint";

/// Entry point for `audit re-sign-manifest`.
pub fn handle_re_sign_manifest() -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let manifest_path = cwd.join(RELEASE_MANIFEST_PATH);
    let fingerprint_path = cwd.join(FINGERPRINT_PATH);

    if !manifest_path.exists() {
        return Err(anyhow!(
            "Release manifest not found at {}. Create it first.",
            manifest_path.display()
        ));
    }

    if !fingerprint_path.exists() {
        return Err(anyhow!(
            "Owner fingerprint not found. Run `aiplus agent audit setup-gpg` first."
        ));
    }

    let fingerprint = fs::read_to_string(&fingerprint_path)
        .with_context(|| "failed to read owner fingerprint")?;
    let fingerprint = fingerprint.trim();

    // Verify GPG signature on the latest commit touching manifest
    let verify_output = Command::new("git")
        .current_dir(&cwd)
        .args([
            "log",
            "-1",
            "--format=%G?%n%GF",
            "--",
            manifest_path.to_str().unwrap_or(""),
        ])
        .output()
        .with_context(|| "failed to run git log for signature check")?;

    if verify_output.status.success() {
        let stdout = String::from_utf8_lossy(&verify_output.stdout);
        let lines: Vec<&str> = stdout.trim().lines().collect();
        if lines.len() >= 2 && lines[0] == "G" && lines[1].trim() == fingerprint {
            println!("Manifest already signed with correct fingerprint.");
            return Ok(());
        }
    }

    // Stage, commit, and sign manifest
    let add_output = Command::new("git")
        .current_dir(&cwd)
        .args(["add", manifest_path.to_str().unwrap_or("")])
        .output()
        .with_context(|| "failed to stage manifest")?;
    if !add_output.status.success() {
        return Err(anyhow!("git add failed"));
    }

    let commit_output = Command::new("git")
        .current_dir(&cwd)
        .args([
            "commit",
            "-m",
            "Re-sign release manifest",
            "--no-verify",
            "-S",
            fingerprint,
        ])
        .output()
        .with_context(|| "failed to commit signed manifest")?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        return Err(anyhow!("Signed commit failed: {}", stderr));
    }

    println!("MANIFEST_SIGNED=PASS");
    println!("FINGERPRINT={}", fingerprint);
    Ok(())
}
