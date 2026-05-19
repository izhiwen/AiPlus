use crate::CliError;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const KEY_COMMENT: &str = "aiplus-secure-enclave";
const KEY_BASENAME: &str = "id_ecdsa_sk_aiplus";
const ALLOWED_SIGNERS_BASENAME: &str = "aiplus_allowed_signers";

pub fn handle_setup_signing(dry_run: bool) -> Result<()> {
    let home = home_dir()?;
    let ssh_dir = home.join(".ssh");
    let key_path = ssh_dir.join(KEY_BASENAME);
    let public_key_path = key_path.with_extension("pub");
    let allowed_signers_path = ssh_dir.join(ALLOWED_SIGNERS_BASENAME);
    let platform = setup_signing_platform();

    if dry_run {
        print_dry_run(
            &platform,
            &key_path,
            &public_key_path,
            &allowed_signers_path,
        );
        return Ok(());
    }

    if platform != "macos" {
        return Err(CliError::new(
            3,
            format!(
                "SETUP_SIGNING_STATUS=UNSUPPORTED platform={platform} reason=secure_enclave_signing_requires_macos"
            ),
        )
        .into());
    }

    fs::create_dir_all(&ssh_dir)
        .with_context(|| format!("failed to create ssh directory {}", ssh_dir.display()))?;

    let planned = planned_git_config(&public_key_path, &allowed_signers_path);
    reject_git_config_conflicts(&planned)?;

    if !key_path.exists() {
        generate_secure_enclave_key(&key_path)?;
    } else {
        println!("signing_key=existing path={}", key_path.display());
    }

    let public_key = fs::read_to_string(&public_key_path).with_context(|| {
        format!(
            "failed to read generated public key {}",
            public_key_path.display()
        )
    })?;
    let allowed_signers_principal =
        git_config_get("user.email")?.unwrap_or_else(|| "aiplus-owner@example.invalid".to_string());
    write_allowed_signers(
        &allowed_signers_path,
        &allowed_signers_principal,
        &public_key,
    )?;
    apply_git_config(&planned)?;

    println!("SETUP_SIGNING_STATUS=PASS");
    println!("key_path={}", key_path.display());
    println!("allowed_signers_file={}", allowed_signers_path.display());
    println!("global_git_config=updated");
    Ok(())
}

fn print_dry_run(
    platform: &str,
    key_path: &Path,
    public_key_path: &Path,
    allowed_signers_path: &Path,
) {
    println!("SETUP_SIGNING_STATUS=DRY_RUN");
    println!("platform={platform}");
    println!("key_path={}", key_path.display());
    println!("allowed_signers_file={}", allowed_signers_path.display());
    println!(
        "planned_ssh_keygen=ssh-keygen -t ecdsa-sk -O resident -O verify-required -C {KEY_COMMENT} -f {}",
        key_path.display()
    );
    println!("planned_git_config=gpg.format ssh");
    println!(
        "planned_git_config=user.signingkey {}",
        public_key_path.display()
    );
    println!("planned_git_config=commit.gpgsign true");
    println!(
        "planned_git_config=gpg.ssh.allowedSignersFile {}",
        allowed_signers_path.display()
    );
}

fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .ok_or_else(|| CliError::new(3, "SETUP_SIGNING_STATUS=FAIL reason=home_not_set").into())
}

fn setup_signing_platform() -> String {
    if let Ok(value) = std::env::var("AIPLUS_SETUP_SIGNING_FORCE_PLATFORM") {
        return match value.trim().to_ascii_lowercase().as_str() {
            "darwin" | "mac" | "macos" => "macos".to_string(),
            other => other.to_string(),
        };
    }
    if cfg!(target_os = "macos") {
        "macos".to_string()
    } else {
        std::env::consts::OS.to_string()
    }
}

fn planned_git_config(
    public_key_path: &Path,
    allowed_signers_path: &Path,
) -> Vec<(&'static str, String)> {
    vec![
        ("gpg.format", "ssh".to_string()),
        ("user.signingkey", public_key_path.display().to_string()),
        ("commit.gpgsign", "true".to_string()),
        (
            "gpg.ssh.allowedSignersFile",
            allowed_signers_path.display().to_string(),
        ),
    ]
}

fn reject_git_config_conflicts(planned: &[(&str, String)]) -> Result<()> {
    for (key, wanted) in planned {
        if let Some(existing) = git_config_get(key)? {
            if existing != *wanted {
                return Err(CliError::new(
                    3,
                    format!(
                        "SETUP_SIGNING_STATUS=REFUSED reason=existing_git_signing_config key={key} current={existing} wanted={wanted}"
                    ),
                )
                .into());
            }
        }
    }
    Ok(())
}

fn git_config_get(key: &str) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(["config", "--global", "--get", key])
        .output()
        .with_context(|| format!("failed to run git config --global --get {key}"))?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
    ))
}

fn generate_secure_enclave_key(key_path: &Path) -> Result<()> {
    if std::env::var_os("AIPLUS_SETUP_SIGNING_FAKE_KEYGEN").is_some() {
        fs::write(key_path, "FAKE_AIPLUS_SECURE_ENCLAVE_PRIVATE_KEY\n")
            .with_context(|| format!("failed to write fake key {}", key_path.display()))?;
        fs::write(
            key_path.with_extension("pub"),
            format!("sk-ecdsa-sha2-nistp256@openssh.com AAAAFakeKey {KEY_COMMENT}\n"),
        )
        .with_context(|| {
            format!(
                "failed to write fake public key {}",
                key_path.with_extension("pub").display()
            )
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(key_path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(key_path, perms)?;
        }
        println!("signing_key=generated_fake path={}", key_path.display());
        return Ok(());
    }

    let status = Command::new("ssh-keygen")
        .args([
            "-t",
            "ecdsa-sk",
            "-O",
            "resident",
            "-O",
            "verify-required",
            "-C",
            KEY_COMMENT,
            "-f",
        ])
        .arg(key_path)
        .status()
        .with_context(|| "failed to spawn ssh-keygen for Secure Enclave signing key")?;

    if !status.success() {
        return Err(CliError::new(
            3,
            format!(
                "SETUP_SIGNING_STATUS=FAIL reason=ssh_keygen_failed status={}",
                status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "signal".to_string())
            ),
        )
        .into());
    }
    println!("signing_key=generated path={}", key_path.display());
    Ok(())
}

fn write_allowed_signers(path: &Path, principal: &str, public_key: &str) -> Result<()> {
    let public_key = public_key.trim();
    if public_key.is_empty() {
        return Err(CliError::new(3, "SETUP_SIGNING_STATUS=FAIL reason=empty_public_key").into());
    }
    fs::write(path, format!("{principal} {public_key}\n"))
        .with_context(|| format!("failed to write allowed signers {}", path.display()))?;
    Ok(())
}

fn apply_git_config(planned: &[(&str, String)]) -> Result<()> {
    for (key, value) in planned {
        let status = Command::new("git")
            .args(["config", "--global", key])
            .arg(value)
            .status()
            .with_context(|| format!("failed to run git config --global {key}"))?;
        if !status.success() {
            return Err(CliError::new(
                3,
                format!(
                    "SETUP_SIGNING_STATUS=FAIL reason=git_config_failed key={key} status={}",
                    status
                        .code()
                        .map(|code| code.to_string())
                        .unwrap_or_else(|| "signal".to_string())
                ),
            )
            .into());
        }
    }
    Ok(())
}
