//! `aiplus agent talk <role>` — open a runtime session in the role's persona.
//!
//! Phase D v0: previously a one-line decorative print. Now this looks up the
//! installed runtime adapter (codex / claude-code / opencode), loads the
//! persona from `.aiplus/agents/personas/<role>.md`, builds a persona-loading
//! prompt, and execs the runtime so the user immediately drops into a session
//! where the agent is in character.

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::agent::core::{
    aieconlab_alias_help, is_unknown_active_aieconlab_alias, resolve_role_for_active_team,
};

pub fn handle_talk(role: &str, runtime_id: Option<&str>) -> Result<()> {
    let project_root = std::env::current_dir()?;
    if is_unknown_active_aieconlab_alias(&project_root, role) {
        return Err(anyhow!(
            "Unknown AiEconLab role alias `{}`. Supported aliases: {}. Canonical role ids continue to work.",
            role,
            aieconlab_alias_help()
        ));
    }
    let resolved = resolve_role_for_active_team(&project_root, role);
    let canonical_role = resolved.canonical.as_str();
    let role_input = resolved.was_alias.then_some(resolved.input.as_str());
    let persona_path = persona_path_for(&project_root, canonical_role)?;
    let persona_text = std::fs::read_to_string(&persona_path).with_context(|| {
        format!(
            "Failed to read persona at {} — is {canonical_role} a known role in this project?",
            persona_path.display()
        )
    })?;

    let runtime = detect_runtime(&project_root, runtime_id)?;
    let prompt = build_talk_prompt(canonical_role, &persona_text);

    let persona_rel = persona_path
        .strip_prefix(&project_root)
        .unwrap_or(&persona_path)
        .display();
    if let Some(input) = role_input {
        println!("Resolved role alias `{input}` -> `{canonical_role}`");
    }
    println!(
        "Opening runtime={} session as role={} — persona loaded from {}",
        runtime.id,
        canonical_role,
        persona_path
            .strip_prefix(&project_root)
            .unwrap_or(&persona_path)
            .display()
    );
    println!(
        "talk_audit runtime={} role={canonical_role} persona={persona_rel}",
        runtime.id
    );
    println!();

    // Replace this process with the runtime so signals and tty pass through.
    let mut cmd = Command::new(&runtime.binary);
    cmd.args(&runtime.args).arg(prompt);
    let status = cmd.status().with_context(|| {
        format!(
            "Failed to spawn `{}` — is it on $PATH? Install it or use a different runtime.",
            runtime.binary
        )
    })?;
    if !status.success() {
        return Err(anyhow!(
            "Runtime `{}` exited with non-zero status: {status}",
            runtime.binary
        ));
    }
    Ok(())
}

fn persona_path_for(project_root: &Path, role: &str) -> Result<PathBuf> {
    let agents_dir = project_root.join(".aiplus").join("agents").join("personas");
    let primary = agents_dir.join(format!("{role}.md"));
    if primary.exists() {
        return Ok(primary);
    }
    // Stub experts live one level deeper.
    let stub = agents_dir.join("_stubs").join(format!("{role}.md"));
    if stub.exists() {
        return Ok(stub);
    }
    Err(anyhow!(
        "Persona file not found for role '{role}' under .aiplus/agents/personas/. \
         Run `aiplus agent list` to see installed roles."
    ))
}

struct Runtime {
    id: &'static str,
    binary: String,
    /// Arguments before the trailing prompt.
    args: Vec<String>,
}

fn detect_runtime(project_root: &Path, selected: Option<&str>) -> Result<Runtime> {
    if let Some(selected) = selected {
        let runtime = selected_runtime_invocation(selected)?;
        let adapters = read_runtime_adapters(project_root);
        if !adapters.is_empty() && !adapters.iter().any(|adapter| adapter == runtime.id) {
            return Err(anyhow!(
                "Runtime `{}` is supported but is not installed for this project. Installed runtimes: {}",
                selected,
                adapters.join(", ")
            ));
        }
        if which(&runtime.binary).is_none() {
            return Err(anyhow!(
                "Selected runtime `{}` uses binary `{}`, but it was not found on $PATH.",
                runtime.id,
                runtime.binary
            ));
        }
        return Ok(runtime);
    }

    // Strategy: look at .aiplus/manifest.json `runtimeAdapters` for the
    // adapters this project has installed. Prefer codex if available, then
    // claude-code, then opencode. If multiple are installed but only one
    // binary is on $PATH, use the one that resolves.
    let adapters = read_runtime_adapters(project_root);
    let candidates: Vec<&str> = if adapters.is_empty() {
        // No manifest yet; try the conventional binaries.
        vec!["codex", "claude", "opencode"]
    } else {
        adapters
            .iter()
            .map(|name| match name.as_str() {
                "codex" => "codex",
                "claude-code" => "claude",
                "opencode" => "opencode",
                other => other,
            })
            .collect()
    };

    for binary in candidates {
        if which(binary).is_some() {
            return Ok(runtime_invocation(binary));
        }
    }
    Err(anyhow!(
        "No supported runtime CLI found on $PATH. Install one of: codex, claude, opencode."
    ))
}

fn runtime_invocation(binary: &str) -> Runtime {
    match binary {
        "codex" => Runtime {
            id: "codex",
            binary: "codex".to_string(),
            // The top-level interactive codex doesn't accept
            // --skip-git-repo-check (only `codex exec` does), so we pass
            // no flags and the user gets prompted if the project isn't a
            // git repo. That prompt is appropriate first-run UX.
            args: vec![],
        },
        "claude" => Runtime {
            id: "claude-code",
            binary: "claude".to_string(),
            args: vec![],
        },
        "opencode" => Runtime {
            id: "opencode",
            binary: "opencode".to_string(),
            args: vec![],
        },
        other => Runtime {
            id: "runtime",
            binary: other.to_string(),
            args: vec![],
        },
    }
}

fn selected_runtime_invocation(runtime_id: &str) -> Result<Runtime> {
    match runtime_id {
        "codex" => Ok(runtime_invocation("codex")),
        "claude-code" => Ok(runtime_invocation("claude")),
        "opencode" => Ok(runtime_invocation("opencode")),
        other => Err(anyhow!(
            "Invalid runtime `{}`. Supported runtime ids: codex, claude-code, opencode.",
            other
        )),
    }
}

fn read_runtime_adapters(project_root: &Path) -> Vec<String> {
    let manifest_path = project_root.join(".aiplus").join("manifest.json");
    let Ok(text) = std::fs::read_to_string(&manifest_path) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    value
        .get("runtimeAdapters")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn which(binary: &str) -> Option<PathBuf> {
    let path_env = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_env) {
        let candidate = dir.join(binary);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn build_talk_prompt(role: &str, persona: &str) -> String {
    format!(
        "You are the `{role}` of the AiPlus virtual team installed in this project. \
         Adopt this persona for the entire conversation that follows. Stay in character. \
         Honor the persona's Forbidden Actions and Escalation rules — escalate STOP-gated \
         actions to the Owner instead of executing them. \
         The persona below is the binding spec for your behavior:\n\n\
         ---\n{persona}\n---\n\n\
         The Owner is now speaking to you. Wait for their first message, then respond \
         in character. If they have not yet said anything, give a brief greeting (1-2 \
         sentences) and ask what they need.",
    )
}
