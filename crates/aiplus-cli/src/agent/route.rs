use crate::agent::cache;
use crate::agent::core::get_role_config;
use crate::agent::state;
use crate::agent::worktree;
use aiplus_core::consult;
use anyhow::{anyhow, Result};
use std::collections::BTreeSet;

pub fn handle_route(role: Option<&str>, task: &str, owner_approved: &[String]) -> Result<()> {
    let approved: BTreeSet<String> = owner_approved.iter().cloned().collect();
    if let Some(candidate) = role {
        match get_role_config(candidate) {
            Ok(config) => {
                let project_root = std::env::current_dir()?;

                // W2: run the gate check *before* worktree provisioning
                // and before recording dispatch. A pending gate means
                // the dispatch itself doesn't happen — we don't want
                // .aiplus/agents/dispatch-log.jsonl to record a dispatch
                // that the gate refused.
                let gate_state = enforce_gates(&project_root, candidate, task, &approved)?;
                if gate_state == GateOutcome::PendingBlocked {
                    return Err(anyhow!(
                        "dispatch refused: owner gate not approved; pass --owner-approved <gate-id> to authorize"
                    ));
                }

                println!("Routing task to {}: {}", candidate, task);
                if let Ok(cache) = cache::global_cache().lock() {
                    cache.invalidate(candidate, cache::InvalidationReason::RoleRouteCalled);
                }
                if config.needs_worktree {
                    // Worktree provisioning requires a git repo. If the project
                    // isn't one, surface a clear note but still record the
                    // dispatch so the audit log entry isn't lost.
                    match worktree::worktree_exists_for_role(&project_root, candidate) {
                        Ok(Some(path)) => {
                            println!("  Using existing worktree: {}", path.display());
                        }
                        Ok(None) => {
                            println!("  Creating worktree for {}...", candidate);
                            let template = config.worktree_path.as_deref();
                            match worktree::create_worktree(&project_root, candidate, template) {
                                Ok(path) => {
                                    println!("  Worktree created: {}", path.display());
                                }
                                Err(e) => {
                                    eprintln!("  ERROR: Failed to create worktree: {}", e);
                                    return Err(e);
                                }
                            }
                        }
                        Err(err) => {
                            // Likely "not a git repository" — non-fatal. Warn
                            // and continue so we still record the dispatch.
                            eprintln!(
                                "  NOTE: skipping worktree provisioning ({err}); \
                                 init this project with `git init` to enable per-role worktrees."
                            );
                        }
                    }
                }
                // Persist the dispatch so this becomes a real side effect, not
                // just narrative. Phase D v0: writes audit log + marks role
                // active. v1: mirrors to project memory and surfaces a
                // consultant nudge for medium/heavy tasks.
                if let Err(e) =
                    state::record_dispatch(&project_root, candidate, task, "aiplus agent route")
                {
                    eprintln!("  WARN: failed to record dispatch: {e}");
                } else {
                    println!("  Dispatch recorded: .aiplus/agents/dispatch-log.jsonl");
                }
                if !task.is_empty() {
                    run_consult(&project_root, candidate, task)?;
                }
                return Ok(());
            }
            Err(_) => {
                // Not a known role — rebuild the full free-form task and
                // route to PI/CEO for scoring.
                let full_task = if task.is_empty() {
                    candidate.to_string()
                } else {
                    format!("{candidate} {task}")
                };
                let project_root = std::env::current_dir()?;
                let gate_state = enforce_gates(&project_root, "pi", &full_task, &approved)?;
                if gate_state == GateOutcome::PendingBlocked {
                    return Err(anyhow!(
                        "dispatch refused: owner gate not approved; pass --owner-approved <gate-id> to authorize"
                    ));
                }
                println!("Routing task to PI/CEO for scoring and dispatch: {full_task}");
                run_consult(&project_root, "pi", &full_task)?;
                return Ok(());
            }
        }
    }
    let project_root = std::env::current_dir()?;
    let gate_state = enforce_gates(&project_root, "pi", task, &approved)?;
    if gate_state == GateOutcome::PendingBlocked {
        return Err(anyhow!(
            "dispatch refused: owner gate not approved; pass --owner-approved <gate-id> to authorize"
        ));
    }
    println!("Routing task to PI/CEO for scoring and dispatch: {task}");
    run_consult(&project_root, "pi", task)?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GateOutcome {
    NoGates,
    AllApproved,
    PendingBlocked,
}

/// W2 contract: check whether the task crosses any owner gate. Writes
/// `gate_pending` or `gate_approved` records to
/// `.aiplus/agent-memory/_team/gates-<task-id>.jsonl` either way so the
/// audit trail captures both refusals and approvals. The function
/// returns the outcome but does NOT exit — the caller propagates the
/// block (Err return → CLI exit non-zero).
fn enforce_gates(
    project_root: &std::path::Path,
    role: &str,
    task: &str,
    approved: &BTreeSet<String>,
) -> Result<GateOutcome> {
    if task.is_empty() {
        return Ok(GateOutcome::NoGates);
    }
    let team = match consult::load_consult_team(project_root) {
        Ok(Some(team)) => team,
        Ok(None) => return Ok(GateOutcome::NoGates),
        Err(_) => return Ok(GateOutcome::NoGates),
    };
    if !consult::is_supported_schema(&team.schema_version) {
        return Ok(GateOutcome::NoGates);
    }
    let today = aiplus_core::now_iso();
    let date_salt: String = today.chars().take(10).collect();
    let task_id = consult::derive_task_id(role, task, &date_salt);
    let complexity = consult::score_complexity(task);
    let risk = consult::score_risk(task);
    let tier = consult::select_tier(complexity, risk);
    let matched_members = consult::match_members(&team, task, tier);
    let fired = consult::match_gates(&team, &matched_members, task);

    if fired.is_empty() {
        return Ok(GateOutcome::NoGates);
    }

    let approver = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_default();

    let mut records: Vec<consult::GateRecord> = Vec::new();
    let mut any_pending = false;
    println!("  Owner gate(s) fired for this task:");
    for gate in &fired {
        let is_approved = approved.contains(&gate.gate_id);
        let status = if is_approved { "approved" } else { "pending" };
        if !is_approved {
            any_pending = true;
        }
        println!(
            "    [{}] {}: {} ({})",
            status, gate.gate_id, gate.description, gate.source,
        );
        records.push(consult::GateRecord {
            schema_version: consult::GATE_RECORD_SCHEMA_VERSION.to_string(),
            timestamp: today.clone(),
            task_id: task_id.clone(),
            task: task.to_string(),
            gate_id: gate.gate_id.clone(),
            description: gate.description.clone(),
            source: gate.source.clone(),
            status: status.to_string(),
            approved_by: if is_approved {
                approver.clone()
            } else {
                String::new()
            },
        });
    }
    if let Err(e) = consult::write_gate_ledger(project_root, &task_id, &records) {
        eprintln!("  WARN: failed to write gate ledger: {e}");
    } else {
        let path = consult::gates_path(project_root, &task_id);
        let rel = path
            .strip_prefix(project_root)
            .map(|p| p.to_path_buf())
            .unwrap_or(path.clone());
        println!("  Gate ledger: {}", rel.display());
    }

    if any_pending {
        eprintln!(
            "  Dispatch refused: pass `--owner-approved <gate-id>` (one flag per gate) to authorize."
        );
        return Ok(GateOutcome::PendingBlocked);
    }
    Ok(GateOutcome::AllApproved)
}

/// Walk the consultant team for `task` and persist per-member findings
/// under `.aiplus/agent-memory/_team/consult-<task-id>.jsonl`. The
/// JSONL is what makes the consult a real side effect instead of a
/// narrative — downstream `agent transcript` reads it, and W2 (owner
/// gates) will gate dispatch on what it finds.
///
/// Failure to consult is intentionally non-fatal: missing config or
/// unsupported schema prints a NOTE and lets dispatch continue. The
/// goal is "consult-as-side-effect when possible," not "force every
/// route through a complete consult."
fn run_consult(project_root: &std::path::Path, role: &str, task: &str) -> Result<()> {
    let team = match consult::load_consult_team(project_root) {
        Ok(Some(team)) => team,
        Ok(None) => {
            return Ok(());
        }
        Err(e) => {
            eprintln!(
                "  NOTE: consultant team config could not be loaded ({e}); skipping consult."
            );
            return Ok(());
        }
    };
    if !consult::is_supported_schema(&team.schema_version) {
        eprintln!(
            "  NOTE: consultant-team.toml schema_version='{}' not in the supported list \
             ({:?}); skipping consult. Run `aiplus doctor` for guidance.",
            team.schema_version,
            consult::SUPPORTED_CONSULT_SCHEMAS,
        );
        return Ok(());
    }
    let today = aiplus_core::now_iso();
    // YYYY-MM-DD slice as the task-id salt so re-running the same
    // command on the same day yields the same id (idempotent), while
    // re-running tomorrow opens a fresh consult file.
    let date_salt: String = today.chars().take(10).collect();
    let task_id = consult::derive_task_id(role, task, &date_salt);
    let (tier, complexity, risk, findings) = consult::build_findings(&team, task, &task_id, &today);

    if findings.is_empty() {
        println!(
            "  Consult tier: {} (complexity {}, risk {:.2}). No member triggers matched — skipping artifact.",
            tier.as_str(),
            complexity,
            risk,
        );
        return Ok(());
    }
    match consult::write_findings(project_root, &task_id, &findings) {
        Ok(path) => {
            // Convert to a project-relative path for the on-screen
            // hint; absolute paths are noisy and break copy/paste.
            let rel = path
                .strip_prefix(project_root)
                .map(|p| p.to_path_buf())
                .unwrap_or(path.clone());
            println!(
                "  Consult tier: {} (complexity {}, risk {:.2}). {} finding(s) recorded: {}",
                tier.as_str(),
                complexity,
                risk,
                findings.len(),
                rel.display(),
            );
        }
        Err(e) => {
            eprintln!("  WARN: failed to write consult findings: {e}");
        }
    }
    Ok(())
}
