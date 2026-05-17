//! Consultant-team engine.
//!
//! Before this module, `aiplus agent route` could only print a "tier" hint;
//! the consult itself ran in spec, not on disk. W1 makes the consult a real
//! side effect: load `.aiplus/consultant-team.toml`, score complexity/risk
//! from the task description, walk the matched members per tier, and write
//! one JSONL finding per member to
//! `.aiplus/agent-memory/_team/consult-<task-id>.jsonl`.
//!
//! Two on-disk schemas are supported in parallel because the bundled
//! configs target different domains:
//!
//! * SWE default (`consultant-team.default.toml`, `schema_version = "0.1"`):
//!   members are bare `id`/`name`; trigger keywords live in separate
//!   `[[triggers]]` blocks pointing at members; `[owner_gates]` is a flat
//!   dict of `<gate-name> = true`; no `[scaling]` block.
//! * AEL (`consultant-team.aieconlab.toml`, `schema_version = "2.1"`):
//!   each `[[members]]` carries its own `triggers` list and `owner_gate`
//!   flag inline; `[owner_gates].gates` is an array of `{id, description}`;
//!   `[scaling]` and `[scoring.*]` blocks tune the staffing rules.
//!
//! We parse both shapes into the same normalized `ConsultTeam` so the
//! rest of the engine doesn't care which file the project happens to
//! ship.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const CONSULT_TEAM_PATH: &str = ".aiplus/consultant-team.toml";
pub const CONSULT_FINDINGS_DIR: &str = ".aiplus/agent-memory/_team";
pub const CONSULT_RECORD_SCHEMA_VERSION: &str = "0.1.0";
pub const GATE_RECORD_SCHEMA_VERSION: &str = "0.1.0";

/// Versions of `consultant-team.toml` that this build knows how to
/// load. The doctor uses this list to flag drift early instead of
/// letting `agent route` silently treat an unknown schema as "no
/// team".
pub const SUPPORTED_CONSULT_SCHEMAS: &[&str] = &["0.1", "2.1"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Tier {
    Light,
    Medium,
    Heavy,
}

impl Tier {
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Light => "LIGHT",
            Tier::Medium => "MEDIUM",
            Tier::Heavy => "HEAVY",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Member {
    pub id: String,
    pub name: String,
    /// Keywords that, when present in the task, mark this member as
    /// applicable. SWE-shape configs feed these from external
    /// `[[triggers]]` blocks; AEL-shape configs read them inline from
    /// the member's own `triggers` field.
    pub triggers: Vec<String>,
    /// Tiers at which this member is eligible to be staffed. Empty
    /// means "all tiers".
    pub default_tiers: Vec<Tier>,
    /// Marker that a "no" from this member must escalate to Owner. W2
    /// reads this to refuse dispatch unless `--owner-approved` is set.
    pub owner_gate: bool,
    /// Output artifact the member is expected to produce. AEL pins
    /// these by name; SWE configs leave it None and the engine just
    /// records the member's id.
    pub output_artifact: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserPersona {
    pub id: String,
    pub name: String,
    pub triggers: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OwnerGate {
    pub id: String,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct ConsultTeam {
    pub schema_version: String,
    pub members: Vec<Member>,
    pub user_personas: Vec<UserPersona>,
    pub owner_gates: Vec<OwnerGate>,
    /// SWE-shape configs declare per-trigger STOP gates via
    /// `[[triggers]] stop_gate = true`. We keep those distinct from
    /// `member.owner_gate` (which we reserve for AEL-shape inline
    /// declarations on the member itself) so a stop_gate fires only
    /// when the trigger pattern actually matches the task, not as a
    /// permanent property of every member that trigger names.
    pub stop_gate_triggers: Vec<StopGateTrigger>,
}

/// A `[[triggers]]` block whose `stop_gate = true`. Used by match_gates
/// to fire a gate when (a) one of the patterns substring-matches the
/// task and (b) at least one of the trigger's named members is in the
/// matched-members set.
#[derive(Debug, Clone)]
pub struct StopGateTrigger {
    pub id: String,
    pub patterns: Vec<String>,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsultFinding {
    pub schema_version: String,
    pub timestamp: String,
    pub task_id: String,
    pub task: String,
    pub tier: String,
    pub complexity: u8,
    pub risk: f32,
    pub member_id: String,
    pub member_name: String,
    pub triggers_matched: Vec<String>,
    pub output_artifact: Option<String>,
    pub kind: String,
}

/// One row in the gate ledger written by `aiplus agent route` whenever
/// an owner-gated action would have to be taken. `status` is one of
/// `"pending"` (dispatch was refused, gate must be approved) or
/// `"approved"` (the user passed `--owner-approved <gate-id>` and the
/// run proceeded). Distinct status values matter for downstream audits:
/// pending blocks dispatch; approved is the audit trail showing who
/// authorized the irreversible step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GateRecord {
    pub schema_version: String,
    pub timestamp: String,
    pub task_id: String,
    pub task: String,
    pub gate_id: String,
    pub description: String,
    /// "member_owner_gate" if the gate fired because a matched member
    /// has owner_gate=true, "declared_gate" if the gate id came from
    /// the [owner_gates] block matching the task description.
    pub source: String,
    /// "pending" or "approved".
    pub status: String,
    /// Username from `USER`/`USERNAME` env at approval time. Empty
    /// when status="pending".
    pub approved_by: String,
}

/// One fired gate, ready to be turned into a `GateRecord` once a status
/// (pending vs approved) is known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiredGate {
    pub gate_id: String,
    pub description: String,
    pub source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateKind {
    Publish,
    Release,
    Deploy,
    RemoteVcs,
    GlobalConfig,
    ExternalAccount,
    SecretExposure,
    PrivateDataUpload,
    Telemetry,
    VersionTag,
    ArtifactUpload,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentKind {
    ExecuteOutwardAction,
    RequestApproval,
    LocalSafetyConstraint,
    MentionOnly,
    QuotedOrCode,
    Negated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateDecision {
    Fire,
    Ignore,
    UncertainAskOwner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextSpan {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoredGateMention {
    pub gate_id: String,
    pub mention: String,
    pub intent: IntentKind,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchGateAnalysis {
    pub fired: Vec<FiredGate>,
    pub ignored: Vec<IgnoredGateMention>,
    pub normalized_spans: Vec<TextSpan>,
    pub confidence: GateConfidence,
}

pub fn consult_team_path(project_root: &Path) -> PathBuf {
    project_root.join(CONSULT_TEAM_PATH)
}

pub fn findings_path(project_root: &Path, task_id: &str) -> PathBuf {
    project_root
        .join(CONSULT_FINDINGS_DIR)
        .join(format!("consult-{task_id}.jsonl"))
}

pub fn gates_path(project_root: &Path, task_id: &str) -> PathBuf {
    project_root
        .join(CONSULT_FINDINGS_DIR)
        .join(format!("gates-{task_id}.jsonl"))
}

/// Load and normalize the consult team config. Returns `Ok(None)` if
/// the file is absent — callers treat that as "no team installed" and
/// skip the consult.
pub fn load_consult_team(project_root: &Path) -> Result<Option<ConsultTeam>> {
    let path = consult_team_path(project_root);
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("read consult team config at {}", path.display()))?;
    let value: toml::Value = text
        .parse()
        .with_context(|| format!("parse {} as TOML", path.display()))?;
    Ok(Some(parse_team(&value)))
}

/// Returns true if the parsed `schema_version` is one the engine knows
/// how to consume. Used by `aiplus doctor`.
pub fn is_supported_schema(version: &str) -> bool {
    SUPPORTED_CONSULT_SCHEMAS.contains(&version)
}

fn parse_team(value: &toml::Value) -> ConsultTeam {
    let schema_version = value
        .get("schema_version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let mut members: Vec<Member> = value
        .get("members")
        .and_then(|m| m.as_array())
        .map(|arr| arr.iter().map(parse_member).collect())
        .unwrap_or_default();

    // SWE-shape: trigger keywords are declared in a top-level
    // [[triggers]] block, each pointing at a list of member ids.
    // Merge those into the matching member's trigger vec so downstream
    // matching is uniform across schemas. `stop_gate = true` on a
    // trigger block is kept as its own data structure (see W2): we
    // can't flip `member.owner_gate` permanently, because the same
    // member typically appears in multiple [[triggers]] blocks, only
    // one of which is a gate. The gate must fire only when that
    // specific trigger's pattern matches the task.
    let mut stop_gate_triggers: Vec<StopGateTrigger> = Vec::new();
    if let Some(trigger_blocks) = value.get("triggers").and_then(|t| t.as_array()) {
        for block in trigger_blocks {
            let id = block
                .get("id")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let patterns: Vec<String> = block
                .get("patterns")
                .and_then(|p| p.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|p| p.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let target_members: Vec<String> = block
                .get("members")
                .and_then(|m| m.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| m.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let tier = block
                .get("tier")
                .and_then(|t| t.as_str())
                .and_then(parse_tier);
            let stop_gate = block
                .get("stop_gate")
                .and_then(|s| s.as_bool())
                .unwrap_or(false);
            for member in members.iter_mut() {
                if target_members.contains(&member.id) {
                    for p in &patterns {
                        if !member.triggers.iter().any(|t| t.eq_ignore_ascii_case(p)) {
                            member.triggers.push(p.clone());
                        }
                    }
                    if let Some(t) = tier {
                        if !member.default_tiers.contains(&t) {
                            member.default_tiers.push(t);
                        }
                    }
                }
            }
            if stop_gate {
                stop_gate_triggers.push(StopGateTrigger {
                    id: if id.is_empty() {
                        format!("trigger-{}", stop_gate_triggers.len() + 1)
                    } else {
                        id
                    },
                    patterns,
                    members: target_members,
                });
            }
        }
    }

    let user_personas = value
        .get("user_evidence")
        .and_then(|v| v.get("personas"))
        .and_then(|p| p.as_array())
        .map(|arr| arr.iter().map(parse_persona).collect())
        .unwrap_or_default();

    let owner_gates = parse_owner_gates(value);

    ConsultTeam {
        stop_gate_triggers,
        schema_version,
        members,
        user_personas,
        owner_gates,
    }
}

fn parse_member(value: &toml::Value) -> Member {
    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| id.clone());
    let triggers = value
        .get("triggers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let default_tiers = value
        .get("default_tiers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t.as_str().and_then(parse_tier))
                .collect()
        })
        .unwrap_or_default();
    let owner_gate = value
        .get("owner_gate")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let output_artifact = value
        .get("output_artifact")
        .and_then(|v| v.as_str())
        .map(String::from);
    Member {
        id,
        name,
        triggers,
        default_tiers,
        owner_gate,
        output_artifact,
    }
}

fn parse_persona(value: &toml::Value) -> UserPersona {
    let id = value
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| id.clone());
    let triggers = value
        .get("triggers")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|t| t.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    UserPersona { id, name, triggers }
}

fn parse_owner_gates(value: &toml::Value) -> Vec<OwnerGate> {
    let Some(section) = value.get("owner_gates") else {
        return Vec::new();
    };
    // AEL shape: `[owner_gates] gates = [{id, description}]`.
    if let Some(arr) = section.get("gates").and_then(|g| g.as_array()) {
        return arr
            .iter()
            .filter_map(|g| {
                let id = g.get("id").and_then(|v| v.as_str())?.to_string();
                let description = g
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_default();
                Some(OwnerGate { id, description })
            })
            .collect();
    }
    // SWE shape: flat dict of `<gate-name> = true`. Treat every truthy
    // entry as a gate; description left blank, gate id used both as id
    // and human label.
    if let Some(table) = section.as_table() {
        return table
            .iter()
            .filter_map(|(k, v)| {
                if v.as_bool().unwrap_or(false) {
                    Some(OwnerGate {
                        id: k.clone(),
                        description: String::new(),
                    })
                } else {
                    None
                }
            })
            .collect();
    }
    Vec::new()
}

fn parse_tier(s: &str) -> Option<Tier> {
    match s.to_ascii_uppercase().as_str() {
        "LIGHT" => Some(Tier::Light),
        "MEDIUM" => Some(Tier::Medium),
        "HEAVY" => Some(Tier::Heavy),
        _ => None,
    }
}

/// Score task complexity on a 1–5 scale from the same keyword shape
/// `state::score_task_tier` uses. Kept here so the engine has the
/// numeric signal `[scaling]` rules expect; the CLI tier label is
/// derived from `select_tier` below.
pub fn score_complexity(task: &str) -> u8 {
    let lower = task.to_lowercase();
    // Heavy signals: structural shifts, submission paths, identification
    // strategy changes. Two or more pushes complexity to 5.
    let heavy = [
        "submit",
        "submission",
        "structural",
        "redesign",
        "rewrite",
        "refactor",
        "schema",
        "migrate",
        "migration",
        "release",
        "deploy",
        "production",
        "r&r",
        "revise",
        "identification strategy",
        "authorship",
    ];
    let medium = [
        "robustness",
        "specification",
        "spec",
        "identification",
        "instrument",
        "fixed effect",
        "cluster",
        "merge",
        "integrate",
        "review",
        "rebuttal",
        "regression",
        "llm",
        "validity",
        "multi-llm",
        "inter-rater",
        "pipeline",
        "replication package",
        "irb",
        "restricted data",
        "small-cell",
        "intro",
        "contribution",
        "target-journal",
    ];
    let h = heavy.iter().filter(|k| lower.contains(*k)).count();
    let m = medium.iter().filter(|k| lower.contains(*k)).count();
    match (h, m) {
        (h, _) if h >= 2 => 5,
        (1, m) if m >= 1 => 5,
        (1, _) => 4,
        (0, m) if m >= 3 => 4,
        (0, m) if m >= 2 => 3,
        (0, 1) => 2,
        _ => 1,
    }
}

/// Score task risk on a 0.0–1.0 scale. Signals that put real-world
/// damage on the table (submission, public posting, money, data
/// sharing, identification rewrite) push past the 0.7 threshold where
/// user personas always join the consult.
pub fn score_risk(task: &str) -> f32 {
    let lower = task.to_lowercase();
    let high = [
        "submit",
        "submission",
        "publish",
        "release",
        "deploy",
        "production",
        "tag",
        "data share",
        "data-share",
        "share data",
        "authorship",
        "r&r",
        "post",
        "nber",
        "ssrn",
        "preprint",
        "irb",
        "restricted data",
        "payment",
        "pii",
        "delete",
        "force",
    ];
    let medium = [
        "merge",
        "migration",
        "schema",
        "estimator",
        "sample frame",
        "robustness",
        "identification",
        "external",
        "co-author",
        "coauthor",
    ];
    let h = high.iter().filter(|k| lower.contains(*k)).count();
    let m = medium.iter().filter(|k| lower.contains(*k)).count();
    let raw = (h as f32) * 0.45 + (m as f32) * 0.15;
    raw.clamp(0.0, 1.0)
}

/// Pick a tier from complexity + risk. Mirrors the AEL `[scaling]`
/// defaults: complexity 1–2 → LIGHT, 3–4 → MEDIUM, ≥5 → HEAVY, and
/// risk ≥ 0.7 escalates to at least MEDIUM.
pub fn select_tier(complexity: u8, risk: f32) -> Tier {
    if complexity >= 5 || risk >= 0.85 {
        Tier::Heavy
    } else if complexity >= 3 || risk >= 0.7 {
        Tier::Medium
    } else {
        Tier::Light
    }
}

fn task_matches_trigger(task_lower: &str, trigger: &str) -> bool {
    let t = trigger.to_lowercase();
    if t.is_empty() {
        return false;
    }
    task_lower.contains(&t)
}

/// Pick the members that should be staffed for `task` at `tier`.
/// Eligibility is the intersection of:
///   * `member.default_tiers` includes `tier` (or is empty, which we
///     read as "any tier"), and
///   * at least one of the member's `triggers` substring-matches the
///     task description.
pub fn match_members<'a>(team: &'a ConsultTeam, task: &str, tier: Tier) -> Vec<&'a Member> {
    let task_lower = task.to_lowercase();
    team.members
        .iter()
        .filter(|m| {
            let tier_ok = m.default_tiers.is_empty() || m.default_tiers.contains(&tier);
            if !tier_ok {
                return false;
            }
            if m.triggers.is_empty() {
                // Members without triggers (e.g., a "coordinator" or
                // "default" seat) join HEAVY consults but not LIGHT —
                // they're noise on small tasks.
                return tier == Tier::Heavy;
            }
            m.triggers
                .iter()
                .any(|t| task_matches_trigger(&task_lower, t))
        })
        .collect()
}

/// User personas join the consult either because tier is HEAVY or
/// because risk ≥ 0.7 (the goal-prompt rule). A persona must also
/// trigger-match unless its trigger list is empty (then it joins
/// unconditionally on HEAVY).
pub fn match_user_personas<'a>(
    team: &'a ConsultTeam,
    task: &str,
    tier: Tier,
    risk: f32,
) -> Vec<&'a UserPersona> {
    if tier != Tier::Heavy && risk < 0.7 {
        return Vec::new();
    }
    let task_lower = task.to_lowercase();
    team.user_personas
        .iter()
        .filter(|p| {
            if p.triggers.is_empty() {
                return tier == Tier::Heavy;
            }
            p.triggers
                .iter()
                .any(|t| task_matches_trigger(&task_lower, t))
        })
        .collect()
}

/// Compute a stable task_id from the task text + a salt. Re-running
/// `aiplus agent route` with the same role+task+date salt collapses to
/// the same id, which is how W1 keeps the consult JSONL idempotent.
pub fn derive_task_id(role: &str, task: &str, date_salt: &str) -> String {
    use crate::stable_hash;
    let raw = format!("{role}::{task}::{date_salt}");
    // Truncate to 12 hex chars — enough to disambiguate inside a
    // project's `_team/` dir without making filenames unreadable.
    let h = stable_hash(&raw);
    h.chars().take(12).collect()
}

/// Append findings for a consult run. Idempotency rule: a given
/// (task_id, member_id) pair is recorded at most once per file —
/// re-running `agent route` after an interrupted partial write will
/// fill in the missing members without duplicating completed ones.
pub fn write_findings(
    project_root: &Path,
    task_id: &str,
    findings: &[ConsultFinding],
) -> Result<PathBuf> {
    let dir = project_root.join(CONSULT_FINDINGS_DIR);
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let path = findings_path(project_root, task_id);

    let mut seen: std::collections::BTreeSet<String> = Default::default();
    if path.exists() {
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        for line in existing.lines() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                if let (Some(tid), Some(mid)) = (
                    v.get("taskId").and_then(|x| x.as_str()),
                    v.get("memberId").and_then(|x| x.as_str()),
                ) {
                    seen.insert(format!("{tid}::{mid}"));
                }
            }
        }
    }
    for finding in findings {
        let key = format!("{}::{}", finding.task_id, finding.member_id);
        if seen.contains(&key) {
            continue;
        }
        let line = serde_json::to_string(finding)?;
        crate::append_jsonl_atomic(&path, &line)?;
        seen.insert(key);
    }
    Ok(path)
}

/// One-shot helper: pick tier, staff members + personas, build
/// per-member finding records ready to be written. Caller passes the
/// timestamp so transcripts and the JSONL agree on the same value.
pub fn build_findings(
    team: &ConsultTeam,
    task: &str,
    task_id: &str,
    timestamp: &str,
) -> (Tier, u8, f32, Vec<ConsultFinding>) {
    let complexity = score_complexity(task);
    let risk = score_risk(task);
    let tier = select_tier(complexity, risk);
    let mut out: Vec<ConsultFinding> = Vec::new();

    for member in match_members(team, task, tier) {
        let matched: Vec<String> = member
            .triggers
            .iter()
            .filter(|t| task.to_lowercase().contains(&t.to_lowercase()))
            .cloned()
            .collect();
        out.push(ConsultFinding {
            schema_version: CONSULT_RECORD_SCHEMA_VERSION.to_string(),
            timestamp: timestamp.to_string(),
            task_id: task_id.to_string(),
            task: task.to_string(),
            tier: tier.as_str().to_string(),
            complexity,
            risk,
            member_id: member.id.clone(),
            member_name: member.name.clone(),
            triggers_matched: matched,
            output_artifact: member.output_artifact.clone(),
            kind: "member".to_string(),
        });
    }
    for persona in match_user_personas(team, task, tier, risk) {
        let matched: Vec<String> = persona
            .triggers
            .iter()
            .filter(|t| task.to_lowercase().contains(&t.to_lowercase()))
            .cloned()
            .collect();
        out.push(ConsultFinding {
            schema_version: CONSULT_RECORD_SCHEMA_VERSION.to_string(),
            timestamp: timestamp.to_string(),
            task_id: task_id.to_string(),
            task: task.to_string(),
            tier: tier.as_str().to_string(),
            complexity,
            risk,
            member_id: persona.id.clone(),
            member_name: persona.name.clone(),
            triggers_matched: matched,
            output_artifact: None,
            kind: "user_persona".to_string(),
        });
    }
    (tier, complexity, risk, out)
}

/// Run the semantic dispatch owner-gate analyzer for `task`.
///
/// Known AiPlus safety gates use deterministic verb-object intent
/// matching: a gate fires only when the task asks for an outward/global
/// action, or asks for permission to take one. Markdown blockquotes,
/// fenced code, quoted/code examples, and clearly negated local safety
/// clauses are ignored. Inline-code or quoted protected commands still
/// fire when the surrounding clause is imperative. Domain-specific gates
/// that are not in the AiPlus safety catalog keep the historical
/// substring behavior so existing project/team owner gates remain
/// conservative.
pub fn analyze_dispatch_gate<'a>(
    task: &str,
    matched_members: &[&'a Member],
    team: &'a ConsultTeam,
) -> DispatchGateAnalysis {
    let mut analysis = DispatchGateAnalysis {
        fired: Vec::new(),
        ignored: Vec::new(),
        normalized_spans: normalize_dispatch_gate_spans(task),
        confidence: GateConfidence::High,
    };

    for member in matched_members {
        if !member.owner_gate {
            continue;
        }
        push_unique_gate(
            &mut analysis.fired,
            FiredGate {
                gate_id: member.id.clone(),
                description: format!("{} member flagged owner gate", member.name),
                source: "member_owner_gate".to_string(),
            },
        );
    }

    for gate in &team.owner_gates {
        if gate.id.is_empty() {
            continue;
        }

        let description = if gate.description.is_empty() {
            format!("owner gate '{}' declared in consult team", gate.id)
        } else {
            gate.description.clone()
        };

        if let Some(kind) = gate_kind_for_id(&gate.id) {
            match semantic_gate_decision(task, &analysis.normalized_spans, kind) {
                GateDecision::Fire | GateDecision::UncertainAskOwner => {
                    push_unique_gate(
                        &mut analysis.fired,
                        FiredGate {
                            gate_id: gate.id.clone(),
                            description,
                            source: "declared_gate".to_string(),
                        },
                    );
                }
                GateDecision::Ignore => {
                    if gate_mentioned(task, &gate.id, kind) {
                        analysis.ignored.push(IgnoredGateMention {
                            gate_id: gate.id.clone(),
                            mention: gate.id.clone(),
                            intent: IntentKind::LocalSafetyConstraint,
                            reason: "gate mention is quoted, negated, descriptive, or local-only"
                                .to_string(),
                        });
                    }
                }
            }
            continue;
        }

        // Unknown/project-specific owner gates retain the previous
        // conservative substring matching. G2 only narrows the common
        // outward/global safety gates that caused route false positives.
        let gate_lower = gate.id.to_lowercase();
        let normalized = gate_lower.replace('-', " ");
        let task_lower = task.to_lowercase();
        let hit = task_lower.contains(&gate_lower) || task_lower.contains(&normalized);
        if hit {
            push_unique_gate(
                &mut analysis.fired,
                FiredGate {
                    gate_id: gate.id.clone(),
                    description,
                    source: "declared_gate".to_string(),
                },
            );
        }
    }

    let matched_ids: std::collections::BTreeSet<&str> =
        matched_members.iter().map(|m| m.id.as_str()).collect();
    for trig in &team.stop_gate_triggers {
        let member_hit = trig
            .members
            .iter()
            .any(|m| matched_ids.contains(m.as_str()));
        if !member_hit {
            continue;
        }
        let pattern_hit =
            semantic_stop_gate_decision(task, &analysis.normalized_spans, &trig.patterns);
        if !pattern_hit {
            if stop_gate_mentioned(task, &trig.patterns) {
                analysis.ignored.push(IgnoredGateMention {
                    gate_id: trig.id.clone(),
                    mention: trig.patterns.join(","),
                    intent: IntentKind::LocalSafetyConstraint,
                    reason:
                        "stop_gate trigger mention is quoted, negated, descriptive, or local-only"
                            .to_string(),
                });
            }
            continue;
        }
        push_unique_gate(
            &mut analysis.fired,
            FiredGate {
                gate_id: trig.id.clone(),
                description: format!(
                    "stop_gate trigger '{}' fired on task keyword + matched member",
                    trig.id
                ),
                source: "stop_gate_trigger".to_string(),
            },
        );
    }

    analysis
}

/// Identify the owner gates that fire for `task`, given the consult
/// team. This compatibility wrapper keeps the route integration API
/// stable while the richer semantic diagnostics live in
/// `analyze_dispatch_gate`.
pub fn match_gates<'a>(
    team: &'a ConsultTeam,
    matched_members: &[&'a Member],
    task: &str,
) -> Vec<FiredGate> {
    analyze_dispatch_gate(task, matched_members, team).fired
}

fn push_unique_gate(fired: &mut Vec<FiredGate>, gate: FiredGate) {
    if !fired.iter().any(|g| g.gate_id == gate.gate_id) {
        fired.push(gate);
    }
}

fn normalize_dispatch_gate_spans(task: &str) -> Vec<TextSpan> {
    let mut spans = Vec::new();
    let mut offset = 0usize;
    let mut in_fence = false;
    for line in task.split_inclusive('\n') {
        let line_start = offset;
        offset += line.len();
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || trimmed.starts_with('>') {
            continue;
        }

        for clause in split_gate_clauses(line) {
            let text = clause.trim();
            if text.is_empty() {
                continue;
            }
            spans.push(TextSpan {
                start: line_start,
                end: line_start + line.len(),
                text: text.to_string(),
            });
        }
    }
    spans
}

fn split_gate_clauses(text: &str) -> Vec<&str> {
    text.split(|ch: char| matches!(ch, '\n' | '.' | ';'))
        .flat_map(|part| part.split(" then "))
        .flat_map(|part| part.split(" Then "))
        .collect()
}

fn gate_kind_for_id(id: &str) -> Option<GateKind> {
    let id = id.to_ascii_lowercase().replace('-', "_");
    match id.as_str() {
        "push" | "remote_vcs" | "remote_vcs_update" => Some(GateKind::RemoteVcs),
        "tag" | "version_tag" => Some(GateKind::VersionTag),
        "release" => Some(GateKind::Release),
        "artifact_upload" => Some(GateKind::ArtifactUpload),
        "package_publish" => Some(GateKind::Publish),
        "deploy" => Some(GateKind::Deploy),
        "global_config_edit" | "global_config" => Some(GateKind::GlobalConfig),
        "external_account_mutation" | "external_account" => Some(GateKind::ExternalAccount),
        "secret_exposure" | "secret" => Some(GateKind::SecretExposure),
        "private_data_upload" => Some(GateKind::PrivateDataUpload),
        "telemetry" => Some(GateKind::Telemetry),
        "send_delete_publish_or_mutate_external_content" => Some(GateKind::ExternalAccount),
        _ => None,
    }
}

fn semantic_gate_decision(task: &str, spans: &[TextSpan], kind: GateKind) -> GateDecision {
    let mut mentioned = false;
    for span in spans {
        let clause = span.text.to_lowercase();
        if !mentions_gate_kind(&clause, kind) {
            continue;
        }
        mentioned = true;
        if has_explicit_gate_execution_context(&clause, kind) {
            return GateDecision::Fire;
        }
        if is_descriptive_quoted_or_code_gate_clause(&clause) {
            continue;
        }
        if is_explicit_approval_request(&clause, kind) || has_imperative_gate_context(&clause, kind)
        {
            return GateDecision::Fire;
        }
        if is_hard_safe_gate_clause(&clause) {
            continue;
        }
        if is_approval_requirement_clause(&clause) && !has_imperative_gate_context(&clause, kind) {
            continue;
        }
        if is_execute_gate_clause(&clause, kind) {
            return GateDecision::Fire;
        }
        if is_safe_gate_clause(&clause) {
            continue;
        }
    }
    if mentioned || gate_mentioned(task, "", kind) {
        GateDecision::Ignore
    } else {
        GateDecision::Ignore
    }
}

fn semantic_stop_gate_decision(task: &str, spans: &[TextSpan], patterns: &[String]) -> bool {
    for pattern in patterns {
        if let Some(kind) = gate_kind_for_stop_pattern(pattern) {
            if semantic_gate_decision(task, spans, kind) == GateDecision::Fire {
                return true;
            }
            continue;
        }
        for span in spans {
            let clause = span.text.to_lowercase();
            if task_matches_trigger(&clause, pattern)
                && !is_safe_gate_clause(&clause)
                && has_outward_action_verb(&clause)
            {
                return true;
            }
        }
    }
    false
}

fn gate_kind_for_stop_pattern(pattern: &str) -> Option<GateKind> {
    let p = pattern.to_ascii_lowercase();
    match p.as_str() {
        "release" => Some(GateKind::Release),
        "tag" => Some(GateKind::VersionTag),
        "publish" => Some(GateKind::Publish),
        "upload" | "artifact" => Some(GateKind::ArtifactUpload),
        "deploy" => Some(GateKind::Deploy),
        _ => None,
    }
}

fn gate_mentioned(task: &str, gate_id: &str, kind: GateKind) -> bool {
    let lower = task.to_lowercase();
    (!gate_id.is_empty() && lower.contains(&gate_id.to_lowercase()))
        || mentions_gate_kind(&lower, kind)
}

fn stop_gate_mentioned(task: &str, patterns: &[String]) -> bool {
    let lower = task.to_lowercase();
    patterns.iter().any(|p| lower.contains(&p.to_lowercase()))
}

fn mentions_gate_kind(clause: &str, kind: GateKind) -> bool {
    match kind {
        GateKind::Publish => {
            has_word(clause, "publish")
                || clause.contains("package registry")
                || has_word(clause, "registry")
        }
        GateKind::Release => has_word(clause, "release") || clause.contains("release notes"),
        GateKind::Deploy => has_word(clause, "deploy") || has_word(clause, "production"),
        GateKind::RemoteVcs => {
            has_word(clause, "push")
                || clause.contains("git push")
                || clause.contains("origin main")
                || clause.contains("origin/main")
                || clause.contains("remote vcs")
                || clause.contains("remote update")
                || clause.contains("update origin")
        }
        GateKind::GlobalConfig => {
            clause.contains("global config")
                || clause.contains("global registry")
                || clause.contains("machine config")
                || clause.contains("~/.config")
                || clause.contains("installed-projects.json")
        }
        GateKind::ExternalAccount => {
            clause.contains("external account")
                || has_word(clause, "contact")
                || has_word(clause, "email")
                || has_word(clause, "send")
                || has_word(clause, "mutate")
        }
        GateKind::SecretExposure => {
            has_word(clause, "secret")
                || has_word(clause, "secrets")
                || has_word(clause, "token")
                || clause.contains("api key")
        }
        GateKind::PrivateDataUpload => {
            clause.contains("private data")
                || clause.contains("restricted data")
                || clause.contains("data upload")
                || clause.contains("upload data")
                || clause.contains("share data")
        }
        GateKind::Telemetry => has_word(clause, "telemetry") || clause.contains("usage data"),
        GateKind::VersionTag => has_word(clause, "tag") || clause.contains("version bump"),
        GateKind::ArtifactUpload => has_word(clause, "upload") || has_word(clause, "artifact"),
    }
}

fn is_safe_gate_clause(clause: &str) -> bool {
    is_hard_safe_gate_clause(clause) || is_approval_requirement_clause(clause)
}

fn is_hard_safe_gate_clause(clause: &str) -> bool {
    let c = clause.trim();
    if is_descriptive_quoted_or_code_gate_clause(c) {
        return true;
    }
    let safe_markers = [
        "do not",
        "don't",
        "dont",
        "never",
        "no ",
        "without",
        "avoid",
        "out of scope",
        "not in scope",
        "scope:",
        "local only",
        "local-only",
        "project-local",
        "read-only",
        "verification-only",
        "no protected operations",
        "owner gates remain active",
        "preserve owner gates",
        "owner-gated external actions are out of scope",
        "forbid",
        "forbidden",
        "escalate before",
        "acceptance",
        "criteria",
        "example",
        "proposal",
        "draft boundary",
        "release boundary",
        "gate criteria",
        "test description:",
        "fixture description:",
    ];
    safe_markers
        .iter()
        .any(|marker| c.starts_with(marker) || c.contains(marker))
}

fn is_approval_requirement_clause(clause: &str) -> bool {
    clause.contains("requires owner approval")
        || clause.contains("require owner approval")
        || clause.contains("owner approval required")
}

fn has_imperative_gate_context(clause: &str, kind: GateKind) -> bool {
    let c =
        clause.trim_start_matches(|ch: char| ch.is_ascii_whitespace() || ch == '"' || ch == '\'');
    let c = c.strip_prefix("please ").unwrap_or(c);

    let starts_with = |verbs: &[&str]| {
        verbs
            .iter()
            .any(|verb| c == *verb || c.starts_with(&format!("{verb} ")))
    };

    let executor_before_gate = |needles: &[&str]| {
        needles.iter().any(|needle| {
            c.find(needle).is_some_and(|idx| {
                let prefix = &c[..idx];
                has_active_executor_before_gate(prefix)
            })
        })
    };

    match kind {
        GateKind::Publish => {
            starts_with(&["publish", "ship", "submit", "post"])
                || executor_before_gate(&["publish", "package registry", "registry"])
        }
        GateKind::Release => {
            starts_with(&["release", "prepare", "cut", "ship"])
                || executor_before_gate(&["release"])
        }
        GateKind::Deploy => starts_with(&["deploy"]) || executor_before_gate(&["deploy"]),
        GateKind::RemoteVcs => {
            starts_with(&["push", "fast-forward", "sync"])
                || c.starts_with("update origin")
                || c.starts_with("update remote")
                || c.starts_with("git push")
                || c.starts_with("rtk git push")
                || executor_before_gate(&[
                    "git push",
                    "rtk git push",
                    "push",
                    "origin main",
                    "origin/main",
                    "remote vcs",
                    "remote update",
                    "update origin",
                ])
        }
        GateKind::GlobalConfig => {
            starts_with(&[
                "edit", "modify", "change", "repair", "write", "update", "touch", "delete",
            ]) || executor_before_gate(&[
                "global config",
                "global registry",
                "machine config",
                "~/.config",
                "installed-projects.json",
            ])
        }
        GateKind::ExternalAccount => {
            starts_with(&[
                "contact", "email", "send", "create", "modify", "delete", "mutate", "publish",
            ]) || executor_before_gate(&["external account"])
        }
        GateKind::SecretExposure => {
            starts_with(&[
                "print", "show", "expose", "read", "rotate", "push", "upload", "resolve",
            ]) || executor_before_gate(&["secret", "secrets", "token", "api key"])
        }
        GateKind::PrivateDataUpload => {
            starts_with(&["upload", "share", "send", "publish"])
                || executor_before_gate(&[
                    "private data",
                    "restricted data",
                    "data upload",
                    "upload data",
                    "share data",
                ])
        }
        GateKind::Telemetry => {
            starts_with(&["add", "enable", "upload", "send"])
                || executor_before_gate(&["telemetry", "usage data"])
        }
        GateKind::VersionTag => {
            starts_with(&["tag", "create", "push", "bump"])
                || executor_before_gate(&["tag", "version bump"])
        }
        GateKind::ArtifactUpload => {
            starts_with(&["upload", "publish", "attach"])
                || executor_before_gate(&["upload", "artifact"])
        }
    }
}

fn is_descriptive_quoted_or_code_gate_clause(clause: &str) -> bool {
    let has_quoted_or_code = clause.contains('`')
        || clause.contains('"')
        || clause.contains('\'')
        || clause.contains('“')
        || clause.contains('‘');
    if !has_quoted_or_code {
        return false;
    }

    let descriptive_subject = [
        "the command",
        "command ",
        "the phrase",
        "phrase ",
        "the text",
        "text ",
        "the sample",
        "sample ",
        "the example",
        "example ",
    ];
    let descriptive_predicate = [
        " is ",
        " was ",
        " would be ",
        " should be ",
        " remains ",
        " means ",
        " mentions ",
        " describes ",
    ];

    descriptive_subject
        .iter()
        .any(|subject| clause.starts_with(subject))
        && descriptive_predicate
            .iter()
            .any(|predicate| clause.contains(predicate))
        || (["add ", "write ", "document ", "update "]
            .iter()
            .any(|prefix| clause.starts_with(prefix))
            && [
                " note ",
                " fixture ",
                " example ",
                " sentence ",
                " saying ",
                " says ",
            ]
            .iter()
            .any(|marker| clause.contains(marker)))
}

fn is_explicit_approval_request(clause: &str, kind: GateKind) -> bool {
    mentions_gate_kind(clause, kind)
        && (clause.contains("approve")
            || clause.contains("approval to")
            || clause.contains("permission to")
            || clause.contains("authorize")
            || clause.contains("owner-approved"))
        && has_outward_action_verb(clause)
}

fn is_execute_gate_clause(clause: &str, kind: GateKind) -> bool {
    match kind {
        GateKind::Publish => {
            has_word(clause, "publish")
                || (has_any_word(clause, &["ship", "submit", "post"])
                    && mentions_gate_kind(clause, kind))
        }
        GateKind::Release => {
            has_word(clause, "release")
                || (has_any_word(clause, &["prepare", "cut", "ship"])
                    && mentions_gate_kind(clause, kind))
        }
        GateKind::Deploy => has_word(clause, "deploy"),
        GateKind::RemoteVcs => {
            clause.contains("git push")
                || has_word(clause, "push")
                || (has_any_word(clause, &["update", "perform", "fast-forward", "sync"])
                    && mentions_gate_kind(clause, kind))
        }
        GateKind::GlobalConfig => {
            has_any_word(
                clause,
                &[
                    "edit", "modify", "change", "repair", "write", "update", "touch", "delete",
                ],
            ) && mentions_gate_kind(clause, kind)
        }
        GateKind::ExternalAccount => {
            has_any_word(
                clause,
                &[
                    "contact", "email", "send", "create", "modify", "delete", "mutate", "publish",
                ],
            ) && mentions_gate_kind(clause, kind)
        }
        GateKind::SecretExposure => {
            has_any_word(
                clause,
                &[
                    "print", "show", "expose", "read", "rotate", "push", "upload", "resolve",
                ],
            ) && mentions_gate_kind(clause, kind)
        }
        GateKind::PrivateDataUpload => {
            has_any_word(clause, &["upload", "share", "send", "publish"])
                && mentions_gate_kind(clause, kind)
        }
        GateKind::Telemetry => {
            has_any_word(clause, &["add", "enable", "upload", "send"])
                && mentions_gate_kind(clause, kind)
        }
        GateKind::VersionTag => {
            has_word(clause, "tag")
                || (has_any_word(clause, &["create", "push", "bump"])
                    && mentions_gate_kind(clause, kind))
        }
        GateKind::ArtifactUpload => {
            has_word(clause, "upload")
                || (has_any_word(clause, &["publish", "attach"])
                    && mentions_gate_kind(clause, kind))
        }
    }
}

fn has_outward_action_verb(clause: &str) -> bool {
    has_any_word(
        clause,
        &[
            "publish", "release", "deploy", "tag", "push", "upload", "contact", "create", "modify",
            "delete", "rotate", "expose", "print", "show", "use", "read", "perform", "update",
            "edit", "repair", "bump", "prepare", "add", "enable", "send", "mutate", "resolve",
        ],
    )
}

fn has_active_executor_before_gate(prefix: &str) -> bool {
    let Some((start, end)) = ["run", "execute", "perform"]
        .iter()
        .filter_map(|verb| last_word_span(prefix, verb))
        .max_by_key(|(start, _)| *start)
    else {
        return false;
    };

    let before_executor = &prefix[..start];
    let between_executor_and_gate = &prefix[end..];
    !has_negated_executor_prefix(before_executor)
        && !has_intervening_safety_marker(between_executor_and_gate)
}

fn has_explicit_gate_execution_context(clause: &str, kind: GateKind) -> bool {
    protected_command_needles(kind).iter().any(|needle| {
        clause.find(needle).is_some_and(|idx| {
            let prefix = &clause[..idx];
            has_actionable_executor_before_gate(prefix)
        })
    })
}

fn protected_command_needles(kind: GateKind) -> &'static [&'static str] {
    match kind {
        GateKind::Publish => &["npm publish", "cargo publish", "publish"],
        GateKind::Release => &["gh release", "release"],
        GateKind::Deploy => &["deploy production", "deploy"],
        GateKind::RemoteVcs => &["rtk git push", "git push", "origin main", "origin/main"],
        GateKind::GlobalConfig => &["~/.config", "installed-projects.json", "global config"],
        GateKind::ExternalAccount => &["external account"],
        GateKind::SecretExposure => &["api key", "secret", "secrets", "token"],
        GateKind::PrivateDataUpload => &["private data", "restricted data", "upload data"],
        GateKind::Telemetry => &["telemetry", "usage data"],
        GateKind::VersionTag => &["git tag", "tag", "version bump"],
        GateKind::ArtifactUpload => &["artifact", "upload"],
    }
}

fn has_actionable_executor_before_gate(prefix: &str) -> bool {
    let Some((start, end)) = [
        "run",
        "running",
        "execute",
        "executing",
        "perform",
        "performing",
    ]
    .iter()
    .filter_map(|verb| last_word_span(prefix, verb))
    .max_by_key(|(start, _)| *start) else {
        return false;
    };

    let before_executor = &prefix[..start];
    let between_executor_and_gate = &prefix[end..];
    !has_negated_executor_prefix(before_executor)
        && is_actionable_executor_connector(before_executor)
        && !has_intervening_safety_marker(between_executor_and_gate)
}

fn is_actionable_executor_connector(before_executor: &str) -> bool {
    let trimmed = before_executor.trim_end_matches(|ch: char| {
        ch.is_ascii_whitespace() || matches!(ch, '`' | '"' | '\'' | ':' | '-' | '(')
    });
    if trimmed.is_empty() || trimmed == "please" {
        return true;
    }
    ["and", "then", "by", "to", "please"]
        .iter()
        .any(|connector| trimmed.ends_with(&format!(" {connector}")))
}

fn last_word_span(text: &str, word: &str) -> Option<(usize, usize)> {
    text.match_indices(word)
        .filter(|(start, _)| {
            let end = *start + word.len();
            is_word_boundary(text, *start, end)
        })
        .last()
        .map(|(start, _)| (start, start + word.len()))
}

fn is_word_boundary(text: &str, start: usize, end: usize) -> bool {
    let before_ok = text[..start]
        .chars()
        .next_back()
        .is_none_or(|ch| !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'));
    let after_ok = text[end..]
        .chars()
        .next()
        .is_none_or(|ch| !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'));
    before_ok && after_ok
}

fn has_negated_executor_prefix(prefix: &str) -> bool {
    prefix.contains("do not")
        || prefix.contains("don't")
        || prefix.contains("dont")
        || prefix.contains("never")
        || prefix.contains("without")
}

fn has_intervening_safety_marker(text: &str) -> bool {
    let trimmed = text.trim_start_matches(|ch: char| {
        ch.is_ascii_whitespace() || matches!(ch, '`' | '"' | '\'' | ':' | '-' | '(')
    });
    trimmed.starts_with("no ")
        || trimmed.starts_with("without ")
        || trimmed.starts_with("avoid ")
        || text.contains(", no ")
        || text.contains("; no ")
        || text.contains(" no ")
        || text.contains(" without ")
        || text.contains(" avoid ")
        || text.contains(" do not ")
        || text.contains(" don't ")
        || text.contains(" dont ")
}

fn has_any_word(text: &str, words: &[&str]) -> bool {
    words.iter().any(|word| has_word(text, word))
}

fn has_word(text: &str, word: &str) -> bool {
    text.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'))
        .any(|token| token == word)
}

/// Append gate ledger records for one consult run. Idempotency rule:
/// each (task_id, gate_id, status) combination is written at most
/// once per file. This lets a `pending` entry from an earlier attempt
/// sit alongside an `approved` entry from a follow-up run.
pub fn write_gate_ledger(
    project_root: &Path,
    task_id: &str,
    records: &[GateRecord],
) -> Result<PathBuf> {
    let dir = project_root.join(CONSULT_FINDINGS_DIR);
    std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let path = gates_path(project_root, task_id);
    let mut seen: std::collections::BTreeSet<String> = Default::default();
    if path.exists() {
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        for line in existing.lines() {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                if let (Some(tid), Some(gid), Some(st)) = (
                    v.get("taskId").and_then(|x| x.as_str()),
                    v.get("gateId").and_then(|x| x.as_str()),
                    v.get("status").and_then(|x| x.as_str()),
                ) {
                    seen.insert(format!("{tid}::{gid}::{st}"));
                }
            }
        }
    }
    for record in records {
        let key = format!("{}::{}::{}", record.task_id, record.gate_id, record.status);
        if seen.contains(&key) {
            continue;
        }
        let line = serde_json::to_string(record)?;
        crate::append_jsonl_atomic(&path, &line)?;
        seen.insert(key);
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> ConsultTeam {
        let value: toml::Value = text.parse().unwrap();
        parse_team(&value)
    }

    fn g2_gate_team() -> ConsultTeam {
        parse(
            r#"
schema_version = "0.1"
[[members]]
id = "release_automation"
name = "Release / Automation"
default_tiers = ["LIGHT", "MEDIUM", "HEAVY"]
triggers = ["release", "tag", "publish", "artifact", "upload", "deploy"]
[[members]]
id = "trust_safety"
name = "Trust / Safety"
default_tiers = ["LIGHT", "MEDIUM", "HEAVY"]
triggers = ["secret", "external account", "private data", "telemetry", "global config"]

[[triggers]]
id = "release"
patterns = ["release", "tag", "publish", "artifact", "upload", "deploy"]
tier = "HEAVY"
members = ["release_automation", "trust_safety"]
stop_gate = true

[owner_gates]
push = true
tag = true
release = true
artifact_upload = true
package_publish = true
deploy = true
global_config_edit = true
external_account_mutation = true
secret_exposure = true
private_data_upload = true
telemetry = true
send_delete_publish_or_mutate_external_content = true
"#,
        )
    }

    #[derive(Debug, Deserialize)]
    struct G2Fixture {
        id: String,
        expected_gate: bool,
        task: String,
    }

    #[test]
    fn tier_selection_table() {
        // (complexity, risk) → expected tier.
        // The asserts here are the documented contract — the goal prompt
        // pins this, and downstream `agent route` UX depends on it.
        let cases = [
            (1, 0.0, Tier::Light),
            (2, 0.2, Tier::Light),
            (3, 0.0, Tier::Medium),
            (4, 0.5, Tier::Medium),
            (5, 0.0, Tier::Heavy),
            (1, 0.7, Tier::Medium),
            (2, 0.85, Tier::Heavy),
            (4, 0.7, Tier::Medium),
            (4, 0.9, Tier::Heavy),
        ];
        for (c, r, want) in cases {
            assert_eq!(select_tier(c, r), want, "select_tier({c}, {r}) wrong");
        }
    }

    #[test]
    fn ael_schema_parses() {
        let cfg = r#"
schema_version = "2.1"
[[members]]
id = "design"
name = "Design Credibility"
default_tiers = ["MEDIUM", "HEAVY"]
triggers = ["identification", "IV"]
owner_gate = false
output_artifact = "design-credibility-check.md"

[[members]]
id = "irb"
name = "IRB Gate"
default_tiers = ["MEDIUM", "HEAVY"]
triggers = ["IRB", "consent"]
owner_gate = true

[user_evidence]
enabled = true
[[user_evidence.personas]]
id = "referee"
name = "Top-Tier Referee"
triggers = ["submission", "QJE"]

[owner_gates]
gates = [
  { id = "submission", description = "Any journal submission" },
]
"#;
        let team = parse(cfg);
        assert_eq!(team.schema_version, "2.1");
        assert_eq!(team.members.len(), 2);
        assert!(team.members[1].owner_gate);
        assert_eq!(team.user_personas.len(), 1);
        assert_eq!(team.owner_gates.len(), 1);
        assert_eq!(team.owner_gates[0].id, "submission");
    }

    #[test]
    fn swe_schema_parses() {
        let cfg = r#"
schema_version = "0.1"
[[members]]
id = "ai_integration"
name = "AI Integration"
default_tiers = ["LIGHT", "MEDIUM", "HEAVY"]
[[members]]
id = "trust_safety"
name = "Trust / Safety"
default_tiers = ["MEDIUM", "HEAVY"]

[[triggers]]
id = "ai_feature"
patterns = ["LLM", "tool use"]
tier = "MEDIUM"
members = ["ai_integration", "trust_safety"]

[[triggers]]
id = "release"
patterns = ["release", "tag"]
tier = "HEAVY"
members = ["trust_safety"]
stop_gate = true

[owner_gates]
push = true
tag = true
"#;
        let team = parse(cfg);
        assert_eq!(team.schema_version, "0.1");
        // The two ai_integration triggers should have merged in from
        // the [[triggers]] block.
        let ai = team
            .members
            .iter()
            .find(|m| m.id == "ai_integration")
            .unwrap();
        assert!(ai.triggers.iter().any(|t| t == "LLM"));
        let ts = team
            .members
            .iter()
            .find(|m| m.id == "trust_safety")
            .unwrap();
        // W2: stop_gate on the release trigger does NOT permanently
        // flip member.owner_gate (the same member can sit in many
        // [[triggers]] blocks, only one of which is the gate). The
        // gate now lives on team.stop_gate_triggers and fires only
        // when the trigger's pattern matches the task.
        assert!(!ts.owner_gate);
        let release_gate = team
            .stop_gate_triggers
            .iter()
            .find(|t| t.id == "release")
            .expect("release stop_gate trigger should be recorded");
        assert!(release_gate.patterns.iter().any(|p| p == "release"));
        assert!(release_gate.members.iter().any(|m| m == "trust_safety"));
        // SWE owner_gates is flat dict — expect at least push/tag.
        let ids: Vec<&str> = team.owner_gates.iter().map(|g| g.id.as_str()).collect();
        assert!(ids.contains(&"push"));
        assert!(ids.contains(&"tag"));
    }

    #[test]
    fn stop_gate_trigger_fires_only_on_pattern_match() {
        // W2 regression: a [[triggers]] block with stop_gate=true
        // must only fire when its own pattern matches. A different
        // [[triggers]] block (e.g. ai_feature with no stop_gate) that
        // also names the same member must NOT cause the gate to fire.
        let cfg = r#"
schema_version = "0.1"
[[members]]
id = "ai_integration"
name = "AI Integration"
default_tiers = ["MEDIUM", "HEAVY"]
[[members]]
id = "trust_safety"
name = "Trust / Safety"
default_tiers = ["MEDIUM", "HEAVY"]

[[triggers]]
id = "ai_feature"
patterns = ["LLM", "tool use"]
tier = "MEDIUM"
members = ["ai_integration", "trust_safety"]

[[triggers]]
id = "release"
patterns = ["release", "tag", "publish"]
tier = "HEAVY"
members = ["trust_safety"]
stop_gate = true
"#;
        let team = parse(cfg);

        // Task that matches ai_feature ("LLM") but not release: gate
        // must NOT fire (this is exactly the W1 test regression).
        let task = "rewrite the LLM tool use context pipeline";
        let matched = match_members(&team, task, Tier::Heavy);
        let gates = match_gates(&team, &matched, task);
        let ids: Vec<&str> = gates.iter().map(|g| g.gate_id.as_str()).collect();
        assert!(
            !ids.contains(&"release"),
            "release stop_gate must not fire without 'release' keyword: {:?}",
            gates
        );

        // Task that matches release: gate must fire.
        let task = "tag and release the LLM tool pipeline";
        let matched = match_members(&team, task, Tier::Heavy);
        let gates = match_gates(&team, &matched, task);
        let ids: Vec<&str> = gates.iter().map(|g| g.gate_id.as_str()).collect();
        assert!(
            ids.contains(&"release"),
            "release stop_gate must fire on release keyword + matched member: {:?}",
            gates
        );
    }

    #[test]
    fn semantic_gate_fixture_matches_gt1_samples() {
        let team = g2_gate_team();
        let fixture = include_str!(
            "../../../crates/aiplus-cli/tests/fixtures/g2_dispatch_gate_samples.jsonl"
        );
        for (line_no, line) in fixture.lines().enumerate() {
            let sample: G2Fixture = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("fixture line {} should parse: {e}", line_no + 1));
            let matched = match_members(&team, &sample.task, Tier::Heavy);
            let gates = match_gates(&team, &matched, &sample.task);
            assert_eq!(
                !gates.is_empty(),
                sample.expected_gate,
                "fixture {} expected gate={} got {:?}",
                sample.id,
                sample.expected_gate,
                gates
            );
        }
    }

    #[test]
    fn semantic_gate_ignores_quoted_code_and_blockquotes() {
        let team = g2_gate_team();
        let task = r#"
> Publish the package.

Do not run `git push origin main`.

```sh
deploy production
```

Implementation task: local-only fixture update; no publish, release, deploy, global config, external accounts, or secrets.
"#;
        let matched = match_members(&team, task, Tier::Heavy);
        let analysis = analyze_dispatch_gate(task, &matched, &team);
        assert!(
            analysis.fired.is_empty(),
            "quoted/code/negated gate mentions should not fire: {:?}",
            analysis
        );
        assert!(
            !analysis.ignored.is_empty(),
            "ignored mentions should be retained for diagnostics"
        );
    }

    #[test]
    fn semantic_gate_fires_on_inline_code_and_quoted_commands() {
        let team = g2_gate_team();
        for task in [
            "Run `git push origin main`.",
            "Run \"git push origin main\".",
            "Please execute 'deploy production' now.",
        ] {
            let matched = match_members(&team, task, Tier::Heavy);
            let gates = match_gates(&team, &matched, task);
            assert!(!gates.is_empty(), "{task:?} should fire a gate");
        }
    }

    #[test]
    fn semantic_gate_fires_on_mixed_descriptive_and_execute_clauses() {
        let team = g2_gate_team();
        for task in [
            "Write a note and run `git push origin main`.",
            "Update a fixture by running `git push origin main`.",
            "Add an example and execute \"git push origin main\".",
        ] {
            let matched = match_members(&team, task, Tier::Heavy);
            let gates = match_gates(&team, &matched, task);
            assert!(
                !gates.is_empty(),
                "{task:?} should fire even with descriptive setup text"
            );
        }
    }

    #[test]
    fn semantic_gate_fires_on_same_clause_approval_requirement() {
        let team = g2_gate_team();
        for task in [
            "Run `git push origin main` because it requires owner approval.",
            "Run \"git push origin main\" because it requires owner approval.",
            "Run `git push origin main` because owner approval required.",
        ] {
            let matched = match_members(&team, task, Tier::Heavy);
            let gates = match_gates(&team, &matched, task);
            assert!(
                !gates.is_empty(),
                "{task:?} should fire despite same-clause approval wording"
            );
        }
    }

    #[test]
    fn semantic_gate_fires_on_imperative_command_with_false_safety_waiver() {
        let team = g2_gate_team();
        for task in [
            "Run `git push origin main` without asking owner.",
            "Run `git push origin main` no owner approval needed.",
            "Run \"git push origin main\" without asking owner.",
            "Run \"git push origin main\" no owner approval needed.",
        ] {
            let matched = match_members(&team, task, Tier::Heavy);
            let gates = match_gates(&team, &matched, task);
            assert!(
                !gates.is_empty(),
                "{task:?} should fire despite same-clause safety waiver"
            );
        }
    }

    #[test]
    fn semantic_gate_ignores_negated_and_descriptive_approval_requirement() {
        let team = g2_gate_team();
        for task in [
            "Do not run `git push origin main`; mention it requires owner approval.",
            "Do not run `git push origin main`; mention no owner approval has been granted.",
            "Verify that `git push origin main` would require owner approval; do not run it.",
            "The command `git push origin main` requires owner approval.",
            "Add a safe example sentence saying \"git push origin main\" requires owner approval; do not execute the quoted command.",
            "Update a fixture note saying `git push origin main` requires owner approval; do not run it.",
            "Document that the phrase `git push origin main` is an example; do not run it.",
            "Test description: running `git push origin main` should block when requested, but this fixture does not execute it.",
        ] {
            let matched = match_members(&team, task, Tier::Heavy);
            let analysis = analyze_dispatch_gate(task, &matched, &team);
            assert!(
                analysis.fired.is_empty(),
                "{task:?} should stay descriptive/negated: {:?}",
                analysis
            );
            assert!(
                !analysis.ignored.is_empty(),
                "{task:?} should retain ignored diagnostics"
            );
        }
    }

    #[test]
    fn semantic_gate_ignores_descriptive_quoted_command() {
        let team = g2_gate_team();
        let task = "The command `git push origin main` is protected; keep it as an example only.";
        let matched = match_members(&team, task, Tier::Heavy);
        let analysis = analyze_dispatch_gate(task, &matched, &team);
        assert!(
            analysis.fired.is_empty(),
            "descriptive quoted command should not fire: {:?}",
            analysis
        );
        assert!(
            !analysis.ignored.is_empty(),
            "descriptive quoted command should be retained as ignored diagnostic"
        );
    }

    #[test]
    fn semantic_gate_fires_on_explicit_approval_request() {
        let team = g2_gate_team();
        let task = "Request Owner approval to deploy the docs page to production.";
        let matched = match_members(&team, task, Tier::Heavy);
        let gates = match_gates(&team, &matched, task);
        let ids: Vec<&str> = gates.iter().map(|g| g.gate_id.as_str()).collect();
        assert!(
            ids.contains(&"deploy"),
            "deploy gate should fire: {gates:?}"
        );
    }

    #[test]
    fn match_members_respects_tier_and_triggers() {
        let cfg = r#"
schema_version = "2.1"
[[members]]
id = "irb"
name = "IRB"
default_tiers = ["MEDIUM", "HEAVY"]
triggers = ["IRB", "consent"]
owner_gate = true

[[members]]
id = "design"
name = "Design"
default_tiers = ["MEDIUM", "HEAVY"]
triggers = ["identification", "IV"]

[[members]]
id = "anything"
name = "Catch-all"
default_tiers = ["HEAVY"]
"#;
        let team = parse(cfg);
        // Light tier with no triggers fires nobody.
        assert!(match_members(&team, "tiny typo fix", Tier::Light).is_empty());
        // Medium tier with IRB keyword fires irb but not design.
        let m = match_members(&team, "draft IRB protocol", Tier::Medium);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].id, "irb");
        // Heavy tier pulls in the catch-all member with no triggers.
        let h = match_members(&team, "rewrite the identification strategy", Tier::Heavy);
        let ids: Vec<&str> = h.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&"design"));
        assert!(ids.contains(&"anything"));
    }

    #[test]
    fn user_personas_only_join_on_heavy_or_high_risk() {
        let cfg = r#"
schema_version = "2.1"
[[members]]
id = "design"
name = "Design"
default_tiers = ["MEDIUM", "HEAVY"]
triggers = ["identification"]

[user_evidence]
enabled = true
[[user_evidence.personas]]
id = "referee"
name = "Referee"
triggers = ["submission", "identification"]
"#;
        let team = parse(cfg);
        // Medium tier + low risk: no personas.
        assert!(match_user_personas(&team, "refine identification", Tier::Medium, 0.3).is_empty());
        // Medium tier + risk >= 0.7: persona joins because it trigger-matches.
        let p = match_user_personas(&team, "refine identification", Tier::Medium, 0.75);
        assert_eq!(p.len(), 1);
        // Heavy tier: persona joins on trigger.
        let p = match_user_personas(&team, "rewrite identification", Tier::Heavy, 0.1);
        assert_eq!(p.len(), 1);
    }

    #[test]
    fn schema_version_allowlist() {
        assert!(is_supported_schema("0.1"));
        assert!(is_supported_schema("2.1"));
        assert!(!is_supported_schema("99.99"));
    }

    #[test]
    fn task_id_is_stable_per_inputs() {
        let a = derive_task_id("pi", "draft intro", "2026-05-13");
        let b = derive_task_id("pi", "draft intro", "2026-05-13");
        assert_eq!(a, b);
        let c = derive_task_id("pi", "draft intro", "2026-05-14");
        assert_ne!(a, c);
    }

    #[test]
    fn gates_fire_from_both_sources() {
        // AEL-shape: a member with owner_gate=true + [owner_gates].gates[]
        // with explicit id/description. A task that hits both should
        // produce two FiredGate entries with distinct sources, and an
        // unrelated declared gate should not fire.
        let cfg = r#"
schema_version = "2.1"
[[members]]
id = "irb"
name = "IRB Gate"
default_tiers = ["MEDIUM", "HEAVY"]
triggers = ["IRB", "consent"]
owner_gate = true

[[members]]
id = "design"
name = "Design"
default_tiers = ["MEDIUM", "HEAVY"]
triggers = ["identification"]
owner_gate = false

[owner_gates]
gates = [
  { id = "submission",        description = "Any journal submission" },
  { id = "authorship-change", description = "Authorship-order change" },
]
"#;
        let team = parse(cfg);
        let task = "draft submission letter and IRB protocol";
        let matched = match_members(&team, task, Tier::Medium);
        let gates = match_gates(&team, &matched, task);

        let ids: Vec<&str> = gates.iter().map(|g| g.gate_id.as_str()).collect();
        assert!(
            ids.contains(&"irb"),
            "irb member gate should fire: {:?}",
            gates
        );
        assert!(
            ids.contains(&"submission"),
            "submission declared gate should fire: {:?}",
            gates
        );
        assert!(
            !ids.contains(&"authorship-change"),
            "authorship-change should not fire: {:?}",
            gates
        );
        let irb = gates.iter().find(|g| g.gate_id == "irb").unwrap();
        assert_eq!(irb.source, "member_owner_gate");
        let sub = gates.iter().find(|g| g.gate_id == "submission").unwrap();
        assert_eq!(sub.source, "declared_gate");
    }

    #[test]
    fn gate_ledger_idempotent_per_status() {
        // Writing the same gate twice with the same status must dedupe;
        // a status flip (pending → approved) must produce a new line.
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let r1 = GateRecord {
            schema_version: GATE_RECORD_SCHEMA_VERSION.to_string(),
            timestamp: "2026-05-13T00:00:00Z".to_string(),
            task_id: "abc".to_string(),
            task: "submit".to_string(),
            gate_id: "submission".to_string(),
            description: "x".to_string(),
            source: "declared_gate".to_string(),
            status: "pending".to_string(),
            approved_by: String::new(),
        };
        write_gate_ledger(root, "abc", std::slice::from_ref(&r1)).unwrap();
        write_gate_ledger(root, "abc", std::slice::from_ref(&r1)).unwrap();
        let body = std::fs::read_to_string(gates_path(root, "abc")).unwrap();
        assert_eq!(
            body.lines().count(),
            1,
            "same (task,gate,status) should dedupe"
        );

        let mut r2 = r1.clone();
        r2.status = "approved".to_string();
        r2.approved_by = "steve".to_string();
        write_gate_ledger(root, "abc", std::slice::from_ref(&r2)).unwrap();
        let body2 = std::fs::read_to_string(gates_path(root, "abc")).unwrap();
        assert_eq!(
            body2.lines().count(),
            2,
            "status flip pending→approved should append"
        );
    }
}
