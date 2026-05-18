use crate::agent::cache;
use crate::agent::core::AgentConfig;
use crate::agent::core::{
    aieconlab_alias_help, get_role_config, get_role_config_for_project,
    is_unknown_active_aieconlab_alias, resolve_role_for_active_team,
};
use crate::agent::state;
use crate::agent::worktree_pool::WorktreePool;
use aiplus_core::consult;
use anyhow::{anyhow, Result};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub fn handle_route(
    role: Option<&str>,
    task: &str,
    owner_approved: &[String],
    workflow: Option<&str>,
) -> Result<()> {
    let workflow = parse_route_workflow(workflow)?;
    let approved: BTreeSet<String> = owner_approved.iter().cloned().collect();
    if let Some(candidate) = role {
        let project_root = std::env::current_dir()?;
        if is_unknown_active_aieconlab_alias(&project_root, candidate) {
            return Err(anyhow!(
                "Unknown AiEconLab role alias `{}`. Supported aliases: {}. Canonical role ids continue to work.",
                candidate,
                aieconlab_alias_help()
            ));
        }
        let resolved = resolve_role_for_active_team(&project_root, candidate);
        let canonical_role = resolved.canonical.as_str();
        let role_input = resolved.was_alias.then_some(resolved.input.as_str());
        match get_role_config(canonical_role) {
            Ok(config) => {
                if let Some(input) = role_input {
                    println!("Resolved role alias `{input}` -> `{canonical_role}`");
                }

                if let Some(RouteWorkflow::AuthorCriticFixer) = workflow {
                    let gate_state = enforce_gates(&project_root, canonical_role, task, &approved)?;
                    if gate_state == GateOutcome::PendingBlocked {
                        let _ = state::record_dispatch_with_outcome(
                            &project_root,
                            canonical_role,
                            task,
                            "aiplus agent route --workflow author-critic-fixer",
                            state::DispatchOutcome::Canceled {
                                reason: "owner_gate_pending",
                            },
                        );
                        return Err(anyhow!(
                            "dispatch refused: owner gate not approved; pass --owner-approved <gate-id> to authorize"
                        ));
                    }
                    return run_author_critic_fixer(
                        &project_root,
                        canonical_role,
                        role_input,
                        task,
                        config,
                    );
                }

                // W2: run the gate check *before* worktree provisioning
                // and before recording dispatch. A pending gate cancels
                // the dispatch. P1.3: we still record the canceled
                // attempt so `dispatch-history --outcome canceled` can
                // surface gate-refusal patterns.
                let gate_state = enforce_gates(&project_root, canonical_role, task, &approved)?;
                if gate_state == GateOutcome::PendingBlocked {
                    let _ = state::record_dispatch_with_outcome(
                        &project_root,
                        canonical_role,
                        task,
                        "aiplus agent route",
                        state::DispatchOutcome::Canceled {
                            reason: "owner_gate_pending",
                        },
                    );
                    return Err(anyhow!(
                        "dispatch refused: owner gate not approved; pass --owner-approved <gate-id> to authorize"
                    ));
                }

                let sidecars = requested_sidecars(canonical_role);
                if sidecars.is_empty() {
                    route_known_role(
                        &project_root,
                        canonical_role,
                        role_input,
                        task,
                        config,
                        None,
                        DispatchKind::Primary,
                        None,
                    )?;
                } else {
                    route_batch(
                        &project_root,
                        canonical_role,
                        role_input,
                        task,
                        config,
                        sidecars,
                    )?;
                }
                return Ok(());
            }
            Err(_) => {
                if workflow.is_some() {
                    return Err(anyhow!(
                        "--workflow author-critic-fixer requires a known role; unknown role `{candidate}` cannot run a multi-phase workflow"
                    ));
                }
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
    if workflow.is_some() {
        return Err(anyhow!(
            "--workflow author-critic-fixer requires an explicit ROLE before the task"
        ));
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RouteWorkflow {
    AuthorCriticFixer,
}

fn parse_route_workflow(workflow: Option<&str>) -> Result<Option<RouteWorkflow>> {
    let Some(raw) = workflow else {
        return Ok(None);
    };
    match raw.trim() {
        "author-critic-fixer" => Ok(Some(RouteWorkflow::AuthorCriticFixer)),
        "" => Err(anyhow!("--workflow requires a workflow name")),
        other => Err(anyhow!(
            "unsupported route workflow `{other}`; supported workflow: author-critic-fixer"
        )),
    }
}

fn run_author_critic_fixer(
    project_root: &Path,
    role: &str,
    role_input: Option<&str>,
    task: &str,
    config: AgentConfig,
) -> Result<()> {
    if task.trim().is_empty() {
        return Err(anyhow!(
            "author-critic-fixer workflow requires a non-empty task prompt"
        ));
    }

    let critic_role = author_critic_fixer_critic_role(project_root);
    if critic_role == role {
        return Err(anyhow!(
            "author-critic-fixer requires an independent critic; ROLE `{role}` is the configured critic role"
        ));
    }
    let critic_config = get_role_config_for_project(project_root, critic_role)?;
    let fixer_config = config.clone();
    let workflow_run_id = format!("acf-{}", aiplus_core::epoch_millis());
    println!(
        "Author/Critic/Fixer workflow {workflow_run_id}: author={role} critic={critic_role} fixer={role}"
    );

    let author_agent_id = format!("{workflow_run_id}:author:{role}");
    let author_task = format!(
        "AUTHOR/CRITIC/FIXER phase 1/3 AUTHOR.\n\
         Produce v1 draft for the task below. Stop after the v1 draft; \
         a separate critic will review it before the fixer pass.\n\n\
         Original task:\n{task}"
    );
    println!("  Phase 1/3 author: dispatching {role} for v1 draft");
    route_known_role(
        project_root,
        role,
        role_input,
        &author_task,
        config,
        Some(&workflow_run_id),
        DispatchKind::Primary,
        None,
    )?;
    record_workflow_phase(
        project_root,
        &workflow_run_id,
        "author",
        role,
        &author_agent_id,
        &author_task,
    )?;

    let critic_agent_id = format!("{workflow_run_id}:critic:{critic_role}");
    let critic_task = format!(
        "AUTHOR/CRITIC/FIXER phase 2/3 CRITIC.\n\
         Independently critique the v1 draft requested from `{role}`. \
         Do not rewrite it. Identify correctness, evidence, structure, \
         omission, and escalation issues the fixer must address.\n\n\
         Original task:\n{task}"
    );
    println!("  Phase 2/3 critic: dispatching independent {critic_role}");
    route_known_role(
        project_root,
        critic_role,
        None,
        &critic_task,
        critic_config,
        Some(&workflow_run_id),
        DispatchKind::Sidecar,
        None,
    )?;
    record_workflow_phase(
        project_root,
        &workflow_run_id,
        "critic",
        critic_role,
        &critic_agent_id,
        &critic_task,
    )?;

    let fixer_agent_id = format!("{workflow_run_id}:fixer:{role}");
    let fixer_task = format!(
        "AUTHOR/CRITIC/FIXER phase 3/3 FIXER.\n\
         Produce the v2 draft for the task below, explicitly incorporating \
         the independent `{critic_role}` critique. If the critique surfaces \
         an Owner gate or missing evidence, preserve that escalation instead \
         of smoothing it over.\n\n\
         Original task:\n{task}"
    );
    println!("  Phase 3/3 fixer: dispatching {role} for v2 draft");
    route_known_role(
        project_root,
        role,
        role_input,
        &fixer_task,
        fixer_config,
        Some(&workflow_run_id),
        DispatchKind::Primary,
        None,
    )?;
    record_workflow_phase(
        project_root,
        &workflow_run_id,
        "fixer",
        role,
        &fixer_agent_id,
        &fixer_task,
    )?;

    println!("  Workflow audit recorded: .aiplus/agents/workflow-log.jsonl");
    println!(
        "  v2 draft dispatched to {role}; PI integrates v2 after reviewing the workflow audit."
    );
    Ok(())
}

fn author_critic_fixer_critic_role(project_root: &Path) -> &'static str {
    match crate::agent::set_team::read_active_team(project_root).as_deref() {
        Some("aieconlab") => "referee",
        _ => "reviewer",
    }
}

fn record_workflow_phase(
    project_root: &Path,
    workflow_run_id: &str,
    phase: &str,
    role: &str,
    agent_id: &str,
    task: &str,
) -> Result<()> {
    let path = project_root.join(".aiplus/agents/workflow-log.jsonl");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let line = serde_json::json!({
        "schema_version": "0.1.0",
        "workflow": "author-critic-fixer",
        "workflow_run_id": workflow_run_id,
        "phase": phase,
        "role": role,
        "agent_id": agent_id,
        "task": task,
        "timestamp": aiplus_core::timestamp(),
        "secret_values": "none"
    });
    aiplus_core::append_jsonl_atomic(&path, &line.to_string())?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DispatchKind {
    Primary,
    Sidecar,
}

impl DispatchKind {
    fn as_str(self) -> &'static str {
        match self {
            DispatchKind::Primary => "primary",
            DispatchKind::Sidecar => "sidecar",
        }
    }
}

fn requested_sidecars(primary_role: &str) -> Vec<String> {
    let raw = std::env::var("AIPLUS_AGENT_ROUTE_SIDECARS")
        .or_else(|_| std::env::var("AIPLUS_PERF1_SIDECARS"))
        .unwrap_or_default();
    raw.split(',')
        .map(str::trim)
        .filter(|role| !role.is_empty())
        .filter(|role| *role != primary_role)
        .filter(|role| matches!(*role, "reviewer" | "qa"))
        .map(ToString::to_string)
        .collect()
}

fn route_batch(
    project_root: &Path,
    primary_role: &str,
    primary_role_input: Option<&str>,
    task: &str,
    primary_config: AgentConfig,
    sidecars: Vec<String>,
) -> Result<()> {
    let batch_id = format!(
        "batch-{}-{}",
        aiplus_core::epoch_millis(),
        primary_role.replace('/', "-")
    );
    println!(
        "Dispatch batch {batch_id}: primary={primary_role} sidecars=[{}]",
        sidecars.join(",")
    );

    let pool = Arc::new(Mutex::new(WorktreePool::default()));
    let mut handles = Vec::new();
    {
        let project_root = project_root.to_path_buf();
        let role = primary_role.to_string();
        let role_input = primary_role_input.map(ToString::to_string);
        let task = task.to_string();
        let batch_id = batch_id.clone();
        let pool = Arc::clone(&pool);
        handles.push(thread::spawn(move || {
            route_known_role(
                &project_root,
                &role,
                role_input.as_deref(),
                &task,
                primary_config,
                Some(&batch_id),
                DispatchKind::Primary,
                Some(pool),
            )
        }));
    }

    for sidecar in sidecars {
        let project_root = project_root.to_path_buf();
        let task = sidecar_task(&sidecar, task);
        let batch_id = batch_id.clone();
        let pool = Arc::clone(&pool);
        handles.push(thread::spawn(move || {
            let config = get_role_config_for_project(&project_root, &sidecar)?;
            route_known_role(
                &project_root,
                &sidecar,
                None,
                &task,
                config,
                Some(&batch_id),
                DispatchKind::Sidecar,
                Some(pool),
            )
        }));
    }

    for handle in handles {
        match handle.join() {
            Ok(result) => result?,
            Err(_) => return Err(anyhow!("dispatch batch worker panicked")),
        }
    }
    Ok(())
}

fn sidecar_task(role: &str, task: &str) -> String {
    match role {
        "reviewer" => format!(
            "{task}\n\nPERF-1 sidecar: review the primary implementation plan and flag correctness, safety, and regression risks."
        ),
        "qa" => format!(
            "{task}\n\nPERF-1 sidecar: verify acceptance criteria and list focused test evidence."
        ),
        _ => task.to_string(),
    }
}

fn route_known_role(
    project_root: &Path,
    role: &str,
    role_input: Option<&str>,
    task: &str,
    config: AgentConfig,
    batch_id: Option<&str>,
    kind: DispatchKind,
    pool: Option<Arc<Mutex<WorktreePool>>>,
) -> Result<()> {
    let started = Instant::now();
    maybe_delay_for_perf_fixture(role);
    println!("Routing task to {}: {}", role, task);
    let mut cache_invalidated = false;
    if kind == DispatchKind::Primary {
        if let Ok(cache) = cache::global_cache().lock() {
            cache.invalidate(role, cache::InvalidationReason::RoleRouteCalled);
            cache_invalidated = true;
        }
    }

    let mut worktree_status = "skipped".to_string();
    if config.needs_worktree {
        // Worktree provisioning requires a git repo. If the project
        // isn't one, surface a clear note but still record the
        // dispatch so the audit log entry isn't lost.
        let template = config.worktree_path.as_deref();
        let acquire_result = if let Some(pool) = pool {
            let mut pool = pool
                .lock()
                .map_err(|_| anyhow!("worktree pool lock poisoned"))?;
            pool.acquire(project_root, role, config.needs_worktree, template)
        } else {
            let mut pool = WorktreePool::default();
            pool.acquire(project_root, role, config.needs_worktree, template)
        };
        match acquire_result {
            Ok(lease) => {
                worktree_status = lease.status.as_str().to_string();
                if let Some(path) = lease.path {
                    match worktree_status.as_str() {
                        "created" => {
                            println!("  Creating worktree for {}...", role);
                            println!("  Worktree created: {}", path.display());
                        }
                        "reused" => println!("  Using existing worktree: {}", path.display()),
                        _ => {}
                    }
                }
            }
            Err(e) => {
                worktree_status = "failed".to_string();
                eprintln!("  ERROR: Failed to acquire worktree: {}", e);
                // P1.3: record the failed dispatch so
                // `dispatch-history --outcome fail` can
                // surface worktree-creation regressions.
                let detail = format!("{e}");
                let _ = state::record_dispatch_with_outcome(
                    project_root,
                    role,
                    task,
                    "aiplus agent route",
                    state::DispatchOutcome::Fail {
                        reason: "worktree_create_failed",
                        detail: &detail,
                    },
                );
                record_dispatch_metric(
                    project_root,
                    batch_id,
                    role,
                    kind,
                    "fail",
                    &worktree_status,
                    cache_invalidated,
                    started.elapsed(),
                );
                return Err(e);
            }
        }
    }
    // Persist the dispatch so this becomes a real side effect, not
    // just narrative. Phase D v0: writes audit log + marks role
    // active. v1: mirrors to project memory and surfaces a
    // consultant nudge for medium/heavy tasks.
    let dispatch_result = if role_input.is_some() {
        state::record_dispatch_with_role_input(
            project_root,
            role,
            role_input,
            task,
            "aiplus agent route",
        )
    } else {
        state::record_dispatch(project_root, role, task, "aiplus agent route")
    };
    if let Err(e) = dispatch_result {
        eprintln!("  WARN: failed to record dispatch: {e}");
    } else {
        println!("  Dispatch recorded: .aiplus/agents/dispatch-log.jsonl");
    }
    // S7: surface this role's secret needs so the agent
    // that receives the dispatch knows which broker
    // aliases to pull. We do NOT auto-resolve here (that
    // would require the keyring unlock at every route);
    // we do print the recommended command. Future v1:
    // detect a child process arg and wrap automatically.
    if let Some(ref needs) = config.secret_needs {
        if !needs.aliases.is_empty() {
            let aliases = needs.aliases.join(",");
            println!(
                "  Secret needs (broker-required): [{aliases}]. \
                 Run via: aiplus secret-broker run --aliases {aliases} \
                 -- <child>"
            );
        }
    }
    if !task.is_empty() {
        run_consult(project_root, role, task)?;
    }
    record_dispatch_metric(
        project_root,
        batch_id,
        role,
        kind,
        "success",
        &worktree_status,
        cache_invalidated,
        started.elapsed(),
    );
    Ok(())
}

fn maybe_delay_for_perf_fixture(role: &str) {
    let key = format!(
        "AIPLUS_PERF1_DELAY_{}_MS",
        role.to_ascii_uppercase().replace('-', "_")
    );
    let Ok(raw) = std::env::var(key) else {
        return;
    };
    let Ok(ms) = raw.parse::<u64>() else {
        return;
    };
    if ms > 0 {
        thread::sleep(Duration::from_millis(ms));
    }
}

fn record_dispatch_metric(
    project_root: &Path,
    batch_id: Option<&str>,
    role: &str,
    kind: DispatchKind,
    outcome: &str,
    worktree_status: &str,
    cache_invalidated: bool,
    elapsed: Duration,
) {
    let Some(batch_id) = batch_id else {
        return;
    };
    let path = project_root.join(".aiplus/agents/dispatch-metrics.jsonl");
    let line = serde_json::json!({
        "schemaVersion": "0.1.0",
        "event": "dispatch_batch_role",
        "batchId": batch_id,
        "role": role,
        "kind": kind.as_str(),
        "outcome": outcome,
        "worktree": worktree_status,
        "cacheInvalidated": cache_invalidated,
        "elapsedMs": elapsed.as_millis(),
        "timestamp": aiplus_core::timestamp(),
        "secretValues": "none"
    });
    let _ = aiplus_core::append_jsonl_atomic(&path, &line.to_string());
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
