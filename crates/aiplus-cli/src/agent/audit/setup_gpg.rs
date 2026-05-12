use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const SENTINEL_PATH: &str = ".aiplus/agent-team/.owner-setup-authorized";
const FINGERPRINT_PATH: &str = ".aiplus/agent-team/owner-key-fingerprint";

const REFUSAL_MESSAGE: &str = "First-run GPG setup requires Owner authorization via sentinel file.\nOwner must create:\n   cat > .aiplus/agent-team/.owner-setup-authorized <<EOF\n   name: <your full name>\n   email: <your email>\n   EOF\nThen re-run: aiplus agent audit setup-gpg";

const REFUSAL_PREFIX: &str = "REFUSAL:";

#[derive(Debug, Deserialize)]
struct Sentinel {
    name: String,
    email: String,
}

/// Entry point for the setup-gpg wizard.
pub fn handle_setup_gpg() -> Result<()> {
    if let Err(e) = run_setup() {
        let msg = e.to_string();
        if msg.starts_with(REFUSAL_PREFIX) {
            eprintln!("{}", msg.trim_start_matches(REFUSAL_PREFIX));
            std::process::exit(3);
        }
        return Err(e);
    }
    println!("SETUP_GPG_STATUS=PASS");
    Ok(())
}

fn run_setup() -> Result<()> {
    // 1. Sentinel verification
    let sentinel = verify_sentinel()?;

    // 2. Determine dev/test mode
    let is_dev = is_dev_mode();

    // 3. Setup GPG home
    let _temp_dir;
    let gnupg_home = if is_dev {
        _temp_dir = tempfile::tempdir().context("failed to create ephemeral GPG home")?;
        let path = _temp_dir.path().to_path_buf();
        std::env::set_var("GNUPGHOME", &path);
        println!("MANIFEST_SIGNING=gpg_ephemeral_dev");
        path
    } else {
        home_dir_gnupg()
    };

    // Ensure the directory exists
    fs::create_dir_all(&gnupg_home)
        .with_context(|| format!("failed to create GPG home: {}", gnupg_home.display()))?;

    // 4. Check GPG availability
    let gpg_bin = find_gpg()?;

    // 5. Prompt for passphrase
    let passphrase = prompt_passphrase()?;

    // 6. Generate key
    generate_key(&gpg_bin, &sentinel, &passphrase, &gnupg_home)?;

    // 7. Configure GPG agent cache TTL
    configure_agent_cache(&gnupg_home)?;

    // 8. Capture fingerprint
    let fingerprint = capture_fingerprint(&gpg_bin, &gnupg_home)?;

    // 9. Write fingerprint
    fs::write(FINGERPRINT_PATH, &fingerprint)
        .with_context(|| format!("failed to write fingerprint to {FINGERPRINT_PATH}"))?;

    // 10. Delete sentinel
    fs::remove_file(SENTINEL_PATH)
        .with_context(|| format!("failed to delete sentinel {SENTINEL_PATH}"))?;

    Ok(())
}

fn is_dev_mode() -> bool {
    std::env::var("CARGO_MANIFEST_DIR").is_ok()
        || std::env::var("AIPLUS_DEV").is_ok()
        || std::env::var("AIPLUS_TEST_GPG").is_ok()
}

fn home_dir_gnupg() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".gnupg")
}

fn verify_sentinel() -> Result<Sentinel> {
    let content = fs::read_to_string(SENTINEL_PATH)
        .map_err(|_| anyhow!("{REFUSAL_PREFIX}{REFUSAL_MESSAGE}"))?;

    let sentinel: Sentinel = serde_yaml_ng::from_str(&content)
        .map_err(|_| anyhow!("{REFUSAL_PREFIX}{REFUSAL_MESSAGE}"))?;

    if sentinel.name.trim().is_empty() || sentinel.email.trim().is_empty() {
        return Err(anyhow!("{REFUSAL_PREFIX}{REFUSAL_MESSAGE}"));
    }

    Ok(sentinel)
}

fn find_gpg() -> Result<String> {
    if let Ok(var) = std::env::var("AIPLUS_GPG_BIN") {
        return Ok(var);
    }
    for name in ["gpg2", "gpg"] {
        if Command::new(name).arg("--version").output().is_ok() {
            return Ok(name.to_string());
        }
    }
    Err(anyhow!("gpg not found in PATH"))
}

fn prompt_passphrase() -> Result<String> {
    // Allow env override for non-interactive tests
    if let Ok(pass) = std::env::var("AIPLUS_TEST_GPG_PASSPHRASE") {
        if pass.is_empty() {
            return Err(anyhow!("Passphrase cannot be empty"));
        }
        return Ok(pass);
    }

    let passphrase =
        rpassword::prompt_password("Enter passphrase: ").context("failed to read passphrase")?;

    if passphrase.is_empty() {
        return Err(anyhow!("Passphrase cannot be empty"));
    }

    if passphrase.len() < 8 {
        eprintln!("WARNING: Passphrase is shorter than 8 characters");
    }

    let confirm = rpassword::prompt_password("Confirm passphrase: ")
        .context("failed to read passphrase confirmation")?;

    if passphrase != confirm {
        return Err(anyhow!("Passphrases do not match"));
    }

    Ok(passphrase)
}

fn generate_key(gpg: &str, sentinel: &Sentinel, passphrase: &str, gnupg_home: &Path) -> Result<()> {
    let batch = format!(
        "%echo Generating key\nKey-Type: EDDSA\nKey-Curve: Ed25519\nSubkey-Type: ECDH\nSubkey-Curve: Curve25519\nName-Real: {}\nName-Email: {}\nExpire-Date: 0\n%commit\n%echo done\n",
        sentinel.name, sentinel.email
    );

    let batch_path = gnupg_home.join("batch.txt");
    fs::write(&batch_path, &batch).context("failed to write batch config")?;

    let mut child = Command::new(gpg)
        .arg("--batch")
        .arg("--pinentry-mode")
        .arg("loopback")
        .arg("--passphrase-fd")
        .arg("0")
        .arg("--gen-key")
        .arg(&batch_path)
        .env("GNUPGHOME", gnupg_home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn {gpg}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(passphrase.as_bytes())
            .and_then(|_| stdin.write_all(b"\n"))
            .context("failed to send passphrase to gpg")?;
    }

    let output = child
        .wait_with_output()
        .context("gpg process failed")?;

    fs::remove_file(&batch_path).ok();

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Fallback to RSA-4096 if EDDSA is unsupported
    if stderr.contains("EDDSA")
        || stderr.contains("Ed25519")
        || stderr.contains("not supported")
        || stderr.contains("unknown algorithm")
    {
        eprintln!("EDDSA not supported, falling back to RSA-4096");
        return generate_key_rsa(gpg, sentinel, passphrase, gnupg_home);
    }

    Err(anyhow!("gpg key generation failed: {stderr}"))
}

fn generate_key_rsa(
    gpg: &str,
    sentinel: &Sentinel,
    passphrase: &str,
    gnupg_home: &Path,
) -> Result<()> {
    let batch = format!(
        "%echo Generating RSA key\nKey-Type: RSA\nKey-Length: 4096\nName-Real: {}\nName-Email: {}\nExpire-Date: 0\n%commit\n%echo done\n",
        sentinel.name, sentinel.email
    );

    let batch_path = gnupg_home.join("batch-rsa.txt");
    fs::write(&batch_path, &batch).context("failed to write RSA batch config")?;

    let mut child = Command::new(gpg)
        .arg("--batch")
        .arg("--pinentry-mode")
        .arg("loopback")
        .arg("--passphrase-fd")
        .arg("0")
        .arg("--gen-key")
        .arg(&batch_path)
        .env("GNUPGHOME", gnupg_home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to spawn {gpg} for RSA fallback"))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(passphrase.as_bytes())
            .and_then(|_| stdin.write_all(b"\n"))
            .context("failed to send passphrase to gpg")?;
    }

    let output = child
        .wait_with_output()
        .context("gpg RSA process failed")?;

    fs::remove_file(&batch_path).ok();

    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow!(
            "gpg RSA key generation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn configure_agent_cache(gnupg_home: &Path) -> Result<()> {
    let conf_path = gnupg_home.join("gpg-agent.conf");

    let content = if conf_path.exists() {
        fs::read_to_string(&conf_path).unwrap_or_default()
    } else {
        String::new()
    };

    if content.contains("default-cache-ttl") {
        return Ok(());
    }

    // In non-interactive contexts (tests / CI), default to yes
    let confirmed = if io::stdin().is_terminal() {
        print!("Add default-cache-ttl 600 to gpg-agent.conf? [Y/n] ");
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();
        trimmed.is_empty() || trimmed.eq_ignore_ascii_case("y")
    } else {
        true
    };

    if confirmed {
        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&conf_path)
            .with_context(|| format!("failed to open {}", conf_path.display()))?;
        writeln!(file, "default-cache-ttl 600")
            .with_context(|| format!("failed to write to {}", conf_path.display()))?;
        println!("Updated gpg-agent.conf");
    }

    Ok(())
}

fn capture_fingerprint(gpg: &str, gnupg_home: &Path) -> Result<String> {
    let output = Command::new(gpg)
        .arg("--list-secret-keys")
        .arg("--with-colons")
        .env("GNUPGHOME", gnupg_home)
        .output()
        .with_context(|| format!("failed to run {gpg} --list-secret-keys"))?;

    if !output.status.success() {
        return Err(anyhow!(
            "gpg list-secret-keys failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // The fingerprint line starts with "fpr:" and the fingerprint is at index 9
    for line in stdout.lines() {
        if line.starts_with("fpr:") {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 10 && !parts[9].is_empty() {
                return Ok(parts[9].to_string());
            }
        }
    }

    Err(anyhow!("could not find key fingerprint in gpg output"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to serialize tests that change the current working directory
    static CWD_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn sentinel_absent_returns_refusal() {
        let _guard = CWD_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp).unwrap();

        let result = verify_sentinel();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.starts_with(REFUSAL_PREFIX));
        assert!(err.contains("First-run GPG setup requires Owner authorization"));

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn sentinel_malformed_returns_refusal() {
        let _guard = CWD_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp).unwrap();

        fs::create_dir_all(".aiplus/agent-team").unwrap();
        fs::write(SENTINEL_PATH, "not yaml at all").unwrap();

        let result = verify_sentinel();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.starts_with(REFUSAL_PREFIX));

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn sentinel_missing_name_returns_refusal() {
        let _guard = CWD_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp).unwrap();

        fs::create_dir_all(".aiplus/agent-team").unwrap();
        fs::write(SENTINEL_PATH, "name: \"\"\nemail: test@example.com\n").unwrap();

        let result = verify_sentinel();
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.starts_with(REFUSAL_PREFIX));

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn sentinel_valid_parses_correctly() {
        let _guard = CWD_LOCK.lock().unwrap();
        let temp = tempfile::tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&temp).unwrap();

        fs::create_dir_all(".aiplus/agent-team").unwrap();
        fs::write(SENTINEL_PATH, "name: Alice Developer\nemail: alice@example.com\n").unwrap();

        let sentinel = verify_sentinel().unwrap();
        assert_eq!(sentinel.name, "Alice Developer");
        assert_eq!(sentinel.email, "alice@example.com");

        std::env::set_current_dir(original_dir).unwrap();
    }

    // Mutex to serialize tests that modify environment variables
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn empty_passphrase_refused() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("AIPLUS_TEST_GPG_PASSPHRASE", "");
        let result = prompt_passphrase();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
        std::env::remove_var("AIPLUS_TEST_GPG_PASSPHRASE");
    }

    #[test]
    fn short_passphrase_warns_but_allowed() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("AIPLUS_TEST_GPG_PASSPHRASE", "short");
        let result = prompt_passphrase();
        // Should succeed (warning goes to stderr, not error)
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "short");
        std::env::remove_var("AIPLUS_TEST_GPG_PASSPHRASE");
    }

    #[cfg(unix)]
    #[test]
    fn capture_fingerprint_parses_colon_output() {
        let gpg_output = "sec:u:255:22:1234567890ABCDEF:1715424000::::::::::\nfpr:::::::::A1B2C3D4E5F678901234567890ABCDEF12345678:\nuid:u::::1715424000::1234567890ABCDEF::Test User <test@example.com>:::::::::::\n";

        let temp = tempfile::tempdir().unwrap();
        let gnupg = temp.path().join("gnupg");
        fs::create_dir_all(&gnupg).unwrap();

        // Create a fake gpg binary that prints the mock output
        let fake_gpg = temp.path().join("gpg");
        let script = format!("#!/bin/sh\nprintf '%s' '{}'\n", gpg_output.replace('\\', "\\\\").replace('\'', "'\\''"));
        fs::write(&fake_gpg, script).unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&fake_gpg).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_gpg, perms).unwrap();
        }

        let result = capture_fingerprint(fake_gpg.to_str().unwrap(), &gnupg);
        assert!(result.is_ok(), "unexpected error: {:?}", result);
        assert_eq!(result.unwrap(), "A1B2C3D4E5F678901234567890ABCDEF12345678");
    }

    #[cfg(unix)]
    #[test]
    fn capture_fingerprint_no_match_returns_error() {
        let gpg_output = "sec:u:255:22:1234567890ABCDEF:1715424000::::::::::\n";

        let temp = tempfile::tempdir().unwrap();
        let gnupg = temp.path().join("gnupg");
        fs::create_dir_all(&gnupg).unwrap();

        let fake_gpg = temp.path().join("gpg");
        let script = format!("#!/bin/sh\nprintf '%s' '{}'\n", gpg_output.replace('\\', "\\\\").replace('\'', "'\\''"));
        fs::write(&fake_gpg, script).unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&fake_gpg).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_gpg, perms).unwrap();
        }

        let result = capture_fingerprint(fake_gpg.to_str().unwrap(), &gnupg);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("could not find key fingerprint"));
    }
}
