use crate::agent::cache;
use crate::agent::coordinator::{self, CoordinatorTier};
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
    score_only: bool,
) -> Result<()> {
    let workflow = parse_route_workflow(workflow)?;
    maybe_print_route_first_run_hint();
    if score_only {
        if workflow.is_some() {
            return Err(anyhow!("--score-only cannot be combined with --workflow"));
        }
        let project_root = std::env::current_dir()?;
        let full_task = joined_route_task(role, task);
        return run_score_only_route(&project_root, &full_task);
    }
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
                    let _ = route_known_role(RouteKnownRoleArgs {
                        project_root: &project_root,
                        role: canonical_role,
                        role_input,
                        task,
                        config,
                        batch_id: None,
                        kind: DispatchKind::Primary,
                        pool: None,
                        consult_after_dispatch: true,
                    })?;
                } else {
                    let _ = route_batch(
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
                return run_adaptive_route(&project_root, &full_task, &approved).map(|_| ());
            }
        }
    }
    if workflow.is_some() {
        return Err(anyhow!(
            "--workflow author-critic-fixer requires an explicit ROLE before the task"
        ));
    }
    let project_root = std::env::current_dir()?;
    run_adaptive_route(&project_root, task, &approved).map(|_| ())
}

fn run_adaptive_route(
    project_root: &Path,
    task: &str,
    approved: &BTreeSet<String>,
) -> Result<Vec<AdapterResult>> {
    let task = task.trim();
    if task.is_empty() {
        return Err(anyhow!("agent route requires a task when ROLE is omitted"));
    }

    let plan = coordinator::plan_task_for_project(project_root, task)?;
    let staffing = if plan.staffing_roles.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", plan.staffing_roles.join(","))
    };
    println!(
        "Adaptive coordinator: complexity={} risk={:.2} tier={} code_change={} design_impact={} consultant={}",
        plan.score.complexity,
        plan.score.risk,
        plan.tier.as_str(),
        plan.score.requires_code_change,
        plan.score.design_impact,
        if plan.fire_consultant { "fire" } else { "skip" },
    );
    println!("Staffing roles: {staffing}");
    if !plan.forced_by_risk.is_empty() {
        println!("Forced by risk: [{}]", plan.forced_by_risk.join(","));
    }
    if !plan.auto_summoned.is_empty() {
        println!("Auto-summoned experts: [{}]", plan.auto_summoned.join(","));
    }
    record_coordinator_decision(project_root, task, &plan, "route")?;

    let gate_state = enforce_gates(project_root, "ceo", task, approved)?;
    if gate_state == GateOutcome::PendingBlocked {
        return Err(anyhow!(
            "dispatch refused: owner gate not approved; pass --owner-approved <gate-id> to authorize"
        ));
    }

    if plan.fire_consultant {
        println!(
            "Plan step: firing consultant for {} task",
            plan.tier.as_str()
        );
        run_consult(project_root, "ceo", task)?;
    } else {
        println!("Plan step: consultant skipped for {}", plan.tier.as_str());
    }

    if plan.tier == CoordinatorTier::LightNoCode {
        println!("Execute step: CEO handles directly; no worktree staffing required.");
        let dispatch_id = state::record_dispatch(
            project_root,
            "ceo",
            task,
            "aiplus agent route adaptive-coordinator",
        )?;
        return Ok(vec![AdapterResult::dispatch_only(dispatch_id)]);
    }

    let batch_id = format!(
        "coord-{}-{}",
        aiplus_core::epoch_millis(),
        plan.tier.as_str().to_ascii_lowercase()
    );
    println!(
        "Execute step: dispatching {} staffed role(s) with batch {batch_id}",
        plan.staffing_roles.len()
    );
    let results = coordinator_batch(project_root, task, &plan.staffing_roles, &batch_id)?;
    println!(
        "Adaptive coordinator dispatch complete: tier={}",
        plan.tier.as_str()
    );
    Ok(results)
}

fn run_score_only_route(project_root: &Path, task: &str) -> Result<()> {
    let task = task.trim();
    if task.is_empty() {
        return Err(anyhow!("agent route --score-only requires a task"));
    }

    let plan = coordinator::plan_task_for_project(project_root, task)?;
    let staffing = if plan.staffing_roles.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", plan.staffing_roles.join(","))
    };
    println!(
        "Adaptive coordinator: complexity={} risk={:.2} tier={} code_change={} design_impact={} consultant={}",
        plan.score.complexity,
        plan.score.risk,
        plan.tier.as_str(),
        plan.score.requires_code_change,
        plan.score.design_impact,
        if plan.fire_consultant { "fire" } else { "skip" },
    );
    if plan.fire_consultant {
        println!(
            "Plan step: would fire consultant for {} task",
            plan.tier.as_str()
        );
    } else {
        println!(
            "Plan step: consultant would be skipped for {}",
            plan.tier.as_str()
        );
    }
    println!("Would staff: {staffing}");
    if !plan.forced_by_risk.is_empty() {
        println!("Forced by risk: [{}]", plan.forced_by_risk.join(","));
    }
    if !plan.auto_summoned.is_empty() {
        println!("Auto-summoned experts: [{}]", plan.auto_summoned.join(","));
    }
    record_coordinator_decision(project_root, task, &plan, "score_only")?;
    Ok(())
}

fn joined_route_task(role: Option<&str>, task: &str) -> String {
    match (role.map(str::trim).filter(|s| !s.is_empty()), task.trim()) {
        (Some(role), "") => role.to_string(),
        (Some(role), task) => format!("{role} {task}"),
        (None, task) => task.to_string(),
    }
}

fn record_coordinator_decision(
    project_root: &Path,
    task: &str,
    plan: &coordinator::CoordinatorPlan,
    mode: &str,
) -> Result<String> {
    let path = project_root.join(".aiplus/agents/dispatch-log.jsonl");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let ttl_expired = coordinator_decision_ttl_expired(project_root, plan, mode);
    let decision_id = format!("coord-{}", aiplus_core::epoch_millis());
    let line = serde_json::json!({
        "schemaVersion": "0.4.0",
        "event": "coordinator_decision",
        "decisionId": decision_id,
        "timestamp": aiplus_core::now_iso(),
        "mode": mode,
        "taskExcerpt": task_excerpt(task),
        "complexity": plan.score.complexity,
        "risk": plan.score.risk,
        "tier": plan.tier.as_str(),
        "codeChange": plan.score.requires_code_change,
        "designImpact": plan.score.design_impact,
        "consultant": if plan.fire_consultant { "fire" } else { "skip" },
        "staffingRoles": plan.staffing_roles,
        "forced_by_risk": plan.forced_by_risk,
        "auto_summoned": plan.auto_summoned,
        "ttl_expired": ttl_expired,
        "dispatched": mode == "route" && !plan.staffing_roles.is_empty(),
        "secretValues": "none"
    });
    let mut line = line;
    crate::agent::audit::verify_log::append_chained_jsonl_value(&path, &mut line)?;
    Ok(decision_id)
}

fn coordinator_decision_ttl_expired(
    project_root: &Path,
    plan: &coordinator::CoordinatorPlan,
    mode: &str,
) -> serde_json::Value {
    if mode == "score_only" {
        return serde_json::Value::Null;
    }
    let mut any_expired = false;
    for role in &plan.staffing_roles {
        let Ok(config) = get_role_config_for_project(project_root, role) else {
            continue;
        };
        let Ok(Some(expired)) = cache::disk_cache_ttl_expired_for_role(
            project_root,
            role,
            config.warm_bench_ttl_seconds,
        ) else {
            continue;
        };
        any_expired |= expired;
    }
    serde_json::Value::Bool(any_expired)
}

fn task_excerpt(task: &str) -> String {
    let redacted = task
        .lines()
        .map(|line| {
            if aiplus_core::reject_sensitive_memory_text(line).is_err() {
                "[REDACTED]"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let normalized = redacted.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= 200 {
        normalized
    } else {
        let prefix: String = normalized.chars().take(200).collect();
        format!("{prefix}...")
    }
}

fn maybe_print_route_first_run_hint() {
    let Some(marker) = route_first_run_marker() else {
        return;
    };
    if marker.exists() {
        return;
    }
    println!(
        "Hint: for BWS-backed runtime keys, wrap live dispatch with `aiplus secret-broker run --aliases anthropic,openai -- aiplus agent route \"<task>\"`."
    );
    if marker_is_inside_current_project(&marker) {
        return;
    }
    if let Some(parent) = marker.parent() {
        if std::fs::create_dir_all(parent).is_ok() {
            let _ = std::fs::write(marker, b"seen\n");
        }
    }
}

fn route_first_run_marker() -> Option<std::path::PathBuf> {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.trim().is_empty() {
            return Some(std::path::PathBuf::from(xdg).join("aiplus/.route_first_run_seen"));
        }
    }
    std::env::var("HOME")
        .ok()
        .filter(|home| !home.trim().is_empty())
        .map(|home| std::path::PathBuf::from(home).join(".config/aiplus/.route_first_run_seen"))
}

fn marker_is_inside_current_project(marker: &Path) -> bool {
    let Ok(project_root) = std::env::current_dir() else {
        return false;
    };
    if marker.starts_with(&project_root) {
        return true;
    }

    let Ok(project_root) = std::fs::canonicalize(project_root) else {
        return false;
    };
    if let Some(parent) = marker.parent() {
        if let Ok(parent) = std::fs::canonicalize(parent) {
            return parent.starts_with(project_root);
        }
    }
    false
}

fn coordinator_role_task(role: &str, position: usize, total: usize, task: &str) -> String {
    format!(
        "ADAPTIVE COORDINATOR P0 staffed role {position}/{total}: `{role}`.\n\
         Work only within your role responsibilities. Coordinate through CEO; do not perform Owner-gated actions.\n\n\
         Original task:\n{task}"
    )
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
    route_known_role(RouteKnownRoleArgs {
        project_root,
        role,
        role_input,
        task: &author_task,
        config,
        batch_id: Some(&workflow_run_id),
        kind: DispatchKind::Primary,
        pool: None,
        consult_after_dispatch: true,
    })?;
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
    route_known_role(RouteKnownRoleArgs {
        project_root,
        role: critic_role,
        role_input: None,
        task: &critic_task,
        config: critic_config,
        batch_id: Some(&workflow_run_id),
        kind: DispatchKind::Sidecar,
        pool: None,
        consult_after_dispatch: true,
    })?;
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
    route_known_role(RouteKnownRoleArgs {
        project_root,
        role,
        role_input,
        task: &fixer_task,
        config: fixer_config,
        batch_id: Some(&workflow_run_id),
        kind: DispatchKind::Primary,
        pool: None,
        consult_after_dispatch: true,
    })?;
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
    CoordinatorPeer,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AdapterResult {
    pub schema_version: String,
    pub session_id: String,
    pub stdout_raw: String,
    pub tool_calls: Option<serde_json::Value>,
    pub final_text: String,
    pub usage_tokens: Option<serde_json::Value>,
    pub exit_status: String,
    pub partial: bool,
}

impl AdapterResult {
    fn dispatch_only(session_id: String) -> Self {
        Self {
            schema_version: "1.0".to_string(),
            session_id,
            stdout_raw: String::new(),
            tool_calls: None,
            final_text: String::new(),
            usage_tokens: None,
            exit_status: "OK".to_string(),
            partial: false,
        }
    }
}

impl DispatchKind {
    fn as_str(self) -> &'static str {
        match self {
            DispatchKind::Primary => "primary",
            DispatchKind::Sidecar => "sidecar",
            DispatchKind::CoordinatorPeer => "coordinator_peer",
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
) -> Result<Vec<AdapterResult>> {
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
            route_known_role(RouteKnownRoleArgs {
                project_root: &project_root,
                role: &role,
                role_input: role_input.as_deref(),
                task: &task,
                config: primary_config,
                batch_id: Some(&batch_id),
                kind: DispatchKind::Primary,
                pool: Some(pool),
                consult_after_dispatch: true,
            })
        }));
    }

    for sidecar in sidecars {
        let project_root = project_root.to_path_buf();
        let task = sidecar_task(&sidecar, task);
        let batch_id = batch_id.clone();
        let pool = Arc::clone(&pool);
        handles.push(thread::spawn(move || {
            let config = get_role_config_for_project(&project_root, &sidecar)?;
            route_known_role(RouteKnownRoleArgs {
                project_root: &project_root,
                role: &sidecar,
                role_input: None,
                task: &task,
                config,
                batch_id: Some(&batch_id),
                kind: DispatchKind::Sidecar,
                pool: Some(pool),
                consult_after_dispatch: true,
            })
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        match handle.join() {
            Ok(result) => results.push(result?),
            Err(_) => return Err(anyhow!("dispatch batch worker panicked")),
        }
    }
    Ok(results)
}

fn coordinator_batch(
    project_root: &Path,
    task: &str,
    staffing_roles: &[String],
    batch_id: &str,
) -> Result<Vec<AdapterResult>> {
    println!(
        "Coordinator batch {batch_id}: peers=[{}]",
        staffing_roles.join(",")
    );

    let pool = Arc::new(Mutex::new(WorktreePool::default()));
    let total = staffing_roles.len();
    let mut handles = Vec::new();

    for (idx, role) in staffing_roles.iter().enumerate() {
        let project_root = project_root.to_path_buf();
        let role = role.clone();
        let role_task = coordinator_role_task(&role, idx + 1, total, task);
        let batch_id = batch_id.to_string();
        let pool = Arc::clone(&pool);
        handles.push((
            role.clone(),
            thread::spawn(move || {
                let config = get_role_config_for_project(&project_root, &role)?;
                route_known_role(RouteKnownRoleArgs {
                    project_root: &project_root,
                    role: &role,
                    role_input: None,
                    task: &role_task,
                    config,
                    batch_id: Some(&batch_id),
                    kind: DispatchKind::CoordinatorPeer,
                    pool: Some(pool),
                    consult_after_dispatch: false,
                })
            }),
        ));
    }

    let mut failures = Vec::new();
    let mut results = Vec::new();
    for (role, handle) in handles {
        match handle.join() {
            Ok(Ok(result)) => results.push(result),
            Ok(Err(e)) => failures.push(format!("{role}: {e}")),
            Err(_) => failures.push(format!("{role}: worker panicked")),
        }
    }

    if failures.is_empty() {
        return Ok(results);
    }

    println!(
        "Coordinator batch {batch_id} completed with partial failure(s): {}",
        failures.join("; ")
    );
    Err(anyhow!(
        "coordinator batch partial failure: {}",
        failures.join("; ")
    ))
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

struct RouteKnownRoleArgs<'a> {
    project_root: &'a Path,
    role: &'a str,
    role_input: Option<&'a str>,
    task: &'a str,
    config: AgentConfig,
    batch_id: Option<&'a str>,
    kind: DispatchKind,
    pool: Option<Arc<Mutex<WorktreePool>>>,
    consult_after_dispatch: bool,
}

fn route_known_role(args: RouteKnownRoleArgs<'_>) -> Result<AdapterResult> {
    let RouteKnownRoleArgs {
        project_root,
        role,
        role_input,
        task,
        config,
        batch_id,
        kind,
        pool,
        consult_after_dispatch,
    } = args;
    let started = Instant::now();
    maybe_delay_for_perf_fixture(role);
    println!("Routing task to {}: {}", role, task);
    if let Some(reason) = perf_fixture_failure(role) {
        let _ = state::record_dispatch_with_outcome(
            project_root,
            role,
            task,
            "aiplus agent route",
            state::DispatchOutcome::Fail {
                reason: "perf_fixture_failure",
                detail: &reason,
            },
        );
        record_dispatch_metric(
            project_root,
            DispatchMetric {
                batch_id,
                role,
                kind,
                outcome: "fail",
                worktree_status: "fixture_failed",
                cache_invalidated: false,
                elapsed: started.elapsed(),
            },
        );
        return Err(anyhow!("{reason}"));
    }
    let cache_source =
        match cache::lookup_disk_snapshot(project_root, role, config.warm_bench_ttl_seconds) {
            Ok(source) => source,
            Err(e) => {
                eprintln!("  WARN: disk cache lookup failed; cold-starting: {e}");
                cache::CacheSource::ColdStart
            }
        };
    if matches!(
        cache_source,
        cache::CacheSource::DiskWarm | cache::CacheSource::ColdStart
    ) {
        println!("  cache_source={}", cache_source.as_str());
    }
    let mut cache_invalidated = false;
    if matches!(kind, DispatchKind::Primary | DispatchKind::CoordinatorPeer) {
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
                    DispatchMetric {
                        batch_id,
                        role,
                        kind,
                        outcome: "fail",
                        worktree_status: &worktree_status,
                        cache_invalidated,
                        elapsed: started.elapsed(),
                    },
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
    let dispatch_id = match dispatch_result {
        Ok(dispatch_id) => {
            println!("  Dispatch recorded: .aiplus/agents/dispatch-log.jsonl");
            dispatch_id
        }
        Err(e) => {
            eprintln!("  WARN: failed to record dispatch: {e}");
            format!("dispatch-unrecorded-{}-{role}", aiplus_core::epoch_millis())
        }
    };
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
    if consult_after_dispatch && !task.is_empty() {
        run_consult(project_root, role, task)?;
    }
    if let Err(e) = cache::write_disk_snapshot(project_root, role, &config, cache_source) {
        eprintln!("  WARN: failed to write disk cache snapshot: {e}");
    }
    record_dispatch_metric(
        project_root,
        DispatchMetric {
            batch_id,
            role,
            kind,
            outcome: "success",
            worktree_status: &worktree_status,
            cache_invalidated,
            elapsed: started.elapsed(),
        },
    );
    Ok(AdapterResult::dispatch_only(dispatch_id))
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

fn perf_fixture_failure(role: &str) -> Option<String> {
    let raw = std::env::var("AIPLUS_PERF1_FAIL_ROLE")
        .or_else(|_| std::env::var("AIPLUS_COORDINATOR_FAIL_ROLE"))
        .unwrap_or_default();
    let matched = raw
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .any(|value| value == role);
    matched.then(|| format!("perf fixture requested failure for role {role}"))
}

struct DispatchMetric<'a> {
    batch_id: Option<&'a str>,
    role: &'a str,
    kind: DispatchKind,
    outcome: &'a str,
    worktree_status: &'a str,
    cache_invalidated: bool,
    elapsed: Duration,
}

fn record_dispatch_metric(project_root: &Path, metric: DispatchMetric<'_>) {
    let DispatchMetric {
        batch_id,
        role,
        kind,
        outcome,
        worktree_status,
        cache_invalidated,
        elapsed,
    } = metric;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_only_adapter_result_matches_contract_shape() {
        let result = AdapterResult::dispatch_only("dispatch-1-engineer-a".to_string());

        assert_eq!(result.schema_version, "1.0");
        assert_eq!(result.session_id, "dispatch-1-engineer-a");
        assert_eq!(result.exit_status, "OK");
        assert!(!result.partial);
        assert!(result.stdout_raw.is_empty());
        assert!(result.final_text.is_empty());
        assert!(result.tool_calls.is_none());
        assert!(result.usage_tokens.is_none());
    }
}
