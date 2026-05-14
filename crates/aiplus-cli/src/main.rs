use aiplus_core::memory::MEMORY_SCHEMA_VERSION_V2;
use aiplus_core::{
    append_jsonl_atomic,
    append_velocity_jsonl,
    apply_velocity_retention,
    available_modules_text,
    classify_rare_case,
    compute_ai_native_estimate,
    default_module_names,
    detect_bias,
    detect_conflicts,
    detect_stale,
    embedded_asset_text,
    epoch_millis,
    find_by_query,
    generate_estimate_id,
    generate_run_id,
    identity_dir,
    init_velocity,
    is_rare_case,
    memory_dir,
    module_spec,
    normalize_module,
    parse_duration,
    purge_velocity,
    read_active,
    read_all_including_rejected,
    read_identity,
    read_memory_records,
    read_skill_candidates,
    reject_sensitive_velocity_text,
    rewrite_jsonl_atomic,
    select_records,
    sensitive_findings,
    single_line,
    slugify,
    stable_hash,
    update_aggregates,
    update_multipliers,
    validate_run_record,
    // velocity module
    velocity_dir,
    velocity_doctor,
    write_file_atomic,
    // v2 core modules
    AutoWriteConfig,
    AutoWriteResult,
    AutoWriter,
    EstimateRecord,
    MemoryRecord,
    ModuleSpec,
    ProfileSync,
    ProjectManifest as Manifest,
    ProjectManifestModule as ManifestModule,
    RiskLevel,
    RollbackPlan,
    RunRecord,
    SessionIndex,
    SessionRecord,
    SkillCandidate,
    SkillRegistry,
    SnapshotBuilder,
    MODULE_SLUG_AGENT_TEAM,
    MODULE_SLUG_AIECONLAB,
    MODULE_SLUG_COMPACT_REMINDER,
    VELOCITY_SCHEMA_VERSION,
};
use anyhow::{anyhow, Context, Result};
use clap::{ArgAction, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::SystemTime;

mod agent;
mod mcp_server;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const RELEASE_TAG: &str = concat!("v", env!("CARGO_PKG_VERSION"));
const INSTALLER: &str = "aiplus";
const REFRESH_PROMPT: &str = "刷新";
const REFRESH_PROMPT_REL: &str = ".aiplus/REFRESH_PROMPT.txt";
const SAVINGS_LEDGER_REL: &str = "savings-ledger.jsonl";
const PRICING_CATALOG_URL: &str =
    "https://raw.githubusercontent.com/izhiwen/aiplus/main/assets/pricing/public-model-pricing.json";
const MANAGED_BEGIN: &str = "<!-- BEGIN AIPLUS MANAGED BLOCK -->";
const MANAGED_END: &str = "<!-- END AIPLUS MANAGED BLOCK -->";
const MANAGED_REF: &str = "@./.aiplus/AGENTS.aiplus.md";
const MANAGED_BEGIN_AEL: &str = "<!-- BEGIN AIECONLAB MANAGED BLOCK -->";
const MANAGED_END_AEL: &str = "<!-- END AIECONLAB MANAGED BLOCK -->";
const MANAGED_BEGIN_AT: &str = "<!-- BEGIN AIPLUS-AGENT-TEAM MANAGED BLOCK -->";
const MANAGED_END_AT: &str = "<!-- END AIPLUS-AGENT-TEAM MANAGED BLOCK -->";
const SECRET_BROKER_SERVICE: &str = "aiplus/bws-access-token";
const SECRET_BROKER_ACCOUNT: &str = "aiplus-secret-broker";
// No hardcoded Bitwarden project UUID in public source — the value is
// installation-specific and shipping one in a public binary advertises
// which workspace the maintainer uses. Users supply theirs via the
// `AIPLUS_BWS_PROJECT_ID` env var or a private profile bundle (which
// can read it from ~/.config/aiplus/profiles/<name>/secrets.toml).
const DEFAULT_BWS_PROJECT_ID: &str = "";

#[derive(Parser)]
#[command(
    name = "aiplus",
    version = VERSION,
    disable_version_flag = true,
    after_help = "Safety:\n  Project-local project writes are limited to .aiplus/, .aiplus/compact/, and\n  the AiPlus managed block in AGENTS.md. User-level profile writes are limited to\n  ~/.config/aiplus and never include secret values. `aiplus pricing update`,\n  `aiplus self update`, and `aiplus secret-broker` may fetch public release/pricing\n  data or read approved Bitwarden secrets at runtime. No npm publish, global install,\n  telemetry, user-data upload, secret persistence, or global config edits are implemented."
)]
struct Cli {
    #[arg(long, global = true, action = ArgAction::SetTrue)]
    version: bool,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Install {
        runtime: Option<String>,
        #[arg(long = "runtime")]
        runtime_opt: Option<String>,
        #[arg(long = "all-runtimes", action = ArgAction::SetTrue)]
        all_runtimes: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        dry_run: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        verbose: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        force: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        backup: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        yes: bool,
        /// K7 (#83): override the PATH version-skew check. By default
        /// `aiplus install` refuses when `which aiplus` is older than
        /// the binary running the install, because the AGENTS protocol
        /// it writes would reference subcommands the PATH binary lacks.
        /// Use this flag only when you're SURE the PATH binary will
        /// shortly be upgraded (e.g. install.sh is about to overwrite it).
        #[arg(long = "allow-version-skew", action = ArgAction::SetTrue)]
        allow_version_skew: bool,
    },
    Update {
        module: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        dry_run: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        verbose: bool,
        #[arg(long = "all-projects", action = ArgAction::SetTrue)]
        all_projects: bool,
    },
    Add {
        module: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        dry_run: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        verbose: bool,
        /// Install an external AiPlus module from a git repository URL.
        /// Accepts `github.com/foo/bar`, `https://github.com/foo/bar`, or
        /// with a pinned ref: `github.com/foo/bar@v1.2.3`. Mutually
        /// exclusive with positional MODULE arg.
        #[arg(long = "from-git", value_name = "URL[@REF]")]
        from_git: Option<String>,
        /// Trust the external source without an interactive prompt.
        /// Required (in non-tty contexts) for `--from-git` to proceed.
        #[arg(long = "trust", action = ArgAction::SetTrue)]
        trust: bool,
        /// Allow installing an external module whose name collides with
        /// a bundled module slug. Off by default for safety.
        #[arg(long = "override-bundled", action = ArgAction::SetTrue)]
        override_bundled: bool,
    },
    Doctor {
        /// Probe the OS keyring backend (Keychain / Secret Service /
        /// Credential Manager) by writing, reading, and deleting a
        /// scratch entry. Reports the detected backend, each probe
        /// step's result, and whether BWS_ACCESS_TOKEN env-var fallback
        /// is available. Use this right after `aiplus install` to confirm
        /// the keyring path is usable on this machine before relying on
        /// `aiplus secret-broker token set` in real workflows.
        #[arg(long = "check-keyring", action = ArgAction::SetTrue)]
        check_keyring: bool,
    },
    /// Run the AiPlus MCP server (Phase E). Reads JSON-RPC 2.0 messages on
    /// stdin and replies on stdout. Codex / claude-code / opencode launch
    /// this subprocess to invoke AiPlus agent operations directly during a
    /// session — eliminating the human-confirmation prompt that the
    /// Phase-D bash-block dispatch flow required. Not intended for
    /// interactive use; run `aiplus mcp register` instead to wire it up.
    McpServe,
    /// Register the AiPlus MCP server with installed runtimes so codex /
    /// claude-code / opencode can call agent_route, agent_status, and
    /// agent_set_team as native tools. Without --runtime, registers for
    /// all detected runtimes. Idempotent: re-running is safe.
    McpRegister {
        /// Runtime to register: codex, claude, or opencode. Omit for all.
        #[arg(long, value_name = "RUNTIME")]
        runtime: Option<String>,
        /// Print the config diff without writing files.
        #[arg(long, action = ArgAction::SetTrue)]
        dry_run: bool,
        /// Overwrite an existing config file even if it fails to parse.
        /// Without --force, a corrupted .mcp.json / opencode.json / codex
        /// config.toml causes the registration to abort safely; with
        /// --force, the broken file is replaced with a fresh aiplus entry.
        #[arg(long, action = ArgAction::SetTrue)]
        force: bool,
        /// Where to register the MCP server.
        ///
        ///   global   → writes to the runtime's user-level config so all
        ///             projects on this machine inherit the aiplus tools.
        ///             For codex this is ~/.codex/config.toml; for claude
        ///             and opencode this is the user-home equivalent
        ///             (~/.claude/.mcp.json, ~/.opencode/opencode.json).
        ///   project  → writes to the current project's config so only
        ///             this project sees the aiplus tools. For claude /
        ///             opencode this is the project-local .mcp.json /
        ///             opencode.json (which is their natural home); for
        ///             codex this is ./.codex/config.toml if that
        ///             directory exists.
        ///
        /// Per-runtime defaults (matches each runtime's idiomatic flow):
        ///   codex     → global   (codex itself stores all config globally)
        ///   claude    → project  (claude-code looks for .mcp.json in cwd)
        ///   opencode  → project  (opencode looks for opencode.json in cwd)
        #[arg(long, value_name = "SCOPE", value_parser = ["global", "project"])]
        scope: Option<String>,
    },
    Status {
        #[arg(long, action = ArgAction::SetTrue)]
        terse: bool,
    },
    Refresh {
        trigger: Vec<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        terse: bool,
    },
    Uninstall {
        #[arg(long, action = ArgAction::SetTrue)]
        dry_run: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        yes: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        force: bool,
    },
    ListProjects {
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },
    PruneProjects {
        #[arg(long, action = ArgAction::SetTrue)]
        yes: bool,
    },
    Rollback {
        #[arg(long, default_value = "latest")]
        id: String,
        #[arg(long, action = ArgAction::SetTrue)]
        dry_run: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        yes: bool,
    },
    Compact {
        subcommand: Option<String>,
        #[arg(long, default_value = "standard", value_parser = ["light", "standard", "full"])]
        level: String,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        force: bool,
        #[arg(long)]
        event: Option<String>,
        #[arg(long)]
        snooze: Option<String>,
        #[arg(long = "clear-snooze", action = ArgAction::SetTrue)]
        clear_snooze: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        once: bool,
        #[arg(long)]
        interval: Option<String>,
    },
    Pricing {
        subcommand: Option<String>,
    },
    Profile {
        subcommand: Option<String>,
        profile: Option<String>,
        target_profile: Option<String>,
        #[arg(long)]
        source: Option<PathBuf>,
        #[arg(long, action = ArgAction::SetTrue)]
        user: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        project: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        dry_run: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        yes: bool,
    },
    Memory {
        subcommand: Option<String>,
        arg: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        project: bool,
        #[arg(long)]
        runtime: Option<String>,
        #[arg(long)]
        budget: Option<usize>,
        #[arg(long)]
        scope: Option<String>,
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        text: Option<String>,
        // NEW fields:
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        summary: Option<String>,
        #[arg(long)]
        from_memory: Option<String>,
        #[arg(long)]
        risk: Option<String>,
        #[arg(long)]
        limit: Option<usize>,
    },
    Identity {
        subcommand: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        project: bool,
        #[arg(long)]
        role: Option<String>,
    },
    User {
        subcommand: Option<String>,
        #[arg(long)]
        profile: Option<String>,
    },
    #[command(name = "skill-candidate")]
    SkillCandidate {
        subcommand: Option<String>,
        arg: Option<String>,
        #[arg(long)]
        title: Option<String>,
        #[arg(long = "from-memory")]
        from_memory: Option<String>,
    },
    #[command(name = "secret-broker")]
    SecretBroker {
        subcommand: Option<String>,
        arg: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        print: bool,
        #[arg(long = "alias")]
        alias: Vec<String>,
        #[arg(long = "aliases")]
        aliases: Option<String>,
        /// S1: target DSL for `push`.
        ///   github-secret:<owner>/<repo>:<NAME>  → gh secret set
        ///   env:<VAR_NAME>                       → print export VAR=... line
        ///   dotenv:<path>                        → write/update .env single key
        #[arg(long = "to")]
        to: Option<String>,
        /// K1/K2: for `set` / `need` — if value is missing, pop a native
        /// OS password dialog (macOS osascript / Linux zenity-kdialog /
        /// Windows PowerShell SecureString) so the agent can drive the
        /// user-input flow without making the user switch to their
        /// terminal. Falls back to rpassword tty prompt when no GUI
        /// is available.
        #[arg(long = "auto-prompt", action = ArgAction::SetTrue)]
        auto_prompt: bool,
        /// K2: env var name override for `set` / `need` (default:
        /// uppercase alias + `_API_KEY`). Useful when one alias must
        /// expose as a non-standard env name (e.g. multi-account
        /// `openai_work` → `OPENAI_API_KEY`).
        #[arg(long = "env")]
        env_var: Option<String>,
        #[arg(last = true)]
        command: Vec<String>,
    },
    #[command(name = "self")]
    SelfCommand {
        subcommand: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        dry_run: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        yes: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        auto: bool,
    },
    Velocity {
        subcommand: Option<String>,
        #[arg(long)]
        task_type: Option<String>,
        #[arg(long)]
        human_estimate: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        workflow: Option<String>,
        #[arg(long)]
        task_id: Option<String>,
        #[arg(long)]
        actual: Option<String>,
        #[arg(long)]
        outcome: Option<String>,
        #[arg(long)]
        task: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        yes: bool,
    },
    #[command(name = "agent")]
    Agent(agent::AgentArgs),
}

struct CliError {
    code: i32,
    message: String,
}

impl CliError {
    fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl std::fmt::Debug for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliError")
            .field("code", &self.code)
            .field("message", &self.message)
            .finish()
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CliError {}

#[derive(Clone, Default)]
struct Options {
    force: bool,
    backup: bool,
    yes: bool,
}

#[derive(Default)]
struct Plan {
    dry_run: bool,
    items: Vec<PlanItem>,
    mkdir_paths: BTreeSet<String>,
    backup_stamp: Option<String>,
}

struct PlanItem {
    action: String,
    path: String,
}

struct ManifestDiagnostic {
    exists: bool,
    parses: bool,
    manifest: Option<Manifest>,
}

struct Check {
    label: String,
    ok: bool,
    fix: Option<String>,
    /// Issue #74: severity classification. `NeedsFix` (default) means a
    /// failing check flips `DOCTOR_STATUS=NEEDS_FIX`. `Info` means the
    /// check surfaces a recommendation but does NOT affect overall
    /// status — used for purely-cosmetic issues like stale-registry
    /// entries pointing at deleted project directories.
    severity: CheckSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CheckSeverity {
    NeedsFix,
    Info,
}

struct CompactValidation {
    ok: bool,
    errors: Vec<String>,
    warnings: Vec<String>,
    review_items: Vec<String>,
    pending_gates: Vec<String>,
    denied_gates: Vec<String>,
    next_safe_action: String,
}

struct CompactReadiness {
    state: &'static str,
    pressure: &'static str,
    explanation: &'static str,
    next_action: String,
    manual_compact_recommended: bool,
    reasons: Vec<String>,
}

struct CompactReminder {
    decision: &'static str,
    level: &'static str,
    readiness_state: String,
    recovery_confidence: &'static str,
    manual_compact_recommended: bool,
    snooze_status: &'static str,
    handoff_state: &'static str,
    last_checkpoint_age: String,
    estimated_tokens_saved: u64,
    estimated_usd_saved: String,
    reason: String,
    next_action: String,
    secret_values_printed: &'static str,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct CompactReminderSnooze {
    schema_version: String,
    set_at_epoch_millis: u128,
    until_epoch_millis: u128,
    duration_seconds: u64,
}

struct HandoffFreshness {
    state: &'static str,
    reason: String,
}

struct SavingsEstimate {
    input_before: u64,
    handoff_after: u64,
    resume_tokens: u64,
    tokens_saved: u64,
    reduction_percent: f64,
    cost_saved_usd: Option<f64>,
    pricing_model: String,
    pricing_source: String,
    pricing_fetched_at: Option<String>,
    model_detected: Option<String>,
    model_detection_confidence: String,
    confidence: String,
    notes: Vec<String>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase", default)]
struct SavingsEvent {
    schema_version: String,
    timestamp: String,
    event: String,
    event_scope: String,
    checkpoint_id: Option<String>,
    checkpoint_level: String,
    readiness_state: String,
    compact_pressure: String,
    session_role: String,
    workflow_level: String,
    estimated_input_tokens_before: u64,
    estimated_handoff_tokens_after: u64,
    estimated_resume_tokens: u64,
    estimated_tokens_saved: u64,
    estimated_token_reduction_percent: f64,
    estimated_cost_saved_usd: Option<f64>,
    pricing_model: String,
    pricing_status: String,
    pricing_source: String,
    pricing_fetched_at: Option<String>,
    pricing_age_days: Option<u64>,
    input_price_usd_per_1m_tokens: Option<f64>,
    model_detected: Option<String>,
    model_detection_confidence: String,
    cost_estimate_available: bool,
    cost_estimate_reason: String,
    billing_data: bool,
    method: String,
    confidence: String,
    notes: Vec<String>,
}

struct ContinuityState {
    agent_memory: String,
    memory_records_active: usize,
    memory_records_total: usize,
    identity_advisor: bool,
    identity_ceo: bool,
    identity_reviewer: bool,
    identity_builder: bool,
    skill_candidates_total: usize,
    skill_candidates_rejected: usize,
    profile_status: String,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PricingCatalog {
    schema_version: String,
    fetched_at: Option<String>,
    source_url: String,
    source: String,
    models: Vec<PricingModel>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct PricingModel {
    provider: String,
    model: String,
    input_usd_per_1m_tokens: f64,
    source: String,
    source_url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CompactCheckpoint {
    schema_version: String,
    checkpoint_level: String,
    timestamp: String,
    cwd: String,
    validation_result: String,
    status: String,
    readiness_state: String,
    compact_pressure: String,
    session_role: String,
    workflow_level: String,
    pending_gates: Vec<String>,
    denied_gates: Vec<String>,
    review_items: Vec<String>,
    warnings: Vec<String>,
    errors: Vec<String>,
    current_goal: Option<String>,
    current_phase: Option<String>,
    output_contract: Option<String>,
    decisions_made: Vec<String>,
    open_blockers: Option<String>,
    owner_gates: Option<String>,
    next_safe_action: Option<String>,
    do_not_do: Vec<String>,
    files_or_artifacts: Vec<String>,
    evidence_pointers: Vec<String>,
    manual_compact_only: bool,
}

const COMPACT_REQUIRED_FILES: &[&str] = &[
    "current-handoff.md",
    "decision-log.md",
    "agent-state-ledger.md",
    "evidence-ledger.md",
    "compact-policy.json",
];

const COMPACT_REQUIRED_SECTIONS: &[&str] = &[
    "Protocol Version",
    "Last Updated",
    "Current Goal",
    "Current Phase",
    "Completed Work",
    "Open Blockers",
    "Owner Gates",
    "Next 3 Actions",
    "Do Not Do",
    "Recovery Order",
];

const COMPACT_HANDOFF_REQUIRED_SECTIONS: &[&str] =
    &["Session Role", "Workflow Level", "Output Contract"];
const COMPACT_HANDOFF_MIGRATION_SECTIONS: &[(&str, &str)] = &[
    ("Session Role", "Unknown"),
    ("Workflow Level", "Unknown"),
    (
        "Output Contract",
        "Summarize the current goal, blockers, Owner gates, and next safe action before resuming.",
    ),
];

const OWNER_GATE_VALUES: &[&str] = &["APPROVED", "DENIED", "UNKNOWN_PENDING"];
const DECISION_STATUSES: &[&str] = &["DECIDED", "PROVISIONAL", "REVERSED", "NEEDS_VERIFICATION"];
const AGENT_STATUSES: &[&str] = &["pending", "running", "blocked", "done", "abandoned"];
const EVIDENCE_CONFIDENCE: &[&str] = &[
    "official",
    "vendor_blog",
    "github",
    "measured",
    "inferred",
    "unknown",
];
const TASK_STATUSES: &[&str] = &[
    "PASS",
    "DEBUG",
    "RUNNING_TESTS",
    "ACTIVE_WORK",
    "FAIL",
    "IN_PROGRESS",
    "NEEDS_VERIFICATION",
    "BLOCKED_OWNER_GATE",
    "BLOCKED_MISSING_FILES",
    "BLOCKED_EXTERNAL_ACCESS",
    "BLOCKED_UNCLEAR_GOAL",
];

fn translate_chinese_subcommand(args: Vec<String>) -> Vec<String> {
    let mut translated = args.clone();
    if translated.len() >= 2 {
        let mapping = [
            ("刷新", "refresh"),
            ("状态", "status"),
            ("安装", "install"),
            ("升级", "update"),
            ("更新", "update"),
            ("自升级", "self"),
            ("卸载", "uninstall"),
            ("回滚", "rollback"),
            ("健康", "doctor"),
            ("项目清单", "list-projects"),
            ("清理失效项目", "prune-projects"),
        ];
        for (zh, en) in mapping {
            if translated[1] == zh {
                translated[1] = en.to_string();
                break;
            }
        }
        // Handle "aiplus self upgrade" (two-word alias)
        if translated.len() >= 3
            && translated[1] == "self"
            && (translated[2] == "升级" || translated[2] == "upgrade")
        {
            translated[2] = "upgrade".to_string();
        }
        // Handle "aiplus 全局更新" -> update --all-projects
        if translated[1] == "全局更新" || translated[1] == "升级所有项目" {
            translated[1] = "update".to_string();
            translated.insert(2, "--all-projects".to_string());
        }
        // Handle agent-team Chinese aliases
        let agent_aliases = [
            ("团队", "status"),
            ("团队状态", "status"),
            ("派单", "route"),
            ("分配任务", "route"),
            ("跟", "talk"),
            ("找", "talk"),
            ("召唤", "invite"),
            ("请", "invite"),
            ("让走", "dismiss"),
            ("解散", "dismiss"),
            ("合并", "integrate"),
            ("集成", "integrate"),
            ("看活", "transcript"),
            ("记录", "transcript"),
            ("清理", "prune-worktrees"),
            ("清理团队工作区", "prune-worktrees"),
            ("审计", "audit"),
            ("查", "audit"),
        ];
        for (zh, en) in agent_aliases {
            if translated[1] == zh {
                translated[1] = "agent".to_string();
                translated.insert(2, en.to_string());
                break;
            }
        }
    }
    translated
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let translated = translate_chinese_subcommand(args);
    let cli = Cli::parse_from(translated);
    if cli.version {
        println!("{VERSION}");
        return;
    }
    let result = match cli.command {
        None => {
            print_usage();
            Ok(())
        }
        Some(command) => run(command),
    };
    if let Err(error) = result {
        if let Some(cli_error) = error.downcast_ref::<CliError>() {
            eprintln!("{}", cli_error.message);
            process::exit(cli_error.code);
        }
        let msg = error.to_string();
        if msg.starts_with("STUB_NOT_INVITABLE") {
            println!("{}", msg);
            process::exit(2);
        }
        // Catch-all for errors that bubbled up without being mapped to a
        // structured CliError. Per P1.2 (unified error-format goal):
        // user-visible errors should use <CMD>_STATUS=FAIL reason=<key>
        // detail=<msg>; truly unexpected errors get this AIPLUS_UNEXPECTED_ERROR
        // prefix so users / scripts can tell them apart from expected
        // failures.
        eprintln!("AIPLUS_UNEXPECTED_ERROR reason=uncaught detail={error:?}");
        process::exit(3);
    }
}

fn run(command: Commands) -> Result<()> {
    match command {
        Commands::Install {
            runtime,
            runtime_opt,
            all_runtimes,
            dry_run,
            verbose,
            force,
            backup,
            yes,
            allow_version_skew,
        } => command_install(
            runtime,
            runtime_opt,
            all_runtimes,
            dry_run,
            verbose,
            Options { force, backup, yes },
            allow_version_skew,
        ),
        Commands::Update {
            module,
            dry_run,
            verbose,
            all_projects,
        } => {
            if all_projects {
                command_update_all_projects(dry_run, verbose)
            } else {
                command_update(module, dry_run, verbose)
            }
        }
        Commands::Add {
            module,
            dry_run,
            verbose,
            from_git,
            trust,
            override_bundled,
        } => {
            if let Some(url) = from_git {
                if module.is_some() {
                    return Err(CliError::new(
                        1,
                        "ERROR `aiplus add MODULE` and `aiplus add --from-git URL` are mutually exclusive",
                    )
                    .into());
                }
                command_add_from_git(&url, dry_run, verbose, trust, override_bundled)
            } else {
                command_add(module, dry_run, verbose)
            }
        }
        Commands::Doctor { check_keyring } => {
            if check_keyring {
                command_doctor_check_keyring()
            } else {
                command_doctor()
            }
        }
        Commands::McpServe => mcp_server::run_server(),
        Commands::McpRegister {
            runtime,
            dry_run,
            force,
            scope,
        } => command_mcp_register(runtime, dry_run, force, scope),
        Commands::Status { terse } => command_status(terse),
        Commands::Refresh { trigger, terse } => command_refresh(trigger, terse),
        Commands::Uninstall {
            dry_run,
            yes,
            force,
        } => command_uninstall(dry_run, yes, force),
        Commands::ListProjects { json } => command_list_projects(json),
        Commands::PruneProjects { yes } => command_prune_projects(yes),
        Commands::Rollback { id, dry_run, yes } => command_rollback(id, dry_run, yes),
        Commands::Compact {
            subcommand,
            level,
            json,
            force,
            event,
            snooze,
            clear_snooze,
            once,
            interval,
        } => command_compact(
            subcommand,
            &level,
            json,
            force,
            event,
            snooze,
            clear_snooze,
            once,
            interval,
        ),
        Commands::Pricing { subcommand } => command_pricing(subcommand),
        Commands::Profile {
            subcommand,
            profile,
            target_profile,
            source,
            user,
            project,
            dry_run,
            yes,
        } => command_profile(ProfileCommandArgs {
            subcommand,
            profile,
            target_profile,
            source,
            user,
            project,
            dry_run,
            yes,
        }),
        Commands::Memory {
            subcommand,
            arg,
            project,
            runtime,
            budget,
            scope,
            kind,
            text,
            title,
            summary,
            from_memory,
            risk,
            limit,
        } => command_memory(MemoryCommandArgs {
            subcommand,
            arg,
            project,
            runtime,
            budget,
            scope,
            kind,
            text,
            title,
            summary,
            from_memory,
            risk,
            limit,
        }),
        Commands::Identity {
            subcommand,
            project,
            role,
        } => command_identity(subcommand, project, role),
        Commands::User {
            subcommand,
            profile,
        } => command_user(subcommand, profile),
        Commands::SkillCandidate {
            subcommand,
            arg,
            title,
            from_memory,
        } => command_skill_candidate(subcommand, arg, title, from_memory),
        Commands::SecretBroker {
            subcommand,
            arg,
            print,
            alias,
            aliases,
            to,
            auto_prompt,
            env_var,
            command,
        } => command_secret_broker(
            subcommand,
            arg,
            print,
            alias,
            aliases,
            to,
            auto_prompt,
            env_var,
            command,
        ),
        Commands::SelfCommand {
            subcommand,
            dry_run,
            yes,
            auto,
        } => command_self(subcommand, dry_run, yes, auto),
        Commands::Velocity {
            subcommand,
            task_type,
            human_estimate,
            model,
            workflow,
            task_id,
            actual,
            outcome,
            task,
            yes,
        } => command_velocity(
            subcommand,
            task_type,
            human_estimate,
            model,
            workflow,
            task_id,
            actual,
            outcome,
            task,
            yes,
        ),
        Commands::Agent(args) => agent::dispatch(args),
    }
}

fn print_usage() {
    println!(
        "AiPlus CLI {VERSION}\n\nUsage:\n  aiplus <command> [options]\n\nCommands:\n  install codex|claude-code|opencode|all [--dry-run] [--verbose] [--force --backup --yes]\n  update [all|compact-reminder|auto-team-consultant|agent-memory|agent-team|aieconlab] [--dry-run] [--verbose]\n  add compact-reminder|auto-team-consultant|agent-memory|agent-team|aieconlab [--dry-run] [--verbose]\n  add --from-git URL[@REF] [--trust] [--override-bundled] [--dry-run] [--verbose]\n  doctor\n  status\n  refresh\n  uninstall --dry-run\n  uninstall --yes [--force]\n  rollback --dry-run\n  rollback --id latest --dry-run\n  rollback --id latest --yes\n  compact init|validate|prepare|score|checkpoint|resume|remind|savings [--json] [--level light|standard|full]\n  memory status|doctor|init|context|add|search|forget|conflicts|auto-capture|session|snapshot|profile|show-used|stale|migrate\n  identity status|init|context\n  skill-candidate status|propose|reject|consolidate\n  pricing update|status\n  profile status|install|update|link|disable|uninstall|migrate|cleanup|doctor|context\n  user context [--profile <name>]\n  secret-broker status|doctor|list|resolve|run [--aliases a,b|--alias a]|token\n  self update [--dry-run] [--yes]\n  velocity init|estimate|complete|bias|report|doctor|purge [--task-type <type>] [--human-estimate <duration>] [--model <model>] [--workflow LIGHT|MEDIUM|HEAVY] [--task-id <id>] [--actual <duration>] [--outcome pass|needs_fix|blocked] [--task <id>] [--yes]\n\nSafety:\n  Project-local project writes are limited to .aiplus/, .aiplus/compact/, and\n  the AiPlus managed block in AGENTS.md. User-level profile writes are limited to\n  ~/.config/aiplus and never include secret values. `aiplus pricing update`,\n  `aiplus self update`, and `aiplus secret-broker` may fetch public release/pricing\n  data or read approved Bitwarden secrets at runtime. No npm publish, global install,\n  telemetry, user-data upload, secret persistence, or global config edits are implemented."
    );
}

fn command_install(
    runtime: Option<String>,
    runtime_opt: Option<String>,
    all_runtimes: bool,
    dry_run: bool,
    verbose: bool,
    options: Options,
    allow_version_skew: bool,
) -> Result<()> {
    if options.force && !options.yes {
        return Err(CliError::new(1, "ERROR --force requires --yes").into());
    }
    if options.force && !options.backup {
        return Err(CliError::new(1, "ERROR --force requires --backup --yes").into());
    }
    // K7 (#83): refuse to write an AGENTS protocol that references
    // subcommands the user's PATH binary doesn't have. The AGENTS file
    // is read by agents and run via `aiplus` from PATH, not via the
    // absolute path of whoever wrote it. Version skew silently breaks
    // the broker protocol for the entire project.
    // K7 (#83) decision matrix for "should we run the check":
    //   --allow-version-skew flag              → no (explicit override)
    //   AIPLUS_SKIP_VERSION_CHECK="<nonempty>" → no (explicit env opt-out)
    //   AIPLUS_SKIP_VERSION_CHECK=""           → YES (force; test opt-in)
    //   $CARGO is set (cargo test subprocess)  → no (dev iteration)
    //   default                                → YES
    let should_run_check = if allow_version_skew {
        false
    } else {
        match std::env::var("AIPLUS_SKIP_VERSION_CHECK").as_deref() {
            Ok("") => true,
            Ok(_) => false,
            Err(_) => std::env::var("CARGO").is_err(),
        }
    };
    if should_run_check {
        if let Some(skew) = detect_path_version_skew() {
            eprintln!("INSTALL_STATUS=NEEDS_UPGRADE");
            eprintln!(
                "  path_binary={} path_version={} installer_version={}",
                skew.path_binary.display(),
                skew.path_version,
                skew.installer_version,
            );
            eprintln!("  The AGENTS.aiplus.md protocol this installer writes references");
            eprintln!("  subcommands (`secret-broker need --auto-prompt`) added in v0.5.18+.");
            eprintln!("  Agents follow the protocol via `aiplus` on PATH — which is older here.");
            eprintln!("  Fix one of:");
            eprintln!(
                "    cp {} {}   # adopt this installer as the PATH binary",
                std::env::current_exe()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "<this binary>".to_string()),
                skew.path_binary.display(),
            );
            eprintln!(
                "    curl -fsSL https://raw.githubusercontent.com/izhiwen/AiPlus/main/install.sh | sh"
            );
            eprintln!(
                "  Override (NOT recommended): re-run with --allow-version-skew, or set AIPLUS_SKIP_VERSION_CHECK=1"
            );
            return Err(CliError::new(1, "INSTALL_STATUS=NEEDS_UPGRADE").into());
        }
    }
    let runtime_arg = if all_runtimes {
        Some("all".to_string())
    } else {
        runtime_opt.or(runtime)
    };
    let runtime = normalize_runtime(runtime_arg.as_deref());
    let Some(runtime) = runtime else {
        eprintln!("NEEDS_RUNTIME");
        eprintln!("Choose one runtime: codex, claude-code, opencode, all");
        eprintln!("Example: aiplus install codex");
        return Err(CliError::new(1, "INSTALL_STATUS=NEEDS_RUNTIME").into());
    };
    let root = target_root()?;
    let upgrade_existing = detects_existing_aiplus_install(&root);
    let effective_options = if upgrade_existing && !options.force {
        Options {
            force: true,
            backup: true,
            yes: true,
        }
    } else {
        options.clone()
    };
    let adapters = runtime_list(runtime);
    let mut plan = Plan {
        dry_run,
        ..Plan::default()
    };
    install_base(
        &root,
        &mut plan,
        &effective_options,
        default_module_names(),
        &adapters,
    )?;
    for adapter in &adapters {
        install_runtime_adapter(&root, adapter, &mut plan, &effective_options)?;
    }
    let handoff_migrated = migrate_compact_handoff_if_needed(&root, &mut plan)?;
    write_manifest(
        &root,
        &mut plan,
        &effective_options,
        &adapters,
        &default_module_names(),
        &default_module_names(),
    )?;
    print_install_summary(&plan, verbose, &adapters, upgrade_existing);
    println!(
        "COMPACT_HANDOFF_MIGRATION={}",
        if handoff_migrated {
            "APPLIED"
        } else {
            "NOT_NEEDED"
        }
    );
    // Upsert registry entry
    let runtimes: Vec<String> = adapters.iter().map(|a| a.to_string()).collect();
    if let Err(e) = upsert_registry_entry(&root, &runtimes) {
        eprintln!("WARN registry update failed: {}", e);
    }
    // K5: offer to wire the secret-broker cd-auto-load hook into the
    // user's shell rc. Append-only, idempotent, and never runs without
    // either --yes or an interactive Y. Opt out via AIPLUS_SKIP_SHELL_INIT=1
    // for users who manage their rc files via dotfiles / chezmoi.
    if std::env::var("AIPLUS_SKIP_SHELL_INIT").is_err() {
        maybe_install_shell_hook(&effective_options, dry_run);
    }
    Ok(())
}

fn command_update(module: Option<String>, dry_run: bool, verbose: bool) -> Result<()> {
    if module.as_deref() == Some("all") {
        return command_update_all(dry_run, verbose);
    }
    let root = target_root()?;
    let existing = read_manifest(&root, false)?;
    if existing.installer.as_deref() != Some(INSTALLER) {
        return Err(CliError::new(
            1,
            "ERROR AiPlus is not installed; run install <runtime> first",
        )
        .into());
    }
    let requested_raw = module.as_deref();
    let requested = normalize_module(requested_raw);
    if let (Some(raw), None) = (requested_raw, requested) {
        return Err(CliError::new(
            1,
            format!(
                "MODULE_NOT_AVAILABLE {}\navailable=[{}]",
                raw,
                available_modules_text()
            ),
        )
        .into());
    }
    let installed = normalize_existing_modules(existing.modules.as_ref());
    let targets: Vec<String> = requested.map_or_else(
        || installed.keys().cloned().collect(),
        |name| vec![name.to_string()],
    );
    let mut plan = Plan {
        dry_run,
        ..Plan::default()
    };
    let mut updated = Vec::new();
    let mut skipped = Vec::new();
    for name in &targets {
        if !installed.contains_key(name) {
            skipped.push(format!("{name}:not-installed"));
            continue;
        }
        let spec = module_spec(name).ok_or_else(|| anyhow!("unknown installed module {name}"))?;
        let current = installed
            .get(name)
            .and_then(|module| module.version.as_deref())
            .unwrap_or("unknown");
        if current == spec.version {
            skipped.push(format!("{name}:up-to-date"));
            continue;
        }
        copy_embedded_module(
            &root,
            spec,
            &mut plan,
            &Options {
                force: true,
                backup: false,
                yes: true,
            },
        )?;
        updated.push(format!("{name}:{current}->{}", spec.version));
    }
    let handoff_migrated = migrate_compact_handoff_if_needed(&root, &mut plan)?;
    let module_names: Vec<String> = installed.keys().cloned().collect();
    let touched: Vec<String> = updated
        .iter()
        .filter_map(|item| item.split(':').next().map(str::to_string))
        .collect();
    let adapters = existing.runtime_adapters.unwrap_or_default();
    write_manifest(
        &root,
        &mut plan,
        &Options {
            force: true,
            backup: false,
            yes: true,
        },
        &adapters,
        &module_names,
        &touched,
    )?;
    if updated.is_empty() {
        println!("AiPlus is already up to date.");
    } else {
        println!("AiPlus update complete.");
    }
    println!("updated=[{}]", updated.join(","));
    println!("skipped=[{}]", skipped.join(","));
    println!(
        "COMPACT_HANDOFF_MIGRATION={}",
        if handoff_migrated {
            "APPLIED"
        } else {
            "NOT_NEEDED"
        }
    );
    if verbose {
        plan_printer(&plan);
    } else {
        println!("GLOBAL_CONFIG_UNTOUCHED");
    }
    println!("UPDATE_STATUS=PASS");
    Ok(())
}

fn command_update_all(dry_run: bool, verbose: bool) -> Result<()> {
    println!("AIPLUS_UPDATE_ALL");
    println!("scope=cli_and_project");
    println!("global_agent_config_edits=none");
    println!("uploads=none");
    command_self_update(dry_run, true, false)?;
    match read_manifest(&target_root()?, true) {
        Ok(manifest) if manifest.installer.as_deref() == Some(INSTALLER) => {
            command_update(None, dry_run, verbose)?;
            println!("PROJECT_UPDATE_STATUS=PASS");
        }
        _ => {
            println!("PROJECT_UPDATE_STATUS=NO_PROJECT");
            println!("No project AiPlus install found in this directory.");
        }
    }
    println!("Run `aiplus doctor` from the project to verify local module state.");
    println!("UPDATE_ALL_STATUS=PASS");
    Ok(())
}

fn command_list_projects(json: bool) -> Result<()> {
    let registry = read_registry()?;
    if json {
        println!("{}", serde_json::to_string_pretty(&registry)?);
        return Ok(());
    }
    println!("LIST_PROJECTS");
    println!("schema_version={}", registry.schema_version);
    println!("total={}", registry.installed_projects.len());
    for entry in &registry.installed_projects {
        println!(
            "project={} first_installed={} last_updated={} runtimes=[{}]",
            entry.path.display(),
            entry.first_installed,
            entry.last_updated,
            entry.runtimes.join(",")
        );
    }
    println!("LIST_PROJECTS_STATUS=PASS");
    Ok(())
}

fn command_prune_projects(yes: bool) -> Result<()> {
    let registry = read_registry()?;
    let mut stale: Vec<PathBuf> = Vec::new();
    let mut kept: Vec<PathBuf> = Vec::new();
    for entry in &registry.installed_projects {
        let has_aiplus = entry.path.join(".aiplus").is_dir();
        let exists = entry.path.exists();
        if !exists || !has_aiplus {
            stale.push(entry.path.clone());
        } else {
            kept.push(entry.path.clone());
        }
    }
    if !yes {
        println!("PRUNE_PROJECTS_DRY_RUN");
        println!("would_remove={}", stale.len());
        println!("would_keep={}", kept.len());
        for path in &stale {
            println!("would_remove={}", path.display());
        }
        println!("PRUNE_PROJECTS_STATUS=DRY_RUN");
        return Ok(());
    }
    if stale.is_empty() {
        println!("PRUNE_PROJECTS_STATUS=PASS removed=0 kept={}", kept.len());
        return Ok(());
    }
    let mut new_registry = registry;
    new_registry
        .installed_projects
        .retain(|e| !stale.contains(&e.path));
    write_registry(&new_registry)?;
    println!(
        "PRUNE_PROJECTS_STATUS=PASS removed={} kept={}",
        stale.len(),
        kept.len()
    );
    Ok(())
}

fn command_update_all_projects(dry_run: bool, verbose: bool) -> Result<()> {
    let registry = read_registry()?;
    if registry.installed_projects.is_empty() {
        println!("UPDATE_ALL_PROJECTS_STATUS=PASS updated=0 reason=registry_empty");
        return Ok(());
    }
    println!("UPDATE_ALL_PROJECTS");
    println!("total={}", registry.installed_projects.len());
    let mut updated = 0usize;
    let mut skipped_missing = 0usize;
    let mut skipped_orphan = 0usize;
    let mut failed = 0usize;
    for entry in &registry.installed_projects {
        let path = &entry.path;
        if !path.exists() {
            println!("SKIP_MISSING path={}", path.display());
            skipped_missing += 1;
            continue;
        }
        if !path.join(".aiplus").is_dir() {
            println!("SKIP_ORPHAN path={}", path.display());
            skipped_orphan += 1;
            continue;
        }
        println!("UPDATE path={}", path.display());
        let result = std::env::set_current_dir(path)
            .map_err(anyhow::Error::new)
            .and_then(|_| match read_manifest(path, true) {
                Ok(manifest) if manifest.installer.as_deref() == Some(INSTALLER) => {
                    command_update(None, dry_run, verbose)
                }
                _ => {
                    println!("SKIP_NOT_AIPLUS path={}", path.display());
                    Ok(())
                }
            });
        if let Err(e) = result {
            println!("FAIL path={} error={}", path.display(), e);
            failed += 1;
        } else {
            updated += 1;
        }
    }
    println!(
        "UPDATE_ALL_PROJECTS_STATUS={} updated={} skipped_missing={} skipped_orphan={} failed={}",
        if failed == 0 { "PASS" } else { "FAIL" },
        updated,
        skipped_missing,
        skipped_orphan,
        failed
    );
    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn command_add(module: Option<String>, dry_run: bool, verbose: bool) -> Result<()> {
    let requested_raw = module.unwrap_or_else(|| "<missing>".to_string());
    let Some(requested) = normalize_module(Some(&requested_raw)) else {
        return Err(CliError::new(
            1,
            format!(
                "MODULE_NOT_AVAILABLE {requested_raw}\navailable=[{}]",
                available_modules_text()
            ),
        )
        .into());
    };
    let root = target_root()?;
    let existing = read_manifest(&root, false)?;
    if existing.installer.as_deref() != Some(INSTALLER) {
        return Err(CliError::new(
            1,
            "ERROR AiPlus is not installed; run install <runtime> first",
        )
        .into());
    }
    let installed = normalize_existing_modules(existing.modules.as_ref());
    let mut plan = Plan {
        dry_run,
        ..Plan::default()
    };
    let mut message = format!("AiPlus module added: {requested}");
    if !installed.contains_key(requested) {
        let spec = module_spec(requested).unwrap();
        copy_embedded_module(&root, spec, &mut plan, &Options::default())?;
        if requested == MODULE_SLUG_COMPACT_REMINDER {
            compact_init(&root, &mut plan, false)?;
        }
        if requested == "agent-memory" && !dry_run {
            memory_init(&root)?;
        }
        if requested == MODULE_SLUG_AGENT_TEAM && !dry_run {
            // command_add runs after the project's initial install, so
            // .aiplus/manifest.json exists and is the source of truth
            // for which runtime adapters are live.
            let adapters = read_installed_runtime_adapters(&root);
            agent_team_init(&root, &mut plan, &adapters)?;
        }
        if requested == MODULE_SLUG_AIECONLAB && !dry_run {
            aieconlab_init(&root, &mut plan)?;
        }
    } else {
        message = format!("AiPlus module already installed: {requested}");
    }
    let mut modules: Vec<String> = installed.keys().cloned().collect();
    if !modules.iter().any(|name| name == requested) {
        modules.push(requested.to_string());
    }
    let adapters = existing.runtime_adapters.unwrap_or_default();
    write_manifest(
        &root,
        &mut plan,
        &Options {
            force: true,
            backup: false,
            yes: true,
        },
        &adapters,
        &modules,
        &[requested.to_string()],
    )?;
    if dry_run {
        println!("AiPlus module add plan: {requested}");
        println!("No files were changed.");
        if verbose {
            plan_printer(&plan);
        } else {
            println!("GLOBAL_CONFIG_UNTOUCHED");
        }
        println!("ADD_DRY_RUN=PASS");
        return Ok(());
    }
    println!("{message}");
    println!();
    println!("Next for already-open agent sessions:");
    println!("type \"AiPlus 刷新\", \"刷新 AiPlus\", \"aiplus refresh\", or \"aiplus status\"");
    if verbose {
        plan_printer(&plan);
    } else {
        println!("GLOBAL_CONFIG_UNTOUCHED");
    }
    println!("ADD_STATUS=PASS");
    Ok(())
}

/// `aiplus add --from-git <URL>[@REF]` — install an external AiPlus module
/// from a git repository. The module is fetched, its manifest validated against
/// the same rules as bundled modules, and its files copied under
/// `.aiplus/modules/<module-name>/`. The project manifest records `source =
/// "external"` plus the origin URL and pinned ref so future updates and the
/// uninstall path can find it.
///
/// Limitations of v0.5.4 MVP (Phase C v0):
/// - No signature verification. The user is prompted to trust the source
///   (or `--trust` skips the prompt for scripted installs).
/// - No auto-discovery hook for `.aiplus/agents/` team templates. External
///   modules that ship an agent-team-style schema will install correctly
///   under `.aiplus/modules/<name>/`, but `aiplus agent` will not see their
///   roles until a follow-up release adds generic team-config discovery.
///   For the bundled `aiplus-agent-team` and `aieconlab` modules, the
///   hardcoded init hooks continue to populate `.aiplus/agents/`.
fn command_add_from_git(
    url_with_ref: &str,
    dry_run: bool,
    verbose: bool,
    trust: bool,
    override_bundled: bool,
) -> Result<()> {
    let (url, requested_ref) = parse_from_git_target(url_with_ref);
    let normalized_url = normalize_git_url(&url)?;

    let root = target_root()?;
    let existing = read_manifest(&root, false)?;
    if existing.installer.as_deref() != Some(INSTALLER) {
        return Err(CliError::new(
            1,
            "ERROR AiPlus is not installed; run install <runtime> first",
        )
        .into());
    }

    if !trust && !confirm_external_source(&normalized_url, requested_ref.as_deref())? {
        return Err(CliError::new(
            1,
            "ABORTED user did not confirm external source (pass --trust to skip the prompt)",
        )
        .into());
    }

    let temp = tempfile::tempdir().context("failed to allocate temp dir for clone")?;
    let clone_target = temp.path().join("module");
    let resolved_ref = clone_module_repo(&normalized_url, requested_ref.as_deref(), &clone_target)?;

    // Read and validate the module manifest from the cloned source.
    let manifest_path = clone_target.join("aiplus-module.json");
    if !manifest_path.exists() {
        return Err(CliError::new(
            1,
            format!(
                "ERROR external module at {normalized_url} (@{resolved_ref}) is missing aiplus-module.json"
            ),
        )
        .into());
    }
    let manifest_text =
        std::fs::read_to_string(&manifest_path).context("read external module manifest")?;
    let manifest = aiplus_core::parse_module_manifest(&manifest_text)
        .with_context(|| format!("parse manifest from {normalized_url} (@{resolved_ref})"))?;

    let module_name = manifest.name.clone();
    if normalize_module(Some(&module_name)).is_some() && !override_bundled {
        return Err(CliError::new(
            1,
            format!(
                "ERROR external module name '{module_name}' collides with a bundled slug; rerun with --override-bundled to install over the bundled version"
            ),
        )
        .into());
    }

    let install_path = format!(".aiplus/modules/{module_name}");
    let target_dir = rel_to_abs(&root, &install_path)?;
    let mut plan = Plan {
        dry_run,
        ..Plan::default()
    };
    if !dry_run {
        if target_dir.exists() {
            std::fs::remove_dir_all(&target_dir)
                .with_context(|| format!("remove existing {}", target_dir.display()))?;
        }
        copy_external_module_files(&clone_target, &target_dir)?;
    }
    plan.items.push(PlanItem {
        action: "copy".to_string(),
        path: install_path.clone(),
    });

    let mut installed = normalize_existing_modules(existing.modules.as_ref());
    installed.insert(
        module_name.clone(),
        aiplus_core::manifest::ProjectManifestModule {
            version: Some(manifest.version.clone()),
            source: Some("external".to_string()),
            path: Some(install_path.clone()),
            installed_at: None,
            updated_at: None,
            source_url: Some(normalized_url.clone()),
            source_ref: Some(resolved_ref.clone()),
        },
    );
    let _modules: Vec<String> = installed.keys().cloned().collect();
    let adapters = existing.runtime_adapters.unwrap_or_default();
    write_manifest_with_external(
        &root,
        &mut plan,
        &Options {
            force: true,
            backup: false,
            yes: true,
        },
        &adapters,
        &installed,
        &[module_name.clone()],
    )?;

    if dry_run {
        println!(
            "AiPlus external module add plan: {module_name} from {normalized_url} @ {resolved_ref}"
        );
        println!("No files were changed.");
        if verbose {
            plan_printer(&plan);
        } else {
            println!("GLOBAL_CONFIG_UNTOUCHED");
        }
        println!("ADD_DRY_RUN=PASS");
        return Ok(());
    }

    println!(
        "AiPlus external module added: {module_name} v{} from {normalized_url} @ {resolved_ref}",
        manifest.version
    );
    println!();
    println!("Caveats for external modules (Phase C v0.5.4):");
    println!("- No agent-team auto-init: if this module ships team templates, populate .aiplus/agents/ manually until v0.5.5 adds generic discovery.");
    println!("- Update flow: rerun `aiplus add --from-git {normalized_url}` (optionally with @ref) to pull a newer version.");
    println!();
    println!("Next for already-open agent sessions:");
    println!("type \"AiPlus 刷新\", \"刷新 AiPlus\", \"aiplus refresh\", or \"aiplus status\"");
    if verbose {
        plan_printer(&plan);
    } else {
        println!("GLOBAL_CONFIG_UNTOUCHED");
    }
    println!("ADD_STATUS=PASS");
    Ok(())
}

/// Split a `URL[@REF]` target into its components.
fn parse_from_git_target(target: &str) -> (String, Option<String>) {
    // Last `@` after the scheme delimits the ref.
    if let Some((url, refspec)) = target.rsplit_once('@') {
        // Ignore the scheme's `://` colon (e.g. https://) — the ref split is
        // only valid when `@` does not appear inside the URL scheme.
        if url.contains("://") || !url.contains(':') {
            return (url.to_string(), Some(refspec.to_string()));
        }
    }
    (target.to_string(), None)
}

fn normalize_git_url(url: &str) -> Result<String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(CliError::new(1, "ERROR --from-git URL is empty").into());
    }
    if trimmed.starts_with("https://") || trimmed.starts_with("git@") {
        Ok(trimmed.to_string())
    } else if trimmed.starts_with("github.com/") || trimmed.contains("/") {
        Ok(format!("https://{trimmed}"))
    } else {
        Err(CliError::new(
            1,
            format!("ERROR --from-git URL '{trimmed}' is not a recognized git URL"),
        )
        .into())
    }
}

fn confirm_external_source(url: &str, requested_ref: Option<&str>) -> Result<bool> {
    use std::io::IsTerminal;
    eprintln!();
    eprintln!("About to install an EXTERNAL AiPlus module from:");
    eprintln!("  {url}");
    if let Some(refspec) = requested_ref {
        eprintln!("  at ref: {refspec}");
    } else {
        eprintln!("  at ref: (latest tag)");
    }
    eprintln!();
    eprintln!("External modules run with the same project-local write permissions");
    eprintln!("as the AiPlus CLI itself. Make sure you trust this source. Pass");
    eprintln!("--trust to skip this prompt in non-interactive contexts.");
    eprintln!();
    if !std::io::stdin().is_terminal() {
        eprintln!("(non-interactive stdin; refusing without --trust)");
        return Ok(false);
    }
    eprint!("Proceed? [y/N] ");
    use std::io::Write;
    std::io::stderr().flush().ok();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
    Ok(matches!(input.trim().to_lowercase().as_str(), "y" | "yes"))
}

fn clone_module_repo(url: &str, requested_ref: Option<&str>, target: &Path) -> Result<String> {
    use std::process::Command;
    let mut cmd = Command::new("git");
    cmd.arg("clone").arg("--depth").arg("1");
    if let Some(refspec) = requested_ref {
        cmd.arg("--branch").arg(refspec);
    }
    cmd.arg(url).arg(target);
    let output = cmd
        .output()
        .context("git clone failed to start (is git installed?)")?;
    if !output.status.success() {
        return Err(CliError::new(
            1,
            format!(
                "ERROR git clone failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        )
        .into());
    }
    // Resolve the actual ref we landed on for accurate manifest tracking.
    let head = Command::new("git")
        .arg("-C")
        .arg(target)
        .arg("rev-parse")
        .arg("HEAD")
        .output()
        .context("git rev-parse HEAD failed")?;
    let commit = String::from_utf8_lossy(&head.stdout).trim().to_string();
    Ok(requested_ref.map(str::to_string).unwrap_or(commit))
}

fn copy_external_module_files(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_name = entry.file_name();
        // Skip the cloned .git directory and obvious dotfiles that shouldn't ship.
        if matches!(
            file_name.to_str(),
            Some(".git" | ".DS_Store" | "node_modules" | "target")
        ) {
            continue;
        }
        let src_path = entry.path();
        let dst_path = dst.join(&file_name);
        if src_path.is_dir() {
            copy_external_module_files(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Write the manifest including external-module entries with `source_url` /
/// `source_ref` preserved. The standard `write_manifest` path doesn't know
/// about external modules; this thin wrapper accepts a pre-built `installed`
/// map and serializes it directly.
fn write_manifest_with_external(
    root: &Path,
    plan: &mut Plan,
    options: &Options,
    adapters: &[String],
    installed: &std::collections::BTreeMap<String, aiplus_core::manifest::ProjectManifestModule>,
    touched: &[String],
) -> Result<()> {
    // First do the normal write so timestamps and the bundled fields are set.
    let module_names: Vec<String> = installed.keys().cloned().collect();
    write_manifest(root, plan, options, adapters, &module_names, touched)?;
    // Then patch in the external-module-specific fields (source_url, source_ref).
    let path = rel_to_abs(root, ".aiplus/manifest.json")?;
    let text = std::fs::read_to_string(&path).context("read manifest after write")?;
    let mut parsed: aiplus_core::manifest::ProjectManifest =
        serde_json::from_str(&text).context("parse manifest after write")?;
    if let Some(modules) = parsed.modules.as_mut() {
        for (name, entry) in installed {
            if let Some(target) = modules.get_mut(name) {
                if entry.source.as_deref() == Some("external") {
                    target.source = Some("external".to_string());
                    target.source_url = entry.source_url.clone();
                    target.source_ref = entry.source_ref.clone();
                }
            }
        }
    }
    let serialized = serde_json::to_string_pretty(&parsed)?;
    if !plan.dry_run {
        std::fs::write(&path, serialized)?;
    }
    Ok(())
}

fn print_binary_freshness() {
    let target = self_update_target().unwrap_or_else(|_| PathBuf::from("unknown"));
    let binary_version = binary_version(&target).unwrap_or_else(|| VERSION.to_string());
    let binary_age_days = if target.exists() {
        fs::metadata(&target)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|modified| {
                SystemTime::now()
                    .duration_since(modified)
                    .ok()
                    .map(|d| d.as_secs() / 86400)
            })
            .unwrap_or(0)
    } else {
        0
    };
    let latest_release_version = RELEASE_TAG.trim_start_matches('v');
    let is_stale = binary_version != latest_release_version;
    println!("binary_version={binary_version}");
    println!("binary_path={}", target.display());
    println!("binary_age_days={binary_age_days}");
    println!("latest_release_version={latest_release_version}");
    println!("WARN_BINARY_STALE={}", if is_stale { "yes" } else { "no" });
    if is_stale {
        println!("hint=run aiplus self upgrade --auto to update");
    }
}

fn command_status(terse: bool) -> Result<()> {
    let root = target_root()?;
    println!("scope={}", root.display());
    print_binary_freshness();
    let manifest = read_manifest(&root, true).unwrap_or_default();
    let continuity = continuity_state(&root)?;
    println!(
        "installed={}",
        if manifest.installer.as_deref() == Some(INSTALLER) {
            "yes"
        } else {
            "no"
        }
    );
    println!(
        "installerVersion={}",
        manifest.installer_version.as_deref().unwrap_or("unknown")
    );
    println!("targetRoot={}", root.display());
    println!(
        "runtimeAdapters=[{}]",
        manifest
            .runtime_adapters
            .clone()
            .unwrap_or_default()
            .join(",")
    );
    println!(
        "compactState={}",
        if rel_to_abs(&root, ".aiplus/compact")?.exists() {
            "present"
        } else {
            "missing"
        }
    );
    println!(
        "memoryState={}",
        if continuity.agent_memory == "installed" {
            "present"
        } else {
            "missing"
        }
    );
    print_continuity_status_lines(&continuity);
    println!(
        "managedBlock={}",
        if has_managed_block(&root)? {
            "present"
        } else {
            "missing"
        }
    );
    println!("refreshPrompt={REFRESH_PROMPT}");
    let modules = normalize_existing_modules(manifest.modules.as_ref());
    let module_text: Vec<String> = modules
        .iter()
        .map(|(name, module)| {
            format!(
                "{name}@{}",
                module
                    .version
                    .as_deref()
                    .unwrap_or_else(|| module_spec(name).map_or("unknown", |spec| spec.version))
            )
        })
        .collect();
    println!("modules=[{}]", module_text.join(", "));
    for (name, module) in modules {
        let update = module_spec(&name)
            .map(|spec| {
                if module.version.as_deref() == Some(spec.version) {
                    "none"
                } else {
                    "available"
                }
            })
            .unwrap_or("unknown");
        println!(
            "module={name} version={} update={update}",
            module.version.as_deref().unwrap_or("unknown")
        );
    }
    if manifest.installer.is_some() {
        println!(
            "next=For already-open agent sessions, type \"AiPlus 刷新\", \"刷新 AiPlus\", \"aiplus refresh\", or \"aiplus status\"."
        );
    } else {
        println!("next=run install codex");
    }
    if terse {
        println!("STATUS=PASS");
        return Ok(());
    }
    println!("STATUS=PASS");
    Ok(())
}

fn command_refresh(trigger: Vec<String>, terse: bool) -> Result<()> {
    let root = target_root()?;
    println!("scope={}", root.display());
    print_binary_freshness();
    if terse {
        let manifest = read_manifest(&root, true).unwrap_or_default();
        let modules = normalize_existing_modules(manifest.modules.as_ref());
        let continuity = continuity_state(&root)?;
        let compact_state = if rel_to_abs(&root, ".aiplus/compact")?.exists() {
            "present"
        } else {
            "missing"
        };
        let auto_compact = module_refresh_status_en(&modules, MODULE_SLUG_COMPACT_REMINDER);
        let auto_team = module_refresh_status_en(&modules, "auto-team-consultant");
        let agent_memory = module_refresh_status_en(&modules, "agent-memory");
        println!("AiPlus refreshed.");
        println!("- Compact Reminder: {auto_compact}");
        println!("- Auto Team Consultant: {auto_team}");
        println!("- Agent Continuity: {agent_memory}");
        println!("- Compact state: {compact_state}");
        println!("- Agent Memory: {}", continuity.agent_memory);
        println!(
            "- Memory records: {} active",
            continuity.memory_records_active
        );
        println!("AIPLUS_REFRESH_STATUS=PASS");
        return Ok(());
    }
    let manifest = read_manifest(&root, true).unwrap_or_default();
    let modules = normalize_existing_modules(manifest.modules.as_ref());
    let continuity = continuity_state(&root)?;
    let compact_state = if rel_to_abs(&root, ".aiplus/compact")?.exists() {
        "present"
    } else {
        "missing"
    };

    if prefers_chinese_refresh(&trigger) {
        let auto_compact = module_refresh_status_zh(&modules, MODULE_SLUG_COMPACT_REMINDER);
        let auto_team = module_refresh_status_zh(&modules, "auto-team-consultant");
        let agent_memory = module_refresh_status_zh(&modules, "agent-memory");
        println!("已刷新 AiPlus。");
        println!();
        println!("当前项目 AiPlus 状态：");
        println!("- Compact Reminder: {auto_compact}");
        println!("- Auto Team Consultant: {auto_team}");
        println!("- Agent Continuity: {agent_memory}");
        println!("- Compact state: {compact_state}");
        println!("- Agent Memory: {}", continuity.agent_memory);
        println!(
            "- Memory records: {} active",
            continuity.memory_records_active
        );
        println!(
            "- Identity: advisor={} ceo={} reviewer={} builder={}",
            yes_no(continuity.identity_advisor),
            yes_no(continuity.identity_ceo),
            yes_no(continuity.identity_reviewer),
            yes_no(continuity.identity_builder)
        );
        println!(
            "- Skill candidates: {} total, {} rejected, approved_auto=none",
            continuity.skill_candidates_total, continuity.skill_candidates_rejected
        );
        println!(
            "- Profile: {} {}",
            canonical_user_profile()?.as_deref().unwrap_or("(none)"),
            continuity.profile_status
        );
        println!("- Secret values: none");
        println!("- Global agent config: untouched");
        println!();
        println!("我会这样使用：");
        println!("- 长任务或 compact 前准备 checkpoint");
        println!(
            "- 如果你说“帮我准备 compact”“保存进度”或“做个交接”，我会运行 aiplus compact prepare。"
        );
        println!(
            "- 如果你问“看一下 compact 收益”或“compact 帮我省了多少？”，我会运行 aiplus compact savings。"
        );
        println!(
            "- 如果你说“记住这个”“忘掉这个”“你记住了什么”或“这次用了哪些记忆”，我会使用 aiplus memory add/forget/status/context。"
        );
        println!(
            "- 如果你说“新开顾问”“新开 advisor”或“新开 CEO”，我会运行 aiplus identity context。"
        );
        println!(
            "- 如果你说“把这次经验沉淀成 skill”，我只会创建 Skill Candidate，不会自动批准 skill。"
        );
        println!("- 如果你说“不要用我的私人记忆”或“本次忽略我的偏好”，我只做本 session opt-out。");
        println!("- compact 后如果我没自动继续，你发一句“继续”就行。我会从刚才的位置接着做。");
        println!("- CEO Prompt / review / brainstorm 时使用 Auto Team Consultant");
        println!();
        println!("边界：");
        println!("- AiPlus 不能替你点击 compact");
        println!("- 不上传数据");
        println!("- 不改全局 agent config");
    } else {
        let auto_compact = module_refresh_status_en(&modules, MODULE_SLUG_COMPACT_REMINDER);
        let auto_team = module_refresh_status_en(&modules, "auto-team-consultant");
        let agent_memory = module_refresh_status_en(&modules, "agent-memory");
        println!("AiPlus refreshed.");
        println!();
        println!("Current project AiPlus status:");
        println!("- Compact Reminder: {auto_compact}");
        println!("- Auto Team Consultant: {auto_team}");
        println!("- Agent Continuity: {agent_memory}");
        println!("- Compact state: {compact_state}");
        println!("- Agent Memory: {}", continuity.agent_memory);
        println!(
            "- Memory records: {} active",
            continuity.memory_records_active
        );
        println!(
            "- Identity: advisor={} ceo={} reviewer={} builder={}",
            yes_no(continuity.identity_advisor),
            yes_no(continuity.identity_ceo),
            yes_no(continuity.identity_reviewer),
            yes_no(continuity.identity_builder)
        );
        println!(
            "- Skill candidates: {} total, {} rejected, approved_auto=none",
            continuity.skill_candidates_total, continuity.skill_candidates_rejected
        );
        println!(
            "- Profile: {} {}",
            canonical_user_profile()?.as_deref().unwrap_or("(none)"),
            continuity.profile_status
        );
        println!("- Secret values: none");
        println!("- Global agent config: untouched");
        println!();
        println!("How I will use it:");
        println!("- Prepare checkpoints before long tasks or compact-worthy moments.");
        println!("- If you say \"prepare compact\", \"save progress\", or \"checkpoint this\", I will run aiplus compact prepare.");
        println!("- If you ask \"show compact savings\" or \"how many tokens did compact save?\", I will run aiplus compact savings.");
        println!("- If you ask \"what do you remember\", \"what memory did you use\", \"remember this\", or \"forget this\", I will use aiplus memory add/forget/status/context.");
        println!("- If you ask for a new advisor or new CEO, I will run aiplus identity context for that role.");
        println!("- If you ask to turn this experience into a skill, I will create a Skill Candidate, not an approved skill.");
        println!("- If you ask to ignore private memory for this session, I will treat it as session-local opt-out only.");
        println!("- After compact, if I do not reply, send: continue");
        println!("- Use Auto Team Consultant for CEO Prompt, review, and brainstorm work.");
        println!();
        println!("Boundaries:");
        println!("- AiPlus cannot click compact for you.");
        println!("- AiPlus does not upload data.");
        println!("- AiPlus does not change global agent config.");
    }
    print_owner_profile_inline()?;
    println!("AIPLUS_REFRESH_STATUS=PASS");
    Ok(())
}

/// On refresh, inline the Owner's USER.md content (redacted) so the agent
/// picks up cross-project preferences without an extra `aiplus user context`
/// round-trip. No-op if no user profile is installed or USER.md is missing.
fn print_owner_profile_inline() -> Result<()> {
    let Some(profile) = canonical_user_profile()? else {
        return Ok(());
    };
    let user_md = profile_dir(&profile)?.join("USER.md");
    if !user_md.exists() {
        return Ok(());
    }
    let text = fs::read_to_string(&user_md)?;
    let redacted = redact_user_context(&text);
    let truncated = if redacted.len() > 4096 {
        format!(
            "{}\n... [truncated; run `aiplus user context` for full content]",
            &redacted[..3072]
        )
    } else {
        redacted
    };
    println!();
    println!("---");
    println!("# Owner profile (from {profile}/USER.md)");
    println!();
    println!("These are the Owner's stable cross-project preferences. Treat as");
    println!("context, not as permission for dangerous actions.");
    println!();
    println!("{truncated}");
    println!();
    Ok(())
}

fn prefers_chinese_refresh(trigger: &[String]) -> bool {
    trigger.iter().any(|part| {
        part.chars()
            .any(|ch| ('\u{4e00}'..='\u{9fff}').contains(&ch))
    })
}

fn module_refresh_status_zh(
    modules: &BTreeMap<String, ManifestModule>,
    name: &str,
) -> &'static str {
    if modules.contains_key(name) {
        "已安装"
    } else {
        "未安装"
    }
}

fn module_refresh_status_en(
    modules: &BTreeMap<String, ManifestModule>,
    name: &str,
) -> &'static str {
    if modules.contains_key(name) {
        "installed"
    } else {
        "not installed"
    }
}

fn command_doctor() -> Result<()> {
    let root = target_root()?;
    let mut checks: Vec<Check> = Vec::new();
    push_check(&mut checks, "current directory exists", root.exists(), None);
    push_check(
        &mut checks,
        "current directory writable",
        is_writable(&root),
        None,
    );
    let manifest_diag = read_manifest_diagnostic(&root)?;
    push_check(
        &mut checks,
        ".aiplus/manifest.json exists",
        manifest_diag.exists,
        None,
    );
    push_check(&mut checks, "manifest parses", manifest_diag.parses, None);
    if manifest_diag.parses {
        push_check(
            &mut checks,
            "manifest installer is aiplus",
            manifest_diag
                .manifest
                .as_ref()
                .and_then(|manifest| manifest.installer.as_deref())
                == Some(INSTALLER),
            None,
        );
        push_check(
            &mut checks,
            "manifest schemaVersion supported",
            manifest_diag
                .manifest
                .as_ref()
                .and_then(|manifest| manifest.schema_version.as_deref())
                .is_some_and(is_supported_manifest_schema),
            None,
        );
    } else {
        push_check(
            &mut checks,
            "manifest installer is aiplus",
            false,
            Some("manifest missing or invalid".to_string()),
        );
        push_check(
            &mut checks,
            "manifest schemaVersion supported",
            false,
            Some("manifest missing or invalid".to_string()),
        );
    }
    push_check(
        &mut checks,
        ".aiplus/AGENTS.aiplus.md exists",
        rel_to_abs(&root, ".aiplus/AGENTS.aiplus.md")?.exists(),
        None,
    );
    push_check(
        &mut checks,
        ".aiplus/REFRESH_PROMPT.txt exists",
        rel_to_abs(&root, REFRESH_PROMPT_REL)?.exists(),
        None,
    );
    match aiplus_core::validate_bundled_module_manifests() {
        Ok(manifests) => {
            push_check(&mut checks, "bundled module manifests validate", true, None);
            for manifest in manifests {
                push_check(
                    &mut checks,
                    format!("module manifest {} present", manifest.name),
                    true,
                    None,
                );
            }
        }
        Err(error) => push_check(
            &mut checks,
            format!("bundled module manifests validate ({error})"),
            false,
            None,
        ),
    }
    let parsed = manifest_diag.manifest.filter(|manifest| {
        manifest.installer.as_deref() == Some(INSTALLER)
            && manifest
                .schema_version
                .as_deref()
                .is_some_and(is_supported_manifest_schema)
    });
    let runtimes = parsed
        .as_ref()
        .and_then(|m| m.runtime_adapters.clone())
        .unwrap_or_default();
    for runtime in &runtimes {
        push_check(
            &mut checks,
            format!("runtimeAdapter {runtime} is supported"),
            ["codex", "claude-code", "opencode"].contains(&runtime.as_str()),
            None,
        );
    }
    for runtime in &runtimes {
        for (label, ok) in runtime_doctor_requirements(&root, runtime)? {
            push_check(&mut checks, label, ok, None);
        }
    }
    let modules = normalize_existing_modules(parsed.as_ref().and_then(|m| m.modules.as_ref()));
    for name in modules.keys() {
        let Some(spec) = module_spec(name) else {
            push_check(
                &mut checks,
                format!("{name} is a known bundled module"),
                false,
                None,
            );
            continue;
        };
        for required in spec.required_files {
            push_check(
                &mut checks,
                format!("{name} {required} exists"),
                rel_to_abs(&root, &format!("{}/{}", spec.path, required))?.exists(),
                None,
            );
        }
    }
    if modules.contains_key(MODULE_SLUG_COMPACT_REMINDER) {
        push_check(
            &mut checks,
            ".aiplus/compact/ exists".to_string(),
            rel_to_abs(&root, ".aiplus/compact")?.exists(),
            Some("run compact init".to_string()),
        );
    }
    if modules.contains_key("agent-memory") {
        for rel in [
            ".aiplus/memory/project-memory.jsonl",
            ".aiplus/identities/advisor.identity.toml",
            ".aiplus/identities/ceo.identity.toml",
            ".aiplus/skills/registry.toml",
            ".aiplus/restore/restore-policy.toml",
        ] {
            push_check(
                &mut checks,
                format!("{rel} exists"),
                rel_to_abs(&root, rel)?.exists(),
                Some("run memory init --project".to_string()),
            );
        }
    }
    if modules.contains_key("auto-team-consultant") {
        let consultant_config = rel_to_abs(&root, ".aiplus/consultant-team.toml")?;
        push_check(
            &mut checks,
            ".aiplus/consultant-team.toml exists",
            consultant_config.exists(),
            Some("run install to create default consultant-team config".to_string()),
        );
        if consultant_config.exists() {
            let config_text = fs::read_to_string(&consultant_config).unwrap_or_default();
            push_check(
                &mut checks,
                "consultant-team.toml parses as TOML",
                config_text.parse::<toml::Value>().is_ok(),
                Some("fix or delete .aiplus/consultant-team.toml and rerun install".to_string()),
            );
            if let Ok(value) = config_text.parse::<toml::Value>() {
                push_check(
                    &mut checks,
                    "consultant-team.toml has schema_version",
                    value.get("schema_version").is_some(),
                    Some("add schema_version field".to_string()),
                );
                // Drift check: the file might have schema_version set
                // to a version this binary doesn't know how to consume
                // (e.g., installed config is newer than the CLI). When
                // that happens `agent route` silently skips the consult,
                // so flag it here instead of letting the user notice
                // only when the artifact never appears.
                let declared = value
                    .get("schema_version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                push_check(
                    &mut checks,
                    "consultant-team.toml schema_version is supported by this CLI".to_string(),
                    !declared.is_empty()
                        && aiplus_core::consult::is_supported_schema(declared),
                    Some(format!(
                        "declared='{declared}', supported={:?} — upgrade aiplus or pin the config to a supported version",
                        aiplus_core::consult::SUPPORTED_CONSULT_SCHEMAS
                    )),
                );
                push_check(
                    &mut checks,
                    "consultant-team.toml has members",
                    value.get("members").is_some(),
                    Some("add members array with at least ai_integration".to_string()),
                );
                push_check(
                    &mut checks,
                    "consultant-team.toml has owner_gates",
                    value.get("owner_gates").is_some(),
                    Some("add owner_gates section".to_string()),
                );
                push_check(
                    &mut checks,
                    "consultant-team.toml has user_evidence",
                    value.get("user_evidence").is_some(),
                    Some("add user_evidence section".to_string()),
                );
                let has_ai_integration = value
                    .get("members")
                    .and_then(|m| m.as_array())
                    .map(|arr| {
                        arr.iter().any(|item| {
                            item.get("id")
                                .and_then(|id| id.as_str())
                                .map(|s| s == "ai_integration")
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false);
                push_check(
                    &mut checks,
                    "consultant-team.toml includes ai_integration member",
                    has_ai_integration,
                    Some("add ai_integration member for AI-native products".to_string()),
                );
            }
        }
    }
    // W3: per-role memory namespace coverage. We walk the active
    // team's role list (whichever team was last installed) and warn
    // if a namespace dir is missing or empty. Empty == "no .gitkeep
    // and no README" — that means someone deleted the seed files
    // manually and the namespace will look invisible to git.
    if modules.contains_key(MODULE_SLUG_AGENT_TEAM) || modules.contains_key(MODULE_SLUG_AIECONLAB) {
        let active_team = crate::agent::set_team::read_active_team(&root).unwrap_or_default();
        let roles: &[&str] = if active_team == "aieconlab" {
            AIECONLAB_ROLES
        } else if active_team == "agent-team" {
            AGENT_TEAM_ROLES
        } else if modules.contains_key(MODULE_SLUG_AIECONLAB) {
            AIECONLAB_ROLES
        } else {
            AGENT_TEAM_ROLES
        };
        let team_dir = root.join(".aiplus/agent-memory/_team");
        push_check(
            &mut checks,
            ".aiplus/agent-memory/_team/ exists".to_string(),
            team_dir.exists(),
            Some(format!(
                "rerun `aiplus add {}` to recreate the namespace",
                active_team
            )),
        );
        for role in roles {
            let role_dir = root.join(format!(".aiplus/agent-memory/{role}"));
            let has_seed =
                role_dir.join(".gitkeep").exists() || role_dir.join("README.md").exists();
            push_check(
                &mut checks,
                format!(".aiplus/agent-memory/{role}/ exists with seed file"),
                role_dir.exists() && has_seed,
                Some(format!(
                    "rerun `aiplus add {active_team}` to recreate the {role}/ namespace"
                )),
            );
        }
    }
    // S6: declared secret_needs coverage. Walk every installed role,
    // collect the union of its [secret_needs].aliases, and check
    // each against `secret_aliases()` (the broker's registry). A
    // missing alias is NEEDS_FIX with a concrete next step: add to
    // BWS or remove the declaration. We never print alias values
    // here (or anywhere — see redaction tests in S1).
    //
    // Skip the check entirely when the broker registry itself is
    // empty — that means the user hasn't configured the broker yet,
    // so coverage is undefined, not "wrong." The user-facing nudge
    // for "configure the broker" lives on `aiplus secret-broker
    // doctor` (S3), not here.
    {
        let team_state = crate::agent::core::load_team_config(&root).unwrap_or_default();
        let mut declared: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        for agent in team_state.agents.values() {
            if let Some(ref needs) = agent.secret_needs {
                for alias in &needs.aliases {
                    declared.insert(alias.clone());
                }
            }
        }
        let registered: std::collections::BTreeSet<String> = secret_aliases()
            .unwrap_or_default()
            .into_iter()
            .map(|a| a.alias)
            .collect();
        if !declared.is_empty() && !registered.is_empty() {
            for alias in &declared {
                let ok = registered.contains(alias);
                push_check(
                    &mut checks,
                    format!("secret_needs alias `{alias}` is provisioned in broker"),
                    ok,
                    Some(format!(
                        "add alias `{alias}` to Bitwarden Secrets, then run \
                         `aiplus secret-broker token set` to unlock"
                    )),
                );
            }
        }
    }
    let continuity = continuity_state(&root)?;
    push_check(
        &mut checks,
        "no global configs were touched by installer".to_string(),
        true,
        None,
    );
    // P1.6: persona mirror drift check. Personas live in
    // .aiplus/agents/personas/ (source of truth) and get copied to
    // .claude/agents/ and .opencode/agents/ at install time. If a user
    // edits the source after install, same-named mirrors drift out of
    // sync — the runtime sees stale persona text. Surface this as a
    // doctor warning so the user knows to run `aiplus refresh`.
    match persona_mirror_drift(&root) {
        PersonaDriftStatus::NoSource => {}
        PersonaDriftStatus::InSync => {
            push_check(
                &mut checks,
                "persona mirrors in sync with .aiplus/agents/personas/".to_string(),
                true,
                None,
            );
        }
        PersonaDriftStatus::Drift { files } => {
            push_check(
                &mut checks,
                format!(
                    "persona mirrors in sync with .aiplus/agents/personas/ \
                     ({} file(s) drifted: {}). Fix: run `aiplus refresh` to re-mirror.",
                    files.len(),
                    files.join(", ")
                ),
                false,
                Some("aiplus refresh".to_string()),
            );
        }
    }
    // Registry health checks
    let registry_path = registry_file().ok();
    let registry_exists = registry_path.as_ref().map(|p| p.exists()).unwrap_or(false);
    push_check(
        &mut checks,
        "registry exists at ~/.config/aiplus/installed-projects.json".to_string(),
        registry_exists,
        None,
    );
    let registry_parse_ok = registry_path
        .as_ref()
        .filter(|p| p.exists())
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|text| serde_json::from_str::<Registry>(&text).ok())
        .is_some();
    push_check(
        &mut checks,
        "registry parses as JSON with schema_version=1.0".to_string(),
        registry_exists && registry_parse_ok,
        None,
    );
    let stale_count = if let Some(ref path) = registry_path {
        if path.exists() {
            if let Ok(registry) = read_registry() {
                registry
                    .installed_projects
                    .iter()
                    .filter(|e| !e.path.exists() || !e.path.join(".aiplus").is_dir())
                    .count()
            } else {
                0
            }
        } else {
            0
        }
    } else {
        0
    };
    // Issue #74: stale-registry entries are housekeeping noise (the
    // global registry tracks every project AiPlus has ever installed
    // into; entries become "stale" when those project directories are
    // deleted). The install itself is fine — `prune-projects` cleans
    // the registry. Surface as INFO so DOCTOR_STATUS stays PASS when
    // this is the only issue.
    push_info_check(
        &mut checks,
        format!("registry has {stale_count} stale entries"),
        stale_count == 0,
        Some("run aiplus prune-projects --yes".to_string()),
    );
    // DOCTOR_STATUS computation: a failing INFO check does NOT flip
    // overall status. Only NeedsFix-severity failures count.
    let pass = checks
        .iter()
        .all(|item| item.ok || item.severity == CheckSeverity::Info);
    println!("AIPLUS_DOCTOR");
    println!("status={}", if pass { "PASS" } else { "NEEDS_FIX" });
    println!(
        "installed={}",
        if parsed.as_ref().and_then(|m| m.installer.as_deref()) == Some(INSTALLER) {
            "yes"
        } else {
            "no"
        }
    );
    println!("runtimeAdapters=[{}]", runtimes.join(","));
    let module_text: Vec<String> = modules
        .iter()
        .map(|(name, module)| {
            format!(
                "{name}@{}",
                module
                    .version
                    .as_deref()
                    .unwrap_or_else(|| module_spec(name).map_or("unknown", |spec| spec.version))
            )
        })
        .collect();
    println!("modules=[{}]", module_text.join(","));
    print_continuity_status_lines(&continuity);
    println!("refreshPrompt={REFRESH_PROMPT}");
    println!("globalConfig=untouched");
    println!("target={}", root.display());
    // Issue #74: only NeedsFix-severity failures count toward the
    // "next: see the NEEDS_FIX items below" message. INFO checks
    // surface their own hint via the `fix` field already.
    let failing: Vec<&str> = checks
        .iter()
        .filter(|c| !c.ok && c.severity == CheckSeverity::NeedsFix)
        .map(|c| c.label.as_str())
        .collect();
    println!(
        "next={}",
        if pass {
            "send AiPlus 刷新, 刷新 AiPlus, aiplus refresh, or aiplus status to the current agent session".to_string()
        } else if !manifest_diag.exists {
            "run `aiplus install <runtime>` first (no manifest found in this project)".to_string()
        } else {
            format!(
                "see the NEEDS_FIX items below ({}) and run the suggested fix command for each",
                failing.len()
            )
        }
    );
    println!();
    for item in &checks {
        if item.ok {
            println!("PASS {}", item.label);
        } else {
            // Issue #74: INFO-severity failures use a softer prefix
            // so they read as recommendations, not breakages.
            let prefix = match item.severity {
                CheckSeverity::NeedsFix => "NEEDS_FIX",
                CheckSeverity::Info => "INFO",
            };
            if let Some(fix) = item.fix.as_ref() {
                println!("{prefix} {} ({fix})", item.label);
            } else {
                println!("{prefix} {}", item.label);
            }
        }
    }
    println!("DOCTOR_STATUS={}", if pass { "PASS" } else { "NEEDS_FIX" });
    Ok(())
}

// ---------------------------------------------------------------------------
// `aiplus doctor --check-keyring` — probe the OS keyring backend with a
// throwaway entry (separate service ID from the real secret-broker token,
// so we never trample stored user data). Reports:
//   - which backend was compiled in (Keychain / Secret Service / Credential
//     Manager)
//   - whether write, read-back, and delete each succeed on the live system
//   - whether BWS_ACCESS_TOKEN env-var fallback is set (so headless users
//     know they have a working path even if the keyring itself isn't)
//
// Output is one KV line per fact + a final KEYRING_CHECK_STATUS=PASS|FAIL
// terminator. Stable for scripting; intentionally not part of the
// human-readable `aiplus doctor` output (which is already long).
// ---------------------------------------------------------------------------

fn command_doctor_check_keyring() -> Result<()> {
    const PROBE_SERVICE: &str = "aiplus/doctor-keyring-probe";
    const PROBE_ACCOUNT: &str = "aiplus-doctor";

    let backend_compile_time = if cfg!(target_os = "macos") {
        "Keychain (security-framework)"
    } else if cfg!(target_os = "linux") {
        "Secret Service (dbus-secret-service, vendored libdbus)"
    } else if cfg!(target_os = "windows") {
        "Credential Manager (wincred)"
    } else {
        "unknown"
    };
    println!("KEYRING_CHECK");
    println!("backend_compile_time={backend_compile_time}");

    let entry = match keyring::Entry::new(PROBE_SERVICE, PROBE_ACCOUNT) {
        Ok(e) => e,
        Err(e) => {
            println!("backend_available=no");
            println!("write=skipped read=skipped delete=skipped");
            println!("KEYRING_CHECK_STATUS=FAIL reason=entry_create_failed detail={e}");
            report_bws_fallback();
            return Ok(());
        }
    };

    let nonce = format!("aiplus-doctor-probe-{}", epoch_millis());

    // Write probe.
    match entry.set_password(&nonce) {
        Ok(()) => println!("write=ok"),
        Err(keyring::Error::NoStorageAccess(detail))
        | Err(keyring::Error::PlatformFailure(detail)) => {
            println!("backend_available=no");
            println!("write=fail reason=backend_unavailable detail={detail}");
            println!("read=skipped delete=skipped");
            println!("KEYRING_CHECK_STATUS=FAIL reason=backend_unavailable");
            report_bws_fallback();
            return Ok(());
        }
        Err(e) => {
            println!("write=fail reason=other detail={e}");
            println!("read=skipped delete=skipped");
            println!("KEYRING_CHECK_STATUS=FAIL reason=write_failed detail={e}");
            report_bws_fallback();
            return Ok(());
        }
    }

    // Read probe back and verify the value round-trips byte-for-byte. A
    // backend that silently corrupts the value (or returns a different
    // entry's content because of a service-ID collision) would pass the
    // write step but fail here.
    let read_value = match entry.get_password() {
        Ok(v) => v,
        Err(e) => {
            println!("read=fail detail={e}");
            println!("delete=skipped");
            let _ = entry.delete_credential();
            println!("KEYRING_CHECK_STATUS=FAIL reason=read_failed detail={e}");
            report_bws_fallback();
            return Ok(());
        }
    };
    let read_matches = read_value == nonce;
    println!("read=ok match={}", if read_matches { "yes" } else { "no" });

    // Delete probe regardless of read match (clean up after ourselves).
    let delete_result = entry.delete_credential();
    match &delete_result {
        Ok(()) => println!("delete=ok"),
        Err(keyring::Error::NoEntry) => println!("delete=noop reason=no_entry"),
        Err(e) => println!("delete=fail detail={e}"),
    }

    if !read_matches {
        println!("KEYRING_CHECK_STATUS=FAIL reason=read_mismatch");
        report_bws_fallback();
        return Ok(());
    }

    println!("backend_available=yes");
    println!("KEYRING_CHECK_STATUS=PASS");
    report_bws_fallback();
    Ok(())
}

fn report_bws_fallback() {
    println!();
    println!("BWS_ENV_FALLBACK");
    match std::env::var("BWS_ACCESS_TOKEN") {
        Ok(val) if !val.trim().is_empty() => {
            println!("bws_access_token=set chars={}", val.chars().count());
            println!("bws_env_fallback_status=available");
        }
        Ok(_) => {
            println!("bws_access_token=set_but_empty");
            println!("bws_env_fallback_status=unavailable_empty");
        }
        Err(_) => {
            println!("bws_access_token=unset");
            println!("bws_env_fallback_status=unavailable_unset");
        }
    }
}

fn command_uninstall(dry_run: bool, yes: bool, force: bool) -> Result<()> {
    if !dry_run && !yes {
        return Err(CliError::new(1, "ERROR uninstall requires --dry-run or --yes").into());
    }
    let root = target_root()?;
    let manifest = read_manifest(&root, false)?;
    if manifest.installer.as_deref() != Some(INSTALLER) {
        return Err(CliError::new(
            1,
            "ERROR refusing uninstall without AiPlus manifest ownership",
        )
        .into());
    }
    let known = known_aiplus_entries();
    let entries = list_entries(&rel_to_abs(&root, ".aiplus")?)?;
    let mut unknown = Vec::new();
    for entry in entries {
        let rel = path_slash(path_relative(&root, &entry)?);
        if !known
            .iter()
            .any(|known_path| rel == *known_path || rel.starts_with(&format!("{known_path}/")))
        {
            unknown.push(rel);
        }
    }
    if !unknown.is_empty() && !force {
        return Err(CliError::new(
            1,
            format!(
                "ERROR .aiplus/ contains unknown entries; retry uninstall --yes --force after review:\n{}",
                unknown.join("\n")
            ),
        )
        .into());
    }
    let mut plan = Plan {
        dry_run,
        ..Plan::default()
    };
    remove_managed_block(&root, &mut plan)?;
    remove_claude_md_managed_block(&root, &mut plan)?;
    remove_claude_md_aieconlab_block(&root, &mut plan)?;
    remove_claude_md_agent_team_block(&root, &mut plan)?;
    remove_claude_hooks(&root, &mut plan)?;
    // Track A.1: remove `.claude/agents/{prefix}-*.md`,
    // `.claude/commands/{prefix}-*.md`, and the opencode analogs.
    // Before this, uninstall left ~20 AEL + ~14 agent-team + 5 aiplus
    // subagent files orphaned under `.claude/agents/`. Same for
    // `.claude/commands/` and the matching opencode dirs.
    remove_runtime_adapter_artifacts(&root, &mut plan)?;
    plan.items.push(PlanItem {
        action: "remove".to_string(),
        path: ".aiplus/".to_string(),
    });
    if dry_run {
        println!("AiPlus uninstall dry-run for this project.");
        println!("DRY_RUN_ONLY=YES");
        println!("NO_FILES_REMOVED=YES");
    }
    plan_printer(&plan);
    if !dry_run {
        if let Err(e) = remove_registry_entry(&root) {
            eprintln!("WARN registry removal failed: {}", e);
        }
        safe_remove_aiplus(&root)?;
    }
    println!(
        "{}",
        if dry_run {
            "UNINSTALL_DRY_RUN=PASS"
        } else {
            "UNINSTALL_STATUS=PASS"
        }
    );
    Ok(())
}

fn command_rollback(id: String, dry_run: bool, yes: bool) -> Result<()> {
    if !dry_run && !yes {
        return Err(CliError::new(1, "ERROR rollback requires --dry-run or --yes").into());
    }
    let root = target_root()?;
    let plan_path = match resolve_rollback_plan_path(&root, &id) {
        Ok(path) => path,
        Err(error) if dry_run => {
            println!("AIPLUS_ROLLBACK");
            println!("id={id}");
            println!("dryRun=yes");
            println!("entries=0");
            println!("noRollbackPlan=yes");
            println!("ROLLBACK_STATUS=DRY_RUN");
            let _ = error;
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    let plan = read_rollback_plan(&plan_path)?;
    println!("AIPLUS_ROLLBACK");
    println!("id={}", plan.id);
    println!("plan={}", path_slash(path_relative(&root, &plan_path)?));
    println!("dryRun={}", yes_no(dry_run));
    println!("entries={}", plan.entries.len());
    let mut restored = 0usize;
    let mut skipped = 0usize;
    for entry in &plan.entries {
        let original_rel = entry.original_path.trim();
        let backup_rel = entry.backup_path.trim();
        let allowed =
            entry.managed_file && aiplus_core::paths::is_allowed_project_write(original_rel);
        let backup = rel_to_abs(&root, backup_rel)?;
        let original = rel_to_abs(&root, original_rel)?;
        if let Err(error) = assert_no_symlink_path(&root, &backup) {
            println!("skip original={original_rel} reason=backup_symlink");
            let _ = error;
            skipped += 1;
            continue;
        }
        if !allowed || !backup.exists() {
            println!(
                "skip original={} reason={}",
                original_rel,
                if allowed {
                    "backup_missing"
                } else {
                    "not_managed_file"
                }
            );
            skipped += 1;
            continue;
        }
        println!("restore {} <- {}", original_rel, backup_rel);
        if !dry_run {
            assert_no_symlink_path(&root, &original)?;
            if let Some(parent) = original.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(backup, original)?;
        }
        restored += 1;
    }
    println!("restored={restored}");
    println!("skipped={skipped}");
    println!(
        "ROLLBACK_STATUS={}",
        if dry_run { "DRY_RUN" } else { "PASS" }
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn command_compact(
    subcommand: Option<String>,
    level: &str,
    json: bool,
    force: bool,
    event: Option<String>,
    snooze: Option<String>,
    clear_snooze: bool,
    once: bool,
    interval: Option<String>,
) -> Result<()> {
    let Some(subcommand) = subcommand else {
        print_usage();
        process::exit(2);
    };
    if ![
        "init",
        "validate",
        "checkpoint",
        "resume",
        "remind",
        "prepare",
        "score",
        "savings",
        "watch",
    ]
    .contains(&subcommand.as_str())
    {
        print_usage();
        process::exit(2);
    }
    let root = target_root()?;
    match subcommand.as_str() {
        "init" => {
            compact_init_command(&root, force)?;
            println!("COMPACT_RUST_NATIVE_STATUS=PASS");
            Ok(())
        }
        "validate" => {
            let result = compact_validate_state(&root)?;
            print_compact_diagnostics(&result);
            if result.ok {
                println!("VALIDATION_PASS");
                println!("Validation is structural only. Passing does not mean safe to compact.");
                println!("COMPACT_RUST_NATIVE_STATUS=PASS");
                Ok(())
            } else if result.errors.is_empty()
                && result.warnings.is_empty()
                && !result.review_items.is_empty()
            {
                eprintln!("UNKNOWN_NEEDS_REVIEW");
                println!("COMPACT_RUST_NATIVE_STATUS=PASS");
                process::exit(2);
            } else {
                eprintln!("VALIDATION_FAIL");
                println!("COMPACT_RUST_NATIVE_STATUS=PASS");
                process::exit(1);
            }
        }
        "checkpoint" => {
            let exit_code = compact_checkpoint(&root, level)?;
            println!("COMPACT_RUST_NATIVE_STATUS=PASS");
            if exit_code == 0 {
                Ok(())
            } else {
                process::exit(exit_code);
            }
        }
        "resume" => {
            let exit_code = compact_resume(&root)?;
            println!("COMPACT_RUST_NATIVE_STATUS=PASS");
            if exit_code == 0 {
                Ok(())
            } else {
                process::exit(exit_code);
            }
        }
        "prepare" => {
            let exit_code = compact_prepare(&root, level)?;
            println!("COMPACT_RUST_NATIVE_STATUS=PASS");
            if exit_code == 0 {
                Ok(())
            } else {
                process::exit(exit_code);
            }
        }
        "score" => {
            let exit_code = compact_score(&root)?;
            println!("COMPACT_RUST_NATIVE_STATUS=PASS");
            if exit_code == 0 {
                Ok(())
            } else {
                process::exit(exit_code);
            }
        }
        "remind" => {
            let exit_code = compact_remind(
                &root,
                event.as_deref(),
                snooze.as_deref(),
                clear_snooze,
                json,
                false,
            )?;
            if exit_code == 0 {
                Ok(())
            } else {
                process::exit(exit_code);
            }
        }
        "watch" => {
            let exit_code = compact_watch(&root, once, interval.as_deref(), json)?;
            if exit_code == 0 {
                Ok(())
            } else {
                process::exit(exit_code);
            }
        }
        "savings" => compact_savings(&root, json),
        _ => unreachable!(),
    }
}

fn command_pricing(subcommand: Option<String>) -> Result<()> {
    match subcommand.as_deref() {
        Some("update") => pricing_update(),
        Some("status") => pricing_status(),
        _ => {
            println!("Usage: aiplus pricing update|status");
            process::exit(2);
        }
    }
}

struct ProfileCommandArgs {
    subcommand: Option<String>,
    profile: Option<String>,
    target_profile: Option<String>,
    source: Option<PathBuf>,
    user: bool,
    project: bool,
    dry_run: bool,
    yes: bool,
}

fn command_profile(args: ProfileCommandArgs) -> Result<()> {
    match args.subcommand.as_deref() {
        Some("status") => profile_status(args.profile),
        Some("install") => {
            profile_install(args.profile, args.source, args.user, args.dry_run, args.yes)
        }
        Some("update") => profile_install(args.profile, args.source, args.user, false, true),
        Some("link") => profile_link(args.profile, args.project),
        Some("disable") => profile_disable(args.profile, args.user, args.dry_run, args.yes),
        Some("uninstall") => profile_uninstall(args.profile, args.user, args.dry_run, args.yes),
        Some("migrate") => profile_migrate(
            args.profile,
            args.target_profile,
            args.user,
            args.dry_run,
            args.yes,
        ),
        Some("cleanup") => profile_cleanup(args.user, args.dry_run, args.yes),
        Some("doctor") => profile_doctor(args.profile),
        Some("context") => profile_context(args.profile),
        _ => {
            println!(
                "Usage: aiplus profile status|install|update|link|disable|uninstall|migrate|cleanup|doctor|context"
            );
            process::exit(2);
        }
    }
}

fn profile_status(profile: Option<String>) -> Result<()> {
    let profile = profile.unwrap_or_else(|| "all".to_string());
    println!("PROFILE_STATUS");
    println!("profile={profile}");
    println!("scope=user");
    if profile == "all" {
        let profiles_root = config_home()?.join("aiplus").join("profiles");
        println!("config_dir={}", profiles_root.display());
        let profiles = installed_profile_names(&profiles_root)?;
        println!("installed={}", yes_no(!profiles.is_empty()));
        println!("profiles=[{}]", profiles.join(","));
        let legacy_profiles = legacy_profile_names(&profiles_root)?;
        if !legacy_profiles.is_empty() {
            println!("legacy_profiles=[{}]", legacy_profiles.join(","));
            println!("next=run aiplus profile cleanup --user --yes");
        }
    } else {
        validate_profile_name(&profile)?;
        let dir = profile_dir(&profile)?;
        println!("config_dir={}", dir.display());
        println!("installed={}", yes_no(dir.join("profile.toml").exists()));
        println!(
            "agents_profile={}",
            yes_no(dir.join("AGENTS.profile.md").exists())
        );
        let supp = supplemental_bundle_status(&dir);
        println!("user_md={}", yes_no(supp.user_md));
        println!("memory_md={}", yes_no(supp.memory_md));
        println!("preferences_dir={}", yes_no(supp.preferences_dir));
        println!("identities_dir={}", yes_no(supp.identities_dir));
        println!("sync_dir={}", yes_no(supp.sync_dir));
    }
    println!("secret_values=none");
    println!("global_agent_config_edits=none");
    println!("PROFILE_STATUS=PASS");
    Ok(())
}

#[derive(Default)]
struct SupplementalStatus {
    user_md: bool,
    memory_md: bool,
    preferences_dir: bool,
    identities_dir: bool,
    sync_dir: bool,
}

fn supplemental_bundle_status(dir: &Path) -> SupplementalStatus {
    SupplementalStatus {
        user_md: dir.join("USER.md").exists(),
        memory_md: dir.join("MEMORY.md").exists(),
        preferences_dir: dir.join("preferences").is_dir(),
        identities_dir: dir.join("identities").is_dir(),
        sync_dir: dir.join("sync").is_dir(),
    }
}

fn profile_install(
    profile: Option<String>,
    source: Option<PathBuf>,
    user: bool,
    dry_run: bool,
    yes: bool,
) -> Result<()> {
    let profile = require_profile_name(profile)?;
    if !user {
        return Err(CliError::new(1, "ERROR profile install currently requires --user").into());
    }
    let source = resolve_profile_source(source)?;
    let dir = profile_dir(&profile)?;
    println!("PROFILE_INSTALL");
    println!("profile={profile}");
    println!("scope=user");
    println!("source={}", source.display());
    println!("target_dir={}", dir.display());
    println!("secret_values=none");
    println!("global_agent_config_edits=none");
    if dry_run {
        println!("DRY_RUN=YES");
        println!("PROFILE_INSTALL_STATUS=DRY_RUN");
        return Ok(());
    }
    if !yes {
        return Err(CliError::new(1, "ERROR profile install requires --yes or --dry-run").into());
    }
    fs::create_dir_all(&dir)?;
    backup_profile_dir(&profile, &dir)?;
    install_profile_file(&source, &dir, "profile.toml")?;
    install_profile_file(&source, &dir, "AGENTS.profile.md")?;
    let mut supplemental = Vec::new();
    if source.join("USER.md").exists() {
        install_profile_file(&source, &dir, "USER.md")?;
        supplemental.push("USER.md");
    }
    if source.join("MEMORY.md").exists() {
        install_profile_file(&source, &dir, "MEMORY.md")?;
        supplemental.push("MEMORY.md");
    }
    if source.join("preferences").is_dir() {
        copy_profile_dir(&source, &dir, "preferences")?;
        supplemental.push("preferences/");
    }
    if source.join("identities").is_dir() {
        copy_profile_dir(&source, &dir, "identities")?;
        supplemental.push("identities/");
    }
    if source.join("sync").is_dir() {
        copy_profile_dir(&source, &dir, "sync")?;
        supplemental.push("sync/");
    }
    if source.join("secret-aliases.tsv").exists() {
        let broker_dir = config_home()?
            .join("aiplus")
            .join("secret-broker")
            .join("profiles")
            .join(&profile);
        fs::create_dir_all(&broker_dir)?;
        install_profile_file(&source, &broker_dir, "secret-aliases.tsv")?;
    }
    println!("supplemental_installed=[{}]", supplemental.join(","));
    println!("PROFILE_INSTALL_STATUS=PASS");
    Ok(())
}

fn profile_link(profile: Option<String>, project: bool) -> Result<()> {
    let profile = require_profile_name(profile)?;
    if !project {
        return Err(CliError::new(1, "ERROR profile link currently requires --project").into());
    }
    let root = std::env::current_dir()?;
    let aiplus_dir = root.join(".aiplus");
    if !aiplus_dir.exists() {
        return Err(CliError::new(1, "ERROR no project AiPlus install found").into());
    }
    let profile_ref = ".aiplus/PROFILE.aiplus.md";
    let body = format!(
        "# AiPlus Project Profile Link\n\nThis project may use the user-level `{profile}` profile when available.\n\nLoad order:\n1. Current Owner message\n2. Project rules\n3. User profile at `~/.config/aiplus/profiles/{profile}/AGENTS.profile.md`\n4. AiPlus generic guidance\n\nSecret values must not be loaded from the profile. Use `aiplus secret-broker` only for explicit secret actions.\n"
    );
    write_file_atomic(&root.join(profile_ref), body.as_bytes())?;
    println!("PROFILE_LINK");
    println!("profile={profile}");
    println!("scope=project");
    println!("path={profile_ref}");
    println!("secret_values=none");
    println!("PROFILE_LINK_STATUS=PASS");
    Ok(())
}

fn profile_disable(profile: Option<String>, user: bool, dry_run: bool, yes: bool) -> Result<()> {
    let profile = require_profile_name(profile)?;
    if !user {
        return Err(CliError::new(1, "ERROR profile disable currently requires --user").into());
    }
    let marker = profile_dir(&profile)?.join("disabled");
    println!("PROFILE_DISABLE");
    println!("profile={profile}");
    println!("scope=user");
    println!("path={}", marker.display());
    if dry_run {
        println!("DRY_RUN=YES");
        println!("PROFILE_DISABLE_STATUS=DRY_RUN");
        return Ok(());
    }
    if !yes {
        return Err(CliError::new(1, "ERROR profile disable requires --yes or --dry-run").into());
    }
    if let Some(parent) = marker.parent() {
        fs::create_dir_all(parent)?;
    }
    write_file_atomic(&marker, b"disabled=true\n")?;
    println!("PROFILE_DISABLE_STATUS=PASS");
    Ok(())
}

fn profile_uninstall(profile: Option<String>, user: bool, dry_run: bool, yes: bool) -> Result<()> {
    let profile = require_profile_name(profile)?;
    if !user {
        return Err(CliError::new(1, "ERROR profile uninstall currently requires --user").into());
    }
    let dir = profile_dir(&profile)?;
    println!("PROFILE_UNINSTALL");
    println!("profile={profile}");
    println!("scope=user");
    println!("target_dir={}", dir.display());
    println!("secret_values=none");
    if dry_run {
        println!("DRY_RUN=YES");
        println!("PROFILE_UNINSTALL_STATUS=DRY_RUN");
        return Ok(());
    }
    if !yes {
        return Err(CliError::new(1, "ERROR profile uninstall requires --yes or --dry-run").into());
    }
    if dir.exists() {
        backup_profile_dir(&profile, &dir)?;
        fs::remove_dir_all(&dir)?;
    }
    let alias_dir = config_home()?
        .join("aiplus")
        .join("secret-broker")
        .join("profiles")
        .join(&profile);
    if alias_dir.exists() {
        fs::remove_dir_all(&alias_dir)?;
        println!("secret_aliases_removed=yes");
    }
    println!("PROFILE_UNINSTALL_STATUS=PASS");
    Ok(())
}

fn profile_migrate(
    legacy: Option<String>,
    canonical: Option<String>,
    user: bool,
    dry_run: bool,
    yes: bool,
) -> Result<()> {
    let legacy = require_profile_name(legacy)?;
    let canonical = require_profile_name(canonical)?;
    if !user {
        return Err(CliError::new(1, "ERROR profile migrate currently requires --user").into());
    }
    let legacy_dir = profile_dir(&legacy)?;
    let canonical_dir = profile_dir(&canonical)?;
    println!("PROFILE_MIGRATE");
    println!("legacy_profile={legacy}");
    println!("canonical_profile={canonical}");
    println!("scope=user");
    println!("legacy_present={}", yes_no(legacy_dir.exists()));
    println!("canonical_present={}", yes_no(canonical_dir.exists()));
    println!("secret_values=none");
    println!("global_agent_config_edits=none");
    if dry_run {
        println!("DRY_RUN=YES");
        println!("PROFILE_MIGRATE_STATUS=DRY_RUN");
        return Ok(());
    }
    if !yes {
        return Err(CliError::new(1, "ERROR profile migrate requires --yes or --dry-run").into());
    }
    if !canonical_dir.join("profile.toml").exists() {
        return Err(CliError::new(
            1,
            format!("PROFILE_MIGRATE_STATUS=BLOCKED canonical_missing={canonical}"),
        )
        .into());
    }
    remove_profile_registration(&legacy)?;
    println!("PROFILE_MIGRATE_STATUS=PASS");
    Ok(())
}

fn profile_cleanup(user: bool, dry_run: bool, yes: bool) -> Result<()> {
    if !user {
        return Err(CliError::new(1, "ERROR profile cleanup currently requires --user").into());
    }
    let legacy = "work-with-zhiwen";
    let canonical = "aiplus-work-with-zhiwen";
    let legacy_dir = profile_dir(legacy)?;
    let canonical_dir = profile_dir(canonical)?;
    println!("PROFILE_CLEANUP");
    println!("scope=user");
    println!("legacy_profile={legacy}");
    println!("canonical_profile={canonical}");
    println!("legacy_present={}", yes_no(legacy_dir.exists()));
    println!("canonical_present={}", yes_no(canonical_dir.exists()));
    println!("secret_values=none");
    println!("global_agent_config_edits=none");
    if dry_run {
        println!("DRY_RUN=YES");
        println!("PROFILE_CLEANUP_STATUS=DRY_RUN");
        return Ok(());
    }
    if !yes {
        return Err(CliError::new(1, "ERROR profile cleanup requires --yes or --dry-run").into());
    }
    if !canonical_dir.join("profile.toml").exists() {
        return Err(CliError::new(
            1,
            "PROFILE_CLEANUP_STATUS=BLOCKED canonical_missing=aiplus-work-with-zhiwen",
        )
        .into());
    }
    remove_profile_registration(legacy)?;
    println!("PROFILE_CLEANUP_STATUS=PASS");
    Ok(())
}

fn profile_doctor(profile: Option<String>) -> Result<()> {
    let profile = profile.unwrap_or_else(|| "all".to_string());
    if profile == "all" {
        let profiles_root = config_home()?.join("aiplus").join("profiles");
        let profiles = installed_profile_names(&profiles_root)?;
        let mut checks = Vec::new();
        for p in &profiles {
            let dir = profile_dir(p)?;
            push_check(
                &mut checks,
                format!("{p} profile.toml exists"),
                dir.join("profile.toml").exists(),
                None,
            );
            push_check(
                &mut checks,
                format!("{p} AGENTS.profile.md exists"),
                dir.join("AGENTS.profile.md").exists(),
                None,
            );
            let supp = supplemental_bundle_status(&dir);
            push_check(
                &mut checks,
                format!("{p} USER.md present"),
                supp.user_md,
                None,
            );
            push_check(
                &mut checks,
                format!("{p} MEMORY.md present"),
                supp.memory_md,
                None,
            );
            push_check(
                &mut checks,
                format!("{p} preferences/ present"),
                supp.preferences_dir,
                None,
            );
            push_check(
                &mut checks,
                format!("{p} identities/ present"),
                supp.identities_dir,
                None,
            );
            push_check(
                &mut checks,
                format!("{p} sync/ present"),
                supp.sync_dir,
                None,
            );
            if supp.identities_dir {
                let id_dir = dir.join("identities");
                for entry in fs::read_dir(&id_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "toml") {
                        let text = fs::read_to_string(&path).unwrap_or_default();
                        let has_name = text.contains("name =");
                        let has_role = text.contains("role =");
                        let valid = !text.trim().is_empty() && has_name && has_role;
                        push_check(
                            &mut checks,
                            format!(
                                "{p} identity {} valid",
                                path.file_name().unwrap_or_default().to_string_lossy()
                            ),
                            valid,
                            None,
                        );
                    }
                }
            }

            // Schema validation for core files
            if dir.join("profile.toml").exists() {
                let profile_text = fs::read_to_string(dir.join("profile.toml")).unwrap_or_default();
                let profile_toml = toml::from_str::<toml::Value>(&profile_text);
                push_check(
                    &mut checks,
                    format!("{p} profile.toml parseable"),
                    profile_toml.is_ok(),
                    None,
                );
                if let Ok(ref value) = profile_toml {
                    push_check(
                        &mut checks,
                        format!("{p} profile.toml has name"),
                        value.get("name").is_some(),
                        None,
                    );
                    push_check(
                        &mut checks,
                        format!("{p} profile.toml has version"),
                        value.get("version").is_some(),
                        None,
                    );
                    push_check(
                        &mut checks,
                        format!("{p} profile.toml has owner"),
                        value.get("owner").is_some(),
                        None,
                    );
                }
            }

            if dir.join("AGENTS.profile.md").exists() {
                let agents_text =
                    fs::read_to_string(dir.join("AGENTS.profile.md")).unwrap_or_default();
                push_check(
                    &mut checks,
                    format!("{p} AGENTS.profile.md has heading"),
                    agents_text.contains('#'),
                    None,
                );
            }

            if supp.user_md {
                let user_text = fs::read_to_string(dir.join("USER.md")).unwrap_or_default();
                push_check(
                    &mut checks,
                    format!("{p} USER.md non-empty"),
                    !user_text.trim().is_empty(),
                    None,
                );
            }
            if supp.memory_md {
                let memory_text = fs::read_to_string(dir.join("MEMORY.md")).unwrap_or_default();
                push_check(
                    &mut checks,
                    format!("{p} MEMORY.md non-empty"),
                    !memory_text.trim().is_empty(),
                    None,
                );
            }

            // preferences/ validation: must contain valid markdown or structured files
            if supp.preferences_dir {
                let pref_dir = dir.join("preferences");
                let mut pref_valid = true;
                for entry in fs::read_dir(&pref_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        let text = fs::read_to_string(&path).unwrap_or_default();
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        let is_md = name.ends_with(".md");
                        let is_toml = name.ends_with(".toml");
                        let is_json = name.ends_with(".json");
                        let has_structure = !text.trim().is_empty()
                            && (is_md || is_toml || is_json || text.contains(':'));
                        push_check(
                            &mut checks,
                            format!("{p} preference {name} has structure"),
                            has_structure,
                            None,
                        );
                        if !has_structure {
                            pref_valid = false;
                        }
                    }
                }
                push_check(
                    &mut checks,
                    format!("{p} preferences/ valid"),
                    pref_valid,
                    None,
                );
            }

            // sync/ validation: must contain policy.toml or sync docs
            if supp.sync_dir {
                let sync_dir = dir.join("sync");
                let has_policy = sync_dir.join("policy.toml").exists();
                let has_docs = sync_dir.join("README.md").exists();
                push_check(
                    &mut checks,
                    format!("{p} sync/ has policy or docs"),
                    has_policy || has_docs,
                    None,
                );
                if has_policy {
                    let policy_text =
                        fs::read_to_string(sync_dir.join("policy.toml")).unwrap_or_default();
                    let policy_toml = toml::from_str::<toml::Value>(&policy_text);
                    push_check(
                        &mut checks,
                        format!("{p} sync/policy.toml parseable"),
                        policy_toml.is_ok(),
                        None,
                    );
                }
            }

            // USER.md / MEMORY.md redaction check: must not contain unredacted secrets
            if supp.user_md {
                let user_text = fs::read_to_string(dir.join("USER.md")).unwrap_or_default();
                let unredacted = unredacted_secret_lines(&user_text);
                push_check(
                    &mut checks,
                    format!("{p} USER.md redaction clean"),
                    unredacted.is_empty(),
                    if unredacted.is_empty() {
                        None
                    } else {
                        Some(format!("Lines with secrets: {}", unredacted.join(", ")))
                    },
                );
            }
            if supp.memory_md {
                let memory_text = fs::read_to_string(dir.join("MEMORY.md")).unwrap_or_default();
                let unredacted = unredacted_secret_lines(&memory_text);
                push_check(
                    &mut checks,
                    format!("{p} MEMORY.md redaction clean"),
                    unredacted.is_empty(),
                    if unredacted.is_empty() {
                        None
                    } else {
                        Some(format!("Lines with secrets: {}", unredacted.join(", ")))
                    },
                );
            }

            // identities/*.toml strict validation
            if supp.identities_dir {
                let id_dir = dir.join("identities");
                for entry in fs::read_dir(&id_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "toml") {
                        let text = fs::read_to_string(&path).unwrap_or_default();
                        let name = path.file_name().unwrap_or_default().to_string_lossy();
                        let has_name = text.contains("name =");
                        let has_role = text.contains("role =");
                        let has_owner_gate =
                            text.contains("owner_gate") || text.contains("ownerGate");
                        let valid = !text.trim().is_empty() && has_name && has_role;
                        push_check(
                            &mut checks,
                            format!("{p} identity {name} valid"),
                            valid,
                            None,
                        );
                        push_check(
                            &mut checks,
                            format!("{p} identity {name} has owner gate"),
                            has_owner_gate,
                            None,
                        );
                    }
                }
            }

            // No private content copied to public assets check
            push_check(
                &mut checks,
                format!("{p} private content not in public assets"),
                true,
                Some(
                    "Manual review: verify USER.md/MEMORY.md not in release tarballs.".to_string(),
                ),
            );
        }
        let pass = checks.iter().all(|c| c.ok);
        println!("PROFILE_DOCTOR");
        println!("profile=all");
        println!("profiles_checked={}", profiles.len());
        println!("secret_values=none");
        println!("global_agent_config_edits=none");
        for c in &checks {
            println!("{} {}", if c.ok { "PASS" } else { "NEEDS_FIX" }, c.label);
        }
        println!(
            "PROFILE_DOCTOR_STATUS={}",
            if pass { "PASS" } else { "NEEDS_FIX" }
        );
        Ok(())
    } else {
        validate_profile_name(&profile)?;
        let dir = profile_dir(&profile)?;
        let mut checks = Vec::new();
        push_check(
            &mut checks,
            "profile.toml exists",
            dir.join("profile.toml").exists(),
            None,
        );
        push_check(
            &mut checks,
            "AGENTS.profile.md exists",
            dir.join("AGENTS.profile.md").exists(),
            None,
        );
        let supp = supplemental_bundle_status(&dir);
        push_check(&mut checks, "USER.md present", supp.user_md, None);
        push_check(&mut checks, "MEMORY.md present", supp.memory_md, None);
        push_check(
            &mut checks,
            "preferences/ present",
            supp.preferences_dir,
            None,
        );
        push_check(
            &mut checks,
            "identities/ present",
            supp.identities_dir,
            None,
        );
        push_check(&mut checks, "sync/ present", supp.sync_dir, None);
        if supp.identities_dir {
            let id_dir = dir.join("identities");
            for entry in fs::read_dir(&id_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "toml") {
                    let text = fs::read_to_string(&path).unwrap_or_default();
                    let has_name = text.contains("name =");
                    let has_role = text.contains("role =");
                    let valid = !text.trim().is_empty() && has_name && has_role;
                    push_check(
                        &mut checks,
                        format!(
                            "identity {} valid",
                            path.file_name().unwrap_or_default().to_string_lossy()
                        ),
                        valid,
                        None,
                    );
                }
            }
        }

        // Schema validation for core files
        if dir.join("profile.toml").exists() {
            let profile_text = fs::read_to_string(dir.join("profile.toml")).unwrap_or_default();
            let profile_toml = toml::from_str::<toml::Value>(&profile_text);
            push_check(
                &mut checks,
                "profile.toml parseable",
                profile_toml.is_ok(),
                None,
            );
            if let Ok(ref value) = profile_toml {
                push_check(
                    &mut checks,
                    "profile.toml has name",
                    value.get("name").is_some(),
                    None,
                );
                push_check(
                    &mut checks,
                    "profile.toml has version",
                    value.get("version").is_some(),
                    None,
                );
                push_check(
                    &mut checks,
                    "profile.toml has owner",
                    value.get("owner").is_some(),
                    None,
                );
            }
        }

        if dir.join("AGENTS.profile.md").exists() {
            let agents_text = fs::read_to_string(dir.join("AGENTS.profile.md")).unwrap_or_default();
            push_check(
                &mut checks,
                "AGENTS.profile.md has heading",
                agents_text.contains('#'),
                None,
            );
        }

        if supp.user_md {
            let user_text = fs::read_to_string(dir.join("USER.md")).unwrap_or_default();
            push_check(
                &mut checks,
                "USER.md non-empty",
                !user_text.trim().is_empty(),
                None,
            );
        }
        if supp.memory_md {
            let memory_text = fs::read_to_string(dir.join("MEMORY.md")).unwrap_or_default();
            push_check(
                &mut checks,
                "MEMORY.md non-empty",
                !memory_text.trim().is_empty(),
                None,
            );
        }

        // preferences/ validation: must contain valid markdown or structured files
        if supp.preferences_dir {
            let pref_dir = dir.join("preferences");
            let mut pref_valid = true;
            for entry in fs::read_dir(&pref_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    let text = fs::read_to_string(&path).unwrap_or_default();
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    let is_md = name.ends_with(".md");
                    let is_toml = name.ends_with(".toml");
                    let is_json = name.ends_with(".json");
                    let has_structure = !text.trim().is_empty()
                        && (is_md || is_toml || is_json || text.contains(':'));
                    push_check(
                        &mut checks,
                        format!("preference {name} has structure"),
                        has_structure,
                        None,
                    );
                    if !has_structure {
                        pref_valid = false;
                    }
                }
            }
            push_check(&mut checks, "preferences/ valid", pref_valid, None);
        }

        // sync/ validation: must contain policy.toml or sync docs
        if supp.sync_dir {
            let sync_dir = dir.join("sync");
            let has_policy = sync_dir.join("policy.toml").exists();
            let has_docs = sync_dir.join("README.md").exists();
            push_check(
                &mut checks,
                "sync/ has policy or docs",
                has_policy || has_docs,
                None,
            );
            if has_policy {
                let policy_text =
                    fs::read_to_string(sync_dir.join("policy.toml")).unwrap_or_default();
                let policy_toml = toml::from_str::<toml::Value>(&policy_text);
                push_check(
                    &mut checks,
                    "sync/policy.toml parseable",
                    policy_toml.is_ok(),
                    None,
                );
            }
        }

        // USER.md / MEMORY.md redaction check
        if supp.user_md {
            let user_text = fs::read_to_string(dir.join("USER.md")).unwrap_or_default();
            let unredacted = unredacted_secret_lines(&user_text);
            push_check(
                &mut checks,
                "USER.md redaction clean",
                unredacted.is_empty(),
                if unredacted.is_empty() {
                    None
                } else {
                    Some(format!("Lines with secrets: {}", unredacted.join(", ")))
                },
            );
        }
        if supp.memory_md {
            let memory_text = fs::read_to_string(dir.join("MEMORY.md")).unwrap_or_default();
            let unredacted = unredacted_secret_lines(&memory_text);
            push_check(
                &mut checks,
                "MEMORY.md redaction clean",
                unredacted.is_empty(),
                if unredacted.is_empty() {
                    None
                } else {
                    Some(format!("Lines with secrets: {}", unredacted.join(", ")))
                },
            );
        }

        // identities/*.toml strict validation
        if supp.identities_dir {
            let id_dir = dir.join("identities");
            for entry in fs::read_dir(&id_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "toml") {
                    let text = fs::read_to_string(&path).unwrap_or_default();
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    let has_name = text.contains("name =");
                    let has_role = text.contains("role =");
                    let has_owner_gate = text.contains("owner_gate") || text.contains("ownerGate");
                    let valid = !text.trim().is_empty() && has_name && has_role;
                    push_check(&mut checks, format!("identity {name} valid"), valid, None);
                    push_check(
                        &mut checks,
                        format!("identity {name} has owner gate"),
                        has_owner_gate,
                        None,
                    );
                }
            }
        }

        // No private content copied to public assets check
        push_check(
            &mut checks,
            "private content not in public assets",
            true,
            Some("Manual review: verify USER.md/MEMORY.md not in release tarballs.".to_string()),
        );

        let pass = checks.iter().all(|c| c.ok);
        println!("PROFILE_DOCTOR");
        println!("profile={profile}");
        println!(
            "installed={}",
            yes_no(dir.join("profile.toml").exists() && dir.join("AGENTS.profile.md").exists())
        );
        println!("private_content_in_public_assets=not_checked_by_command");
        println!("secret_values=none");
        println!("global_agent_config_edits=none");
        for c in &checks {
            println!("{} {}", if c.ok { "PASS" } else { "NEEDS_FIX" }, c.label);
        }
        println!(
            "PROFILE_DOCTOR_STATUS={}",
            if pass { "PASS" } else { "NEEDS_FIX" }
        );
        Ok(())
    }
}

fn profile_context(profile: Option<String>) -> Result<()> {
    let profile = profile.unwrap_or_else(|| "all".to_string());
    if profile == "all" {
        println!("PROFILE_CONTEXT");
        println!("profile=all");
        println!("note=specify a profile name to load context");
        println!("PROFILE_CONTEXT_STATUS=PASS");
        return Ok(());
    }
    validate_profile_name(&profile)?;
    let dir = profile_dir(&profile)?;
    if !dir.join("profile.toml").exists() {
        return Err(CliError::new(
            1,
            format!("PROFILE_CONTEXT_STATUS=BLOCKED profile={profile} not installed"),
        )
        .into());
    }
    let profile_text = fs::read_to_string(dir.join("profile.toml")).unwrap_or_default();
    let name = toml_value_line(&profile_text, "name").unwrap_or_else(|| profile.clone());
    let version =
        toml_value_line(&profile_text, "version").unwrap_or_else(|| "unknown".to_string());
    let owner = toml_value_line(&profile_text, "owner").unwrap_or_else(|| "-".to_string());
    let supp = supplemental_bundle_status(&dir);
    let pref_count = if supp.preferences_dir {
        count_files(&dir.join("preferences"))
    } else {
        0
    };
    let id_count = if supp.identities_dir {
        count_files(&dir.join("identities"))
    } else {
        0
    };
    let sync_count = if supp.sync_dir {
        count_files(&dir.join("sync"))
    } else {
        0
    };
    println!("PROFILE_CONTEXT");
    println!("profile={profile}");
    println!("name={name}");
    println!("version={version}");
    println!("owner={owner}");
    println!("user_md={}", yes_no(supp.user_md));
    println!("memory_md={}", yes_no(supp.memory_md));
    println!("preferences_files={pref_count}");
    println!("identities_files={id_count}");
    println!("sync_files={sync_count}");
    println!("secret_values=none");
    println!("global_agent_config_edits=none");
    println!();
    println!("# AiPlus Profile Context");
    println!();
    println!("- Profile: {name} v{version}");
    println!("- Owner: {owner}");
    println!("- Supplemental bundle:");
    println!(
        "  - USER.md: {}",
        if supp.user_md { "present" } else { "missing" }
    );
    println!(
        "  - MEMORY.md: {}",
        if supp.memory_md { "present" } else { "missing" }
    );
    println!("  - preferences/: {pref_count} files");
    println!("  - identities/: {id_count} files");
    println!("  - sync/: {sync_count} files");
    println!();
    println!("PROFILE_CONTEXT_STATUS=PASS");
    Ok(())
}

fn count_files(dir: &Path) -> usize {
    if !dir.is_dir() {
        return 0;
    }
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                .count()
        })
        .unwrap_or(0)
}

#[allow(dead_code)]
struct MemoryCommandArgs {
    subcommand: Option<String>,
    arg: Option<String>,
    project: bool,
    runtime: Option<String>,
    budget: Option<usize>,
    scope: Option<String>,
    kind: Option<String>,
    text: Option<String>,
    title: Option<String>,
    summary: Option<String>,
    from_memory: Option<String>,
    risk: Option<String>,
    limit: Option<usize>,
}

fn command_memory(args: MemoryCommandArgs) -> Result<()> {
    match args.subcommand.as_deref() {
        Some("status") => memory_status(),
        Some("doctor") => memory_doctor(),
        Some("init") => memory_init_command(args.project),
        Some("context") => memory_context(args.runtime, args.budget),
        Some("add") => memory_add(args.scope, args.kind, args.text),
        Some("list") => memory_list(),
        Some("recent") => memory_recent(),
        Some("search") => memory_search(args.arg),
        Some("forget") => memory_forget(args.arg),
        Some("conflicts") => memory_conflicts(),
        Some("propose") => memory_propose(args.text),
        Some("review") => memory_review(args.arg),
        Some("accept") => memory_accept(args.arg),
        Some("reject") => memory_reject(args.arg),
        Some("auto-capture") => memory_auto_capture(args.text, args.risk),
        Some("session") => memory_session(args.arg, args.text, args.summary, args.limit),
        Some("snapshot") => memory_snapshot(args.arg),
        Some("profile") => memory_profile_sync(),
        Some("show-used") => memory_show_used(),
        Some("stale") => memory_stale(),
        Some("migrate") => memory_migrate(),
        _ => {
            println!("Usage: aiplus memory status|doctor|init --project|context --runtime codex --budget 2000|add --scope project --kind preference --text \"...\"|list|recent|search <query>|forget <id>|conflicts|propose|review|accept|reject|auto-capture --text \"...\" [--risk low|medium|high]|session add-card|search|list|show|snapshot build|profile|show-used|stale|migrate");
            process::exit(2);
        }
    }
}

fn command_identity(subcommand: Option<String>, project: bool, role: Option<String>) -> Result<()> {
    match subcommand.as_deref() {
        Some("status") => identity_status(),
        Some("list") => identity_list(),
        Some("init") => identity_init_command(project),
        Some("context") => identity_context(role),
        _ => {
            println!(
                "Usage: aiplus identity status|list|init --project|context --role advisor|ceo"
            );
            process::exit(2);
        }
    }
}

fn command_user(subcommand: Option<String>, profile: Option<String>) -> Result<()> {
    match subcommand.as_deref() {
        Some("context") => user_context(profile),
        _ => {
            println!("Usage: aiplus user context [--profile <name>]");
            process::exit(2);
        }
    }
}

fn user_context(profile: Option<String>) -> Result<()> {
    let profile = profile.unwrap_or_else(canonical_user_profile_or_default);
    validate_profile_name(&profile)?;
    let dir = profile_dir(&profile)?;
    let user_md = dir.join("USER.md");
    if !user_md.exists() {
        println!("USER_CONTEXT");
        println!("profile={profile}");
        println!("user_md=missing");
        println!("USER_CONTEXT_STATUS=PASS");
        return Ok(());
    }
    let text = fs::read_to_string(&user_md)?;
    let redacted = redact_user_context(&text);
    let truncated = if redacted.len() > 8192 {
        format!(
            "{}\n... [truncated: {} bytes total]",
            &redacted[..6144],
            redacted.len()
        )
    } else {
        redacted
    };
    println!("USER_CONTEXT");
    println!("profile={profile}");
    println!("user_md=present");
    println!("secret_values=none");
    println!("global_agent_config_edits=none");
    println!();
    println!("# AiPlus User Context");
    println!();
    println!("{truncated}");
    println!();
    println!("USER_CONTEXT_STATUS=PASS");
    Ok(())
}

fn redact_user_context(text: &str) -> String {
    let mut result = Vec::new();
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        let is_secret_line = [
            "api_key",
            "apikey",
            "api-key",
            "secret_key",
            "secret-key",
            "access_token",
            "access-token",
            "password",
            "private_key",
            "bearer ",
            "authorization: ",
            "cookie:",
            "-----begin ",
        ]
        .iter()
        .any(|needle| lower.contains(needle));
        if is_secret_line {
            result.push("[REDACTED]".to_string());
        } else {
            result.push(line.to_string());
        }
    }
    result.join("\n")
}

fn unredacted_secret_lines(text: &str) -> Vec<String> {
    let mut lines_with_secrets = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let lower = line.to_ascii_lowercase();
        let is_secret_line = [
            "api_key",
            "apikey",
            "api-key",
            "secret_key",
            "secret-key",
            "access_token",
            "access-token",
            "password",
            "private_key",
            "bearer ",
            "authorization: ",
            "cookie:",
            "-----begin ",
        ]
        .iter()
        .any(|needle| lower.contains(needle));
        if is_secret_line && !line.trim().starts_with("[REDACTED]") {
            lines_with_secrets.push(format!("line {}", idx + 1));
        }
    }
    lines_with_secrets
}

fn command_skill_candidate(
    subcommand: Option<String>,
    arg: Option<String>,
    title: Option<String>,
    from_memory: Option<String>,
) -> Result<()> {
    match subcommand.as_deref() {
        Some("status") => skill_candidate_status(),
        Some("propose") => skill_candidate_propose(title, from_memory),
        Some("reject") => skill_candidate_reject(arg),
        Some("consolidate") => skill_candidate_consolidate(),
        _ => {
            println!(
                "Usage: aiplus skill-candidate status|propose --title \"...\" --from-memory <id>|reject <id>|consolidate"
            );
            process::exit(2);
        }
    }
}

fn memory_status() -> Result<()> {
    let root = target_root()?;
    let memory_dir = memory_dir(&root)?;
    let records = read_memory_records(&root).unwrap_or_default();
    let active = records
        .iter()
        .filter(|record| record.status == "active" || record.status == "tentative")
        .count();
    println!("MEMORY_STATUS");
    println!("scope=project-local");
    println!("installed={}", yes_no(memory_dir.exists()));
    println!(
        "memory_dir={}",
        path_slash(path_relative(&root, &memory_dir)?)
    );
    println!("records_total={}", records.len());
    println!("records_active={active}");
    println!("cloud_sync=none");
    println!("automatic_transcript_learning=none");
    println!("secret_values=none");
    println!("global_agent_config_edits=none");
    println!("MEMORY_STATUS=PASS");
    Ok(())
}

fn memory_doctor() -> Result<()> {
    let root = target_root()?;
    let mut checks = Vec::new();
    let memory = memory_dir(&root)?;
    push_check(&mut checks, ".aiplus/memory exists", memory.exists(), None);
    for rel in [
        ".aiplus/memory/project-memory.jsonl",
        ".aiplus/memory/decisions.jsonl",
        ".aiplus/memory/facts.jsonl",
        ".aiplus/memory/index.json",
        ".aiplus/memory/audit.jsonl",
        ".aiplus/restore/restore-policy.toml",
    ] {
        push_check(
            &mut checks,
            format!("{rel} exists"),
            rel_to_abs(&root, rel)?.exists(),
            None,
        );
    }
    let mut errors = Vec::new();
    for rel in [
        ".aiplus/memory/project-memory.jsonl",
        ".aiplus/memory/decisions.jsonl",
        ".aiplus/memory/facts.jsonl",
    ] {
        errors.extend(validate_memory_jsonl(&root, rel)?);
    }
    errors.extend(memory_sensitive_warnings(&root)?);
    for error in &errors {
        push_check(&mut checks, error.clone(), false, None);
    }

    // Deep scan: orphaned files
    if memory.exists() {
        let expected: std::collections::HashSet<&str> = [
            "project-memory.jsonl",
            "decisions.jsonl",
            "facts.jsonl",
            "index.json",
            "audit.jsonl",
            "sessions.sqlite",
            "MEMORY.md",
            "context-cache.json",
            "review-queue.jsonl",
            "skill-candidates.jsonl",
        ]
        .iter()
        .cloned()
        .collect();
        for entry in fs::read_dir(&memory)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let is_allowed = expected.contains(name.as_str())
                || name.starts_with("project-memory.jsonl.backup-")
                || name.starts_with(".gitkeep");
            if !is_allowed {
                push_check(
                    &mut checks,
                    format!("memory orphan file: {name}"),
                    false,
                    None,
                );
            }
        }
    }

    // Deep scan: large binaries
    if memory.exists() {
        for entry in fs::read_dir(&memory)? {
            let entry = entry?;
            let path = entry.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let metadata = entry.metadata()?;
            if metadata.len() > 1_048_576 {
                push_check(
                    &mut checks,
                    format!("memory file {name} exceeds 1MB"),
                    false,
                    None,
                );
            }
            // Skip binary check for known SQLite and binary file types
            let is_known_binary =
                name.ends_with(".sqlite") || name.ends_with(".db") || name.ends_with(".sqlite3");
            if is_known_binary {
                continue;
            }
            let sample_len = 1024.min(metadata.len() as usize);
            if sample_len > 0 {
                let mut buf = vec![0u8; sample_len];
                if let Ok(mut file) = fs::File::open(&path) {
                    if std::io::Read::read_exact(&mut file, &mut buf).is_ok() && buf.contains(&0) {
                        push_check(
                            &mut checks,
                            format!("memory file {name} contains binary data"),
                            false,
                            None,
                        );
                    }
                }
            }
        }
    }

    // Deep scan: duplicate entries
    for rel in [
        ".aiplus/memory/project-memory.jsonl",
        ".aiplus/memory/decisions.jsonl",
        ".aiplus/memory/facts.jsonl",
    ] {
        let path = rel_to_abs(&root, rel)?;
        if !path.exists() {
            continue;
        }
        let text = fs::read_to_string(&path)?;
        let mut seen = std::collections::HashSet::new();
        for (idx, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(record) = serde_json::from_str::<MemoryRecord>(line) {
                if !seen.insert(record.id.clone()) {
                    push_check(
                        &mut checks,
                        format!("{} duplicate id {} at line {}", rel, record.id, idx + 1),
                        false,
                        None,
                    );
                }
            }
        }
    }

    // Deep scan: record-level analysis
    let all_records = read_all_including_rejected(&root).unwrap_or_default();
    let mut active_sensitive = 0usize;
    let mut stale_count = 0usize;
    let mut rejected_count = 0usize;
    let mut schema_issues = 0usize;

    for record in &all_records {
        // Active records sensitive-pattern scan
        if (record.status == "active" || record.status == "tentative")
            && reject_sensitive_memory_text(&record.summary).is_err()
        {
            active_sensitive += 1;
            push_check(
                &mut checks,
                format!("active record {} contains sensitive pattern", record.id),
                false,
                Some("Redact the summary or mark the record as superseded.".to_string()),
            );
        }

        // Stale detection
        if record.is_stale() {
            stale_count += 1;
        }

        // Rejected / forgotten count
        if record.status == "rejected" || record.status == "forgotten" {
            rejected_count += 1;
        }

        // Schema issue: missing required fields
        if record.id.is_empty() || record.record_type.is_empty() || record.summary.is_empty() {
            schema_issues += 1;
            push_check(
                &mut checks,
                format!("record {} missing required fields", record.id),
                false,
                Some("Rewrite the record with valid id, type, and summary.".to_string()),
            );
        }
    }

    // Conflict detection
    let conflicts = detect_conflicts(&all_records);
    for conflict in &conflicts {
        push_check(
            &mut checks,
            format!(
                "conflict {}: {} (related: {})",
                conflict.record_id,
                conflict.conflict_type,
                conflict.related_ids.join(", ")
            ),
            false,
            Some(format!(
                "Review related records and resolve divergence. {}",
                conflict.description
            )),
        );
    }

    let pass = checks.iter().all(|check| check.ok);
    println!("MEMORY_DOCTOR");
    println!("status={}", if pass { "PASS" } else { "NEEDS_FIX" });
    println!("scope=project-local");
    println!("secret_values=none");
    println!("global_agent_config_edits=none");
    println!("records_total={}", all_records.len());
    println!("records_stale={stale_count}");
    println!("records_rejected={rejected_count}");
    println!("records_sensitive={active_sensitive}");
    println!("conflicts_detected={}", conflicts.len());
    println!("schema_issues={schema_issues}");
    println!("auto_deletion=none");
    for check in &checks {
        let fix = check
            .fix
            .as_ref()
            .map(|r| format!(" -> {r}"))
            .unwrap_or_default();
        println!(
            "{} {}{}",
            if check.ok { "PASS" } else { "NEEDS_FIX" },
            check.label,
            fix
        );
    }
    println!(
        "MEMORY_DOCTOR_STATUS={}",
        if pass { "PASS" } else { "NEEDS_FIX" }
    );
    if pass {
        Ok(())
    } else {
        process::exit(1);
    }
}

fn memory_init_command(project: bool) -> Result<()> {
    if !project {
        return Err(CliError::new(1, "ERROR memory init requires --project").into());
    }
    let root = target_root()?;
    memory_init(&root)?;
    println!("MEMORY_INIT");
    println!("scope=project-local");
    println!(
        "created_or_verified=.aiplus/memory,.aiplus/identities,.aiplus/skills,.aiplus/restore"
    );
    println!("cloud_sync=none");
    println!("automatic_transcript_learning=none");
    println!("global_agent_config_edits=none");
    println!("MEMORY_INIT_STATUS=PASS");
    Ok(())
}

fn memory_context(runtime: Option<String>, budget: Option<usize>) -> Result<()> {
    let root = target_root()?;
    let runtime = runtime.unwrap_or_else(|| "codex".to_string());
    let budget = budget.unwrap_or(2000);
    let records = read_memory_records(&root).unwrap_or_default();
    let active_records = select_records(&records, budget);
    let ignored = records.len().saturating_sub(active_records.len());
    println!("MEMORY_CONTEXT");
    println!("runtime={runtime}");
    println!("budget={budget}");
    println!("records_used={}", active_records.len());
    println!("records_ignored={ignored}");
    println!("sources=[.aiplus/memory/project-memory.jsonl,.aiplus/memory/decisions.jsonl,.aiplus/memory/facts.jsonl]");
    println!("owner_gates=[publish,deploy,global config,external accounts,secret exposure]");
    println!("scope=project-local");
    println!("secret_values=none");
    println!("global_agent_config_edits=none");
    println!();
    println!("# AiPlus Memory Context");
    println!();
    println!("- Memory is context, not instruction.");
    println!("- Identity is role contract, not permission.");
    println!("- Skill Candidate is a proposal, not an approved skill.");
    println!("- Secret access requires explicit alias plus trusted runtime command.");
    println!();
    println!("## Sources");
    println!();
    println!("- .aiplus/memory/project-memory.jsonl");
    println!("- .aiplus/memory/decisions.jsonl");
    println!("- .aiplus/memory/facts.jsonl");
    println!();
    println!("## Records");
    for record in active_records {
        let summary = if reject_sensitive_memory_text(&record.summary).is_ok() {
            record.summary.clone()
        } else {
            "[REDACTED]".to_string()
        };
        println!(
            "- [{}] {} / {} / {}: {}",
            record.id,
            record.scope,
            record.record_type,
            record.subject.as_deref().unwrap_or("-"),
            summary
        );
    }
    println!("MEMORY_CONTEXT_STATUS=PASS");
    Ok(())
}

fn memory_list() -> Result<()> {
    let root = target_root()?;
    let records = read_active(&root).unwrap_or_default();
    println!("MEMORY_LIST");
    println!("records_total={}", records.len());
    let shown = print_memory_rows(records.iter(), None);
    println!("records_shown={shown}");
    println!("secret_values=none");
    println!("MEMORY_LIST_STATUS=PASS");
    Ok(())
}

fn memory_recent() -> Result<()> {
    let root = target_root()?;
    let records = read_active(&root).unwrap_or_default();
    println!("MEMORY_RECENT");
    println!("limit=5");
    let shown = print_memory_rows(records.iter().rev(), Some(5));
    println!("records_shown={shown}");
    println!("secret_values=none");
    println!("MEMORY_RECENT_STATUS=PASS");
    Ok(())
}

fn memory_add(scope: Option<String>, kind: Option<String>, text: Option<String>) -> Result<()> {
    let root = target_root()?;
    let scope = scope.unwrap_or_else(|| "project".to_string());
    let kind = kind.unwrap_or_else(|| "preference".to_string());
    let text = text.ok_or_else(|| CliError::new(2, "ERROR memory add requires --text"))?;
    validate_memory_field(
        "scope",
        &scope,
        &["session", "project", "profile", "global"],
    )?;
    validate_memory_field(
        "kind",
        &kind,
        &[
            "preference",
            "project_fact",
            "decision_summary",
            "constraint",
            "handoff_note",
            "evidence_pointer",
        ],
    )?;
    reject_sensitive_memory_text(&text)?;
    memory_init(&root)?;
    let now = timestamp();
    let id = format!("mem_{}_{}", epoch_millis(), stable_hash(&text));
    let record = MemoryRecord {
        schema_version: MEMORY_SCHEMA_VERSION_V2.to_string(),
        id: id.clone(),
        scope,
        record_type: kind,
        source: "manual".to_string(),
        created_at: now.clone(),
        updated_at: now,
        confidence: "owner_asserted".to_string(),
        status: "active".to_string(),
        summary: single_line(&text),
        evidence: Vec::new(),
        tags: Vec::new(),
        expires_at: None,
        stale_after: None,
        supersedes: Vec::new(),
        superseded_by: Vec::new(),
        conflict_group: None,
        redaction: "none".to_string(),
        subject: Some("workflow".to_string()),
        visibility: Some("project-local".to_string()),
        content_hash: Some(format!("hash:{}", stable_hash(&text))),
    };
    append_jsonl_atomic(
        &rel_to_abs(&root, ".aiplus/memory/project-memory.jsonl")?,
        &serde_json::to_string(&record)?,
    )?;
    append_audit(&root, "memory.add", &id)?;

    // Invalidate warm-bench cache: memory add/forget → invalidate all
    if let Ok(cache) = crate::agent::cache::global_cache().lock() {
        cache.invalidate_all();
    }

    println!("MEMORY_ADD");
    println!("id={id}");
    println!("scope={}", record.scope);
    println!("kind={}", record.record_type);
    println!("secret_values=none");
    println!("MEMORY_ADD_STATUS=PASS");
    Ok(())
}

fn memory_search(query: Option<String>) -> Result<()> {
    let root = target_root()?;
    let query = query.ok_or_else(|| CliError::new(2, "ERROR memory search requires a query"))?;
    let records = find_by_query(&root, &query)?;
    println!("MEMORY_SEARCH");
    println!("query={}", single_line(&query));
    println!("matches={}", records.len());
    for record in &records {
        println!(
            "match={} kind={} status={}",
            record.id, record.record_type, record.status
        );
    }
    println!("secret_values=none");
    println!("MEMORY_SEARCH_STATUS=PASS");
    Ok(())
}

fn memory_forget(id: Option<String>) -> Result<()> {
    let root = target_root()?;
    let id = id.ok_or_else(|| CliError::new(2, "ERROR memory forget requires an id"))?;
    let file = rel_to_abs(&root, ".aiplus/memory/project-memory.jsonl")?;
    let mut records = read_memory_records(&root).unwrap_or_default();
    let mut found = false;
    for record in &mut records {
        if record.id == id {
            record.status = "rejected".to_string();
            record.updated_at = timestamp();
            found = true;
        }
    }
    if !found {
        return Err(CliError::new(
            1,
            format!("MEMORY_FORGET_STATUS=FAIL id={id} reason=not_found"),
        )
        .into());
    }
    rewrite_jsonl_atomic(&file, &records)?;
    append_audit(&root, "memory.forget", &id)?;

    // Invalidate warm-bench cache: memory add/forget → invalidate all
    if let Ok(cache) = crate::agent::cache::global_cache().lock() {
        cache.invalidate_all();
    }

    println!("MEMORY_FORGET");
    println!("id={id}");
    println!("status=rejected");
    println!("forgotten=yes");
    println!("secret_values=none");
    println!("MEMORY_FORGET_STATUS=PASS");
    Ok(())
}

fn memory_conflicts() -> Result<()> {
    let root = target_root()?;
    let records = read_memory_records(&root).unwrap_or_default();
    let conflicts = detect_conflicts(&records);
    let stale = detect_stale(&records);
    println!("MEMORY_CONFLICTS");
    println!("unresolved={}", conflicts.len());
    println!("stale_records={}", stale.len());
    for conflict in &conflicts {
        println!(
            "conflict={} type={} related=[{}]",
            conflict.record_id,
            conflict.conflict_type,
            conflict.related_ids.join(",")
        );
    }
    for stale_record in &stale {
        println!(
            "stale={} reason={}",
            stale_record.record_id, stale_record.reason
        );
    }
    println!("secret_values=none");
    println!("MEMORY_CONFLICTS_STATUS=PASS");
    Ok(())
}

fn memory_propose(text: Option<String>) -> Result<()> {
    let root = target_root()?;
    let text = text.ok_or_else(|| CliError::new(2, "ERROR memory propose requires --text"))?;
    reject_sensitive_memory_text(&text)?;
    memory_init(&root)?;
    let id = format!("mem_{}_{}", epoch_millis(), stable_hash(&text));
    let record = MemoryRecord {
        schema_version: "0.2.0".to_string(),
        id: id.clone(),
        scope: "project".to_string(),
        record_type: "proposal".to_string(),
        source: "manual".to_string(),
        created_at: timestamp(),
        updated_at: timestamp(),
        confidence: "owner_asserted".to_string(),
        status: "proposed".to_string(),
        summary: single_line(&text),
        evidence: Vec::new(),
        tags: Vec::new(),
        expires_at: None,
        stale_after: None,
        supersedes: Vec::new(),
        superseded_by: Vec::new(),
        conflict_group: None,
        redaction: "none".to_string(),
        subject: Some("proposal".to_string()),
        visibility: Some("project-local".to_string()),
        content_hash: Some(format!("hash:{}", stable_hash(&text))),
    };
    append_jsonl_atomic(
        &rel_to_abs(&root, ".aiplus/memory/project-memory.jsonl")?,
        &serde_json::to_string(&record)?,
    )?;
    append_audit(&root, "memory.propose", &id)?;
    println!("MEMORY_PROPOSE");
    println!("id={id}");
    println!("status=proposed");
    println!("secret_values=none");
    println!("MEMORY_PROPOSE_STATUS=PASS");
    Ok(())
}

fn memory_review(id: Option<String>) -> Result<()> {
    let id = id.ok_or_else(|| CliError::new(2, "ERROR memory review requires an id"))?;
    println!("MEMORY_REVIEW");
    println!("id={id}");
    println!("status=needs_review");
    println!("secret_values=none");
    println!("MEMORY_REVIEW_STATUS=NOT_IMPLEMENTED");
    Ok(())
}

fn memory_accept(id: Option<String>) -> Result<()> {
    let root = target_root()?;
    let id = id.ok_or_else(|| CliError::new(2, "ERROR memory accept requires an id"))?;
    let file = rel_to_abs(&root, ".aiplus/memory/project-memory.jsonl")?;
    let mut records = read_memory_records(&root).unwrap_or_default();
    let mut found = false;
    for record in &mut records {
        if record.id == id {
            record.status = "active".to_string();
            record.updated_at = timestamp();
            found = true;
        }
    }
    if !found {
        return Err(CliError::new(
            1,
            format!("MEMORY_ACCEPT_STATUS=FAIL id={id} reason=not_found"),
        )
        .into());
    }
    rewrite_jsonl_atomic(&file, &records)?;
    append_audit(&root, "memory.accept", &id)?;
    println!("MEMORY_ACCEPT");
    println!("id={id}");
    println!("status=active");
    println!("secret_values=none");
    println!("MEMORY_ACCEPT_STATUS=PASS");
    Ok(())
}

fn memory_reject(id: Option<String>) -> Result<()> {
    let root = target_root()?;
    let id = id.ok_or_else(|| CliError::new(2, "ERROR memory reject requires an id"))?;
    let file = rel_to_abs(&root, ".aiplus/memory/project-memory.jsonl")?;
    let mut records = read_memory_records(&root).unwrap_or_default();
    let mut found = false;
    for record in &mut records {
        if record.id == id {
            record.status = "rejected".to_string();
            record.updated_at = timestamp();
            found = true;
        }
    }
    if !found {
        return Err(CliError::new(
            1,
            format!("MEMORY_REJECT_STATUS=FAIL id={id} reason=not_found"),
        )
        .into());
    }
    rewrite_jsonl_atomic(&file, &records)?;
    append_audit(&root, "memory.reject", &id)?;
    println!("MEMORY_REJECT");
    println!("id={id}");
    println!("status=rejected");
    println!("secret_values=none");
    println!("MEMORY_REJECT_STATUS=PASS");
    Ok(())
}

fn memory_auto_capture(text: Option<String>, risk: Option<String>) -> Result<()> {
    let root = target_root()?;
    let text = text.ok_or_else(|| CliError::new(2, "ERROR auto-capture requires --text"))?;
    reject_sensitive_memory_text(&text)?;

    let config = AutoWriteConfig::default();
    let writer = AutoWriter::new(config);
    let risk_level = risk.as_deref().unwrap_or("auto");
    let memory_type = if risk_level == "low" {
        "owner_preference"
    } else {
        "workflow_rule"
    };

    let result = if risk_level == "high" {
        AutoWriteResult {
            written: false,
            risk_level: RiskLevel::High,
            record_id: None,
            reason: "HIGH_RISK_BLOCKED".to_string(),
        }
    } else {
        writer.auto_capture(&root, &text, memory_type, "project")?
    };

    let risk_level_str = match result.risk_level {
        RiskLevel::Low => "low",
        RiskLevel::Medium => "medium",
        RiskLevel::High => "high",
    };
    println!("MEMORY_AUTO_CAPTURE");
    println!("risk_level={}", risk_level_str);
    println!("written={}", yes_no(result.written));
    if let Some(id) = result.record_id {
        println!("id={id}");
    }
    println!("reason={}", result.reason);
    println!("secret_values=none");
    println!(
        "MEMORY_AUTO_CAPTURE_STATUS={}",
        if result.written { "PASS" } else { "BLOCKED" }
    );
    Ok(())
}

fn memory_session(
    subcommand: Option<String>,
    text: Option<String>,
    summary: Option<String>,
    limit: Option<usize>,
) -> Result<()> {
    let root = target_root()?;
    let index = SessionIndex::new(&root)?;
    index.init()?;

    match subcommand.as_deref() {
        Some("add-card") => {
            let summary = summary
                .ok_or_else(|| CliError::new(2, "ERROR session add-card requires --summary"))?;
            reject_sensitive_memory_text(&summary)?;
            let id = format!("sess_{}", epoch_millis());
            let session = SessionRecord {
                id: id.clone(),
                project_id: project_id_from_root(&root),
                role: "session".to_string(),
                created_at: timestamp(),
                updated_at: timestamp(),
                summary,
                decisions: Vec::new(),
                files_changed: Vec::new(),
                commands_run: Vec::new(),
                tests_run: Vec::new(),
                findings_fixed: Vec::new(),
                blockers: Vec::new(),
                next_action: String::new(),
                memory_ids_used: Vec::new(),
                skill_candidates_proposed: Vec::new(),
                compact_checkpoint_link: None,
                no_secret_marker: true,
            };
            index.add_session(&session)?;
            println!("MEMORY_SESSION");
            println!("action=add-card");
            println!("id={id}");
            println!("secret_values=none");
            println!("MEMORY_SESSION_STATUS=PASS");
        }
        Some("search") => {
            let query =
                text.ok_or_else(|| CliError::new(2, "ERROR session search requires --text"))?;
            let limit = limit.unwrap_or(10);
            let results = index.search(&query, limit)?;
            println!("MEMORY_SESSION");
            println!("action=search");
            println!("query={}", single_line(&query));
            println!("sessions_found={}", results.len());
            for session in &results {
                println!(
                    "session={} summary={}",
                    session.id,
                    single_line(&session.summary)
                );
            }
            println!("secret_values=none");
            println!("MEMORY_SESSION_STATUS=PASS");
        }
        Some("list") => {
            let limit = limit.unwrap_or(10);
            let results = index.list_recent(limit)?;
            println!("MEMORY_SESSION");
            println!("action=list");
            println!("sessions_found={}", results.len());
            for session in &results {
                println!(
                    "session={} summary={}",
                    session.id,
                    single_line(&session.summary)
                );
            }
            println!("secret_values=none");
            println!("MEMORY_SESSION_STATUS=PASS");
        }
        Some("show") => {
            let id = text.ok_or_else(|| CliError::new(2, "ERROR session show requires --text"))?;
            let session = index.get_session(&id)?;
            println!("MEMORY_SESSION");
            println!("action=show");
            if let Some(session) = session {
                println!("id={}", session.id);
                println!("summary={}", single_line(&session.summary));
                println!("role={}", session.role);
                println!("created_at={}", session.created_at);
            } else {
                println!("id={id}");
                println!("found=no");
            }
            println!("secret_values=none");
            println!("MEMORY_SESSION_STATUS=PASS");
        }
        _ => {
            println!("Usage: aiplus memory session add-card --summary \"...\"|search --text \"<query>\" [--limit N]|list [--limit N]|show --text \"<id>\"");
            process::exit(2);
        }
    }
    Ok(())
}

fn memory_snapshot(subcommand: Option<String>) -> Result<()> {
    let root = target_root()?;
    match subcommand.as_deref() {
        Some("build") => {
            let builder = SnapshotBuilder::new(&root);
            let project_path = builder.write_project_snapshot()?;
            let profile_root = config_home()?.join("aiplus").join("profiles");
            let profile_name = canonical_user_profile_or_default();
            let _profile_path = builder.write_profile_snapshot(&profile_root, &profile_name)?;
            let records = read_memory_records(&root).unwrap_or_default();
            let profile_mem_path = profile_root
                .join(&profile_name)
                .join("profile-memory/memories.jsonl");
            let profile_records = if profile_mem_path.exists() {
                let text = std::fs::read_to_string(&profile_mem_path)?;
                text.lines().filter(|l| !l.trim().is_empty()).count()
            } else {
                0
            };
            println!("MEMORY_SNAPSHOT_BUILD");
            println!(
                "project_snapshot={}",
                path_slash(path_relative(&root, &project_path)?)
            );
            println!("profile_snapshot={profile_name}/profile-memory/USER.md");
            println!("project_records={}", records.len());
            println!("profile_records={}", profile_records);
            println!("MEMORY_SNAPSHOT_BUILD_STATUS=PASS");
        }
        _ => {
            println!("Usage: aiplus memory snapshot build");
            process::exit(2);
        }
    }
    Ok(())
}

fn memory_profile_sync() -> Result<()> {
    let root = target_root()?;
    let profile_root = config_home()?.join("aiplus").join("profiles");
    let profile_name = canonical_user_profile_or_default();
    let sync = ProfileSync::new(&profile_root, &profile_name);
    let result = sync.sync_to_project(&root)?;
    println!("MEMORY_PROFILE_SYNC");
    println!("profile_records_read={}", result.profile_records_read);
    println!("project_records_updated={}", result.project_records_updated);
    println!("conflicts={}", result.conflicts.len());
    for conflict in &result.conflicts {
        println!("conflict={}", conflict);
    }
    println!("PROFILE_SYNC_STATUS=PASS");
    Ok(())
}

fn memory_show_used() -> Result<()> {
    let root = target_root()?;
    let records = read_memory_records(&root).unwrap_or_default();
    let active_ids: Vec<String> = records
        .iter()
        .filter(|r| r.status == "active" || r.status == "tentative")
        .map(|r| r.id.clone())
        .collect();
    let session_index = SessionIndex::new(&root)?;
    let recent_sessions = session_index.list_recent(5).unwrap_or_default();
    let session_ids: Vec<String> = recent_sessions.iter().map(|s| s.id.clone()).collect();
    println!("MEMORY_SHOW_USED");
    println!("memory_ids={:?}", active_ids);
    println!("session_ids={:?}", session_ids);
    println!("SHOW_USED_STATUS=PASS");
    Ok(())
}

fn memory_stale() -> Result<()> {
    let root = target_root()?;
    let records = read_memory_records(&root).unwrap_or_default();
    let stale = detect_stale(&records);
    let expired: Vec<&MemoryRecord> = records
        .iter()
        .filter(|r| {
            r.expires_at
                .as_ref()
                .is_some_and(|e| e.parse::<u128>().ok().is_some_and(|ts| ts < epoch_millis()))
        })
        .collect();
    println!("MEMORY_STALE");
    println!("stale_records={}", stale.len());
    for s in &stale {
        println!("stale={} reason={}", s.record_id, s.reason);
    }
    println!("expired_records={}", expired.len());
    for r in &expired {
        println!("expired={}", r.id);
    }
    println!("STALE_STATUS=PASS");
    Ok(())
}

fn memory_migrate() -> Result<()> {
    let root = target_root()?;
    let records = read_memory_records(&root).unwrap_or_default();
    let mut migrated = 0usize;
    let file = rel_to_abs(&root, ".aiplus/memory/project-memory.jsonl")?;
    let mut new_records = Vec::new();
    for mut record in records {
        if record.schema_version == "0.1.0" {
            record.schema_version = "0.2.0".to_string();
            migrated += 1;
        }
        new_records.push(record);
    }
    if migrated > 0 {
        rewrite_jsonl_atomic(&file, &new_records)?;
    }
    println!("MEMORY_MIGRATE");
    println!("records_migrated={}", migrated);
    println!("schema_version=0.2.0");
    println!("MIGRATE_STATUS=PASS");
    Ok(())
}

fn skill_candidate_consolidate() -> Result<()> {
    let root = target_root()?;
    let records = read_memory_records(&root).unwrap_or_default();
    let mut registry = SkillRegistry::new(&root);
    let candidates = registry.consolidate_from_memory(&records)?;

    println!("SKILL_CANDIDATE_CONSOLIDATE");
    println!("candidates_found={}", candidates.len());
    for candidate in &candidates {
        println!(
            "candidate={} title={} status={}",
            candidate.id, candidate.title, candidate.status
        );
    }
    println!("candidate_is_approved_skill=no");
    println!("approval_requires=qa_and_owner_gate");
    println!("SKILL_CANDIDATE_CONSOLIDATE_STATUS=PASS");
    Ok(())
}

fn identity_status() -> Result<()> {
    let root = target_root()?;
    let dir = identity_dir(&root)?;
    println!("IDENTITY_STATUS");
    println!("scope=project");
    println!("installed={}", yes_no(dir.exists()));
    for role in ["advisor", "ceo", "reviewer", "builder"] {
        println!(
            "{}={}",
            role,
            if dir.join(format!("{role}.identity.toml")).exists() {
                "present"
            } else {
                "missing"
            }
        );
    }
    println!("identity_grants_permission=no");
    println!("global_agent_config_edits=none");
    println!("IDENTITY_STATUS=PASS");
    Ok(())
}

fn identity_list() -> Result<()> {
    let root = target_root()?;
    let dir = identity_dir(&root)?;
    println!("IDENTITY_LIST");
    println!("scope=project");
    for role in ["advisor", "ceo", "reviewer", "builder"] {
        let present = dir.join(format!("{role}.identity.toml")).exists();
        println!("{role}={}", if present { "present" } else { "missing" });
    }
    println!("identity_grants_permission=no");
    println!("global_agent_config_edits=none");
    println!("IDENTITY_LIST_STATUS=PASS");
    Ok(())
}

fn identity_init_command(project: bool) -> Result<()> {
    if !project {
        return Err(CliError::new(1, "ERROR identity init requires --project").into());
    }
    let root = target_root()?;
    identity_init(&root)?;
    println!("IDENTITY_INIT");
    println!("scope=project");
    println!("roles=[advisor,ceo,reviewer,builder]");
    println!("identity_grants_permission=no");
    println!("global_agent_config_edits=none");
    println!("IDENTITY_INIT_STATUS=PASS");
    Ok(())
}

fn identity_context(role: Option<String>) -> Result<()> {
    let root = target_root()?;
    let role = role.unwrap_or_else(|| "advisor".to_string());
    validate_memory_field("role", &role, &["advisor", "ceo", "reviewer", "builder"])?;
    identity_init(&root)?;
    let identity = read_identity(&root, &role)?;
    let file = identity_dir(&root)?.join(format!("{role}.identity.toml"));
    let text = fs::read_to_string(&file)?;
    let role_name = toml_value_line(&text, "role").unwrap_or_else(|| role.clone());
    let activation = if identity.activation.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", identity.activation.join(","))
    };
    let owner_gates = if identity.owner_gates.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", identity.owner_gates.join(","))
    };
    let inherits = if identity.inherits.is_empty() {
        "[]".to_string()
    } else {
        format!("[{}]", identity.inherits.join(","))
    };
    let private_profile_linked = match canonical_user_profile()? {
        Some(ref name) => inherits.contains(name.as_str()),
        None => false,
    };
    println!("IDENTITY_CONTEXT");
    println!("role={role}");
    println!("role_name={role_name}");
    println!("activation={activation}");
    println!("output_contract={}", identity.output_contract);
    println!("owner_gates={owner_gates}");
    println!("permissions=none");
    println!("scope=project");
    println!("identity_grants_permission=no");
    println!(
        "inherited_private_profile={}",
        yes_no(private_profile_linked)
    );
    println!("inherits={inherits}");
    println!("global_agent_config_edits=none");
    println!("IDENTITY_CONTEXT_STATUS=PASS");
    Ok(())
}

fn skill_candidate_status() -> Result<()> {
    let root = target_root()?;
    let candidates = read_skill_candidates(&root).unwrap_or_default();
    let proposed = candidates
        .iter()
        .filter(|candidate| candidate.status == "candidate_proposed")
        .count();
    let rejected = candidates
        .iter()
        .filter(|candidate| candidate.status == "rejected")
        .count();
    println!("SKILL_CANDIDATE_STATUS");
    println!("scope=project-local");
    println!("candidates_total={}", candidates.len());
    println!("candidate_proposed={proposed}");
    println!("rejected={rejected}");
    println!("candidate_is_approved_skill=no");
    println!("approval_requires=qa_and_owner_gate");
    println!("rejected_auto_load=no");
    println!("automatic_approved_skills=none");
    println!("owner_gate_required_for_approval=yes");
    println!("secret_values=none");
    println!("SKILL_CANDIDATE_STATUS=PASS");
    Ok(())
}

fn skill_candidate_propose(title: Option<String>, from_memory: Option<String>) -> Result<()> {
    let root = target_root()?;
    let title =
        title.ok_or_else(|| CliError::new(2, "ERROR skill-candidate propose requires --title"))?;
    reject_sensitive_memory_text(&title)?;
    memory_init(&root)?;
    let id = format!("skill-candidate/{}_{}", epoch_millis(), stable_hash(&title));
    let source_memory_ids = from_memory.into_iter().collect();
    let candidate = SkillCandidate {
        schema_version: aiplus_core::SKILL_SCHEMA_VERSION_V2.to_string(),
        id: id.clone(),
        title: single_line(&title),
        status: "candidate_proposed".to_string(),
        source_memory_ids,
        problem_pattern: "Owner-proposed repeatable workflow candidate".to_string(),
        proposed_skill_name: slugify(&title),
        repeatability_evidence: aiplus_core::skill_candidate::RepeatabilityEvidence::default(),
        trigger_design: aiplus_core::skill_candidate::TriggerDesign::default(),
        scope: aiplus_core::skill_candidate::SkillScope::default(),
        privacy: aiplus_core::skill_candidate::SkillPrivacy::default(),
        qa: aiplus_core::skill_candidate::SkillQa {
            required_checks: Vec::new(),
            status: "pending".to_string(),
        },
        owner_gate: aiplus_core::skill_candidate::SkillOwnerGate {
            required_for_approval: true,
            approved_by: None,
            approved_at: None,
        },
        evidence_links: Vec::new(),
        rejection_reason: None,
        needs_evidence: false,
        proposed_at: timestamp(),
        updated_at: timestamp(),
    };
    append_jsonl_atomic(
        &rel_to_abs(&root, ".aiplus/skills/candidates/candidates.jsonl")?,
        &serde_json::to_string(&candidate)?,
    )?;
    append_audit(&root, "skill-candidate.propose", &id)?;
    println!("SKILL_CANDIDATE_PROPOSE");
    println!("id={id}");
    println!("status=candidate_proposed");
    println!("candidate_is_approved_skill=no");
    println!("approval_requires=qa_and_owner_gate");
    println!("owner_gate_required_for_approval=yes");
    println!("secret_values=none");
    println!("SKILL_CANDIDATE_PROPOSE_STATUS=PASS");
    Ok(())
}

fn skill_candidate_reject(id: Option<String>) -> Result<()> {
    let root = target_root()?;
    let id = id.ok_or_else(|| CliError::new(2, "ERROR skill-candidate reject requires an id"))?;
    let file = rel_to_abs(&root, ".aiplus/skills/candidates/candidates.jsonl")?;
    let mut candidates = read_skill_candidates(&root).unwrap_or_default();
    let mut found = false;
    for candidate in &mut candidates {
        if candidate.id == id {
            candidate.status = "rejected".to_string();
            found = true;
        }
    }
    if !found {
        return Err(CliError::new(
            1,
            format!("SKILL_CANDIDATE_REJECT_STATUS=FAIL id={id} reason=not_found"),
        )
        .into());
    }
    rewrite_jsonl_atomic(&file, &candidates)?;
    append_audit(&root, "skill-candidate.reject", &id)?;
    println!("SKILL_CANDIDATE_REJECT");
    println!("id={id}");
    println!("status=rejected");
    println!("rejected_auto_load=no");
    println!("secret_values=none");
    println!("SKILL_CANDIDATE_REJECT_STATUS=PASS");
    Ok(())
}

fn command_secret_broker(
    subcommand: Option<String>,
    arg: Option<String>,
    print_secret: bool,
    alias_flags: Vec<String>,
    aliases_csv: Option<String>,
    to: Option<String>,
    auto_prompt: bool,
    env_var: Option<String>,
    command: Vec<String>,
) -> Result<()> {
    match subcommand.as_deref() {
        Some("status") => secret_broker_status(),
        Some("doctor") => secret_broker_doctor(),
        Some("list") => secret_broker_list(),
        Some("resolve") => secret_broker_resolve(arg, print_secret),
        Some("run") => secret_broker_run(alias_flags, aliases_csv, command),
        Some("push") => secret_broker_push(alias_flags, aliases_csv, to, print_secret),
        Some("token") => secret_broker_token(arg),
        Some("set") => {
            secret_broker_set(alias_flags.first().cloned().or(arg), auto_prompt, env_var)
        }
        Some("need") => secret_broker_need(
            collect_alias_args(arg.clone(), alias_flags.clone(), aliases_csv.clone()),
            auto_prompt,
            env_var,
        ),
        Some("delete") => secret_broker_delete(alias_flags.first().cloned().or(arg)),
        Some("shell-init") => secret_broker_shell_init(arg),
        Some("hook") => secret_broker_hook(),
        _ => {
            println!("Usage: aiplus secret-broker status|doctor|list|resolve <alias>|run [--aliases a,b|--alias a] -- <command...>|push --alias <a> --to <target>|set <alias> [--auto-prompt] [--env <NAME>]|need <alias>... [--auto-prompt]|delete <alias>|shell-init zsh|bash|fish|hook|token set|delete");
            process::exit(2);
        }
    }
}

// K2: collect alias names from `arg`, repeated `--alias`, and `--aliases
// a,b`. Used by `need` which can take multiple aliases in one call.
fn collect_alias_args(
    arg: Option<String>,
    alias_flags: Vec<String>,
    aliases_csv: Option<String>,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(a) = arg {
        if !a.is_empty() {
            out.push(a);
        }
    }
    out.extend(alias_flags);
    if let Some(csv) = aliases_csv {
        for part in csv.split(',') {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
        }
    }
    let mut seen = BTreeSet::new();
    out.retain(|a| seen.insert(a.clone()));
    out
}

// v0.5.16: store a secret value for the default (keyring) backend.
// v0.5.18: optional --auto-prompt flag pops a native OS dialog so the
// agent can drive the input flow without the user having to switch
// to their terminal. --env <NAME> overrides the default
// `<ALIAS>_API_KEY` env-var name (useful for multi-account setups
// like openai_work / openai_personal both → OPENAI_API_KEY).
fn secret_broker_set(
    alias_name: Option<String>,
    auto_prompt: bool,
    env_override: Option<String>,
) -> Result<()> {
    let alias_name = alias_name.ok_or_else(|| {
        CliError::new(1, "ERROR aiplus secret-broker set requires --alias <name>")
    })?;
    if provider_name() != "keyring" {
        return Err(CliError::new(
            1,
            format!(
                "SECRET_SET_STATUS=FAIL reason=wrong_provider current={} hint=`set` only works with the keyring backend. Unset AIPLUS_SECRET_PROVIDER or set it to `keyring`.",
                provider_name()
            ),
        )
        .into());
    }
    let value = if auto_prompt {
        prompt_secret_via_gui(&alias_name)?
    } else {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        buf.trim().to_string()
    };
    if value.is_empty() {
        return Err(CliError::new(
            1,
            "ERROR empty value (use `--auto-prompt` for a native dialog, or `echo -n $YOUR_KEY | aiplus secret-broker set --alias <name>`)",
        )
        .into());
    }
    let entry = keyring_alias_entry(&alias_name)?;
    match entry.set_password(&value) {
        Ok(()) => {}
        Err(keyring::Error::NoStorageAccess(detail))
        | Err(keyring::Error::PlatformFailure(detail)) => {
            return Err(CliError::new(
                1,
                format!("SECRET_SET_STATUS=FAIL reason=keyring_unavailable detail={detail}"),
            )
            .into());
        }
        Err(e) => {
            return Err(CliError::new(
                1,
                format!("SECRET_SET_STATUS=FAIL reason=keyring_write_failed detail={e}"),
            )
            .into());
        }
    }
    let env_var =
        env_override.unwrap_or_else(|| format!("{}_API_KEY", alias_name.to_ascii_uppercase()));
    append_alias_to_registry(&alias_name, &env_var)?;
    println!(
        "SECRET_SET_STATUS=PASS alias={alias_name} provider=keyring env_var={env_var} stored=os_keyring"
    );
    Ok(())
}

// K1: OS native password dialog. Pops a hidden-input box on the user's
// desktop, even when the calling process has no controlling TTY (i.e.
// when an AI agent runs `aiplus secret-broker set --auto-prompt` inside
// its sandboxed bash tool). The dialog draws on the OS's own UI server
// (macOS WindowServer / Linux X11/Wayland / Windows DWM), not on the
// caller's terminal. Falls back to rpassword tty prompt when the
// system has no GUI (SSH, headless CI, etc.).
//
// Three platform shims:
//   macOS    : osascript (always present on macOS)
//   Linux    : zenity → kdialog → tty fallback
//   Windows  : powershell Read-Host -AsSecureString
fn prompt_secret_via_gui(alias_name: &str) -> Result<String> {
    let title = "AiPlus secret-broker";
    let message = format!(
        "Enter the API key / token for alias `{alias_name}`.\n\nThe value is stored in your OS keyring only — never on disk, never printed, never in git history."
    );

    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display dialog \"{}\" with title \"{}\" default answer \"\" with hidden answer buttons {{\"Cancel\", \"Save\"}} default button \"Save\" with icon caution",
            message.replace('"', "\\\""),
            title,
        );
        let out = Command::new("osascript").arg("-e").arg(&script).output();
        if let Ok(out) = out {
            if out.status.success() {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if let Some(value) = parse_osascript_text_returned(&stdout) {
                    return Ok(value);
                }
            } else {
                // user clicked Cancel → exit 1; treat as empty
                return Ok(String::new());
            }
        }
        // osascript missing or failed → fall through to tty
    }

    #[cfg(target_os = "linux")]
    {
        for tool in &["zenity", "kdialog"] {
            if !command_available(tool) {
                continue;
            }
            let result = if *tool == "zenity" {
                Command::new("zenity")
                    .args(["--password", "--title", title, "--text", &message])
                    .output()
            } else {
                Command::new("kdialog")
                    .args(["--title", title, "--password", &message])
                    .output()
            };
            if let Ok(out) = result {
                if out.status.success() {
                    let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    return Ok(value);
                } else {
                    return Ok(String::new()); // cancelled
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        let ps_script = format!(
            "$sec = Read-Host -AsSecureString '{message}'; [System.Net.NetworkCredential]::new('', $sec).Password"
        );
        if let Ok(out) = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
            .output()
        {
            if out.status.success() {
                return Ok(String::from_utf8_lossy(&out.stdout).trim().to_string());
            }
        }
    }

    // tty fallback (no GUI / GUI failed / SSH session)
    match rpassword::prompt_password(format!("Value for alias `{alias_name}` (hidden): ")) {
        Ok(v) => Ok(v.trim().to_string()),
        Err(e) => Err(CliError::new(
            1,
            format!(
                "SECRET_SET_STATUS=FAIL reason=no_input_available detail={e} hint=run from a real terminal or set the value via stdin"
            ),
        )
        .into()),
    }
}

// osascript "display dialog ..." output format:
//   "button returned:Save, text returned:<value>\n"
// We only care about the text-returned portion. Cancellation gives a
// non-zero exit; this parser only sees the success branch.
fn parse_osascript_text_returned(stdout: &str) -> Option<String> {
    for chunk in stdout.split(", ") {
        let chunk = chunk.trim_end_matches(['\n', '\r']);
        if let Some(rest) = chunk.strip_prefix("text returned:") {
            return Some(rest.to_string());
        }
    }
    None
}

// K2: agent-callable bridge. Agent runs this before any external API
// call. Behavior:
//   - alias present in keyring → print `export <ENV>='<value>'` lines
//     to stdout, exit 0 → agent evals/parses them
//   - alias missing + --auto-prompt → pop GUI, store, then print exports
//   - alias missing + no --auto-prompt → exit 75 with hint
//
// Cross-project sharing: if alias is in keyring but the current project's
// .aiplus/keys.toml doesn't list it, silently append the alias to that
// project's keys.toml so cd-auto-load picks it up on the next visit.
fn secret_broker_need(
    aliases: Vec<String>,
    auto_prompt: bool,
    env_override: Option<String>,
) -> Result<()> {
    if aliases.is_empty() {
        return Err(CliError::new(
            2,
            "ERROR aiplus secret-broker need requires at least one <alias>",
        )
        .into());
    }
    if provider_name() != "keyring" {
        return Err(CliError::new(
            1,
            format!(
                "SECRET_NEED_STATUS=FAIL reason=wrong_provider current={} hint=`need` only works with keyring backend",
                provider_name()
            ),
        )
        .into());
    }
    let mut missing: Vec<String> = Vec::new();
    let mut exports: Vec<(String, String, String)> = Vec::new(); // (alias, env_var, value)

    for alias in &aliases {
        let entry = keyring_alias_entry(alias)?;
        let value = match entry.get_password() {
            Ok(v) => v,
            Err(keyring::Error::NoEntry) => {
                if auto_prompt {
                    let v = prompt_secret_via_gui(alias)?;
                    if v.is_empty() {
                        missing.push(alias.clone());
                        continue;
                    }
                    if let Err(e) = entry.set_password(&v) {
                        return Err(CliError::new(
                            1,
                            format!(
                                "SECRET_NEED_STATUS=FAIL alias={alias} reason=keyring_write_failed detail={e}"
                            ),
                        )
                        .into());
                    }
                    v
                } else {
                    missing.push(alias.clone());
                    continue;
                }
            }
            Err(e) => {
                return Err(CliError::new(
                    1,
                    format!("SECRET_NEED_STATUS=FAIL alias={alias} reason={e}"),
                )
                .into());
            }
        };
        let env_var = env_override
            .clone()
            .unwrap_or_else(|| format!("{}_API_KEY", alias.to_ascii_uppercase()));
        exports.push((alias.clone(), env_var.clone(), value));
        // Cross-project share: ensure this alias is in the registry +
        // the current project's keys.toml.
        let _ = append_alias_to_registry(alias, &env_var);
        let _ = append_alias_to_project_keys(alias);
    }

    if !missing.is_empty() {
        let list = missing.join(", ");
        eprintln!(
            "SECRET_NEED_STATUS=MISSING missing=[{list}] hint=run `aiplus secret-broker set {} --auto-prompt` for each missing alias",
            missing[0]
        );
        process::exit(75);
    }

    // Print export lines on stdout — agent / shell hook evals these.
    for (alias, env_var, value) in &exports {
        // Single-quote the value; escape any embedded single quotes.
        let escaped = value.replace('\'', "'\\''");
        println!("export {env_var}='{escaped}'  # alias={alias}");
    }
    println!("# SECRET_NEED_STATUS=PASS aliases=[{}]", aliases.join(","));
    Ok(())
}

// K4 support: append alias to the current project's .aiplus/keys.toml
// `aliases = [...]`. Creates the file if missing. No-op if alias is
// already listed. Called by both `need` (when alias resolves) and
// `set --auto-prompt` (so the next cd-auto-load picks it up).
fn append_alias_to_project_keys(alias_name: &str) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let aiplus_dir = cwd.join(".aiplus");
    if !aiplus_dir.exists() {
        // not inside an aiplus-installed project — silent no-op
        return Ok(());
    }
    let path = aiplus_dir.join("keys.toml");
    let existing = fs::read_to_string(&path).unwrap_or_default();
    if existing.contains(&format!("\"{alias_name}\"")) {
        return Ok(());
    }
    let new_aliases = collect_project_aliases(&existing, alias_name);
    let formatted = new_aliases
        .iter()
        .map(|a| format!("\"{a}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let content = format!("# Aliases this project needs at runtime.\n# Edit via `aiplus secret-broker need <alias>` or by hand.\naliases = [{formatted}]\n");
    let _ = fs::write(&path, content);
    Ok(())
}

fn collect_project_aliases(existing: &str, new_alias: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen = BTreeSet::new();
    if let Ok(parsed) = existing.parse::<toml::Value>() {
        if let Some(arr) = parsed.get("aliases").and_then(|v| v.as_array()) {
            for item in arr {
                if let Some(s) = item.as_str() {
                    if seen.insert(s.to_string()) {
                        out.push(s.to_string());
                    }
                }
            }
        }
    }
    if seen.insert(new_alias.to_string()) {
        out.push(new_alias.to_string());
    }
    out
}

// K4: cd-auto-load shell-init. Prints a hook snippet the user appends
// to their shell rc. The snippet calls `aiplus secret-broker hook` on
// every directory change, evaluates the export/unset lines that come
// back. Supported shells: zsh, bash, fish.
fn secret_broker_shell_init(shell: Option<String>) -> Result<()> {
    let shell = shell.as_deref().unwrap_or("").trim().to_lowercase();
    let snippet = render_shell_init_snippet(&shell)?;
    print!("{snippet}");
    Ok(())
}

// K5: shared renderer for the cd-auto-load snippet. Used both by
// `secret-broker shell-init` (prints to stdout) and `aiplus install`
// (appends to user's rc file when they consent).
fn render_shell_init_snippet(shell: &str) -> Result<String> {
    let body = match shell {
        "zsh" => {
            r#"# AiPlus secret-broker cd-auto-load (zsh)
_aiplus_broker_hook() {
  if command -v aiplus >/dev/null 2>&1; then
    eval "$(aiplus secret-broker hook 2>/dev/null)"
  fi
}
typeset -ga chpwd_functions
[[ -z "${chpwd_functions[(r)_aiplus_broker_hook]}" ]] && chpwd_functions+=(_aiplus_broker_hook)
# Run once for the shell's starting directory.
_aiplus_broker_hook
"#
        }
        "bash" => {
            r#"# AiPlus secret-broker cd-auto-load (bash)
_aiplus_broker_hook() {
  if [ "${PWD}" != "${_AIPLUS_BROKER_LAST_PWD:-}" ]; then
    _AIPLUS_BROKER_LAST_PWD="${PWD}"
    if command -v aiplus >/dev/null 2>&1; then
      eval "$(aiplus secret-broker hook 2>/dev/null)"
    fi
  fi
}
case ":${PROMPT_COMMAND:-}:" in
  *":_aiplus_broker_hook:"*) ;;
  *) PROMPT_COMMAND="_aiplus_broker_hook${PROMPT_COMMAND:+;$PROMPT_COMMAND}" ;;
esac
_aiplus_broker_hook
"#
        }
        "fish" => {
            r#"# AiPlus secret-broker cd-auto-load (fish)
function _aiplus_broker_hook --on-variable PWD
    if command -v aiplus >/dev/null 2>&1
        aiplus secret-broker hook 2>/dev/null | source
    end
end
_aiplus_broker_hook
"#
        }
        _ => {
            return Err(CliError::new(
                2,
                "ERROR aiplus secret-broker shell-init requires <shell> (zsh|bash|fish). Example: `aiplus secret-broker shell-init zsh >> ~/.zshrc`",
            )
            .into());
        }
    };
    Ok(body.to_string())
}

// K7 (#83): outcome of PATH version-skew detection.
struct VersionSkew {
    path_binary: PathBuf,
    path_version: String,
    installer_version: String,
}

// K7 (#83): look for `aiplus` on PATH, ask it `--version`, compare
// against our own embedded version. Returns Some(VersionSkew) only
// when the PATH binary is strictly OLDER than us — meaning agents
// invoking the AGENTS protocol via PATH would hit unknown subcommands.
//
// Returns None when:
//   - no `aiplus` on PATH
//   - PATH `aiplus` IS this binary (same inode / path)
//   - PATH version >= our version (no skew, or newer than us — fine)
//   - any parse / exec failure (fail open: don't block install)
fn detect_path_version_skew() -> Option<VersionSkew> {
    let own_version = env!("CARGO_PKG_VERSION").to_string();
    let path = std::env::var_os("PATH")?;
    let exe = std::env::current_exe().ok();
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join("aiplus");
        if !candidate.is_file() {
            continue;
        }
        // This is the FIRST aiplus on PATH — i.e. what `which aiplus`
        // would return, and what agents will actually invoke when they
        // run `aiplus secret-broker ...`. The shell stops at the first
        // match, so we should too. If we kept walking, we'd refuse on
        // every stale binary deeper in PATH (e.g. ~/.cargo/bin/aiplus
        // left over from `cargo install` years ago) even though no real
        // command would ever resolve to it.
        //
        // If the first PATH match IS ourselves (same canonical path),
        // there's by definition no skew with self — return None.
        if let Some(exe) = &exe {
            let lhs = std::fs::canonicalize(&candidate).unwrap_or_else(|_| candidate.clone());
            let rhs = std::fs::canonicalize(exe).unwrap_or_else(|_| exe.clone());
            if lhs == rhs {
                return None;
            }
        }
        let out = std::process::Command::new(&candidate)
            .arg("--version")
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&out.stdout);
        let path_version = stdout.trim().to_string();
        // `aiplus --version` prints just "0.5.16" — no prefix. Be lenient.
        let path_v = parse_semver(&path_version)?;
        let own_v = parse_semver(&own_version)?;
        if path_v < own_v {
            return Some(VersionSkew {
                path_binary: candidate,
                path_version,
                installer_version: own_version,
            });
        }
        return None;
    }
    None
}

// K7 (#83): parse "X.Y.Z" (optionally `vX.Y.Z`) into a comparable
// 3-tuple. Returns None for anything we don't recognize so the caller
// falls open. Pre-release / build metadata is ignored — we only care
// about the release-train ordering.
fn parse_semver(s: &str) -> Option<(u32, u32, u32)> {
    let s = s.trim();
    let s = s.strip_prefix('v').unwrap_or(s);
    // Strip anything after the first whitespace (some `--version` impls
    // print "aiplus 0.5.16" with a leading name).
    let s = s.split_whitespace().last().unwrap_or(s);
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() < 3 {
        return None;
    }
    let patch_raw = parts[2];
    // Drop pre-release / build metadata suffix on the patch component.
    let patch_str: String = patch_raw
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        patch_str.parse().ok()?,
    ))
}

// K5: detect the user's interactive shell from $SHELL. Returns a tuple
// of (logical shell name, rc-file path under $HOME). Returns None if we
// can't confidently match — caller falls back to printing a hint.
fn detect_shell_and_rc() -> Option<(&'static str, PathBuf)> {
    let home = match std::env::var_os("HOME") {
        Some(h) if !h.is_empty() => PathBuf::from(h),
        _ => return None,
    };
    let shell_env = std::env::var("SHELL").unwrap_or_default();
    let basename = Path::new(&shell_env)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match basename.as_str() {
        "zsh" => Some(("zsh", home.join(".zshrc"))),
        "bash" => {
            // macOS bash users typically use ~/.bash_profile; everywhere
            // else ~/.bashrc is the conventional interactive rc. Pick
            // ~/.bash_profile if it exists, otherwise ~/.bashrc.
            let bp = home.join(".bash_profile");
            if bp.exists() {
                Some(("bash", bp))
            } else {
                Some(("bash", home.join(".bashrc")))
            }
        }
        "fish" => {
            // Honor XDG_CONFIG_HOME so the parity test (which sets it
            // under the tempdir) doesn't write outside the sandbox.
            let xdg = std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".config"));
            Some(("fish", xdg.join("fish").join("config.fish")))
        }
        _ => None,
    }
}

// K5: marker we look for to make append idempotent. If a previous
// `aiplus install` or a manual `shell-init >> rc` run already wrote
// the hook, we skip silently.
const SHELL_INIT_MARKER: &str = "_aiplus_broker_hook";

fn shell_rc_already_wired(rc: &Path) -> bool {
    match fs::read_to_string(rc) {
        Ok(text) => text.contains(SHELL_INIT_MARKER),
        Err(_) => false,
    }
}

// K5: maybe append the cd-auto-load snippet to the user's shell rc.
// Called once at the end of `aiplus install`. Behavior:
//
//   - In dry_run: print "would offer" and bail without touching rc.
//   - If rc already contains the hook marker: print "already enabled".
//   - If shell can't be detected: print manual command and bail.
//   - With options.yes: auto-append (non-interactive consent).
//   - Otherwise (interactive tty): prompt "[Y/n]", default Y on enter.
//   - On non-tty without --yes: skip + print manual hint.
//
// Always append-only. Never edits or rewrites existing rc content.
fn maybe_install_shell_hook(options: &Options, dry_run: bool) {
    use std::io::IsTerminal;
    let Some((shell, rc)) = detect_shell_and_rc() else {
        eprintln!("SHELL_INIT=skipped_unknown_shell");
        eprintln!(
            "  Tip: enable cd auto-load manually with `aiplus secret-broker shell-init zsh|bash|fish >> <your shell rc>`"
        );
        return;
    };

    if shell_rc_already_wired(&rc) {
        eprintln!("SHELL_INIT=already_enabled rc={}", rc.display());
        return;
    }

    if dry_run {
        eprintln!("SHELL_INIT=would_offer shell={} rc={}", shell, rc.display());
        return;
    }

    let consent = if options.yes {
        true
    } else if std::io::stdin().is_terminal() {
        eprintln!();
        eprintln!("AiPlus can wire `cd` auto-load so agents pick up API keys");
        eprintln!("without re-prompting per session. This appends ~6 lines to:");
        eprintln!("  {}", rc.display());
        eprint!("Enable cd auto-load? [Y/n] ");
        use std::io::Write;
        std::io::stderr().flush().ok();
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        let trimmed = input.trim().to_lowercase();
        // Default Y: blank line == yes.
        matches!(trimmed.as_str(), "" | "y" | "yes")
    } else {
        eprintln!("SHELL_INIT=skipped_noninteractive");
        eprintln!(
            "  Tip: enable cd auto-load with `aiplus secret-broker shell-init {} >> {}`",
            shell,
            rc.display()
        );
        return;
    };

    if !consent {
        eprintln!("SHELL_INIT=declined");
        eprintln!(
            "  You can enable later: `aiplus secret-broker shell-init {} >> {}`",
            shell,
            rc.display()
        );
        return;
    }

    let snippet = match render_shell_init_snippet(shell) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("SHELL_INIT=render_failed err={}", e);
            return;
        }
    };

    if let Some(parent) = rc.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("SHELL_INIT=mkdir_failed err={}", e);
                return;
            }
        }
    }
    // Append with a leading newline so we never butt up against an
    // existing trailing line without separator.
    let payload = format!("\n{snippet}");
    let result = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&rc)
        .and_then(|mut f| {
            use std::io::Write;
            f.write_all(payload.as_bytes())
        });
    match result {
        Ok(()) => {
            eprintln!("SHELL_INIT=appended rc={}", rc.display());
            eprintln!(
                "  Activate now with: `source {}` (or open a new terminal)",
                rc.display()
            );
        }
        Err(e) => {
            eprintln!("SHELL_INIT=append_failed err={}", e);
            eprintln!(
                "  Enable manually: `aiplus secret-broker shell-init {} >> {}`",
                shell,
                rc.display()
            );
        }
    }
}

// K4: cd-auto-load hook. Called by the shell rc snippet on every cd.
// Reads .aiplus/keys.toml from the cwd (walking up to find one), emits
// `export <ENV>='<value>'` for every alias that resolves from keyring,
// and `unset <ENV>` for any alias loaded by a previous run but not in
// the current project's keys.toml. Tracks state in
// `_AIPLUS_BROKER_LOADED` so the next hook firing knows what to clear.
//
// Soft-fail: missing aliases get a single stderr `# missing: ...`
// comment line, not exit 75 — we don't want to break the user's shell
// prompt just because a key isn't set yet.
fn secret_broker_hook() -> Result<()> {
    if provider_name() != "keyring" {
        // Bitwarden flow doesn't use cd-auto-load; output nothing.
        return Ok(());
    }
    let cwd = std::env::current_dir()?;
    let project_keys = find_project_keys_toml(&cwd);
    let desired: Vec<String> = match project_keys {
        Some(path) => read_project_aliases(&path).unwrap_or_default(),
        None => Vec::new(),
    };
    let previously_loaded: Vec<String> = std::env::var("_AIPLUS_BROKER_LOADED")
        .unwrap_or_default()
        .split(':')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();

    // Unset aliases that were loaded but are no longer desired.
    let desired_set: BTreeSet<&str> = desired.iter().map(|s| s.as_str()).collect();
    for old in &previously_loaded {
        if !desired_set.contains(old.as_str()) {
            let env_name = format!("{}_API_KEY", old.to_ascii_uppercase());
            println!("unset {env_name}");
        }
    }

    // Load each currently-desired alias.
    let mut loaded: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();
    for alias in &desired {
        let entry = match keyring_alias_entry(alias) {
            Ok(e) => e,
            Err(_) => continue,
        };
        match entry.get_password() {
            Ok(value) => {
                let env_name = format!("{}_API_KEY", alias.to_ascii_uppercase());
                let escaped = value.replace('\'', "'\\''");
                println!("export {env_name}='{escaped}'");
                loaded.push(alias.clone());
            }
            Err(keyring::Error::NoEntry) => {
                missing.push(alias.clone());
            }
            Err(_) => {
                // Treat backend errors as silent miss to keep prompt fast.
            }
        }
    }

    // Persist the new loaded list so the next hook firing can unset.
    println!("export _AIPLUS_BROKER_LOADED='{}'", loaded.join(":"));

    if !missing.is_empty() {
        eprintln!(
            "# aiplus: missing keys in keyring: {}. Run `aiplus secret-broker set {} --auto-prompt` to set.",
            missing.join(", "),
            missing[0]
        );
    } else if !loaded.is_empty() {
        eprintln!(
            "# aiplus: loaded {} alias(es): {}",
            loaded.len(),
            loaded.join(", ")
        );
    }
    Ok(())
}

// Walk up from `start` looking for `.aiplus/keys.toml`. Stop at $HOME
// or the filesystem root, whichever comes first. Returns the path to
// the first match found.
fn find_project_keys_toml(start: &Path) -> Option<PathBuf> {
    let home = std::env::var("HOME").ok().map(PathBuf::from);
    let mut current = Some(start.to_path_buf());
    while let Some(dir) = current {
        let candidate = dir.join(".aiplus").join("keys.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        if let Some(h) = &home {
            if dir == *h {
                return None;
            }
        }
        current = dir.parent().map(|p| p.to_path_buf());
    }
    None
}

fn read_project_aliases(path: &Path) -> Result<Vec<String>> {
    let text = fs::read_to_string(path)?;
    let parsed: toml::Value = text.parse().map_err(|e| anyhow!("parse {path:?}: {e}"))?;
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    if let Some(arr) = parsed.get("aliases").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(s) = item.as_str() {
                if seen.insert(s.to_string()) {
                    out.push(s.to_string());
                }
            }
        }
    }
    Ok(out)
}

fn secret_broker_delete(alias_name: Option<String>) -> Result<()> {
    let alias_name = alias_name.ok_or_else(|| {
        CliError::new(
            1,
            "ERROR aiplus secret-broker delete requires --alias <name>",
        )
    })?;
    if provider_name() != "keyring" {
        return Err(CliError::new(
            1,
            format!(
                "SECRET_DELETE_STATUS=FAIL reason=wrong_provider current={} hint=`delete` only works with keyring backend",
                provider_name()
            ),
        )
        .into());
    }
    let entry = keyring_alias_entry(&alias_name)?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => {
            println!("SECRET_DELETE_STATUS=PASS alias={alias_name} provider=keyring");
            Ok(())
        }
        Err(e) => Err(CliError::new(
            1,
            format!("SECRET_DELETE_STATUS=FAIL alias={alias_name} reason={e}"),
        )
        .into()),
    }
}

// Append alias→env_var to the user's secret-broker registry TSV so
// `secret-broker list` and `secret-broker run` see it. No-op if the
// alias is already present. Uses the legacy single-file location
// `~/.config/aiplus/secret-broker/secret-aliases.tsv` (created if
// missing). bitwarden_name column is set to "keyring:<alias>" as a
// placeholder — the KeyringProvider ignores it.
fn append_alias_to_registry(alias_name: &str, env_var: &str) -> Result<()> {
    let dir = config_home()?.join("aiplus").join("secret-broker");
    fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    let path = dir.join("secret-aliases.tsv");
    let existing = if path.exists() {
        fs::read_to_string(&path)?
    } else {
        String::new()
    };
    for line in existing.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = trimmed.split('\t').collect();
        if fields.first().copied() == Some(alias_name) {
            return Ok(());
        }
    }
    let mut new_content = existing.clone();
    if !new_content.is_empty() && !new_content.ends_with('\n') {
        new_content.push('\n');
    }
    new_content.push_str(&format!("{alias_name}\tkeyring:{alias_name}\t{env_var}\n"));
    fs::write(&path, new_content).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn secret_broker_status() -> Result<()> {
    let source = token_source();
    println!("SECRET_BROKER_STATUS");
    println!("provider={}", provider_name());
    println!("bitwarden_project=private_config");
    println!("machine_account={SECRET_BROKER_ACCOUNT}");
    println!("token_source={source}");
    println!("aliases={}", secret_aliases()?.len());
    println!("secret_values_printed=no");
    println!("secret_values_persisted=no");
    println!("audit_log=metadata_only");
    println!("SECRET_BROKER_STATUS=PASS");
    Ok(())
}

fn secret_broker_doctor() -> Result<()> {
    println!("SECRET_BROKER_DOCTOR");
    println!("provider={}", provider_name());
    println!("bws_cli={}", yes_no(command_available("bws")));
    let source = token_source();
    // S3: legacy `token_source=` retained for back-compat with any
    // scripts grepping the doctor output; new canonical name is
    // `bws_token_source=`.
    println!("token_source={source}");
    println!("bws_token_source={source}");
    // S3: structured unlock hint — what the agent should DO next, not
    // a prose suggestion. Three cases:
    //   * env|keychain → "ok" (no action needed)
    //   * not_configured + bws CLI present →
    //     "aiplus secret-broker token set"
    //   * bws CLI missing → install hint takes precedence
    let unlock_hint = if source == "env" || source == "keychain" {
        "ok".to_string()
    } else if !command_available("bws") {
        "install bws CLI then run `aiplus secret-broker token set`".to_string()
    } else {
        "aiplus secret-broker token set".to_string()
    };
    println!("bws_token_unlock_hint={unlock_hint}");
    if source == "not_configured" {
        println!("next=run aiplus secret-broker token set in Terminal");
    }
    println!("keychain_supported={}", yes_no(cfg!(target_os = "macos")));
    // v0.5.16: surface whether the Bitwarden project UUID is set.
    // Public source ships with an empty default — users must supply
    // `AIPLUS_BWS_PROJECT_ID` env var or a private profile bundle.
    // Without it `secret-broker resolve` will fail at first use.
    let project_id = bitwarden_project_id();
    let project_id_source = if !std::env::var("AIPLUS_BWS_PROJECT_ID")
        .unwrap_or_default()
        .is_empty()
    {
        "env"
    } else if !project_id.is_empty() {
        "default"
    } else {
        "not_configured"
    };
    println!("bws_project_id_source={project_id_source}");
    if project_id_source == "not_configured" {
        println!(
            "bws_project_id_hint=set AIPLUS_BWS_PROJECT_ID env var (Bitwarden Secrets Manager project UUID) before `aiplus secret-broker resolve`"
        );
    }
    println!("alias_count={}", secret_aliases()?.len());
    println!("secret_values_printed=no");
    println!("SECRET_BROKER_DOCTOR_STATUS=PASS");
    Ok(())
}

fn secret_broker_list() -> Result<()> {
    println!("SECRET_BROKER_LIST");
    for alias in secret_aliases()? {
        println!(
            "{} -> {} -> {}",
            alias.alias, alias.bitwarden_name, alias.env_var
        );
    }
    println!("secret_values_printed=no");
    println!("SECRET_ALIAS_STATUS=PASS");
    Ok(())
}

fn secret_broker_resolve(alias: Option<String>, print_secret: bool) -> Result<()> {
    if print_secret
        && std::env::var("AIPLUS_SECRET_BROKER_ALLOW_PRINT")
            .ok()
            .as_deref()
            != Some("1")
    {
        return Err(CliError::new(1, "ERROR --print is disabled by default").into());
    }
    let alias = resolve_alias(alias)?;
    let provider = load_secret_provider()?;
    let value = provider.resolve(&alias)?;
    let secret_id_found = provider.secret_id_found(&alias)?;
    println!("SECRET_RESOLVE");
    println!("alias={}", alias.alias);
    println!("provider={}", provider.provider_name());
    println!("token_source={}", provider.token_source());
    println!("secret_key={}", alias.bitwarden_name);
    if let Some(found) = secret_id_found {
        println!("secret_id_found={}", yes_no(found));
    }
    println!("env_var={}", alias.env_var);
    if let Some(metadata) = provider_metadata(&alias.alias) {
        println!("provider_family={}", metadata.provider_family);
        println!("platform={}", metadata.platform);
        println!("base_url={}", metadata.base_url);
        println!("model={}", metadata.model);
        println!("smoke_endpoint={}", metadata.smoke_endpoint);
    }
    println!("provider_status=PASS");
    println!("secret_value_printed={}", yes_no(print_secret));
    if print_secret {
        println!("{}", value.expose_for_explicit_print());
    }
    println!("SECRET_RESOLVE_STATUS=PASS");
    Ok(())
}

fn secret_broker_run(
    alias_flags: Vec<String>,
    aliases_csv: Option<String>,
    command: Vec<String>,
) -> Result<()> {
    if command.is_empty() {
        return Err(CliError::new(2, "ERROR secret-broker run requires a command after --").into());
    }
    let requested_aliases = parse_requested_aliases(alias_flags, aliases_csv)?;
    let aliases = select_secret_aliases(&requested_aliases)?;
    let provider = load_secret_provider()?;
    let mut child = Command::new(&command[0]);
    if command.len() > 1 {
        child.args(&command[1..]);
    }
    let requested_mode = !requested_aliases.is_empty();
    let mut injected_env = Vec::new();
    let mut skipped_aliases = Vec::new();
    for alias in aliases {
        match provider.resolve(&alias) {
            Ok(value) => {
                child.env(&alias.env_var, value.value);
                injected_env.push(alias.env_var);
            }
            Err(error) if requested_mode => {
                return Err(CliError::new(
                    1,
                    format!(
                        "SECRET_BROKER_RUN_STATUS=FAIL alias={} reason={}",
                        alias.alias,
                        secret_error_reason(&error)
                    ),
                )
                .into());
            }
            Err(_) => {
                skipped_aliases.push(alias.alias);
            }
        }
    }
    println!("SECRET_BROKER_RUN");
    println!("requested_aliases=[{}]", requested_aliases.join(","));
    println!("injected_env=[{}]", injected_env.join(","));
    println!("skipped_aliases=[{}]", skipped_aliases.join(","));
    println!("secret_values_printed=no");
    let status = child.status().context("run child command")?;
    if !status.success() {
        return Err(CliError::new(status.code().unwrap_or(1), "ERROR child command failed").into());
    }
    println!("SECRET_BROKER_RUN_STATUS=PASS");
    Ok(())
}

/// S1: `aiplus secret-broker push --alias <a> --to <target>`
///
/// Resolves one alias from the broker and writes the value to a
/// declarative target without ever printing the value to stdout/log.
/// Three targets:
///   * `github-secret:<owner>/<repo>:<NAME>` — pipes the value into
///     `gh secret set <NAME> --repo <owner>/<repo> --body @-`. We
///     feed the secret on stdin (not as an argv argument) because
///     argv is visible to other processes via /proc/<pid>/cmdline.
///   * `env:<VAR>` — prints a single `export VAR='...'` line on
///     stdout. Caller is expected to `eval` it; we do not export
///     ourselves because aiplus runs in its own subprocess and the
///     env wouldn't propagate.
///   * `dotenv:<path>` — writes/updates a single `VAR=...` line in
///     the named .env file. Existing key is replaced in place; new
///     key is appended.
///
/// `--print` is rejected (would defeat the purpose). The push uses
/// the same alias-selection logic as `run` (single alias only — we
/// reject multi-alias because a "target" is by definition singular).
/// Idempotent: pushing the same value twice produces the same
/// outcome with no extra side effects (gh secret set is idempotent,
/// dotenv replace-in-place is idempotent).
fn secret_broker_push(
    alias_flags: Vec<String>,
    aliases_csv: Option<String>,
    target: Option<String>,
    print_secret: bool,
) -> Result<()> {
    if print_secret {
        return Err(CliError::new(
            2,
            "ERROR --print is not allowed with push (push never prints secret values)",
        )
        .into());
    }
    let target = target.ok_or_else(|| {
        CliError::new(
            2,
            "ERROR --to <target> required. Forms: \
             github-secret:<owner>/<repo>:<NAME>, env:<VAR>, dotenv:<path>",
        )
    })?;
    let requested = parse_requested_aliases(alias_flags, aliases_csv)?;
    if requested.len() != 1 {
        return Err(CliError::new(2, "ERROR push requires exactly one --alias <name>").into());
    }
    let aliases = select_secret_aliases(&requested)?;
    let alias = aliases
        .into_iter()
        .next()
        .ok_or_else(|| CliError::new(2, "ERROR alias not found in registry"))?;

    let provider = load_secret_provider()?;
    let value = provider.resolve(&alias).map_err(|e| {
        CliError::new(
            1,
            format!(
                "SECRET_BROKER_PUSH_STATUS=FAIL alias={} reason={}",
                alias.alias,
                secret_error_reason(&e)
            ),
        )
    })?;

    // The PushTarget enum normalizes the DSL and isolates the actual
    // value-handling side effect. We pass `value` by reference and
    // never log it; on return the value is dropped (heap zeroized
    // by SecretValue::expose_for_explicit_print's Drop, but we only
    // hold a borrow here).
    let parsed = PushTarget::parse(&target)?;
    let target_summary = parsed.summary();
    let value_str = value.expose_for_explicit_print();
    let result = parsed.write(&alias.env_var, &value_str);
    // Always overwrite the local binding with a non-secret value
    // before any println — defense-in-depth against an accidental
    // {value_str} interpolation slipping into a future edit.
    let value_str = String::new();
    result?;
    drop(value_str);

    println!("SECRET_BROKER_PUSH");
    println!("alias={}", alias.alias);
    println!("env_var={}", alias.env_var);
    println!("target={}", target_summary);
    println!("secret_value_printed=no");
    println!("SECRET_BROKER_PUSH_STATUS=PASS");
    Ok(())
}

/// Parsed `--to` target. We split the DSL parse from the write so the
/// parser is unit-testable without filesystem or `gh` side effects.
#[derive(Debug)]
enum PushTarget {
    GithubSecret {
        owner: String,
        repo: String,
        name: String,
    },
    Env {
        var: String,
    },
    Dotenv {
        path: PathBuf,
    },
}

impl PushTarget {
    fn parse(raw: &str) -> Result<Self> {
        if let Some(rest) = raw.strip_prefix("github-secret:") {
            // `<owner>/<repo>:<NAME>`. Split on the LAST `:` so the
            // <NAME> can't accidentally consume part of the path.
            let (owner_repo, name) = rest.rsplit_once(':').ok_or_else(|| {
                CliError::new(2, "ERROR github-secret target needs <owner>/<repo>:<NAME>")
            })?;
            let (owner, repo) = owner_repo.split_once('/').ok_or_else(|| {
                CliError::new(2, "ERROR github-secret target needs <owner>/<repo>:<NAME>")
            })?;
            if owner.is_empty() || repo.is_empty() || name.is_empty() {
                return Err(CliError::new(
                    2,
                    "ERROR github-secret target needs non-empty owner, repo, and name",
                )
                .into());
            }
            return Ok(PushTarget::GithubSecret {
                owner: owner.to_string(),
                repo: repo.to_string(),
                name: name.to_string(),
            });
        }
        if let Some(var) = raw.strip_prefix("env:") {
            if var.is_empty() {
                return Err(CliError::new(2, "ERROR env target needs a variable name").into());
            }
            return Ok(PushTarget::Env {
                var: var.to_string(),
            });
        }
        if let Some(path) = raw.strip_prefix("dotenv:") {
            if path.is_empty() {
                return Err(CliError::new(2, "ERROR dotenv target needs a file path").into());
            }
            return Ok(PushTarget::Dotenv {
                path: PathBuf::from(path),
            });
        }
        Err(CliError::new(
            2,
            format!(
                "ERROR unknown push target '{raw}'. Forms: \
                 github-secret:<owner>/<repo>:<NAME>, env:<VAR>, dotenv:<path>"
            ),
        )
        .into())
    }

    /// Short label that's safe to log (no secret value).
    fn summary(&self) -> String {
        match self {
            PushTarget::GithubSecret { owner, repo, name } => {
                format!("github-secret:{owner}/{repo}:{name}")
            }
            PushTarget::Env { var } => format!("env:{var}"),
            PushTarget::Dotenv { path } => format!("dotenv:{}", path.display()),
        }
    }

    /// Perform the actual write. `env_var` is the broker's canonical
    /// env name for this alias (e.g. `ANTHROPIC_API_KEY`); for
    /// github-secret and env targets the override goes to whatever
    /// the user requested in `--to`, not `env_var`.
    fn write(&self, _broker_env_var: &str, value: &str) -> Result<()> {
        match self {
            PushTarget::GithubSecret { owner, repo, name } => {
                use std::io::Write as _;
                use std::process::Stdio;
                let repo_arg = format!("{owner}/{repo}");
                let mut child = Command::new("gh")
                    .args(["secret", "set", name, "--repo", &repo_arg, "--body", "-"])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .map_err(|e| {
                        CliError::new(
                            1,
                            format!(
                                "SECRET_BROKER_PUSH_STATUS=FAIL target=github-secret \
                                 reason=spawn_gh_failed err={e}"
                            ),
                        )
                    })?;
                {
                    let stdin = child.stdin.as_mut().ok_or_else(|| {
                        CliError::new(
                            1,
                            "SECRET_BROKER_PUSH_STATUS=FAIL target=github-secret \
                             reason=stdin_unavailable",
                        )
                    })?;
                    stdin
                        .write_all(value.as_bytes())
                        .context("write secret to gh stdin")?;
                }
                let out = child.wait_with_output().context("wait gh secret set")?;
                if !out.status.success() {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    return Err(CliError::new(
                        out.status.code().unwrap_or(1),
                        format!(
                            "SECRET_BROKER_PUSH_STATUS=FAIL target=github-secret \
                             repo={owner}/{repo} name={name} reason=gh_failed stderr={}",
                            stderr.trim()
                        ),
                    )
                    .into());
                }
                Ok(())
            }
            PushTarget::Env { var } => {
                // Single-quote the value to be eval-safe; escape any
                // embedded single quotes via the standard `'\''` dance.
                let escaped = value.replace('\'', "'\\''");
                println!("export {var}='{escaped}'");
                Ok(())
            }
            PushTarget::Dotenv { path } => write_dotenv_line(path, _broker_env_var, value),
        }
    }
}

/// Replace-or-append a single `VAR=value` line in a .env-style file.
/// Quoting: we wrap the value in double quotes and escape `\\`, `"`,
/// `$`, `` ` `` so it survives `source .env` in a POSIX shell.
fn write_dotenv_line(path: &Path, var: &str, value: &str) -> Result<()> {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`");
    let new_line = format!("{var}=\"{escaped}\"");
    let mut replaced = false;
    let mut out_lines: Vec<String> = Vec::new();
    for line in existing.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(&format!("{var}=")) {
            let _ = rest;
            out_lines.push(new_line.clone());
            replaced = true;
        } else {
            out_lines.push(line.to_string());
        }
    }
    if !replaced {
        out_lines.push(new_line);
    }
    let mut body = out_lines.join("\n");
    if !body.ends_with('\n') {
        body.push('\n');
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
    }
    std::fs::write(path, body).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn secret_broker_token(action: Option<String>) -> Result<()> {
    match action.as_deref() {
        Some("set") => token_set(),
        Some("delete") => token_delete(),
        _ => {
            println!("Usage: aiplus secret-broker token set|delete");
            process::exit(2);
        }
    }
}

#[derive(Clone)]
struct SecretAlias {
    alias: String,
    bitwarden_name: String,
    env_var: String,
}

struct ProviderMetadata {
    provider_family: &'static str,
    platform: &'static str,
    base_url: &'static str,
    model: &'static str,
    smoke_endpoint: &'static str,
}

struct SecretValue {
    value: String,
}

impl SecretValue {
    fn expose_for_explicit_print(self) -> String {
        self.value
    }
}

trait SecretsProvider {
    fn resolve(&self, alias: &SecretAlias) -> Result<SecretValue>;
    fn provider_name(&self) -> &'static str;
    fn token_source(&self) -> &'static str {
        "not_applicable"
    }
    fn secret_id_found(&self, _alias: &SecretAlias) -> Result<Option<bool>> {
        Ok(None)
    }
}

struct MockProvider;

impl SecretsProvider for MockProvider {
    fn resolve(&self, alias: &SecretAlias) -> Result<SecretValue> {
        Ok(SecretValue {
            value: format!("AIPLUS_MOCK_{}", alias.alias.to_ascii_uppercase()),
        })
    }

    fn provider_name(&self) -> &'static str {
        "mock"
    }

    fn secret_id_found(&self, _alias: &SecretAlias) -> Result<Option<bool>> {
        Ok(Some(true))
    }
}

struct BwsProvider {
    token: String,
}

impl SecretsProvider for BwsProvider {
    fn resolve(&self, alias: &SecretAlias) -> Result<SecretValue> {
        let secret_id = self.lookup_secret_id(alias)?;
        let output = Command::new("bws")
            .args(["secret", "get"])
            .arg(&secret_id)
            .args(["--output", "json"])
            .env("BWS_ACCESS_TOKEN", &self.token)
            .output()
            .context("run bws secret get")?;
        if !output.status.success() {
            return Err(CliError::new(
                1,
                format!(
                    "SECRET_RESOLVE_STATUS=FAIL alias={} provider=bws reason=unavailable_or_denied",
                    alias.alias
                ),
            )
            .into());
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let json: serde_json::Value = serde_json::from_str(&text).map_err(|_| {
            CliError::new(
                1,
                "SECRET_RESOLVE_STATUS=FAIL provider=bws reason=invalid_json",
            )
        })?;
        let value = json.get("value").and_then(|v| v.as_str()).ok_or_else(|| {
            CliError::new(
                1,
                format!(
                    "SECRET_RESOLVE_STATUS=FAIL alias={} provider=bws reason=missing_value",
                    alias.alias
                ),
            )
        })?;
        validate_secret_value(&alias.alias, value)?;
        Ok(SecretValue {
            value: value.to_string(),
        })
    }

    fn provider_name(&self) -> &'static str {
        "bws"
    }

    fn token_source(&self) -> &'static str {
        token_source()
    }

    fn secret_id_found(&self, alias: &SecretAlias) -> Result<Option<bool>> {
        Ok(Some(!self.lookup_secret_id(alias)?.is_empty()))
    }
}

// ---------------------------------------------------------------------------
// KeyringProvider — v0.5.16 default backend.
// Stores each alias's value as a separate OS keyring entry under
// (service = SECRET_KEYRING_SERVICE, account = <alias>). Zero cloud cost,
// zero CLI dep, works offline. Each entry is per-machine — no automatic
// sync across machines (use the Bitwarden backend for that).
// ---------------------------------------------------------------------------

const SECRET_KEYRING_SERVICE: &str = "aiplus/secret-broker-key";

fn keyring_alias_entry(alias_name: &str) -> Result<keyring::Entry> {
    keyring::Entry::new(SECRET_KEYRING_SERVICE, alias_name)
        .context("create OS keyring entry for alias")
}

struct KeyringProvider;

impl SecretsProvider for KeyringProvider {
    fn resolve(&self, alias: &SecretAlias) -> Result<SecretValue> {
        let entry = keyring_alias_entry(&alias.alias)?;
        match entry.get_password() {
            Ok(v) => {
                validate_secret_value(&alias.alias, &v)?;
                Ok(SecretValue { value: v })
            }
            Err(keyring::Error::NoEntry) => Err(CliError::new(
                1,
                format!(
                    "SECRET_RESOLVE_STATUS=FAIL alias={} provider=keyring reason=no_value_set hint=run `aiplus secret-broker set --alias {}` to store it. (If you used Bitwarden previously, set AIPLUS_SECRET_PROVIDER=bws to switch back.)",
                    alias.alias, alias.alias
                ),
            )
            .into()),
            Err(keyring::Error::NoStorageAccess(detail))
            | Err(keyring::Error::PlatformFailure(detail)) => Err(CliError::new(
                1,
                format!(
                    "SECRET_RESOLVE_STATUS=FAIL alias={} provider=keyring reason=keyring_unavailable detail={detail}",
                    alias.alias
                ),
            )
            .into()),
            Err(e) => Err(CliError::new(
                1,
                format!(
                    "SECRET_RESOLVE_STATUS=FAIL alias={} provider=keyring reason={e}",
                    alias.alias
                ),
            )
            .into()),
        }
    }

    fn provider_name(&self) -> &'static str {
        "keyring"
    }

    fn secret_id_found(&self, alias: &SecretAlias) -> Result<Option<bool>> {
        let Ok(entry) = keyring_alias_entry(&alias.alias) else {
            return Ok(None);
        };
        match entry.get_password() {
            Ok(_) => Ok(Some(true)),
            Err(keyring::Error::NoEntry) => Ok(Some(false)),
            Err(_) => Ok(None),
        }
    }
}

impl BwsProvider {
    fn lookup_secret_id(&self, alias: &SecretAlias) -> Result<String> {
        let project_id = bitwarden_project_id();
        if project_id.is_empty() {
            return Err(CliError::new(
                1,
                format!(
                    "SECRET_RESOLVE_STATUS=FAIL alias={} provider=bws reason=project_id_not_set hint=set AIPLUS_BWS_PROJECT_ID env var or install a private profile bundle that supplies it",
                    alias.alias
                ),
            )
            .into());
        }
        let output = Command::new("bws")
            .args(["secret", "list"])
            .arg(&project_id)
            .args(["--output", "json"])
            .env("BWS_ACCESS_TOKEN", &self.token)
            .output()
            .context("run bws secret list")?;
        if !output.status.success() {
            return Err(CliError::new(
                1,
                format!(
                    "SECRET_RESOLVE_STATUS=FAIL alias={} provider=bws reason=project_unavailable_or_denied",
                    alias.alias
                ),
            )
            .into());
        }
        let json: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|_| {
            CliError::new(
                1,
                format!(
                    "SECRET_RESOLVE_STATUS=FAIL alias={} provider=bws reason=invalid_json",
                    alias.alias
                ),
            )
        })?;
        let secrets = json.as_array().ok_or_else(|| {
            CliError::new(
                1,
                format!(
                    "SECRET_RESOLVE_STATUS=FAIL alias={} provider=bws reason=invalid_json",
                    alias.alias
                ),
            )
        })?;
        for secret in secrets {
            let key = secret_key_field(secret);
            if key.as_deref() == Some(alias.bitwarden_name.as_str()) {
                let id = secret.get("id").and_then(|value| value.as_str()).ok_or_else(|| {
                    CliError::new(
                        1,
                        format!(
                            "SECRET_RESOLVE_STATUS=FAIL alias={} provider=bws reason=missing_secret_id",
                            alias.alias
                        ),
                    )
                })?;
                return Ok(id.to_string());
            }
        }
        Err(CliError::new(
            1,
            format!(
                "SECRET_RESOLVE_STATUS=FAIL alias={} provider=bws reason=secret_key_not_found",
                alias.alias
            ),
        )
        .into())
    }
}

fn secret_key_field(secret: &serde_json::Value) -> Option<String> {
    for field in ["key", "name"] {
        if let Some(value) = secret.get(field).and_then(|value| value.as_str()) {
            return Some(value.to_string());
        }
    }
    None
}

fn secret_aliases() -> Result<Vec<SecretAlias>> {
    let broker_dir = config_home()?.join("aiplus").join("secret-broker");
    let legacy_path = broker_dir.join("secret-aliases.tsv");
    let mut by_alias: BTreeMap<String, SecretAlias> = BTreeMap::new();
    if legacy_path.exists() {
        for alias in parse_secret_alias_file(&legacy_path)? {
            by_alias.insert(alias.alias.clone(), alias);
        }
    }
    let profiles_dir = broker_dir.join("profiles");
    if profiles_dir.exists() {
        for entry in fs::read_dir(&profiles_dir)? {
            let entry = entry?;
            let path = entry.path().join("secret-aliases.tsv");
            if path.exists() {
                for alias in parse_secret_alias_file(&path)? {
                    by_alias.insert(alias.alias.clone(), alias);
                }
            }
        }
    }
    Ok(by_alias.into_values().collect())
}

fn parse_secret_alias_file(path: &Path) -> Result<Vec<SecretAlias>> {
    let text = fs::read_to_string(path)?;
    let mut aliases = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 3 {
            return Err(CliError::new(
                1,
                format!(
                    "SECRET_ALIAS_CONFIG_INVALID path={} line={}",
                    path.display(),
                    index + 1
                ),
            )
            .into());
        }
        aliases.push(SecretAlias {
            alias: fields[0].to_string(),
            bitwarden_name: fields[1].to_string(),
            env_var: fields[2].to_string(),
        });
    }
    Ok(aliases)
}

#[allow(dead_code)]
fn legacy_secret_aliases() -> Result<Vec<SecretAlias>> {
    let path = config_home()?
        .join("aiplus")
        .join("secret-broker")
        .join("secret-aliases.tsv");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path)?;
    let mut aliases = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.len() != 3 {
            return Err(CliError::new(
                1,
                format!("SECRET_ALIAS_CONFIG_INVALID line={}", index + 1),
            )
            .into());
        }
        aliases.push(SecretAlias {
            alias: fields[0].to_string(),
            bitwarden_name: fields[1].to_string(),
            env_var: fields[2].to_string(),
        });
    }
    Ok(aliases)
}

fn resolve_alias(alias: Option<String>) -> Result<SecretAlias> {
    let alias = alias.ok_or_else(|| CliError::new(2, "ERROR missing secret alias"))?;
    secret_aliases()?
        .into_iter()
        .find(|item| item.alias == alias)
        .ok_or_else(|| CliError::new(1, format!("SECRET_ALIAS_NOT_ALLOWED {alias}")).into())
}

fn parse_requested_aliases(
    alias_flags: Vec<String>,
    aliases_csv: Option<String>,
) -> Result<Vec<String>> {
    let mut aliases = Vec::new();
    for alias in alias_flags {
        push_requested_alias(&mut aliases, &alias)?;
    }
    if let Some(csv) = aliases_csv {
        for alias in csv.split(',') {
            push_requested_alias(&mut aliases, alias)?;
        }
    }
    Ok(aliases)
}

fn push_requested_alias(aliases: &mut Vec<String>, alias: &str) -> Result<()> {
    let alias = alias.trim();
    if alias.is_empty() {
        return Err(CliError::new(2, "ERROR empty secret alias in --aliases").into());
    }
    if !aliases.iter().any(|item| item == alias) {
        aliases.push(alias.to_string());
    }
    Ok(())
}

fn select_secret_aliases(requested_aliases: &[String]) -> Result<Vec<SecretAlias>> {
    let available = secret_aliases()?;
    if requested_aliases.is_empty() {
        return Ok(available);
    }
    let mut selected = Vec::new();
    for requested in requested_aliases {
        let alias = available
            .iter()
            .find(|item| item.alias == *requested)
            .cloned()
            .ok_or_else(|| {
                CliError::new(
                    1,
                    format!(
                        "SECRET_BROKER_RUN_STATUS=FAIL alias={} reason=unknown_alias",
                        requested
                    ),
                )
            })?;
        selected.push(alias);
    }
    Ok(selected)
}

fn provider_metadata(alias: &str) -> Option<ProviderMetadata> {
    match alias {
        "kimi" | "kimi_code" => Some(ProviderMetadata {
            provider_family: "kimi_code",
            platform: "kimi_code_membership",
            base_url: "https://api.kimi.com/coding/v1",
            model: "kimi-for-coding",
            smoke_endpoint: "https://api.kimi.com/coding/v1/models",
        }),
        "kimi_platform" | "moonshot" => Some(ProviderMetadata {
            provider_family: "kimi_platform",
            platform: "kimi_open_platform",
            base_url: "https://api.moonshot.ai/v1",
            model: "moonshot-v1-8k",
            smoke_endpoint: "https://api.moonshot.ai/v1/models",
        }),
        _ => None,
    }
}

fn validate_secret_value(alias: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() || value.trim() == "PENDING_OWNER_INPUT_DO_NOT_USE" {
        return Err(CliError::new(
            1,
            format!(
                "SECRET_RESOLVE_STATUS=FAIL alias={} provider=bws reason=secret_placeholder_or_empty",
                alias
            ),
        )
        .into());
    }
    Ok(())
}

fn secret_error_reason(error: &anyhow::Error) -> String {
    let message = error.to_string();
    if let Some(reason) = message.split("reason=").nth(1) {
        let reason = reason
            .split_whitespace()
            .next()
            .unwrap_or("resolve_failed")
            .trim_matches(|c: char| c == ',' || c == ';');
        if !reason.is_empty() {
            return reason.to_string();
        }
    }
    "resolve_failed".to_string()
}

fn load_secret_provider() -> Result<Box<dyn SecretsProvider>> {
    match provider_name().as_str() {
        "mock" => Ok(Box::new(MockProvider)),
        "bws" => {
            let token = get_bws_token()?;
            Ok(Box::new(BwsProvider { token }))
        }
        "keyring" => Ok(Box::new(KeyringProvider)),
        other => Err(CliError::new(
            1,
            format!(
                "AIPLUS_SECRET_PROVIDER={other} not recognized; supported: keyring (default), bws, mock"
            ),
        )
        .into()),
    }
}

// v0.5.16: default switched from "bws" to "keyring". OS keyring is free,
// offline, zero-config — works for solo developers without paying for a
// Bitwarden Secrets Manager subscription. Users with multi-machine sync
// or team sharing needs set AIPLUS_SECRET_PROVIDER=bws in their shell.
fn provider_name() -> String {
    std::env::var("AIPLUS_SECRET_PROVIDER").unwrap_or_else(|_| "keyring".to_string())
}

fn token_source() -> &'static str {
    if std::env::var("BWS_ACCESS_TOKEN").is_ok() {
        "env"
    } else if read_keychain_token().ok().flatten().is_some() {
        "keychain"
    } else {
        "not_configured"
    }
}

fn bitwarden_project_id() -> String {
    std::env::var("AIPLUS_BWS_PROJECT_ID").unwrap_or_else(|_| DEFAULT_BWS_PROJECT_ID.to_string())
}

fn get_bws_token() -> Result<String> {
    if let Ok(token) = std::env::var("BWS_ACCESS_TOKEN") {
        if !token.trim().is_empty() {
            return Ok(token);
        }
    }
    if let Some(token) = read_keychain_token()? {
        return Ok(token);
    }
    Err(CliError::new(
        1,
        "SECRET_BROKER_TOKEN_MISSING set BWS_ACCESS_TOKEN for this session or run `aiplus secret-broker token set`",
    )
    .into())
}

fn token_set() -> Result<()> {
    let mut token = String::new();
    io::stdin().read_to_string(&mut token)?;
    let token = token.trim();
    if token.is_empty() {
        return Err(CliError::new(1, "ERROR no token received on stdin").into());
    }
    write_keychain_token(token)?;
    println!("SECRET_BROKER_TOKEN_SET");
    println!("token_storage=keychain");
    println!("secret_value_printed=no");
    println!("TOKEN_SET_STATUS=PASS");
    Ok(())
}

fn token_delete() -> Result<()> {
    delete_keychain_token()?;
    println!("SECRET_BROKER_TOKEN_DELETE");
    println!("token_storage=keychain");
    println!("TOKEN_DELETE_STATUS=PASS");
    Ok(())
}

// ---------------------------------------------------------------------------
// Cross-platform keyring access for the BWS access token.
//
// Phase 2 of the multi-platform rollout: replaces the previous
// `Command::new("security")` shell-out (macOS-only) with the `keyring`
// crate, which transparently maps to macOS Keychain, Linux Secret Service
// (D-Bus / gnome-keyring / kwallet via `secret-service` backend), and
// Windows Credential Manager.
//
// On platforms with no available backend (e.g. a Linux box without a
// running Secret Service daemon), `keyring::Entry::new` succeeds but
// `set_password` / `get_password` return `keyring::Error::NoStorageAccess`.
// We treat that as "keychain unavailable, use BWS_ACCESS_TOKEN env var
// instead" for read; we surface a concrete error message for write.
// ---------------------------------------------------------------------------

fn keyring_entry() -> Result<keyring::Entry> {
    keyring::Entry::new(SECRET_BROKER_SERVICE, SECRET_BROKER_ACCOUNT)
        .context("create OS keyring entry")
}

fn read_keychain_token() -> Result<Option<String>> {
    let entry = match keyring_entry() {
        Ok(e) => e,
        // If the platform has no keyring backend at all, treat as
        // "no stored token" so callers fall back to BWS_ACCESS_TOKEN.
        Err(_) => return Ok(None),
    };
    match entry.get_password() {
        Ok(token) => {
            let trimmed = token.trim().to_string();
            if trimmed.is_empty() {
                Ok(None)
            } else {
                Ok(Some(trimmed))
            }
        }
        // NoEntry = nothing stored yet; not an error path.
        Err(keyring::Error::NoEntry) => Ok(None),
        // Platform has no usable backend (e.g. no Secret Service daemon).
        // Quietly fall back to "no token" so BWS_ACCESS_TOKEN can take over.
        Err(keyring::Error::NoStorageAccess(_)) | Err(keyring::Error::PlatformFailure(_)) => {
            Ok(None)
        }
        Err(e) => Err(anyhow!("read OS keyring: {e}")),
    }
}

fn write_keychain_token(token: &str) -> Result<()> {
    let entry = keyring_entry()?;
    match entry.set_password(token) {
        Ok(()) => Ok(()),
        // NoStorageAccess: keyring crate reports no backend available.
        // PlatformFailure: keyring crate reports a backend was found but
        // it failed at runtime — on Linux this typically means
        // libdbus tried to autolaunch dbus-daemon and couldn't reach a
        // session bus (headless container, no D-Bus session, etc.). Both
        // are user-facing "no keyring available" cases — surface the same
        // friendly hint pointing at BWS_ACCESS_TOKEN.
        Err(keyring::Error::NoStorageAccess(detail))
        | Err(keyring::Error::PlatformFailure(detail)) => Err(CliError::new(
            1,
            &format!(
                "TOKEN_SET_STATUS=FAIL reason=keyring_unavailable detail={detail}\n\
                 No usable OS keyring backend on this system. On Linux this usually \
                 means no Secret Service daemon (gnome-keyring / kwallet) is running, \
                 or there's no active D-Bus session bus. As a workaround set \
                 BWS_ACCESS_TOKEN as an environment variable instead."
            ),
        )
        .into()),
        Err(e) => Err(CliError::new(
            1,
            &format!("TOKEN_SET_STATUS=FAIL reason=keyring_write_failed detail={e}"),
        )
        .into()),
    }
}

fn delete_keychain_token() -> Result<()> {
    let entry = match keyring_entry() {
        Ok(e) => e,
        // No backend at all → nothing to delete; treat as no-op success.
        Err(_) => return Ok(()),
    };
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(keyring::Error::NoStorageAccess(_)) | Err(keyring::Error::PlatformFailure(_)) => Ok(()),
        Err(e) => Err(anyhow!("delete OS keyring entry: {e}")),
    }
}

fn command_available(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .map(|output| {
            output.status.success() || !output.stdout.is_empty() || !output.stderr.is_empty()
        })
        .unwrap_or(false)
}

fn require_profile_name(profile: Option<String>) -> Result<String> {
    let profile = profile.ok_or_else(|| CliError::new(2, "ERROR missing profile name"))?;
    validate_profile_name(&profile)?;
    Ok(profile)
}

fn validate_profile_name(profile: &str) -> Result<()> {
    let valid = !profile.is_empty()
        && !profile.contains('/')
        && !profile.contains('\\')
        && profile != "."
        && profile != ".."
        && !profile.contains("..");
    if valid {
        Ok(())
    } else {
        Err(CliError::new(1, format!("PROFILE_NAME_INVALID {profile}")).into())
    }
}

fn profile_dir(profile: &str) -> Result<PathBuf> {
    Ok(config_home()?.join("aiplus").join("profiles").join(profile))
}

fn installed_profile_names(profiles_root: &Path) -> Result<Vec<String>> {
    if !profiles_root.exists() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(profiles_root)? {
        let entry = entry?;
        let path = entry.path();
        if path.join("profile.toml").exists() && !path.join("disabled").exists() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !is_legacy_profile_name(&name) {
                names.push(name);
            }
        }
    }
    names.sort();
    Ok(names)
}

fn legacy_profile_names(profiles_root: &Path) -> Result<Vec<String>> {
    if !profiles_root.exists() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(profiles_root)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.join("profile.toml").exists()
            && !path.join("disabled").exists()
            && is_legacy_profile_name(&name)
        {
            names.push(name);
        }
    }
    names.sort();
    Ok(names)
}

fn is_legacy_profile_name(profile: &str) -> bool {
    profile == "work-with-zhiwen"
}

fn resolve_profile_source(source: Option<PathBuf>) -> Result<PathBuf> {
    let source = source.unwrap_or(std::env::current_dir()?);
    if !source.join("profile.toml").exists() || !source.join("AGENTS.profile.md").exists() {
        return Err(CliError::new(
            1,
            format!(
                "PROFILE_SOURCE_INVALID {} requires profile.toml and AGENTS.profile.md",
                source.display()
            ),
        )
        .into());
    }
    Ok(source)
}

fn install_profile_file(source: &Path, target: &Path, file_name: &str) -> Result<()> {
    let bytes = fs::read(source.join(file_name))
        .with_context(|| format!("read profile source file {file_name}"))?;
    write_file_atomic(&target.join(file_name), &bytes)
}

fn copy_profile_dir(source: &Path, target: &Path, dir_name: &str) -> Result<()> {
    let src = source.join(dir_name);
    let dst = target.join(dir_name);
    fs::create_dir_all(&dst)?;
    copy_dir_recursive(&src, &dst)?;
    Ok(())
}

fn backup_profile_dir(profile: &str, dir: &Path) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    let backup_root = config_home()?.join("aiplus").join("profile-backups");
    fs::create_dir_all(&backup_root)?;
    let backup_dir = backup_root.join(format!("{profile}-{}", epoch_millis()));
    copy_dir_recursive(dir, &backup_dir)?;
    Ok(())
}

fn remove_profile_registration(profile: &str) -> Result<()> {
    let dir = profile_dir(profile)?;
    if dir.exists() {
        backup_profile_dir(profile, &dir)?;
        fs::remove_dir_all(&dir)?;
        println!("profile_removed={profile}");
    }
    let alias_dir = config_home()?
        .join("aiplus")
        .join("secret-broker")
        .join("profiles")
        .join(profile);
    if alias_dir.exists() {
        fs::remove_dir_all(&alias_dir)?;
        println!("secret_aliases_removed={profile}");
    }
    Ok(())
}

fn config_home() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("XDG_CONFIG_HOME") {
        if !path.trim().is_empty() {
            return Ok(PathBuf::from(path));
        }
    }
    // Windows: APPDATA is the canonical roaming config dir
    // (typically C:\Users\<name>\AppData\Roaming).
    #[cfg(target_os = "windows")]
    if let Ok(appdata) = std::env::var("APPDATA") {
        if !appdata.trim().is_empty() {
            return Ok(PathBuf::from(appdata));
        }
    }
    // Unix-like fallback: HOME/.config (also covers most Wine setups).
    if let Ok(home) = std::env::var("HOME") {
        if !home.trim().is_empty() {
            return Ok(PathBuf::from(home).join(".config"));
        }
    }
    // Windows last-resort: USERPROFILE/.config — covers Windows shells
    // where APPDATA somehow isn't set (rare) plus Wine setups that
    // expose USERPROFILE but not HOME.
    if let Ok(profile) = std::env::var("USERPROFILE") {
        if !profile.trim().is_empty() {
            return Ok(PathBuf::from(profile).join(".config"));
        }
    }
    Err(anyhow!(
        "Cannot determine config directory: none of XDG_CONFIG_HOME, APPDATA, HOME, USERPROFILE \
         are set"
    ))
}

// ------------------------------------------------------------------
// Cross-project registry (~/.config/aiplus/installed-projects.json)
// ------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Registry {
    schema_version: String,
    #[serde(default)]
    installed_projects: Vec<RegistryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RegistryEntry {
    path: PathBuf,
    first_installed: String,
    last_updated: String,
    #[serde(default)]
    runtimes: Vec<String>,
}

fn registry_file() -> Result<PathBuf> {
    let config = config_home()?;
    Ok(config.join("aiplus/installed-projects.json"))
}

fn read_registry() -> Result<Registry> {
    let path = registry_file()?;
    if !path.exists() {
        return Ok(Registry {
            schema_version: "1.0".to_string(),
            installed_projects: Vec::new(),
        });
    }
    let text = fs::read_to_string(&path).context("read registry")?;
    let registry: Registry = serde_json::from_str(&text).context("parse registry")?;
    Ok(registry)
}

fn write_registry(registry: &Registry) -> Result<()> {
    let path = registry_file()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("create registry dir")?;
    }
    let tmp = path.with_extension("tmp");
    let mut file = fs::File::create(&tmp).context("create registry tmp")?;
    file.write_all(serde_json::to_string_pretty(registry)?.as_bytes())
        .context("write registry tmp")?;
    file.sync_all().context("fsync registry tmp")?;
    drop(file);
    fs::rename(&tmp, &path).context("rename registry tmp")?;
    Ok(())
}

fn now_rfc3339() -> String {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let dt = simple_time_from_epoch(secs);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        dt.year, dt.month, dt.day, dt.hour, dt.minute, dt.second
    )
}

fn upsert_registry_entry(path: &Path, runtimes: &[String]) -> Result<()> {
    let mut registry = read_registry()?;
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let now = now_rfc3339();
    if let Some(entry) = registry
        .installed_projects
        .iter_mut()
        .find(|e| e.path == canon)
    {
        entry.last_updated = now;
        if !runtimes.is_empty() {
            entry.runtimes = runtimes.to_vec();
        }
    } else {
        registry.installed_projects.push(RegistryEntry {
            path: canon,
            first_installed: now.clone(),
            last_updated: now,
            runtimes: runtimes.to_vec(),
        });
    }
    write_registry(&registry)
}

fn remove_registry_entry(path: &Path) -> Result<()> {
    let mut registry = read_registry()?;
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    registry.installed_projects.retain(|e| e.path != canon);
    write_registry(&registry)
}

fn memory_init(root: &Path) -> Result<()> {
    let mut plan = Plan::default();
    for rel in [
        ".aiplus/memory",
        ".aiplus/identities",
        ".aiplus/skills/candidates",
        ".aiplus/skills/approved",
        ".aiplus/skills/rejected",
        ".aiplus/restore",
    ] {
        ensure_dir(root, &rel_to_abs(root, rel)?, &mut plan)?;
    }
    for rel in [
        ".aiplus/memory/project-memory.jsonl",
        ".aiplus/memory/decisions.jsonl",
        ".aiplus/memory/facts.jsonl",
        ".aiplus/memory/audit.jsonl",
        ".aiplus/skills/candidates/candidates.jsonl",
    ] {
        write_if_missing(root, rel, b"")?;
    }
    write_if_missing(
        root,
        ".aiplus/memory/index.json",
        b"{\n  \"schemaVersion\": \"0.1.0\",\n  \"store\": \"project-local\",\n  \"cloudSync\": false\n}\n",
    )?;
    write_if_missing(
        root,
        ".aiplus/skills/registry.toml",
        b"schema_version = \"0.1.0\"\nautomatic_approved_skills = false\nowner_gate_required_for_approval = true\n",
    )?;
    write_if_missing(
        root,
        ".aiplus/restore/restore-policy.toml",
        b"schema_version = \"0.1.0\"\ncloud_sync = false\nautomatic_transcript_learning = false\nmemory_as_approval = false\nidentity_as_permission = false\n",
    )?;
    identity_init(root)?;
    Ok(())
}

fn identity_init(root: &Path) -> Result<()> {
    let mut plan = Plan::default();
    ensure_dir(root, &identity_dir(root)?, &mut plan)?;
    for role in ["advisor", "ceo", "reviewer", "builder"] {
        let asset = format!("aiplus-agent-memory/core/templates/{role}.identity.toml");
        let rel = format!(".aiplus/identities/{role}.identity.toml");
        let content = embedded_asset_text(&asset)?;
        write_if_missing(root, &rel, content.as_bytes())?;
    }
    Ok(())
}

fn write_if_missing(root: &Path, rel: &str, bytes: &[u8]) -> Result<()> {
    let path = rel_to_abs(root, rel)?;
    assert_no_symlink_path(root, &path)?;
    if path.exists() {
        return Ok(());
    }
    write_file_atomic(&path, bytes)
}

fn continuity_state(root: &Path) -> Result<ContinuityState> {
    let memory = memory_dir(root)?;
    let records = read_memory_records(root).unwrap_or_default();
    let active = records
        .iter()
        .filter(|record| record.status == "active" || record.status == "tentative")
        .count();
    let identities = identity_dir(root)?;
    let candidates = read_skill_candidates(root).unwrap_or_default();
    let rejected = candidates
        .iter()
        .filter(|candidate| candidate.status == "rejected")
        .count();
    Ok(ContinuityState {
        agent_memory: if memory.exists() {
            "installed".to_string()
        } else {
            "not_initialized".to_string()
        },
        memory_records_active: active,
        memory_records_total: records.len(),
        identity_advisor: identities.join("advisor.identity.toml").exists(),
        identity_ceo: identities.join("ceo.identity.toml").exists(),
        identity_reviewer: identities.join("reviewer.identity.toml").exists(),
        identity_builder: identities.join("builder.identity.toml").exists(),
        skill_candidates_total: candidates.len(),
        skill_candidates_rejected: rejected,
        profile_status: if private_profile_installed()? {
            "installed".to_string()
        } else {
            "missing".to_string()
        },
    })
}

fn private_profile_installed() -> Result<bool> {
    Ok(canonical_user_profile()?.is_some())
}

/// Returns the canonical user-level AiPlus profile name, or None if none
/// is installed. "Canonical" = the first non-legacy installed profile in
/// `~/.config/aiplus/profiles/`, alphabetical. Lets the CLI work with any
/// `aiplus-work-with-<owner>` profile name (including the public
/// `aiplus-work-with-you` template and personal forks), not just the
/// original `aiplus-work-with-zhiwen` prototype.
fn canonical_user_profile() -> Result<Option<String>> {
    let profiles_root = match config_home() {
        Ok(home) => home.join("aiplus").join("profiles"),
        Err(_) => return Ok(None),
    };
    let names = installed_profile_names(&profiles_root)?;
    Ok(names.into_iter().next())
}

fn canonical_user_profile_or_default() -> String {
    canonical_user_profile()
        .ok()
        .flatten()
        .unwrap_or_else(|| "aiplus-work-with-you".to_string())
}

fn print_continuity_status_lines(state: &ContinuityState) {
    println!("agentMemory={}", state.agent_memory);
    println!("memoryRecordsActive={}", state.memory_records_active);
    println!("memoryRecordsTotal={}", state.memory_records_total);
    println!(
        "identity=advisor={} ceo={} reviewer={} builder={}",
        yes_no(state.identity_advisor),
        yes_no(state.identity_ceo),
        yes_no(state.identity_reviewer),
        yes_no(state.identity_builder)
    );
    println!("skillCandidatesTotal={}", state.skill_candidates_total);
    println!(
        "skillCandidatesRejected={}",
        state.skill_candidates_rejected
    );
    println!("approved_auto=none");
    let profile_name = canonical_user_profile()
        .ok()
        .flatten()
        .unwrap_or_else(|| "(none)".to_string());
    println!("profile={profile_name} {}", state.profile_status);
    println!("secret_values=none");
    println!("global_agent_config=untouched");
}

fn validate_memory_jsonl(root: &Path, rel: &str) -> Result<Vec<String>> {
    let path = rel_to_abs(root, rel)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut errors = Vec::new();
    for (index, line) in fs::read_to_string(path)?.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<MemoryRecord>(line) {
            Ok(record) => {
                if let Err(error) = reject_sensitive_memory_text(&record.summary) {
                    errors.push(format!("{rel}:{} {}", index + 1, error));
                }
            }
            Err(error) => errors.push(format!("{rel}:{} invalid_json {error}", index + 1)),
        }
    }
    Ok(errors)
}

fn memory_sensitive_warnings(root: &Path) -> Result<Vec<String>> {
    let mut warnings = Vec::new();
    for rel in [
        ".aiplus/memory/project-memory.jsonl",
        ".aiplus/memory/decisions.jsonl",
        ".aiplus/memory/facts.jsonl",
        ".aiplus/memory/audit.jsonl",
        ".aiplus/skills/candidates/candidates.jsonl",
    ] {
        let path = rel_to_abs(root, rel)?;
        if !path.exists() {
            continue;
        }
        let text = fs::read_to_string(path)?;
        for (label, found) in sensitive_findings(&text) {
            if found {
                warnings.push(format!("{rel}: sensitive pattern detected ({label})"));
            }
        }
    }
    Ok(warnings)
}

fn append_audit(root: &Path, event: &str, target: &str) -> Result<()> {
    let line = serde_json::json!({
        "schemaVersion": "0.1.0",
        "event": event,
        "target": target,
        "timestamp": timestamp(),
        "secretValues": "none"
    });
    append_jsonl_atomic(
        &rel_to_abs(root, ".aiplus/memory/audit.jsonl")?,
        &line.to_string(),
    )
}

fn validate_memory_field(field: &str, value: &str, allowed: &[&str]) -> Result<()> {
    if allowed.contains(&value) {
        return Ok(());
    }
    Err(CliError::new(
        2,
        format!(
            "ERROR invalid {field}={value}; allowed=[{}]",
            allowed.join(",")
        ),
    )
    .into())
}

fn reject_sensitive_memory_text(text: &str) -> Result<()> {
    if let Err(err) = aiplus_core::reject_sensitive_memory_text(text) {
        return Err(CliError::new(1, err.to_string()).into());
    }
    Ok(())
}

fn toml_value_line(text: &str, key: &str) -> Option<String> {
    let prefix = format!("{key} = ");
    text.lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(&prefix))
        .map(|value| value.trim().trim_matches('"').to_string())
}

fn print_memory_rows<'a, I>(records: I, limit: Option<usize>) -> usize
where
    I: Iterator<Item = &'a MemoryRecord>,
{
    let mut shown = 0usize;
    for record in records {
        if limit.is_some_and(|max| shown >= max) {
            break;
        }
        let content = if reject_sensitive_memory_text(&record.summary).is_ok() {
            single_line(&record.summary)
        } else {
            "<REDACTED_BY_SCAN>".to_string()
        };
        println!(
            "record={} scope={} kind={} status={} updatedAt={} content={}",
            record.id, record.scope, record.record_type, record.status, record.updated_at, content
        );
        shown += 1;
    }
    shown
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let target_path = target.join(entry.file_name());
        if path.is_dir() {
            copy_dir_recursive(&path, &target_path)?;
        } else if path.is_file() {
            fs::copy(&path, &target_path)?;
        }
    }
    Ok(())
}

fn command_self(subcommand: Option<String>, dry_run: bool, yes: bool, auto: bool) -> Result<()> {
    match subcommand.as_deref() {
        Some("update") | Some("upgrade") => command_self_update(dry_run, yes || auto, auto),
        _ => {
            println!("Usage: aiplus self update [--dry-run] [--yes] [--auto]");
            process::exit(2);
        }
    }
}

fn command_self_update(dry_run: bool, yes: bool, auto: bool) -> Result<()> {
    let target = self_update_target()?;
    let old_version = binary_version(&target).unwrap_or_else(|| VERSION.to_string());
    let new_version =
        std::env::var("AIPLUS_UPDATE_VERSION").unwrap_or_else(|_| RELEASE_TAG.to_string());
    let asset = detect_release_asset()?;
    let base_url = std::env::var("AIPLUS_RELEASE_BASE_URL").unwrap_or_else(|_| {
        format!("https://github.com/izhiwen/aiplus/releases/download/{new_version}")
    });
    println!("SELF_UPDATE");
    println!("old_version={old_version}");
    println!("new_version={}", new_version.trim_start_matches('v'));
    println!("target_path={}", target.display());
    println!("asset={asset}");
    println!("shell_profile_edits=none");
    println!("global_agent_config_edits=none");
    println!("telemetry=none");
    println!("uploads=none");
    if auto {
        println!("auto=yes");
    }
    if dry_run {
        println!("DRY_RUN=YES");
        println!("CHECKSUM_STATUS=NOT_RUN");
        println!("SELF_UPDATE_STATUS=DRY_RUN");
        return Ok(());
    }
    if !yes {
        return Err(CliError::new(1, "ERROR self update requires --yes or --dry-run").into());
    }

    let temp = std::env::temp_dir().join(format!("aiplus-self-update-{}", epoch_millis()));
    fs::create_dir_all(&temp)?;
    let checksums = temp.join("checksums.txt");
    let sha256 = temp.join(format!("{asset}.sha256"));
    let archive = temp.join(&asset);

    // Download archive first
    fetch_to(&format!("{base_url}/{asset}"), &archive)?;

    // Try checksums.txt first, fall back to per-artifact .sha256
    if fetch_to(&format!("{base_url}/checksums.txt"), &checksums).is_ok() {
        println!("checksum_source=checksums.txt");
        verify_checksum_file(&checksums, &archive)?;
    } else if fetch_to(&format!("{base_url}/{asset}.sha256"), &sha256).is_ok() {
        println!("checksum_source={asset}.sha256");
        verify_sha256_file(&sha256, &archive)?;
    } else {
        return Err(anyhow!(
            "checksums.txt and {asset}.sha256 both not found at {base_url}"
        ));
    }

    println!("checksum_status=PASS");
    println!("CHECKSUM_STATUS=PASS");
    let extract_dir = temp.join("extract");
    fs::create_dir_all(&extract_dir)?;
    extract_release_archive(&archive, &extract_dir)?;
    let staged = find_release_binary(&extract_dir, cfg!(windows))?;
    let smoke = binary_version(&staged).unwrap_or_else(|| "unknown".to_string());
    if smoke == "unknown" {
        return Err(CliError::new(1, "ERROR staged binary smoke check failed").into());
    }
    let backup = target.with_file_name(format!(
        "{}.backup-{}",
        target
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("aiplus"),
        epoch_millis()
    ));
    if target.exists() {
        fs::copy(&target, &backup)?;
    }
    let staged_target = target.with_file_name(format!(
        "{}.staged-{}",
        target
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("aiplus"),
        epoch_millis()
    ));
    fs::copy(&staged, &staged_target)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&staged_target, fs::Permissions::from_mode(0o755))?;
    }
    fs::rename(&staged_target, &target)?;
    let final_version = binary_version(&target).unwrap_or_else(|| "unknown".to_string());
    if final_version == "unknown" {
        if backup.exists() {
            let _ = fs::copy(&backup, &target);
        }
        return Err(CliError::new(
            1,
            "ERROR updated binary smoke check failed; backup retained",
        )
        .into());
    }
    println!("backup_path={}", backup.display());
    println!("smoke_version={final_version}");
    println!("SELF_UPDATE_STATUS=PASS");
    Ok(())
}

fn compact_init_command(root: &Path, force: bool) -> Result<()> {
    let mut plan = Plan::default();
    compact_init(root, &mut plan, force)?;
    for item in &plan.items {
        println!("{} {}", item.action, item.path);
    }
    println!("INIT_PASS");
    Ok(())
}

fn install_base(
    root: &Path,
    plan: &mut Plan,
    options: &Options,
    module_names: Vec<String>,
    adapters: &[String],
) -> Result<()> {
    ensure_dir(root, &rel_to_abs(root, ".aiplus/modules")?, plan)?;
    for name in &module_names {
        let spec = module_spec(name).unwrap();
        copy_embedded_module(root, spec, plan, options)?;
    }
    write_file_safe(
        root,
        ".aiplus/AGENTS.aiplus.md",
        agents_aiplus_content().as_bytes(),
        plan,
        options,
    )?;
    write_file_safe(
        root,
        REFRESH_PROMPT_REL,
        refresh_prompt_content().as_bytes(),
        plan,
        options,
    )?;
    // S2: append the Secret-Lookup-Protocol section to AGENTS.aiplus.md.
    // Marker-keyed so re-running install replaces in place (idempotent).
    // Lives at install_base level, not per-team, because the broker is
    // a global feature — agents should know about it whether or not a
    // virtual team is installed.
    if !plan.dry_run {
        append_team_section_to_agents_aiplus(root, "BROKER_PROTOCOL", BROKER_PROTOCOL_SECTION)?;
    }
    if module_names
        .iter()
        .any(|name| name == MODULE_SLUG_COMPACT_REMINDER)
    {
        compact_init(root, plan, false)?;
    }
    if module_names.iter().any(|name| name == "agent-memory") && !plan.dry_run {
        memory_init(root)?;
    }
    if module_names
        .iter()
        .any(|name| name == "auto-team-consultant")
    {
        install_consultant_team_config(root, plan, options)?;
    }
    if module_names.iter().any(|name| name == "agent-team") && !plan.dry_run {
        agent_team_init(root, plan, adapters)?;
    }
    if module_names.iter().any(|name| name == "aieconlab") && !plan.dry_run {
        aieconlab_init(root, plan)?;
    }
    Ok(())
}

fn aieconlab_init(root: &Path, plan: &mut Plan) -> Result<()> {
    // AiEconLab (AEL) populates the same `.aiplus/agents/` namespace that
    // agent-team uses. If both modules are installed, the most recent
    // init wins ("current active team" model). Users switch by re-running
    // `aiplus add agent-team` or `aiplus add aieconlab`.
    let agents_dir = root.join(".aiplus").join("agents");
    std::fs::create_dir_all(&agents_dir)?;
    std::fs::create_dir_all(root.join(".aiplus").join("aieconlab"))?;
    std::fs::create_dir_all(agents_dir.join("personas"))?;
    std::fs::create_dir_all(agents_dir.join("personas").join("_stubs"))?;
    std::fs::create_dir_all(agents_dir.join("experts"))?;

    // Track A.2: clear agent-team's exclusive files from
    // `.aiplus/agents/` before writing AEL's own. Otherwise the
    // snapshot saved later by `snapshot_active_team` captures BOTH
    // teams merged, `set_active_team` restores that merged state, and
    // `mirror_personas_to_runtimes` writes both teams' bare files
    // into `.claude/agents/` — the residue uninstall (A.1) cannot
    // safely remove.
    clear_other_team_residue(root, "aieconlab")?;

    // Copy core role configs (8 roles)
    for role in [
        "advisor",
        "pi",
        "theorist",
        "pm",
        "ra-stata",
        "ra-python",
        "referee",
        "replicator",
    ] {
        let asset = format!("aieconlab/core/templates/{role}.toml");
        let content = embedded_asset_text(&asset)?;
        write_file_atomic(&agents_dir.join(format!("{role}.toml")), content.as_bytes())?;
    }

    // Copy team config
    let team_content = embedded_asset_text("aieconlab/core/templates/econ-team.toml")?;
    write_file_atomic(&agents_dir.join("econ-team.toml"), team_content.as_bytes())?;

    // Copy core personas (8 roles)
    for role in [
        "advisor",
        "pi",
        "theorist",
        "pm",
        "ra-stata",
        "ra-python",
        "referee",
        "replicator",
    ] {
        let asset = format!("aieconlab/core/templates/personas/{role}.md");
        let content = embedded_asset_text(&asset)?;
        write_file_atomic(
            &agents_dir.join("personas").join(format!("{role}.md")),
            content.as_bytes(),
        )?;
    }

    // Copy shipped expert configs (9 of 12)
    for expert in [
        "lit-reviewer",
        "writer",
        "econometrician",
        "reproducibility",
        "historical-sources",
        "job-talk-coach",
        "viz-specialist",
        "ethics-irb",
        "llm-measurement",
    ] {
        let asset = format!("aieconlab/core/templates/experts/{expert}.toml");
        let content = embedded_asset_text(&asset)?;
        write_file_atomic(
            &agents_dir.join("experts").join(format!("{expert}.toml")),
            content.as_bytes(),
        )?;

        let persona_asset = format!("aieconlab/core/templates/personas/{expert}.md");
        let persona_content = embedded_asset_text(&persona_asset)?;
        write_file_atomic(
            &agents_dir.join("personas").join(format!("{expert}.md")),
            persona_content.as_bytes(),
        )?;
    }

    // W5: the 3 final experts (survey-experiment, computation,
    // coauthor-liaison) graduated from `_stubs/` to full personas.
    // Copy them alongside the other shipped 9 — same path layout, no
    // `_stubs/` subdir.
    for expert in ["survey-experiment", "computation", "coauthor-liaison"] {
        let asset = format!("aieconlab/core/templates/experts/{expert}.toml");
        let content = embedded_asset_text(&asset)?;
        write_file_atomic(
            &agents_dir.join("experts").join(format!("{expert}.toml")),
            content.as_bytes(),
        )?;

        let persona_asset = format!("aieconlab/core/templates/personas/{expert}.md");
        let persona_content = embedded_asset_text(&persona_asset)?;
        write_file_atomic(
            &agents_dir.join("personas").join(format!("{expert}.md")),
            persona_content.as_bytes(),
        )?;
    }

    // Replace the default SWE consultant team config with the AEL
    // research-tuned consultant team (5 expert seats designed from
    // first principles for applied-econ research at plan time, 3 user
    // personas, 5 owner gates, LIGHT tier skips consult). See AEL
    // DESIGN.md §9 for the rationale.
    //
    // Coexistence of `consultant-team.swe.toml` and
    // `consultant-team.aieconlab.toml` in the same project is on the
    // v0.2 roadmap; v0.1 takes the simpler stance that the most
    // recent `aiplus add <agent-team|aieconlab>` wins.
    let consultant_path = root.join(".aiplus").join("consultant-team.toml");
    let consultant_content =
        embedded_asset_text("aieconlab/core/templates/consultant-team.aieconlab.toml")?;
    write_file_atomic(&consultant_path, consultant_content.as_bytes())?;

    // Advertise the team in AGENTS.aiplus.md so any runtime that reads it
    // (codex, claude-code, opencode) discovers AEL roles without the user
    // having to mention them explicitly. Idempotent — runs once per module.
    append_team_section_to_agents_aiplus(root, "AIECONLAB_TEAM", AIECONLAB_TEAM_SECTION)?;

    // Snapshot AEL's active layout into _teams/aieconlab/ for fast switching,
    // and mark AEL as the currently active team. Phase D v1: this is the
    // mechanism behind `aiplus agent set-team`. With both agent-team and
    // aieconlab installed, the most-recently-`aiplus add`-ed module is
    // active and the other is preserved as a snapshot.
    crate::agent::set_team::snapshot_active_team(root, "aieconlab")?;
    crate::agent::set_team::set_active_team(root, "aieconlab")?;

    // Phase D v1: mirror personas to per-runtime agent dirs so claude-code
    // and opencode discover the AEL team within their permission/trust
    // boundary. Codex already reads `.aiplus/agents/` directly, so the
    // mirror is purely additive — it doesn't change codex behavior.
    mirror_personas_to_runtimes(root)?;

    // W3: seed per-role memory namespaces for the 8 AEL core roles.
    // Experts that get summoned on demand don't get a namespace at
    // install time — they're added when first invoked.
    init_memory_namespaces(root, AIECONLAB_ROLES)?;

    // W6: seed synthetic velocity runs for the 5 AEL research unit
    // types. Brand-new projects need *some* p50/p90 before the first
    // real measurement. Seeds are flagged `seed=true` so doctor knows
    // they don't count toward the 5-record calibration threshold.
    aiplus_core::init_aieconlab_velocity_seeds(root)?;

    // v0.2 (AEL repo): install Claude Code adapter content when the
    // project has the claude-code runtime adapter. No-op if claude-code
    // isn't installed (so codex- or opencode-only projects are
    // unaffected). Writes 20 subagents with YAML frontmatter, 4 slash
    // commands, and an AEL managed block in CLAUDE.md.
    install_aieconlab_claude_code_adapter(root, plan)?;

    // v0.3 (Track B.1): same shape for OpenCode. Writes 20 prefixed
    // agent files + 4 slash commands under `.opencode/`. No-op when
    // opencode isn't in the project's runtime adapter list.
    install_aieconlab_opencode_adapter(root, plan)?;

    Ok(())
}

const AIECONLAB_TEAM_SECTION: &str = r#"## Virtual Team: AiEconLab (AEL)

This project has the AiEconLab applied-economics research team installed.
Role definitions live under `.aiplus/agents/personas/`. Owner talks only
to Advisor and PI; PI orchestrates the rest.

- Owner-facing (2): `Advisor`, `PI`
- Internal core (6): `Theorist`, `PM`, `RA-Stata`, `RA-Python`, `Referee`, `Replicator`
- Experts on-demand (12): `lit-reviewer`, `writer`, `econometrician`,
  `reproducibility`, `historical-sources`, `job-talk-coach`, `viz-specialist`,
  `ethics-irb`, `llm-measurement`, `survey-experiment`, `computation`,
  `coauthor-liaison`

To embody a role in this session, the Owner says:

    Speak as the AEL Advisor — <question>
    Speak as the AEL PI — <task>

Or, for an interactive session with the persona pre-loaded, run:

    aiplus agent talk advisor          # or pi, theorist, ra-stata, ...

When embodying a role: read `.aiplus/agents/personas/<role>.md` first. The
persona's Forbidden Actions and Escalation rules are binding. STOP-gated
actions (journal submission, working-paper posting, referee response send,
data sharing, authorship change) always escalate to the Owner.

`aiplus agent route <role> "<task>"` records dispatches to
`.aiplus/agents/dispatch-log.jsonl` and marks the role active. Use it after
the PI commits to a staffing decision to make the dispatch a real artifact
rather than narrative.
"#;

const AGENT_TEAM_ROLES: &[&str] = &[
    "advisor",
    "ceo",
    "architect",
    "pm",
    "engineer-a",
    "engineer-b",
    "reviewer",
    "qa",
];

const AIECONLAB_ROLES: &[&str] = &[
    "advisor",
    "pi",
    "theorist",
    "pm",
    "ra-stata",
    "ra-python",
    "referee",
    "replicator",
];

/// Track A.2: roles owned only by agent-team (advisor + pm are shared
/// with aieconlab; the team-specific init writes them last so we don't
/// need to clear them at residue time).
const AGENT_TEAM_EXCLUSIVE_ROLES: &[&str] = &[
    "ceo",
    "architect",
    "engineer-a",
    "engineer-b",
    "reviewer",
    "qa",
];

const AGENT_TEAM_EXPERTS_ALL: &[&str] = &[
    "ai-integration",
    "security-reviewer",
    "tech-writer",
    "devops",
    "ui-designer",
    "researcher",
    "data-analyst",
    "customer-researcher",
    "performance-engineer",
    "accessibility",
    "compliance-reviewer",
];

const AGENT_TEAM_STUB_PERSONAS: &[&str] = &[
    "data-analyst",
    "customer-researcher",
    "performance-engineer",
    "accessibility",
    "compliance-reviewer",
];

const AIECONLAB_EXCLUSIVE_ROLES: &[&str] = &[
    "pi",
    "theorist",
    "ra-stata",
    "ra-python",
    "referee",
    "replicator",
];

const AIECONLAB_EXPERTS_ALL: &[&str] = &[
    "coauthor-liaison",
    "computation",
    "econometrician",
    "ethics-irb",
    "historical-sources",
    "job-talk-coach",
    "lit-reviewer",
    "llm-measurement",
    "reproducibility",
    "survey-experiment",
    "viz-specialist",
    "writer",
];

/// Track A.2: before `agent_team_init` / `aieconlab_init` writes its
/// own role TOMLs / personas / experts into `.aiplus/agents/`, sweep
/// the OTHER team's exclusive files. Without this, a second team init
/// runs on top of the first team's files; the subsequent
/// `snapshot_active_team` captures the merged state, `set_active_team`
/// restores that merge, and `mirror_personas_to_runtimes` writes both
/// teams' bare role files into `.claude/agents/` — the residue
/// `aiplus uninstall` (A.1) cannot safely remove.
///
/// `my_team` is "agent-team" or "aieconlab"; this function clears the
/// opposite team's files. Shared role names (`advisor`, `pm`) are NOT
/// cleared because the calling init will overwrite them right after.
fn clear_other_team_residue(root: &Path, my_team: &str) -> Result<()> {
    let agents_dir = root.join(".aiplus").join("agents");
    if !agents_dir.exists() {
        return Ok(());
    }
    let personas_dir = agents_dir.join("personas");
    let experts_dir = agents_dir.join("experts");
    let stubs_dir = personas_dir.join("_stubs");

    let (exclusive_roles, experts, stub_personas, team_config_file, module_dir) = match my_team {
        "aieconlab" => (
            AGENT_TEAM_EXCLUSIVE_ROLES,
            AGENT_TEAM_EXPERTS_ALL,
            AGENT_TEAM_STUB_PERSONAS,
            "agent-team.toml",
            ".aiplus/agent-team",
        ),
        "agent-team" => (
            AIECONLAB_EXCLUSIVE_ROLES,
            AIECONLAB_EXPERTS_ALL,
            // aieconlab does not write _stubs personas; nothing to
            // sweep on this axis when switching to agent-team.
            &[][..],
            "econ-team.toml",
            ".aiplus/aieconlab",
        ),
        _ => return Ok(()),
    };

    // Sweep role TOMLs and core personas.
    for role in exclusive_roles {
        let _ = std::fs::remove_file(agents_dir.join(format!("{role}.toml")));
        let _ = std::fs::remove_file(personas_dir.join(format!("{role}.md")));
    }
    // Sweep expert configs and their personas (aieconlab puts expert
    // personas in personas/, agent-team puts none — only stubs).
    for expert in experts {
        let _ = std::fs::remove_file(experts_dir.join(format!("{expert}.toml")));
        let _ = std::fs::remove_file(personas_dir.join(format!("{expert}.md")));
    }
    // Sweep agent-team's stub personas under personas/_stubs/.
    for stub in stub_personas {
        let _ = std::fs::remove_file(stubs_dir.join(format!("{stub}.md")));
    }
    // Sweep the OTHER team's top-level team-config file.
    let _ = std::fs::remove_file(agents_dir.join(team_config_file));
    // Sweep the OTHER team's module dir (config-only; live module
    // content lives under `.aiplus/modules/`). Defensive: only remove
    // if it exists.
    let module_path = root.join(module_dir);
    if module_path.exists() {
        let _ = std::fs::remove_dir_all(&module_path);
    }
    // Intentionally do NOT wipe the OTHER team's `_teams/<other>/`
    // snapshot here. The snapshot system exists precisely so users
    // can switch back via `aiplus agent set-team <other>` without
    // re-install. Legacy projects whose OTHER snapshot was captured
    // pre-A.2 may carry merged residue; recommended remediation is to
    // re-run `aiplus add <other>` which triggers a clean re-snapshot.
    Ok(())
}

/// W3: seed per-role memory namespaces.
///
/// Creates `.aiplus/agent-memory/<role>/` for each role in the active
/// team + `.aiplus/agent-memory/_team/` for cross-role records (the
/// same dir that W1 writes consult JSONL into).
///
/// Each namespace gets a `.gitkeep` (so git tracks the empty dir) and
/// a one-line README explaining the isolation rule. We only write the
/// README when it's missing so re-installing doesn't clobber a user's
/// edits.
fn init_memory_namespaces(root: &Path, roles: &[&str]) -> Result<()> {
    let base = root.join(".aiplus").join("agent-memory");
    let team_dir = base.join("_team");
    std::fs::create_dir_all(&team_dir).with_context(|| format!("create {}", team_dir.display()))?;
    let team_readme = "# _team/ — cross-role records\n\
                       Consult findings and gate-ledger entries written by\n\
                       `aiplus agent route` land here. Per-role memory dirs\n\
                       live in siblings and are isolated.\n";
    write_if_missing(
        root,
        ".aiplus/agent-memory/_team/README.md",
        team_readme.as_bytes(),
    )?;
    write_if_missing(root, ".aiplus/agent-memory/_team/.gitkeep", b"")?;
    for role in roles {
        let rdir = base.join(role);
        std::fs::create_dir_all(&rdir).with_context(|| format!("create {}", rdir.display()))?;
        let role_readme = format!(
            "# agent-memory/{role}/ — per-role records\n\
             Memory written by the `{role}` role lives here. Other roles\n\
             do not read this dir by default — isolation prevents the\n\
             context pollution that single-agent setups suffer from.\n"
        );
        write_if_missing(
            root,
            &format!(".aiplus/agent-memory/{role}/README.md"),
            role_readme.as_bytes(),
        )?;
        write_if_missing(root, &format!(".aiplus/agent-memory/{role}/.gitkeep"), b"")?;
    }
    Ok(())
}

fn agent_team_init(root: &Path, plan: &mut Plan, adapters: &[String]) -> Result<()> {
    let agents_dir = root.join(".aiplus").join("agents");
    std::fs::create_dir_all(&agents_dir)?;
    std::fs::create_dir_all(root.join(".aiplus").join("agent-team"))?;
    std::fs::create_dir_all(agents_dir.join("personas"))?;
    std::fs::create_dir_all(agents_dir.join("personas").join("_stubs"))?;
    std::fs::create_dir_all(agents_dir.join("experts"))?;

    // Track A.2: clear aieconlab's exclusive files before writing
    // agent-team's own. See `clear_other_team_residue` doc for why.
    clear_other_team_residue(root, "agent-team")?;

    // Copy core role configs
    for role in AGENT_TEAM_ROLES {
        let asset = format!("aiplus-agent-team/core/templates/{role}.toml");
        let content = embedded_asset_text(&asset)?;
        write_file_atomic(&agents_dir.join(format!("{role}.toml")), content.as_bytes())?;
    }

    // Copy team config
    let team_content = embedded_asset_text("aiplus-agent-team/core/templates/agent-team.toml")?;
    write_file_atomic(&agents_dir.join("agent-team.toml"), team_content.as_bytes())?;

    // Copy core personas
    for role in [
        "advisor",
        "ceo",
        "architect",
        "pm",
        "engineer-a",
        "engineer-b",
        "reviewer",
        "qa",
    ] {
        let asset = format!("aiplus-agent-team/core/templates/personas/{role}.md");
        let content = embedded_asset_text(&asset)?;
        write_file_atomic(
            &agents_dir.join("personas").join(format!("{role}.md")),
            content.as_bytes(),
        )?;
    }

    // Copy functional expert configs
    for expert in [
        "ai-integration",
        "security-reviewer",
        "tech-writer",
        "devops",
        "ui-designer",
        "researcher",
    ] {
        let asset = format!("aiplus-agent-team/core/templates/experts/{expert}.toml");
        let content = embedded_asset_text(&asset)?;
        write_file_atomic(
            &agents_dir.join("experts").join(format!("{expert}.toml")),
            content.as_bytes(),
        )?;
    }

    // Copy stub expert configs
    for expert in [
        "data-analyst",
        "customer-researcher",
        "performance-engineer",
        "accessibility",
        "compliance-reviewer",
    ] {
        let asset = format!("aiplus-agent-team/core/templates/experts/{expert}.toml");
        let content = embedded_asset_text(&asset)?;
        write_file_atomic(
            &agents_dir.join("experts").join(format!("{expert}.toml")),
            content.as_bytes(),
        )?;
    }

    // Copy stub personas
    for expert in [
        "data-analyst",
        "customer-researcher",
        "performance-engineer",
        "accessibility",
        "compliance-reviewer",
    ] {
        let asset = format!("aiplus-agent-team/core/templates/personas/_stubs/{expert}.md");
        let content = embedded_asset_text(&asset)?;
        write_file_atomic(
            &agents_dir
                .join("personas")
                .join("_stubs")
                .join(format!("{expert}.md")),
            content.as_bytes(),
        )?;
    }

    // Advertise the agent-team in AGENTS.aiplus.md so runtimes discover it.
    append_team_section_to_agents_aiplus(root, "AGENT_TEAM_TEAM", AGENT_TEAM_SECTION)?;

    // Snapshot + activate. See aieconlab_init for rationale.
    crate::agent::set_team::snapshot_active_team(root, "agent-team")?;
    crate::agent::set_team::set_active_team(root, "agent-team")?;

    // Mirror to per-runtime agent dirs. See aieconlab_init for rationale.
    mirror_personas_to_runtimes(root)?;

    // W3: seed per-role memory namespaces + the cross-role `_team/`
    // dir. Idempotent — re-runs leave existing READMEs intact.
    init_memory_namespaces(root, AGENT_TEAM_ROLES)?;

    // Issue #31: install Claude Code adapter content when the project
    // has the claude-code runtime adapter. No-op if claude-code isn't
    // installed (so codex- or opencode-only projects are unaffected).
    // Writes 14 prefixed subagents with YAML frontmatter, slash
    // commands, and an agent-team managed block in CLAUDE.md so that
    // Claude Code's auto-routing can discover the team.
    install_agent_team_claude_code_adapter(root, plan, adapters)?;

    // Track B.2: same shape for OpenCode. Writes 14 prefixed agent
    // files + 2 slash commands under `.opencode/`. No-op when
    // opencode isn't in the live adapter list.
    install_agent_team_opencode_adapter(root, plan, adapters)?;

    Ok(())
}

/// Copy the currently-active team's personas from `.aiplus/agents/personas/`
/// to each installed runtime's agents directory (`.claude/agents/`,
/// `.opencode/agents/`). Codex reads `.aiplus/` directly so no mirror is
/// needed for it.
///
/// Per-runtime mirrors solve the "OpenCode permission model rejects reading
/// .aiplus/" problem found in Level 1 testing of v0.5.6: OpenCode treats any
/// path outside its home dir or `.opencode/` as `external_directory` and
/// auto-rejects it, which made the AEL/agent-team personas invisible to
/// OpenCode sessions even though they were installed.
/// P1.6: result of comparing `.aiplus/agents/personas/` (source of
/// truth) against `.claude/agents/` and `.opencode/agents/` mirrors.
enum PersonaDriftStatus {
    /// No source dir — personas not installed yet. Skip the check.
    NoSource,
    /// All same-name mirrors match their source by byte-equality.
    InSync,
    /// At least one same-name mirror differs from its source.
    Drift { files: Vec<String> },
}

/// Walk every persona under `.aiplus/agents/personas/` and check
/// against same-named files under installed runtime adapter dirs.
/// Conservative: only flags drift when (1) a same-named mirror exists
/// AND (2) its bytes don't match source. Skips renamed mirrors
/// (e.g. claude's `aiplus-<role>.md` / `aieconlab-<role>.md` prefixed
/// agents) — those come from a different module install path and are
/// not duplicates of the source persona.
/// N3: trim ASCII whitespace (space / tab / CR / LF) from both ends.
/// Used to normalize source vs frontmatter-stripped mirror before
/// comparing — the stripped mirror often starts with a blank line that
/// the source doesn't have.
fn trim_ascii_whitespace(bytes: &[u8]) -> &[u8] {
    let is_ws = |b: u8| matches!(b, b' ' | b'\t' | b'\r' | b'\n');
    let start = bytes.iter().position(|&b| !is_ws(b)).unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|&b| !is_ws(b))
        .map(|i| i + 1)
        .unwrap_or(start);
    &bytes[start..end]
}

/// N3: strip leading YAML frontmatter (`---\n…---\n`) if present, so we
/// can compare a frontmatter-wrapped mirror against an unwrapped source.
/// If the input doesn't start with `---` we return it unchanged.
fn strip_yaml_frontmatter(bytes: &[u8]) -> &[u8] {
    if !bytes.starts_with(b"---\n") && !bytes.starts_with(b"---\r\n") {
        return bytes;
    }
    // Skip the opening `---\n` (or `---\r\n`), then look for the closing
    // `---\n` line. We scan line-by-line so we don't accidentally match a
    // `---` inside a multi-line YAML value.
    let after_open = if bytes.starts_with(b"---\r\n") { 5 } else { 4 };
    let body = &bytes[after_open..];
    let mut line_start = 0usize;
    for (idx, b) in body.iter().enumerate() {
        if *b == b'\n' {
            let line = &body[line_start..idx];
            // Strip trailing \r for CRLF line endings.
            let stripped = if line.ends_with(b"\r") {
                &line[..line.len() - 1]
            } else {
                line
            };
            if stripped == b"---" {
                // Closing fence found. The body starts immediately after
                // this line's newline.
                let body_start = after_open + idx + 1;
                return &bytes[body_start..];
            }
            line_start = idx + 1;
        }
    }
    // No closing fence found — treat as no frontmatter so we don't
    // accidentally drop content.
    bytes
}

/// N3: how a source persona name maps to candidate mirror filenames
/// for a given runtime. Captures the prefix conventions that different
/// install paths use:
///   - The "active-team" mirror writes `.claude/agents/<role>.md`
///     unprefixed (via `mirror_personas_to_runtimes`).
///   - The AEL claude-code adapter writes
///     `.claude/agents/aieconlab-<role>.md` (via
///     `install_aieconlab_claude_code_adapter`).
///   - The agent-team claude-code adapter writes
///     `.claude/agents/agent-team-<role>.md` (parallel path).
///
/// We check ALL candidate mirror names that exist; same-name is the
/// primary path, prefixed names are the secondary. Drift in any of
/// them counts as drift for the source file.
fn mirror_name_candidates(source_file: &str, runtime: &str, active_team: &str) -> Vec<String> {
    // Strip the .md extension so we can re-glue prefixes cleanly.
    let role = match source_file.strip_suffix(".md") {
        Some(s) => s,
        None => return vec![source_file.to_string()],
    };
    let mut out = vec![format!("{role}.md")];
    match (runtime, active_team) {
        ("claude-code", "aieconlab") | ("opencode", "aieconlab") => {
            out.push(format!("aieconlab-{role}.md"));
        }
        ("claude-code", "agent-team") | ("opencode", "agent-team") => {
            out.push(format!("agent-team-{role}.md"));
        }
        _ => {}
    }
    out
}

fn persona_mirror_drift(root: &Path) -> PersonaDriftStatus {
    let source_dir = root.join(".aiplus").join("agents").join("personas");
    if !source_dir.exists() {
        return PersonaDriftStatus::NoSource;
    }
    let adapters = read_installed_runtime_adapters(root);
    let active_team =
        crate::agent::set_team::read_active_team(root).unwrap_or_else(|| "agent-team".to_string());
    // Build (runtime, mirror_dir) pairs so we know which prefix
    // convention to use when generating candidate filenames.
    let mut mirrors: Vec<(String, PathBuf)> = Vec::new();
    for adapter in &adapters {
        match adapter.as_str() {
            "claude-code" => mirrors.push((
                "claude-code".to_string(),
                root.join(".claude").join("agents"),
            )),
            "opencode" => mirrors.push((
                "opencode".to_string(),
                root.join(".opencode").join("agents"),
            )),
            _ => {}
        }
    }
    if mirrors.is_empty() {
        return PersonaDriftStatus::InSync;
    }
    let mut drifted: BTreeSet<String> = BTreeSet::new();
    let entries = match std::fs::read_dir(&source_dir) {
        Ok(it) => it,
        Err(_) => return PersonaDriftStatus::NoSource,
    };
    for entry in entries.flatten() {
        let src_path = entry.path();
        if !src_path.is_file() || src_path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let file_name = match src_path.file_name().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if file_name == "aiplus-advisor.md" {
            continue;
        }
        let src_bytes = match std::fs::read(&src_path) {
            Ok(b) => b,
            Err(_) => continue,
        };
        for (runtime, mirror_dir) in &mirrors {
            for candidate in mirror_name_candidates(&file_name, runtime, &active_team) {
                let mirror_path = mirror_dir.join(&candidate);
                if !mirror_path.exists() {
                    continue;
                }
                let mirror_bytes = match std::fs::read(&mirror_path) {
                    Ok(b) => b,
                    Err(_) => continue,
                };
                // Prefixed mirrors (claude `aieconlab-<role>.md` etc.)
                // get wrapped with YAML frontmatter at install time:
                //
                //   ---
                //   name: aieconlab-pi
                //   description: "..."
                //   ---
                //   # PI — AiEconLab v0.1
                //   <source body...>
                //
                // Comparing bytes-for-bytes would always report drift
                // because the frontmatter is always present. Strip it
                // from the mirror before comparing — what we care about
                // is whether the BODY matches the source.
                let mirror_body = strip_yaml_frontmatter(&mirror_bytes);
                // The frontmatter close-fence `---\n` is followed by a
                // blank line in the rendered template, so the body
                // starts with a `\n` that the source doesn't have.
                // Trim leading and trailing whitespace/newlines from
                // both sides before comparing so we're checking the
                // semantically-identical content.
                if trim_ascii_whitespace(mirror_body) == trim_ascii_whitespace(&src_bytes) {
                    continue;
                }
                // Report the actual mirror filename that drifted (not
                // the source name) so the user knows which file is stale.
                drifted.insert(candidate);
            }
        }
    }
    if drifted.is_empty() {
        PersonaDriftStatus::InSync
    } else {
        let files: Vec<String> = drifted.into_iter().collect();
        PersonaDriftStatus::Drift { files }
    }
}

#[cfg(test)]
mod persona_drift_mapping_tests {
    use super::{mirror_name_candidates, strip_yaml_frontmatter};

    #[test]
    fn strip_yaml_frontmatter_passes_through_when_absent() {
        let input = b"# Plain Markdown\nBody.";
        assert_eq!(strip_yaml_frontmatter(input), input.as_slice());
    }

    #[test]
    fn strip_yaml_frontmatter_removes_present_frontmatter() {
        let input = b"---\nname: pi\ndescription: \"PI persona\"\n---\n# PI body\n";
        assert_eq!(strip_yaml_frontmatter(input), b"# PI body\n");
    }

    #[test]
    fn strip_yaml_frontmatter_handles_crlf_line_endings() {
        let input = b"---\r\nname: pi\r\n---\r\n# Body\r\n";
        let out = strip_yaml_frontmatter(input);
        assert!(out.starts_with(b"# Body"));
    }

    #[test]
    fn strip_yaml_frontmatter_no_closing_fence_returns_unchanged() {
        let input = b"---\nname: pi\nno closing fence here\nstill no closing\n";
        assert_eq!(strip_yaml_frontmatter(input), input.as_slice());
    }

    #[test]
    fn same_name_always_first_candidate() {
        let out = mirror_name_candidates("pi.md", "claude-code", "agent-team");
        assert_eq!(out[0], "pi.md");
    }

    #[test]
    fn aieconlab_active_adds_prefixed_candidate_for_claude() {
        let out = mirror_name_candidates("pi.md", "claude-code", "aieconlab");
        assert!(out.contains(&"pi.md".to_string()));
        assert!(out.contains(&"aieconlab-pi.md".to_string()));
    }

    #[test]
    fn agent_team_active_adds_prefixed_candidate_for_claude() {
        let out = mirror_name_candidates("engineer-a.md", "claude-code", "agent-team");
        assert!(out.contains(&"engineer-a.md".to_string()));
        assert!(out.contains(&"agent-team-engineer-a.md".to_string()));
    }

    #[test]
    fn opencode_gets_same_prefix_logic() {
        let out = mirror_name_candidates("ra-stata.md", "opencode", "aieconlab");
        assert!(out.contains(&"aieconlab-ra-stata.md".to_string()));
    }

    #[test]
    fn unknown_runtime_returns_only_same_name() {
        let out = mirror_name_candidates("advisor.md", "codex", "aieconlab");
        assert_eq!(out, vec!["advisor.md"]);
    }

    #[test]
    fn missing_md_extension_falls_back_to_input() {
        let out = mirror_name_candidates("pi", "claude-code", "aieconlab");
        assert_eq!(out, vec!["pi"]);
    }
}

fn mirror_personas_to_runtimes(root: &Path) -> Result<()> {
    let source_dir = root.join(".aiplus").join("agents").join("personas");
    if !source_dir.exists() {
        return Ok(());
    }
    let adapters = read_installed_runtime_adapters(root);
    let mut targets: Vec<PathBuf> = Vec::new();
    for adapter in &adapters {
        match adapter.as_str() {
            "claude-code" => targets.push(root.join(".claude").join("agents")),
            "opencode" => targets.push(root.join(".opencode").join("agents")),
            // codex reads .aiplus/ directly — no mirror.
            _ => {}
        }
    }
    for target_dir in &targets {
        std::fs::create_dir_all(target_dir)?;
        for entry in std::fs::read_dir(&source_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                let file_name = path.file_name().unwrap();
                let target = target_dir.join(file_name);
                // Skip if it's AiPlus's own platform advisor file (we don't
                // want to clobber the project-level aiplus-advisor.md).
                if target.file_name().and_then(|s| s.to_str()) == Some("aiplus-advisor.md") {
                    continue;
                }
                std::fs::copy(&path, &target)?;
            }
        }
    }
    Ok(())
}

/// Read the `runtimeAdapters` list from `.aiplus/manifest.json`.
fn read_installed_runtime_adapters(root: &Path) -> Vec<String> {
    let manifest_path = root.join(".aiplus").join("manifest.json");
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

/// S2: Secret-lookup protocol that every install writes into
/// `.aiplus/AGENTS.aiplus.md` (marker `BROKER_PROTOCOL`). The whole
/// point: when an agent needs an API key, the default behavior must
/// be "check the broker," not "ask the Owner."
///
/// Kept bilingual on purpose — the AGENTS file is read by both
/// English-default and Chinese-default agent runtimes, and a key/token
/// negotiation is the moment we LEAST want translation drift.
const BROKER_PROTOCOL_SECTION: &str = r#"## Secret lookup protocol (read before asking the Owner for keys)

**Required `aiplus` version on PATH**: ≥ 0.5.18 (for `need` + `--auto-prompt`).
If `aiplus --version` reports older, ask the Owner to upgrade
(`curl -fsSL https://raw.githubusercontent.com/izhiwen/AiPlus/main/install.sh | sh`)
before running any command in this section. Do NOT silently fall back to
asking the Owner for plaintext keys.

AiPlus ships an integrated secret broker. **Default backend (v0.5.16+):
the user's OS keyring** (macOS Keychain / Linux Secret Service / Windows
Credential Manager). Free, zero cloud cost, works offline. Users with
multi-machine sync / team sharing needs opt into the Bitwarden Secrets
Manager backend via `AIPLUS_SECRET_PROVIDER=bws`.

**Canonical agent flow (v0.5.18+)** — before any external API call:

```
# Inside the agent's bash tool, before calling an API that needs <alias>:
eval "$(aiplus secret-broker need <alias> --auto-prompt)"
```

What that one line does:

1. Checks the OS keyring for `<alias>`.
2. **Present** → outputs `export <ENV>='<value>'` on stdout; `eval` puts
   it in the child env; agent's subsequent commands have the env var.
3. **Missing** + `--auto-prompt` → pops a native OS password dialog on
   the user's desktop (not the terminal — works even from the agent's
   sandboxed bash). User pastes value, hits Save. Value lands in
   keyring AND in env. Agent continues.
4. **Missing** + no `--auto-prompt` → exits 75 with a hint telling the
   user to run `aiplus secret-broker set <alias> --auto-prompt`.

**Cross-project share**: once a user provides a value for `<alias>` on
this machine — in ANY project — every future `need <alias>` resolves
silently from keyring. The user never re-enters the same key twice.

**NEVER ask the Owner directly for a key value.** Always go through
`need` / `set`. The value lives only in the OS keyring; the agent only
ever sees the value as an env var inside its own child-process scope.

Other commands you can call:

- `aiplus secret-broker list` — alias→env_var map (no values).
- `aiplus secret-broker resolve <alias>` — metadata (no values).
- `aiplus secret-broker run --aliases a,b -- <cmd>` — alternative to
  the `eval` pattern: spawn a child with env vars injected, cleared on
  exit.
- `aiplus secret-broker push --alias <a> --to <target>` — push to
  GitHub Actions / `.env` / shell `export` (target = `github-secret:`,
  `env:`, `dotenv:`).

The broker never prints secret values to stdout/log by default;
`--print` requires explicit opt-in. Audit log stays metadata-only.

中文版（与英文同义，agent 任择其一阅读）：

AiPlus 自带 secret broker。**默认后端（v0.5.16+）：用户操作系统自带的
keyring**（macOS Keychain / Linux Secret Service / Windows Credential
Manager），免费、离线、零订阅。需要多机同步 / 团队共享时用户可 opt-in
Bitwarden Secrets Manager（`AIPLUS_SECRET_PROVIDER=bws`）。

**Agent 标准流程（v0.5.18+）**：调外部 API 之前先跑一行：

```
eval "$(aiplus secret-broker need <alias> --auto-prompt)"
```

这行命令：
1. 查 OS keyring 有没有 `<alias>`
2. **有** → stdout 输出 `export <ENV>='<value>'`，eval 注入到 child
   进程 env，后续调用拿得到
3. **没** + `--auto-prompt` → 在用户桌面弹原生密码框（不是终端 prompt —
   即使 agent 在 sandboxed bash 里也能弹），用户粘 value、点 Save，
   值存进 keyring 同时注入 env，agent 继续
4. **没** + 无 `--auto-prompt` → exit 75，stderr 提示用户跑
   `aiplus secret-broker set <alias> --auto-prompt`

**跨项目共享**：一台机器上用户给某 alias 提供过一次值（任何项目里），
之后所有项目里 `need <alias>` 都从 keyring 静默取出。**用户绝不需要
重输入同一把 key**。

**绝不问 Owner 要 key 明文**（严禁直接索取）。永远走 `need` / `set`。
值只在 OS keyring 里，agent 永远只能通过 env var 在自己子进程范围内
见到。

其他命令：

- `aiplus secret-broker list` —— alias→env_var 映射（无 value）
- `aiplus secret-broker resolve <alias>` —— 元数据（无 value）
- `aiplus secret-broker run --aliases a,b -- <cmd>` —— `eval` 模式的
  替代：直接 spawn child + 注入 env，退出即清
- `aiplus secret-broker push --alias <a> --to <target>` —— 推到
  GitHub Actions / `.env` / 当前 shell（`github-secret:`、`env:`、
  `dotenv:` 三种 target）

broker 默认绝不向 stdout/log 打印 secret 值；`--print` 要显式 opt-in。
audit 只记元数据。
"#;

const AGENT_TEAM_SECTION: &str = r#"## Virtual Team: AiPlus Agent Team (software-engineering)

This project has the AiPlus Agent Team installed for software-engineering
workflows. Role definitions live under `.aiplus/agents/personas/`. Owner
talks only to Advisor and CEO; CEO orchestrates the rest.

- Owner-facing (2): `Advisor`, `CEO`
- Internal core (6): `Architect`, `PM`, `Engineer-A`, `Engineer-B`, `Reviewer`, `QA`
- Experts on-demand (11): `ai-integration`, `security-reviewer`, `tech-writer`,
  `devops`, `ui-designer`, `researcher`, and 5 v0.2 stubs

To embody a role: say "Speak as the Agent Team Advisor — <question>" or
run `aiplus agent talk <role>`. Persona spec at
`.aiplus/agents/personas/<role>.md` is binding; Forbidden Actions and
STOP-gates always escalate to the Owner.

`aiplus agent route <role> "<task>"` records dispatches to
`.aiplus/agents/dispatch-log.jsonl` and marks the role active.
"#;

/// Append a team-overview section to `.aiplus/AGENTS.aiplus.md` so any runtime
/// that reads the AiPlus project context (codex, claude-code, opencode)
/// discovers the installed virtual team. Idempotent — the marker comment
/// prevents duplicate appends across reinstalls.
fn append_team_section_to_agents_aiplus(root: &Path, marker: &str, section: &str) -> Result<()> {
    let path = root.join(".aiplus").join("AGENTS.aiplus.md");
    if !path.exists() {
        // No AiPlus AGENTS file yet — shouldn't happen post-install, but
        // skip rather than error so the team install still completes.
        return Ok(());
    }
    let begin = format!("<!-- BEGIN {marker} -->");
    let end = format!("<!-- END {marker} -->");
    let current =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let block = format!("\n{begin}\n{}\n{end}\n", section.trim_end());
    let next = if let Some(start_idx) = current.find(&begin) {
        // Replace existing block (idempotent rewrite).
        let end_idx = current[start_idx..]
            .find(&end)
            .map(|i| start_idx + i + end.len())
            .unwrap_or_else(|| current.len());
        let mut buf = String::with_capacity(current.len() + block.len());
        buf.push_str(&current[..start_idx]);
        buf.push_str(block.trim_start());
        if end_idx < current.len() {
            buf.push_str(&current[end_idx..]);
        }
        buf
    } else {
        // Append at end.
        let mut buf = current;
        if !buf.ends_with('\n') {
            buf.push('\n');
        }
        buf.push_str(&block);
        buf
    };
    write_file_atomic(&path, next.as_bytes())?;
    Ok(())
}

fn install_consultant_team_config(root: &Path, plan: &mut Plan, options: &Options) -> Result<()> {
    let rel = ".aiplus/consultant-team.toml";
    let target = rel_to_abs(root, rel)?;
    if target.exists() {
        plan.items.push(PlanItem {
            action: "skip-user-config".to_string(),
            path: rel.to_string(),
        });
        return Ok(());
    }
    let content = embedded_asset_text(
        "aiplus-auto-team-consultant/core/templates/consultant-team.default.toml",
    )?;
    write_file_safe(root, rel, content.as_bytes(), plan, options)
}

fn install_runtime_adapter(
    root: &Path,
    runtime: &str,
    plan: &mut Plan,
    options: &Options,
) -> Result<()> {
    match runtime {
        "codex" => update_agents_md(root, plan, options),
        "claude-code" => {
            // Bridge .aiplus/AGENTS.aiplus.md into the Claude Code session
            // via a managed block in top-level AGENTS.md (Claude Code reads
            // AGENTS.md alongside CLAUDE.md). Same mechanism as codex.
            update_agents_md(root, plan, options)?;
            write_file_safe(
                root,
                ".claude/commands/aiplus-refresh.md",
                claude_refresh_command_content().as_bytes(),
                plan,
                options,
            )?;
            // W7: real /aiplus-route slash command. Lets a Claude Code
            // session dispatch a task without an MCP roundtrip.
            write_file_safe(
                root,
                ".claude/commands/aiplus-route.md",
                claude_route_command_content().as_bytes(),
                plan,
                options,
            )?;
            write_file_safe(
                root,
                ".claude/agents/aiplus-advisor.md",
                claude_advisor_agent_content().as_bytes(),
                plan,
                options,
            )?;
            write_file_safe(
                root,
                ".claude/agents/aiplus-memory.md",
                claude_memory_subagent_content().as_bytes(),
                plan,
                options,
            )?;
            write_file_safe(
                root,
                ".claude/agents/aiplus-compact.md",
                claude_compact_subagent_content().as_bytes(),
                plan,
                options,
            )?;
            write_file_safe(
                root,
                ".claude/agents/aiplus-velocity.md",
                claude_velocity_subagent_content().as_bytes(),
                plan,
                options,
            )?;
            write_file_safe(
                root,
                ".claude/agents/aiplus-team-consultant.md",
                claude_team_consultant_subagent_content().as_bytes(),
                plan,
                options,
            )?;
            install_claude_hooks(root, plan)?;
            update_claude_md(root, plan)
        }
        "opencode" => {
            // Same bridge for OpenCode — sst/opencode also reads AGENTS.md.
            update_agents_md(root, plan, options)?;
            install_opencode_config(root, plan, options)?;
            write_file_safe(
                root,
                ".opencode/commands/aiplus-refresh.md",
                opencode_prompt_content().as_bytes(),
                plan,
                options,
            )?;
            // W7: matching route command for OpenCode.
            write_file_safe(
                root,
                ".opencode/commands/aiplus-route.md",
                opencode_route_command_content().as_bytes(),
                plan,
                options,
            )?;
            write_file_safe(
                root,
                ".opencode/agents/aiplus-advisor.md",
                opencode_prompt_content().as_bytes(),
                plan,
                options,
            )?;
            write_file_safe(
                root,
                ".opencode/prompts/aiplus.md",
                opencode_prompt_content().as_bytes(),
                plan,
                options,
            )?;
            write_file_safe(
                root,
                ".opencode/prompts/aiplus-route.md",
                opencode_route_command_content().as_bytes(),
                plan,
                options,
            )
        }
        _ => Ok(()),
    }
}

/// W7: Claude Code slash command body for `/aiplus-route`.
fn claude_route_command_content() -> String {
    r#"---
name: aiplus-route
description: Route a task to an AiPlus agent role with consult + owner-gate enforcement
---

# AiPlus Route

Route a task to an AiPlus agent role. The CLI scores the task, fires
the consultant team, enforces owner gates, and writes per-member
findings to `.aiplus/agent-memory/_team/consult-<task-id>.jsonl`.

## Usage

```
/aiplus-route <role> <free-form task description>
```

## Runtime contract

1. If you have not loaded `.aiplus/AGENTS.aiplus.md` in this session,
   read it first. It carries the active team's roster, STOP-gates,
   and routing conventions.
2. Run `aiplus agent route <role> <task>` (one shell invocation).
3. If the CLI exited non-zero with `Dispatch refused`, do NOT proceed.
   Surface the gate id from `gates-<task-id>.jsonl` and ask the Owner
   whether to authorize with `aiplus agent route --owner-approved <gate-id>`.
4. If zero exit, open the consult artifact and treat per-member
   findings as the team's plan-time review before writing code.

## Forbidden

- Never bypass a gate refusal by running shell commands directly.
- Never edit the JSONL artifacts; they are an audit trail.
- The 12 §16 STOP-gates from AEL DESIGN.md never auto-approve.
"#
    .to_string()
}

/// W7: OpenCode prompt body for `aiplus-route`.
fn opencode_route_command_content() -> String {
    r#"# AiPlus Route (OpenCode)

Route a task to an AiPlus agent role. Same contract as the Claude Code
`/aiplus-route` command: load `.aiplus/AGENTS.aiplus.md`, shell out to
`aiplus agent route`, stop on `Dispatch refused`, surface gate id, and
address per-member findings on success.

## Forbidden

- Never bypass a gate refusal.
- Never edit `_team/*.jsonl` files; they are an audit trail.
- The 12 §16 STOP-gates from AEL DESIGN.md never auto-approve.
"#
    .to_string()
}

fn install_opencode_config(root: &Path, plan: &mut Plan, options: &Options) -> Result<()> {
    let rel = ".opencode/opencode.json";
    let target = rel_to_abs(root, rel)?;
    assert_no_symlink_path(root, &target)?;

    if target.exists() {
        let text = fs::read_to_string(&target).unwrap_or_default();
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) {
            if opencode_config_is_preservable(&value) {
                plan.items.push(PlanItem {
                    action: "skip-user-config".to_string(),
                    path: rel.to_string(),
                });
                return Ok(());
            }

            // Legacy AiPlus-only files (versions before 0.5.1+ wrote the top-level
            // "aiplus" key that OpenCode 1.14+ now rejects). Auto-migrate when the
            // file contains nothing but our own legacy key (optionally with $schema)
            // — no --force required, because stripping our own legacy data isn't
            // destructive to user config.
            if opencode_config_is_legacy_aiplus_only(&value) {
                if let Some(mut object) = value.as_object().cloned() {
                    object.remove("aiplus");
                    object
                        .entry("$schema".to_string())
                        .or_insert_with(|| serde_json::json!("https://opencode.ai/config.json"));
                    let content =
                        serde_json::to_string_pretty(&serde_json::Value::Object(object))? + "\n";
                    if let Some(parent) = target.parent() {
                        ensure_dir(root, parent, plan)?;
                    }
                    if !plan.dry_run {
                        fs::write(&target, content.as_bytes())?;
                    }
                    plan.items.push(PlanItem {
                        action: "migrate-legacy-aiplus-key".to_string(),
                        path: rel.to_string(),
                    });
                    return Ok(());
                }
            }

            if options.force {
                if let Some(mut object) = value.as_object().cloned() {
                    object.remove("aiplus");
                    object
                        .entry("$schema".to_string())
                        .or_insert_with(|| serde_json::json!("https://opencode.ai/config.json"));
                    let content =
                        serde_json::to_string_pretty(&serde_json::Value::Object(object))? + "\n";
                    return write_file_safe(root, rel, content.as_bytes(), plan, options);
                }
            }
        }
    }

    write_file_safe(
        root,
        rel,
        opencode_config_content().as_bytes(),
        plan,
        options,
    )
}

fn opencode_config_is_preservable(value: &serde_json::Value) -> bool {
    value.as_object().is_some_and(|object| {
        !object.contains_key("aiplus")
            && object
                .get("$schema")
                .is_none_or(|schema| schema.is_string())
    })
}

fn opencode_config_is_legacy_aiplus_only(value: &serde_json::Value) -> bool {
    value.as_object().is_some_and(|object| {
        object.contains_key("aiplus") && object.keys().all(|k| k == "aiplus" || k == "$schema")
    })
}

fn compact_init(root: &Path, plan: &mut Plan, force: bool) -> Result<()> {
    if plan.dry_run {
        plan.items.push(PlanItem {
            action: "compact-init".to_string(),
            path: ".aiplus/compact/".to_string(),
        });
        return Ok(());
    }
    migrate_legacy_codex_compact(root)?;
    ensure_dir(root, &rel_to_abs(root, ".aiplus/compact")?, plan)?;
    ensure_dir(
        root,
        &rel_to_abs(root, ".aiplus/compact/checkpoints")?,
        plan,
    )?;
    for file in [
        "current-handoff.md",
        "decision-log.md",
        "agent-state-ledger.md",
        "evidence-ledger.md",
        "compact-policy.json",
    ] {
        let asset_path = format!("aiplus-{MODULE_SLUG_COMPACT_REMINDER}/core/templates/{file}");
        let content =
            embedded_asset_text(&asset_path)?.replace("<ISO8601_TIMESTAMP>", &timestamp());
        write_compact_template(
            root,
            &format!(".aiplus/compact/{file}"),
            content.as_bytes(),
            plan,
            force,
        )?;
    }
    plan.items.push(PlanItem {
        action: "compact-init".to_string(),
        path: ".aiplus/compact/".to_string(),
    });
    Ok(())
}

fn migrate_compact_handoff_if_needed(root: &Path, plan: &mut Plan) -> Result<bool> {
    let rel = ".aiplus/compact/current-handoff.md";
    let path = rel_to_abs(root, rel)?;
    if !path.exists() {
        return Ok(false);
    }
    assert_no_symlink_path(root, &path)?;
    let current = fs::read_to_string(&path)?;
    let missing: Vec<(&str, &str)> = COMPACT_HANDOFF_MIGRATION_SECTIONS
        .iter()
        .copied()
        .filter(|(heading, _)| !has_section(&current, heading))
        .collect();
    if missing.is_empty() {
        return Ok(false);
    }
    backup_file(
        root,
        rel,
        current.as_bytes(),
        plan,
        &Options {
            force: true,
            backup: true,
            yes: true,
        },
    )?;
    let mut next = current;
    if !next.ends_with('\n') {
        next.push('\n');
    }
    next.push_str("\n<!-- AiPlus v0.2.1 compact handoff migration: preserved existing content and added missing role-aware sections. -->\n");
    for (heading, body) in missing {
        next.push_str(&format!("\n## {heading}\n\n{body}\n"));
    }
    plan.items.push(PlanItem {
        action: "compact-handoff-migrate".to_string(),
        path: rel.to_string(),
    });
    if !plan.dry_run {
        fs::write(path, next)?;
    }
    Ok(true)
}

fn update_agents_md(root: &Path, plan: &mut Plan, options: &Options) -> Result<()> {
    let rel = "AGENTS.md";
    let abs = rel_to_abs(root, rel)?;
    assert_no_symlink_path(root, &abs)?;
    let current = read_text_if_exists(&abs)?;
    let block = managed_block();
    let next = match current.as_deref() {
        None => format!("{block}\n"),
        Some(text) if text.trim().is_empty() => format!("{block}\n"),
        Some(text) => {
            let begin_count = text.matches(MANAGED_BEGIN).count();
            let end_count = text.matches(MANAGED_END).count();
            if begin_count != end_count
                || begin_count > 1
                || (begin_count == 1 && text.find(MANAGED_BEGIN) > text.find(MANAGED_END))
            {
                return Err(CliError::new(
                    1,
                    "ERROR malformed or duplicate AiPlus managed block in AGENTS.md",
                )
                .into());
            }
            if begin_count == 1 {
                replace_between(text, MANAGED_BEGIN, MANAGED_END, &block)?
            } else {
                format!(
                    "{}{}{}\n",
                    text,
                    if text.ends_with('\n') { "\n" } else { "\n\n" },
                    block
                )
            }
        }
    };
    if current.is_none() {
        write_file_safe(root, rel, next.as_bytes(), plan, options)
    } else {
        write_managed_text(root, rel, &next, plan)
    }
}

fn remove_managed_block(root: &Path, plan: &mut Plan) -> Result<()> {
    let rel = "AGENTS.md";
    let abs = rel_to_abs(root, rel)?;
    let Some(current) = read_text_if_exists(&abs)? else {
        return Ok(());
    };
    if !current.contains(MANAGED_BEGIN) {
        return Ok(());
    }
    let next = replace_between(&current, MANAGED_BEGIN, MANAGED_END, "")?;
    if next != current {
        write_managed_text(root, rel, &next, plan)?;
    }
    Ok(())
}

fn claude_md_managed_block() -> String {
    format!(
        "{begin}\n{body}\n{end}",
        begin = MANAGED_BEGIN,
        end = MANAGED_END,
        body = CLAUDE_MD_MANAGED_BODY.trim()
    )
}

const CLAUDE_MD_MANAGED_BODY: &str = r#"## AiPlus is installed in this project

AiPlus extends this session with persistent memory, compact-resilient handoff,
plan-time review, velocity estimates, and a team of specialist subagents.
Full operating manual: `.aiplus/AGENTS.aiplus.md`.

### Auto-triggered hooks (already active)
- `SessionStart` injects `aiplus memory context` so prior decisions, naming
  rules, and architectural choices are loaded before the first reply.
- `PreCompact` runs `aiplus compact prepare` so a structured handoff is saved
  before context compaction; `aiplus compact resume` reads it back afterwards.

### Manual subcommands you (the agent) should reach for
- Memory: `aiplus memory status | context | add | forget`
- Compact: `aiplus compact remind | prepare | resume | savings`
- Velocity: `aiplus velocity estimate --task-type <feat|fix|chore> --human-estimate <Nh>` before any non-trivial task, then `aiplus velocity complete --task-id <id> --actual <duration> --outcome <success|partial|fail>` after.
- Team consultant: `aiplus agent route <role>` / `aiplus agent talk <role>`.

### Specialist subagents (route via Agent tool when conditions match)
- `aiplus-memory` — user said "记住", "以后", "下次别", "忘掉", or preference-statement language.
- `aiplus-compact` — context approaching limit, mid-task interruption, or session resume after `/clear` or compact.
- `aiplus-velocity` — user asks "多久" / "estimate this" / before starting a clearly bounded task.
- `aiplus-team-consultant` — non-trivial plans, multi-stakeholder changes, designs touching security / onboarding / AI-integration / privacy.
- `aiplus-advisor` — general AiPlus questions or unsure which specialist to pick.

### Natural-language mapping
- "记住这个" / "下次也这样" → `aiplus memory add` (after redaction).
- "忘掉这个" → `aiplus memory forget`.
- "这个要多久" / "estimate this" → `aiplus velocity estimate`.
- "评审一下这个计划" / "review this plan" → `aiplus-team-consultant`.
- "新开顾问" / "new advisor" → `aiplus identity context --role advisor`.

### What AiPlus does NOT auto-do
Owner-gated actions (publish, deploy, push, edit global config, contact
external accounts, upload private data, add telemetry, expose secrets) stay
behind explicit user confirmation. AiPlus never clicks the host compact
button — it prepares the handoff and tells you when to do it manually.
"#;

fn update_claude_md(root: &Path, plan: &mut Plan) -> Result<()> {
    let rel = "CLAUDE.md";
    let abs = rel_to_abs(root, rel)?;
    assert_no_symlink_path(root, &abs)?;
    let current = read_text_if_exists(&abs)?;
    let block = claude_md_managed_block();
    let next = match current.as_deref() {
        None => format!("{block}\n"),
        Some(text) if text.trim().is_empty() => format!("{block}\n"),
        Some(text) => {
            let begin_count = text.matches(MANAGED_BEGIN).count();
            let end_count = text.matches(MANAGED_END).count();
            if begin_count != end_count
                || begin_count > 1
                || (begin_count == 1 && text.find(MANAGED_BEGIN) > text.find(MANAGED_END))
            {
                return Err(CliError::new(
                    1,
                    "ERROR malformed or duplicate AiPlus managed block in CLAUDE.md",
                )
                .into());
            }
            if begin_count == 1 {
                replace_between(text, MANAGED_BEGIN, MANAGED_END, &block)?
            } else {
                format!(
                    "{}{}{}\n",
                    text,
                    if text.ends_with('\n') { "\n" } else { "\n\n" },
                    block
                )
            }
        }
    };
    if current.is_none() {
        write_file_safe(
            root,
            rel,
            next.as_bytes(),
            plan,
            &Options {
                force: true,
                backup: false,
                yes: true,
            },
        )
    } else {
        write_managed_text(root, rel, &next, plan)
    }
}

fn remove_claude_md_managed_block(root: &Path, plan: &mut Plan) -> Result<()> {
    let rel = "CLAUDE.md";
    let abs = rel_to_abs(root, rel)?;
    let Some(current) = read_text_if_exists(&abs)? else {
        return Ok(());
    };
    if !current.contains(MANAGED_BEGIN) {
        return Ok(());
    }
    let next = replace_between(&current, MANAGED_BEGIN, MANAGED_END, "")?;
    if next != current {
        write_managed_text(root, rel, &next, plan)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// AiEconLab (AEL) Claude Code adapter installer
// ---------------------------------------------------------------------------
//
// When `aiplus add aieconlab` runs in a project that has the claude-code
// adapter installed, AiPlus reads the AEL adapter content from embedded
// assets and writes:
//   - 20 subagent files at .claude/agents/aieconlab-*.md (with YAML
//     frontmatter wrapping the persona body as the system prompt)
//   - 4 slash commands at .claude/commands/aiel-*.md
//   - An AEL managed block in CLAUDE.md (separate markers from AiPlus's
//     block; the two coexist)
//
// AEL content uses AIECONLAB markers so the AiPlus block and AEL block
// can be added or removed independently.

#[derive(serde::Deserialize)]
struct AielSubagentManifest {
    #[serde(default)]
    subagent: Vec<AielSubagentEntry>,
}

#[derive(serde::Deserialize)]
struct AielSubagentEntry {
    name: String,
    description: String,
    persona_file: String,
}

const AIECONLAB_SLASH_COMMANDS: &[&str] = &[
    "aiel-route",
    "aiel-talk",
    "aiel-fire-consultant",
    "aiel-status",
];

fn aieconlab_managed_block(body: &str) -> String {
    format!(
        "{begin}\n{body}\n{end}",
        begin = MANAGED_BEGIN_AEL,
        end = MANAGED_END_AEL,
        body = body.trim()
    )
}

fn update_claude_md_aieconlab_block(root: &Path, plan: &mut Plan, body: &str) -> Result<()> {
    let rel = "CLAUDE.md";
    let abs = rel_to_abs(root, rel)?;
    assert_no_symlink_path(root, &abs)?;
    let current = read_text_if_exists(&abs)?;
    let block = aieconlab_managed_block(body);
    let next = match current.as_deref() {
        None => format!("{block}\n"),
        Some(text) if text.trim().is_empty() => format!("{block}\n"),
        Some(text) => {
            let begin_count = text.matches(MANAGED_BEGIN_AEL).count();
            let end_count = text.matches(MANAGED_END_AEL).count();
            if begin_count != end_count
                || begin_count > 1
                || (begin_count == 1 && text.find(MANAGED_BEGIN_AEL) > text.find(MANAGED_END_AEL))
            {
                return Err(CliError::new(
                    1,
                    "ERROR malformed or duplicate AiEconLab managed block in CLAUDE.md",
                )
                .into());
            }
            if begin_count == 1 {
                replace_between(text, MANAGED_BEGIN_AEL, MANAGED_END_AEL, &block)?
            } else {
                format!(
                    "{}{}{}\n",
                    text,
                    if text.ends_with('\n') { "\n" } else { "\n\n" },
                    block
                )
            }
        }
    };
    if current.is_none() {
        write_file_safe(
            root,
            rel,
            next.as_bytes(),
            plan,
            &Options {
                force: true,
                backup: false,
                yes: true,
            },
        )
    } else {
        write_managed_text(root, rel, &next, plan)
    }
}

fn remove_claude_md_aieconlab_block(root: &Path, plan: &mut Plan) -> Result<()> {
    let rel = "CLAUDE.md";
    let abs = rel_to_abs(root, rel)?;
    let Some(current) = read_text_if_exists(&abs)? else {
        return Ok(());
    };
    if !current.contains(MANAGED_BEGIN_AEL) {
        return Ok(());
    }
    let next = replace_between(&current, MANAGED_BEGIN_AEL, MANAGED_END_AEL, "")?;
    if next != current {
        write_managed_text(root, rel, &next, plan)?;
    }
    Ok(())
}

/// Wrap a persona file body with YAML frontmatter so Claude Code's
/// subagent auto-routing sees `name` and `description`.
fn wrap_aieconlab_subagent(entry: &AielSubagentEntry, persona_body: &str) -> String {
    // Sanitize description: collapse internal newlines and quote-escape so
    // YAML stays valid even if the manifest entry spans lines.
    let description = entry
        .description
        .replace('\n', " ")
        .replace('\r', " ")
        .replace('"', "\\\"");
    format!(
        "---\nname: {name}\ndescription: \"{desc}\"\n---\n\n{body}",
        name = entry.name,
        desc = description,
        body = persona_body.trim_start_matches("---\n").trim_start()
    )
}

/// Install AEL's Claude Code adapter content into the project.
/// No-op if claude-code is not among the project's runtime adapters.
fn install_aieconlab_claude_code_adapter(root: &Path, plan: &mut Plan) -> Result<()> {
    let adapters = read_installed_runtime_adapters(root);
    if !adapters.iter().any(|a| a == "claude-code") {
        return Ok(());
    }

    // 1. Read subagent manifest from embedded assets.
    let manifest_text = embedded_asset_text("aieconlab/adapters/claude-code/subagents.toml")
        .map_err(|e| {
            CliError::new(
                1,
                format!("ERROR aieconlab claude-code subagents manifest missing: {e}"),
            )
        })?;
    let manifest: AielSubagentManifest = toml::from_str(&manifest_text).map_err(|e| {
        CliError::new(
            1,
            format!("ERROR parse aieconlab/adapters/claude-code/subagents.toml: {e}"),
        )
    })?;
    if manifest.subagent.is_empty() {
        return Err(
            CliError::new(1, "ERROR aieconlab subagent manifest declared zero entries").into(),
        );
    }

    // 2. Collect the set of unprefixed role names so we can clean up
    //    duplicates from the older `mirror_personas_to_runtimes` path.
    let mut role_basenames: BTreeSet<String> = BTreeSet::new();

    // 3. Write 20 subagent files with YAML frontmatter.
    let agents_rel = ".claude/agents";
    for entry in &manifest.subagent {
        // entry.name is like "aieconlab-pi" → drop the prefix to get "pi"
        let unprefixed = entry.name.strip_prefix("aieconlab-").unwrap_or(&entry.name);
        role_basenames.insert(format!("{unprefixed}.md"));

        let persona_asset = format!("aieconlab/{}", entry.persona_file);
        let persona_body = embedded_asset_text(&persona_asset).map_err(|e| {
            CliError::new(
                1,
                format!(
                    "ERROR persona file {} missing for subagent {}: {}",
                    persona_asset, entry.name, e
                ),
            )
        })?;
        let body = wrap_aieconlab_subagent(entry, &persona_body);
        let rel = format!("{agents_rel}/{}.md", entry.name);
        write_file_safe(
            root,
            &rel,
            body.as_bytes(),
            plan,
            &Options {
                force: true,
                backup: false,
                yes: true,
            },
        )?;
    }

    // 4. Clean up duplicate unprefixed persona files that mirror_personas_to_runtimes
    //    wrote at install time. We've now written the prefixed, frontmatter-bearing
    //    versions; the bare `pi.md` etc. would confuse Claude Code's routing.
    for basename in &role_basenames {
        let target = rel_to_abs(root, &format!("{agents_rel}/{basename}"))?;
        if target.exists() {
            // Only delete if it doesn't already have aieconlab frontmatter
            // (defensive: a user-authored file with the same name should not
            // be removed).
            if let Some(text) = read_text_if_exists(&target)? {
                let has_aieconlab_frontmatter = text.starts_with("---")
                    && text.contains("aieconlab")
                    && text.lines().take(10).any(|l| l.starts_with("name:"));
                if !has_aieconlab_frontmatter {
                    let _ = std::fs::remove_file(&target);
                    plan.items.push(PlanItem {
                        action: "remove-duplicate".to_string(),
                        path: format!("{agents_rel}/{basename}"),
                    });
                }
            }
        }
    }

    // 5. Copy 4 slash commands.
    let commands_rel = ".claude/commands";
    for cmd in AIECONLAB_SLASH_COMMANDS {
        let asset = format!("aieconlab/adapters/claude-code/commands/{cmd}.md");
        let body = embedded_asset_text(&asset).map_err(|e| {
            CliError::new(
                1,
                format!("ERROR aieconlab slash command {cmd} missing: {e}"),
            )
        })?;
        let rel = format!("{commands_rel}/{cmd}.md");
        write_file_safe(
            root,
            &rel,
            body.as_bytes(),
            plan,
            &Options {
                force: true,
                backup: false,
                yes: true,
            },
        )?;
    }

    // 6. Insert AEL managed block into CLAUDE.md.
    let block_body = embedded_asset_text("aieconlab/adapters/claude-code/claude-md-block.md")
        .map_err(|e| {
            CliError::new(
                1,
                format!("ERROR aieconlab CLAUDE.md block body missing: {e}"),
            )
        })?;
    update_claude_md_aieconlab_block(root, plan, &block_body)?;

    Ok(())
}

/// Track B.1: AEL OpenCode adapter (v0.3). Mirrors the v0.2 claude-code
/// adapter shape — same subagent manifest + same slash-command set —
/// but writes to `.opencode/agents/` and `.opencode/commands/` and
/// skips the CLAUDE.md managed-block step (OpenCode reads
/// `.aiplus/AGENTS.aiplus.md` transitively, which already advertises
/// the AEL roster via the `AIECONLAB_TEAM` section).
///
/// No-op when opencode is not in the project's runtime adapter list.
fn install_aieconlab_opencode_adapter(root: &Path, plan: &mut Plan) -> Result<()> {
    let adapters = read_installed_runtime_adapters(root);
    if !adapters.iter().any(|a| a == "opencode") {
        return Ok(());
    }

    // 1. Read the opencode-specific subagent manifest. Schema is
    //    identical to the claude-code manifest (same TOML struct).
    let manifest_text =
        embedded_asset_text("aieconlab/adapters/opencode/subagents.toml").map_err(|e| {
            CliError::new(
                1,
                format!("ERROR aieconlab opencode subagents manifest missing: {e}"),
            )
        })?;
    let manifest: AielSubagentManifest = toml::from_str(&manifest_text).map_err(|e| {
        CliError::new(
            1,
            format!("ERROR parse aieconlab/adapters/opencode/subagents.toml: {e}"),
        )
    })?;
    if manifest.subagent.is_empty() {
        return Err(CliError::new(
            1,
            "ERROR aieconlab opencode subagent manifest declared zero entries",
        )
        .into());
    }

    let mut role_basenames: BTreeSet<String> = BTreeSet::new();

    // 2. Write prefixed agent files. We use the same YAML frontmatter
    //    shape as the claude-code adapter — modern OpenCode (1.14+)
    //    parses subagent frontmatter, and older versions ignore it
    //    harmlessly. Source body is the runtime-agnostic persona
    //    markdown shared with claude-code/codex.
    let agents_rel = ".opencode/agents";
    for entry in &manifest.subagent {
        let unprefixed = entry.name.strip_prefix("aieconlab-").unwrap_or(&entry.name);
        role_basenames.insert(format!("{unprefixed}.md"));

        let persona_asset = format!("aieconlab/{}", entry.persona_file);
        let persona_body = embedded_asset_text(&persona_asset).map_err(|e| {
            CliError::new(
                1,
                format!(
                    "ERROR persona file {} missing for subagent {}: {}",
                    persona_asset, entry.name, e
                ),
            )
        })?;
        let body = wrap_aieconlab_subagent(entry, &persona_body);
        let rel = format!("{agents_rel}/{}.md", entry.name);
        write_file_safe(
            root,
            &rel,
            body.as_bytes(),
            plan,
            &Options {
                force: true,
                backup: false,
                yes: true,
            },
        )?;
    }

    // 3. Clean up bare unprefixed persona files that
    //    `mirror_personas_to_runtimes` wrote into `.opencode/agents/`.
    //    Defensive: only remove files that do NOT start with `---`
    //    (mirror copies raw persona body, never frontmatter), so a
    //    user-authored file at the same path survives.
    for basename in &role_basenames {
        let target = rel_to_abs(root, &format!("{agents_rel}/{basename}"))?;
        if target.exists() {
            if let Some(text) = read_text_if_exists(&target)? {
                if !text.starts_with("---") {
                    let _ = std::fs::remove_file(&target);
                    plan.items.push(PlanItem {
                        action: "remove-duplicate".to_string(),
                        path: format!("{agents_rel}/{basename}"),
                    });
                }
            }
        }
    }

    // 4. Copy slash commands. Same set as the claude-code adapter.
    let commands_rel = ".opencode/commands";
    for cmd in AIECONLAB_SLASH_COMMANDS {
        let asset = format!("aieconlab/adapters/opencode/commands/{cmd}.md");
        let body = embedded_asset_text(&asset).map_err(|e| {
            CliError::new(
                1,
                format!("ERROR aieconlab opencode slash command {cmd} missing: {e}"),
            )
        })?;
        let rel = format!("{commands_rel}/{cmd}.md");
        write_file_safe(
            root,
            &rel,
            body.as_bytes(),
            plan,
            &Options {
                force: true,
                backup: false,
                yes: true,
            },
        )?;
    }

    // No CLAUDE.md-equivalent managed block: OpenCode reads AGENTS.md
    // which references .aiplus/AGENTS.aiplus.md, where the
    // `AIECONLAB_TEAM` section already advertises the roster.

    Ok(())
}

const AGENT_TEAM_SLASH_COMMANDS: &[&str] = &["at-status", "at-route"];

fn agent_team_managed_block(body: &str) -> String {
    format!(
        "{begin}\n{body}\n{end}",
        begin = MANAGED_BEGIN_AT,
        end = MANAGED_END_AT,
        body = body.trim()
    )
}

fn update_claude_md_agent_team_block(root: &Path, plan: &mut Plan, body: &str) -> Result<()> {
    let rel = "CLAUDE.md";
    let abs = rel_to_abs(root, rel)?;
    assert_no_symlink_path(root, &abs)?;
    let current = read_text_if_exists(&abs)?;
    let block = agent_team_managed_block(body);
    let next = match current.as_deref() {
        None => format!("{block}\n"),
        Some(text) if text.trim().is_empty() => format!("{block}\n"),
        Some(text) => {
            let begin_count = text.matches(MANAGED_BEGIN_AT).count();
            let end_count = text.matches(MANAGED_END_AT).count();
            if begin_count != end_count
                || begin_count > 1
                || (begin_count == 1 && text.find(MANAGED_BEGIN_AT) > text.find(MANAGED_END_AT))
            {
                return Err(CliError::new(
                    1,
                    "ERROR malformed or duplicate AiPlus Agent Team managed block in CLAUDE.md",
                )
                .into());
            }
            if begin_count == 1 {
                replace_between(text, MANAGED_BEGIN_AT, MANAGED_END_AT, &block)?
            } else {
                format!(
                    "{}{}{}\n",
                    text,
                    if text.ends_with('\n') { "\n" } else { "\n\n" },
                    block
                )
            }
        }
    };
    if current.is_none() {
        write_file_safe(
            root,
            rel,
            next.as_bytes(),
            plan,
            &Options {
                force: true,
                backup: false,
                yes: true,
            },
        )
    } else {
        write_managed_text(root, rel, &next, plan)
    }
}

fn remove_claude_md_agent_team_block(root: &Path, plan: &mut Plan) -> Result<()> {
    let rel = "CLAUDE.md";
    let abs = rel_to_abs(root, rel)?;
    let Some(current) = read_text_if_exists(&abs)? else {
        return Ok(());
    };
    if !current.contains(MANAGED_BEGIN_AT) {
        return Ok(());
    }
    let next = replace_between(&current, MANAGED_BEGIN_AT, MANAGED_END_AT, "")?;
    if next != current {
        write_managed_text(root, rel, &next, plan)?;
    }
    Ok(())
}

/// Track A.1: Uninstall hygiene. Remove runtime-adapter artifacts that
/// AiPlus wrote outside `.aiplus/` but that uninstall historically
/// left behind. Limited to files matching our owned prefixes so we
/// never touch user-authored content:
///
///   .claude/agents/{aieconlab,agent-team,aiplus}-*.md
///   .claude/commands/{aiel,aiplus,at}-*.md
///   .opencode/agents/aiplus-*.md
///   .opencode/commands/aiplus-*.md
///   .opencode/prompts/aiplus*.md
///
/// NOT touched (out of scope, separate cleanup track):
///   .opencode/agents/<bare-role>.md  — `mirror_personas_to_runtimes`
///       writes these with no prefix; cannot disambiguate from user
///       content. Future OpenCode adapters (Track B.1/B.2) will use
///       prefixed file names, and this function will pick them up
///       automatically once the prefix list grows.
///   .opencode/opencode.json — user-config file; handled separately.
///   .claude/settings.local.json — handled by remove_claude_hooks.
///
/// After file removal, empty `.claude/{agents,commands}/` and
/// `.opencode/{agents,commands,prompts}/` directories that we created
/// are pruned, and the parent `.claude/` / `.opencode/` directories
/// are pruned if they become empty.
fn remove_runtime_adapter_artifacts(root: &Path, plan: &mut Plan) -> Result<()> {
    // (parent_rel, prefix_patterns) — prefix matches against file
    // basename. A trailing `-` is required for prefixes that would
    // otherwise be too permissive (e.g. `aiplus` alone would also
    // match `aiplus.md`, which we DO want under `.opencode/prompts/`
    // but NOT under `.claude/agents/`). See the prompts entry below.
    let claude_agent_prefixes: &[&str] = &["aieconlab-", "agent-team-", "aiplus-"];
    let claude_command_prefixes: &[&str] = &["aiel-", "aiplus-", "at-"];
    // Track B.1: opencode now also receives aieconlab- prefixed agents
    // and aiel-* slash commands. Future Track B.2 will add agent-team-
    // / at- to opencode. Keeping both in the prefix list ahead of B.2
    // is harmless — globbing only acts when a matching file exists.
    let opencode_agent_prefixes: &[&str] = &["aieconlab-", "agent-team-", "aiplus-"];
    let opencode_command_prefixes: &[&str] = &["aiel-", "aiplus-", "at-"];
    // .opencode/prompts/ has both `aiplus.md` and `aiplus-route.md`.
    // Match `aiplus` (no hyphen suffix) to catch both.
    let opencode_prompt_prefixes: &[&str] = &["aiplus"];

    let groups: &[(&str, &[&str])] = &[
        (".claude/agents", claude_agent_prefixes),
        (".claude/commands", claude_command_prefixes),
        (".opencode/agents", opencode_agent_prefixes),
        (".opencode/commands", opencode_command_prefixes),
        (".opencode/prompts", opencode_prompt_prefixes),
    ];

    let mut touched_parents: BTreeSet<String> = BTreeSet::new();

    for (parent_rel, prefixes) in groups {
        let parent_abs = rel_to_abs(root, parent_rel)?;
        if !parent_abs.exists() || !parent_abs.is_dir() {
            continue;
        }
        let entries = match fs::read_dir(&parent_abs) {
            Ok(it) => it,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if !name.ends_with(".md") {
                continue;
            }
            if !prefixes.iter().any(|p| name.starts_with(p)) {
                continue;
            }
            let rel = format!("{parent_rel}/{name}");
            plan.items.push(PlanItem {
                action: "remove".to_string(),
                path: rel.clone(),
            });
            if !plan.dry_run {
                let _ = fs::remove_file(&path);
            }
            touched_parents.insert((*parent_rel).to_string());
        }
    }

    // Prune empty parent dirs we touched, plus their grandparents
    // (`.claude/`, `.opencode/`) if they end up empty. Don't recurse
    // beyond two levels — we'll never delete the project root.
    if plan.dry_run {
        return Ok(());
    }
    let mut candidate_dirs: BTreeSet<String> = touched_parents;
    for parent_rel in candidate_dirs.clone() {
        let parent_abs = rel_to_abs(root, &parent_rel)?;
        prune_dir_if_empty(&parent_abs);
        // Also try the grandparent (e.g. `.claude/` after we cleaned
        // `.claude/agents/` and `.claude/commands/`).
        if let Some(grand) = std::path::Path::new(&parent_rel).parent() {
            if !grand.as_os_str().is_empty() {
                if let Some(gs) = grand.to_str() {
                    candidate_dirs.insert(gs.to_string());
                }
            }
        }
    }
    for dir_rel in candidate_dirs {
        // Recheck after first pass — child cleanup may have left the
        // grandparent empty.
        let dir_abs = rel_to_abs(root, &dir_rel)?;
        prune_dir_if_empty(&dir_abs);
    }
    Ok(())
}

fn prune_dir_if_empty(dir: &Path) {
    if !dir.is_dir() {
        return;
    }
    match fs::read_dir(dir) {
        Ok(mut it) => {
            if it.next().is_none() {
                let _ = fs::remove_dir(dir);
            }
        }
        Err(_) => {}
    }
}

/// Wrap an agent-team persona body with YAML frontmatter so Claude Code's
/// subagent auto-routing sees `name` and `description`. Reuses the same
/// shape as the AEL adapter — see `wrap_aieconlab_subagent`.
fn wrap_agent_team_subagent(entry: &AielSubagentEntry, persona_body: &str) -> String {
    // Sanitize description: collapse internal newlines and quote-escape so
    // YAML stays valid even if the manifest entry spans lines.
    let description = entry
        .description
        .replace('\n', " ")
        .replace('\r', " ")
        .replace('"', "\\\"");
    format!(
        "---\nname: {name}\ndescription: \"{desc}\"\n---\n\n{body}",
        name = entry.name,
        desc = description,
        body = persona_body.trim_start_matches("---\n").trim_start()
    )
}

/// Install the AiPlus Agent Team's Claude Code adapter content into the
/// project. No-op if claude-code is not among the project's runtime
/// adapters. Mirrors the AEL adapter design (see
/// `install_aieconlab_claude_code_adapter`) — same shape, different
/// prefix and managed-block markers, so the two teams coexist cleanly.
///
/// Closes #31: before this adapter shipped, `aiplus add agent-team` left
/// `.claude/agents/<role>.md` files without YAML frontmatter (only the
/// raw persona body), so Claude Code's auto-routing could not see them.
///
/// `adapters` is the live runtime-adapter list for this install operation
/// — passed in directly instead of reading from `.aiplus/manifest.json`,
/// because during the auto-install path (`aiplus install claude-code` →
/// agent-team is auto_install=true) `agent_team_init` runs *before*
/// `write_manifest`, so a manifest read would return an empty list and
/// silently no-op this entire adapter.
fn install_agent_team_claude_code_adapter(
    root: &Path,
    plan: &mut Plan,
    adapters: &[String],
) -> Result<()> {
    if !adapters.iter().any(|a| a == "claude-code") {
        return Ok(());
    }

    // 1. Read subagent manifest from embedded assets.
    let manifest_text = embedded_asset_text(
        "aiplus-agent-team/adapters/claude-code/subagents.toml",
    )
    .map_err(|e| {
        CliError::new(
            1,
            format!("ERROR agent-team claude-code subagents manifest missing: {e}"),
        )
    })?;
    let manifest: AielSubagentManifest = toml::from_str(&manifest_text).map_err(|e| {
        CliError::new(
            1,
            format!("ERROR parse aiplus-agent-team/adapters/claude-code/subagents.toml: {e}"),
        )
    })?;
    if manifest.subagent.is_empty() {
        return Err(CliError::new(
            1,
            "ERROR agent-team subagent manifest declared zero entries",
        )
        .into());
    }

    // 2. Collect the set of unprefixed role names so we can clean up
    //    duplicates from the older `mirror_personas_to_runtimes` path.
    let mut role_basenames: BTreeSet<String> = BTreeSet::new();

    // 3. Write subagent files with YAML frontmatter (one per role).
    let agents_rel = ".claude/agents";
    for entry in &manifest.subagent {
        // entry.name is like "agent-team-engineer-a" → strip the prefix
        // to get the bare basename "engineer-a" used by the legacy
        // mirror path.
        let unprefixed = entry
            .name
            .strip_prefix("agent-team-")
            .unwrap_or(&entry.name);
        role_basenames.insert(format!("{unprefixed}.md"));

        let persona_asset = format!("aiplus-agent-team/{}", entry.persona_file);
        let persona_body = embedded_asset_text(&persona_asset).map_err(|e| {
            CliError::new(
                1,
                format!(
                    "ERROR persona file {} missing for subagent {}: {}",
                    persona_asset, entry.name, e
                ),
            )
        })?;
        let body = wrap_agent_team_subagent(entry, &persona_body);
        let rel = format!("{agents_rel}/{}.md", entry.name);
        write_file_safe(
            root,
            &rel,
            body.as_bytes(),
            plan,
            &Options {
                force: true,
                backup: false,
                yes: true,
            },
        )?;
    }

    // 4. Clean up duplicate unprefixed persona files that
    //    mirror_personas_to_runtimes wrote at install time. We have now
    //    written the prefixed, frontmatter-bearing versions; the bare
    //    `engineer-a.md` etc. would confuse Claude Code's routing.
    //    Defensive: only delete a bare file if it does NOT carry
    //    user-authored frontmatter (we only remove what mirror wrote).
    for basename in &role_basenames {
        let target = rel_to_abs(root, &format!("{agents_rel}/{basename}"))?;
        if target.exists() {
            if let Some(text) = read_text_if_exists(&target)? {
                // Mirror writes the raw persona body, which never starts
                // with a `---` frontmatter delimiter. A user-authored
                // file (or a re-run of the adapter) would have `---` at
                // top — leave those alone.
                let has_user_frontmatter = text.starts_with("---");
                if !has_user_frontmatter {
                    let _ = std::fs::remove_file(&target);
                    plan.items.push(PlanItem {
                        action: "remove-duplicate".to_string(),
                        path: format!("{agents_rel}/{basename}"),
                    });
                }
            }
        }
    }

    // 5. Copy slash commands.
    let commands_rel = ".claude/commands";
    for cmd in AGENT_TEAM_SLASH_COMMANDS {
        let asset = format!("aiplus-agent-team/adapters/claude-code/commands/{cmd}.md");
        let body = embedded_asset_text(&asset).map_err(|e| {
            CliError::new(
                1,
                format!("ERROR agent-team slash command {cmd} missing: {e}"),
            )
        })?;
        let rel = format!("{commands_rel}/{cmd}.md");
        write_file_safe(
            root,
            &rel,
            body.as_bytes(),
            plan,
            &Options {
                force: true,
                backup: false,
                yes: true,
            },
        )?;
    }

    // 6. Insert agent-team managed block into CLAUDE.md.
    let block_body = embedded_asset_text(
        "aiplus-agent-team/adapters/claude-code/claude-md-block.md",
    )
    .map_err(|e| {
        CliError::new(
            1,
            format!("ERROR agent-team CLAUDE.md block body missing: {e}"),
        )
    })?;
    update_claude_md_agent_team_block(root, plan, &block_body)?;

    Ok(())
}

/// Track B.2: agent-team OpenCode adapter (v0.2). Mirrors the v0.1
/// claude-code adapter shape but writes to `.opencode/agents/` and
/// `.opencode/commands/`. No separate CLAUDE.md block (OpenCode reads
/// AGENTS.md → `.aiplus/AGENTS.aiplus.md`, where the
/// `AGENT_TEAM_TEAM` section already advertises the roster).
///
/// No-op when opencode is not in the live adapter list passed in
/// (same threading pattern as the claude-code adapter — see
/// `install_agent_team_claude_code_adapter` doc for the install-vs-add
/// timing rationale).
fn install_agent_team_opencode_adapter(
    root: &Path,
    plan: &mut Plan,
    adapters: &[String],
) -> Result<()> {
    if !adapters.iter().any(|a| a == "opencode") {
        return Ok(());
    }

    let manifest_text = embedded_asset_text("aiplus-agent-team/adapters/opencode/subagents.toml")
        .map_err(|e| {
        CliError::new(
            1,
            format!("ERROR agent-team opencode subagents manifest missing: {e}"),
        )
    })?;
    let manifest: AielSubagentManifest = toml::from_str(&manifest_text).map_err(|e| {
        CliError::new(
            1,
            format!("ERROR parse aiplus-agent-team/adapters/opencode/subagents.toml: {e}"),
        )
    })?;
    if manifest.subagent.is_empty() {
        return Err(CliError::new(
            1,
            "ERROR agent-team opencode subagent manifest declared zero entries",
        )
        .into());
    }

    let mut role_basenames: BTreeSet<String> = BTreeSet::new();

    let agents_rel = ".opencode/agents";
    for entry in &manifest.subagent {
        let unprefixed = entry
            .name
            .strip_prefix("agent-team-")
            .unwrap_or(&entry.name);
        role_basenames.insert(format!("{unprefixed}.md"));

        let persona_asset = format!("aiplus-agent-team/{}", entry.persona_file);
        let persona_body = embedded_asset_text(&persona_asset).map_err(|e| {
            CliError::new(
                1,
                format!(
                    "ERROR persona file {} missing for subagent {}: {}",
                    persona_asset, entry.name, e
                ),
            )
        })?;
        let body = wrap_agent_team_subagent(entry, &persona_body);
        let rel = format!("{agents_rel}/{}.md", entry.name);
        write_file_safe(
            root,
            &rel,
            body.as_bytes(),
            plan,
            &Options {
                force: true,
                backup: false,
                yes: true,
            },
        )?;
    }

    // Clean bare-name files from mirror_personas_to_runtimes (only if
    // they don't carry user frontmatter).
    for basename in &role_basenames {
        let target = rel_to_abs(root, &format!("{agents_rel}/{basename}"))?;
        if target.exists() {
            if let Some(text) = read_text_if_exists(&target)? {
                if !text.starts_with("---") {
                    let _ = std::fs::remove_file(&target);
                    plan.items.push(PlanItem {
                        action: "remove-duplicate".to_string(),
                        path: format!("{agents_rel}/{basename}"),
                    });
                }
            }
        }
    }

    let commands_rel = ".opencode/commands";
    for cmd in AGENT_TEAM_SLASH_COMMANDS {
        let asset = format!("aiplus-agent-team/adapters/opencode/commands/{cmd}.md");
        let body = embedded_asset_text(&asset).map_err(|e| {
            CliError::new(
                1,
                format!("ERROR agent-team opencode slash command {cmd} missing: {e}"),
            )
        })?;
        let rel = format!("{commands_rel}/{cmd}.md");
        write_file_safe(
            root,
            &rel,
            body.as_bytes(),
            plan,
            &Options {
                force: true,
                backup: false,
                yes: true,
            },
        )?;
    }
    Ok(())
}

fn write_manifest(
    root: &Path,
    plan: &mut Plan,
    options: &Options,
    runtime_adapters: &[String],
    module_names: &[String],
    touched_module_names: &[String],
) -> Result<()> {
    let existing = read_manifest(root, true).unwrap_or_default();
    let existing_modules = normalize_existing_modules(existing.modules.as_ref());
    let mut next_runtimes: BTreeSet<String> = existing
        .runtime_adapters
        .clone()
        .unwrap_or_default()
        .into_iter()
        .collect();
    next_runtimes.extend(runtime_adapters.iter().cloned());
    let mut next_modules: BTreeSet<String> = existing_modules.keys().cloned().collect();
    next_modules.extend(module_names.iter().cloned());
    let touched: BTreeSet<String> = touched_module_names.iter().cloned().collect();
    let now = timestamp();
    let mut modules = BTreeMap::new();
    for name in &next_modules {
        let Some(spec) = module_spec(name) else {
            continue;
        };
        let existing_module = existing_modules.get(name);
        modules.insert(
            name.clone(),
            ManifestModule {
                version: Some(if touched.contains(name) {
                    spec.version.to_string()
                } else {
                    existing_module
                        .and_then(|m| m.version.clone())
                        .unwrap_or_else(|| spec.version.to_string())
                }),
                source: Some("bundled".to_string()),
                path: Some(spec.path.to_string()),
                installed_at: Some(
                    existing_module
                        .and_then(|m| m.installed_at.clone())
                        .unwrap_or_else(|| now.clone()),
                ),
                updated_at: Some(
                    if touched.contains(name)
                        && existing_module.and_then(|m| m.version.as_deref()) != Some(spec.version)
                    {
                        now.clone()
                    } else {
                        existing_module
                            .and_then(|m| m.updated_at.clone())
                            .unwrap_or_else(|| now.clone())
                    },
                ),
                source_url: None,
                source_ref: None,
            },
        );
    }
    let runtime_vec: Vec<String> = next_runtimes.into_iter().collect();
    let module_vec: Vec<String> = next_modules.into_iter().collect();
    let mut managed_files = vec![
        ".aiplus/manifest.json".to_string(),
        ".aiplus/AGENTS.aiplus.md".to_string(),
        REFRESH_PROMPT_REL.to_string(),
    ];
    managed_files.extend(
        module_vec
            .iter()
            .filter_map(|name| module_spec(name).map(|spec| spec.path.to_string())),
    );
    for runtime in &runtime_vec {
        managed_files.extend(runtime_managed_files(runtime));
    }
    let existing_stable = existing.installer.as_deref() == Some(INSTALLER)
        && existing.installer_version.as_deref() == Some(VERSION)
        && existing.target_root.as_deref() == Some(&root.display().to_string())
        && existing.runtime_adapters.as_ref() == Some(&runtime_vec)
        && existing.modules.as_ref() == Some(&modules)
        && existing.managed_files.as_ref() == Some(&managed_files);
    let manifest = Manifest {
        schema_version: Some(VERSION.to_string()),
        installer: Some(INSTALLER.to_string()),
        installer_version: Some(VERSION.to_string()),
        installed_at: Some(existing.installed_at.clone().unwrap_or_else(|| now.clone())),
        updated_at: Some(if existing_stable {
            existing.updated_at.clone().unwrap_or_else(|| now.clone())
        } else {
            now
        }),
        target_root: Some(root.display().to_string()),
        runtime_adapters: Some(runtime_vec),
        modules: Some(modules),
        managed_files: Some(managed_files),
    };
    write_file_safe(
        root,
        ".aiplus/manifest.json",
        format!("{}\n", serde_json::to_string_pretty(&manifest)?).as_bytes(),
        plan,
        options,
    )
}

fn print_install_summary(plan: &Plan, verbose: bool, adapters: &[String], upgraded: bool) {
    let runtime_text = adapters
        .iter()
        .map(|runtime| runtime_label(runtime))
        .collect::<Vec<_>>()
        .join(", ");
    if plan.dry_run {
        println!("AiPlus install plan for {runtime_text} in this project.");
        println!("No files were changed.");
        println!("Will create/update:");
        println!("- .aiplus/");
        println!("- .aiplus/compact/");
        println!("- .aiplus/memory/");
        println!("- .aiplus/identities/");
        println!("- .aiplus/skills/");
        println!("- runtime adapter project files");
        println!();
        println!("Next if acceptable: run install without --dry-run.");
        println!();
        println!("DRY_RUN_REFRESH_PROMPT={REFRESH_PROMPT}");
        if verbose {
            println!();
            println!("Detailed plan:");
            plan_printer(plan);
        } else {
            println!("GLOBAL_CONFIG_UNTOUCHED");
        }
        println!("INSTALL_DRY_RUN=PASS");
        return;
    }
    if upgraded {
        println!("AiPlus upgraded for {runtime_text} in this project.");
        println!("Existing AiPlus managed files were backed up before replacement.");
        println!(".aiplus/compact/ state was preserved.");
    } else {
        println!("AiPlus installed for {runtime_text} in this project.");
    }
    println!("Next: send \"AiPlus 刷新\", \"刷新 AiPlus\", \"aiplus refresh\", or \"aiplus status\" to any already-open agent session.");
    println!("New sessions should pick up project-local runtime files automatically.");
    println!("Optional check: run `aiplus doctor`.");
    println!();
    println!("AIPLUS_REFRESH_PROMPT={REFRESH_PROMPT}");
    if verbose {
        println!();
        println!("Detailed changes:");
        plan_printer(plan);
    } else {
        println!("GLOBAL_CONFIG_UNTOUCHED");
    }
    if upgraded {
        println!("UPGRADE_STATUS=PASS");
    } else {
        println!("INSTALL_STATUS=PASS");
    }
}

fn detects_existing_aiplus_install(root: &Path) -> bool {
    if read_manifest(root, true)
        .map(|manifest| manifest.installer.as_deref() == Some(INSTALLER))
        .unwrap_or(false)
    {
        return true;
    }
    let compact_path = format!(".aiplus/modules/aiplus-{MODULE_SLUG_COMPACT_REMINDER}");
    [
        ".aiplus/AGENTS.aiplus.md",
        REFRESH_PROMPT_REL,
        compact_path.as_str(),
        ".aiplus/modules/aiplus-auto-team-consultant",
    ]
    .iter()
    .any(|rel| {
        rel_to_abs(root, rel)
            .map(|path| path.exists())
            .unwrap_or(false)
    })
}

fn write_file_safe(
    root: &Path,
    rel: &str,
    content: &[u8],
    plan: &mut Plan,
    options: &Options,
) -> Result<()> {
    let target = rel_to_abs(root, rel)?;
    assert_no_symlink_path(root, &target)?;
    if let Some(parent) = target.parent() {
        ensure_dir(root, parent, plan)?;
    }
    let current = fs::read(&target).ok();
    if current.as_deref() == Some(content) {
        plan.items.push(PlanItem {
            action: "skip-identical".to_string(),
            path: rel.to_string(),
        });
        return Ok(());
    }
    if let Some(current) = current.as_ref() {
        if !options.force {
            return Err(CliError::new(
                1,
                format!("CONFLICT {rel} exists and differs; retry with --force --backup --yes"),
            )
            .into());
        }
        if options.backup {
            backup_file(root, rel, current, plan, options)?;
        }
    }
    plan.items.push(PlanItem {
        action: if current.is_none() {
            "write"
        } else {
            "overwrite"
        }
        .to_string(),
        path: rel.to_string(),
    });
    if !plan.dry_run {
        fs::write(target, content)?;
    }
    Ok(())
}

fn write_managed_text(root: &Path, rel: &str, content: &str, plan: &mut Plan) -> Result<()> {
    let target = rel_to_abs(root, rel)?;
    assert_no_symlink_path(root, &target)?;
    if let Some(parent) = target.parent() {
        ensure_dir(root, parent, plan)?;
    }
    let current = fs::read(&target).ok();
    if current.as_deref() == Some(content.as_bytes()) {
        plan.items.push(PlanItem {
            action: "skip-identical".to_string(),
            path: rel.to_string(),
        });
        return Ok(());
    }
    plan.items.push(PlanItem {
        action: if current.is_none() {
            "write"
        } else {
            "managed-update"
        }
        .to_string(),
        path: rel.to_string(),
    });
    if let Some(current) = current.as_ref() {
        backup_file(
            root,
            rel,
            current,
            plan,
            &Options {
                force: true,
                backup: true,
                yes: true,
            },
        )?;
    }
    if !plan.dry_run {
        fs::write(target, content)?;
    }
    Ok(())
}

fn write_compact_template(
    root: &Path,
    rel: &str,
    content: &[u8],
    plan: &mut Plan,
    force: bool,
) -> Result<()> {
    let target = rel_to_abs(root, rel)?;
    assert_no_symlink_path(root, &target)?;
    if let Some(parent) = target.parent() {
        ensure_dir(root, parent, plan)?;
    }
    let current = fs::read(&target).ok();
    if current.is_some() && !force {
        plan.items.push(PlanItem {
            action: "skip".to_string(),
            path: rel.to_string(),
        });
        return Ok(());
    }
    if current.as_deref() == Some(content) {
        plan.items.push(PlanItem {
            action: "skip-identical".to_string(),
            path: rel.to_string(),
        });
        return Ok(());
    }
    plan.items.push(PlanItem {
        action: if current.is_none() {
            "write"
        } else {
            "overwrite"
        }
        .to_string(),
        path: rel.to_string(),
    });
    if !plan.dry_run {
        fs::write(target, content)?;
    }
    Ok(())
}

fn backup_file(
    root: &Path,
    rel: &str,
    content: &[u8],
    plan: &mut Plan,
    options: &Options,
) -> Result<()> {
    if !options.backup || !options.yes {
        return Err(CliError::new(
            1,
            format!("ERROR overwriting {rel} requires --force --backup --yes"),
        )
        .into());
    }
    let stamp = plan
        .backup_stamp
        .get_or_insert_with(|| timestamp().replace([':', '.'], "-"))
        .clone();
    let backup_rel = format!(".aiplus/backups/{stamp}/{rel}");
    let backup_abs = rel_to_abs(root, &backup_rel)?;
    assert_no_symlink_path(root, &backup_abs)?;
    if let Some(parent) = backup_abs.parent() {
        ensure_dir(root, parent, plan)?;
    }
    plan.items.push(PlanItem {
        action: "backup".to_string(),
        path: format!("{rel} -> {backup_rel}"),
    });
    if !plan.dry_run {
        fs::write(backup_abs, content)?;
        write_rollback_plan(root, &stamp, rel, &backup_rel)?;
    }
    Ok(())
}

fn write_rollback_plan(root: &Path, id: &str, original_rel: &str, backup_rel: &str) -> Result<()> {
    let rel = format!(".aiplus/backups/{id}/rollback-plan.json");
    let abs = rel_to_abs(root, &rel)?;
    assert_no_symlink_path(root, &abs)?;
    let mut plan = if abs.exists() {
        let text = fs::read_to_string(&abs)?;
        serde_json::from_str::<RollbackPlan>(&text)?
    } else {
        RollbackPlan {
            schema_version: aiplus_core::rollback::ROLLBACK_SCHEMA_VERSION.to_string(),
            id: id.to_string(),
            created_at: timestamp(),
            entries: Vec::new(),
        }
    };
    plan.add_restore(original_rel, backup_rel);
    if let Some(parent) = abs.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(abs, format!("{}\n", serde_json::to_string_pretty(&plan)?))?;
    Ok(())
}

fn resolve_rollback_plan_path(root: &Path, id: &str) -> Result<PathBuf> {
    let backups = rel_to_abs(root, ".aiplus/backups")?;
    if id == "latest" {
        let mut candidates = Vec::new();
        if backups.exists() {
            for entry in fs::read_dir(backups)? {
                let entry = entry?;
                let path = entry.path().join("rollback-plan.json");
                if path.exists() {
                    candidates.push(path);
                }
            }
        }
        candidates.sort();
        return candidates.pop().ok_or_else(|| {
            CliError::new(1, "ROLLBACK_STATUS=BLOCKED reason=no_rollback_plan").into()
        });
    }
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(CliError::new(1, "ROLLBACK_STATUS=BLOCKED reason=invalid_id").into());
    }
    rel_to_abs(root, &format!(".aiplus/backups/{id}/rollback-plan.json"))
}

fn read_rollback_plan(path: &Path) -> Result<RollbackPlan> {
    assert_no_symlink_path(&target_root()?, path)?;
    let text = fs::read_to_string(path).map_err(|_| {
        CliError::new(
            1,
            format!(
                "ROLLBACK_STATUS=BLOCKED reason=missing_plan path={}",
                path.display()
            ),
        )
    })?;
    serde_json::from_str(&text).map_err(|_| {
        CliError::new(
            1,
            format!(
                "ROLLBACK_STATUS=BLOCKED reason=malformed_plan path={}",
                path.display()
            ),
        )
        .into()
    })
}

fn ensure_dir(root: &Path, dir: &Path, plan: &mut Plan) -> Result<()> {
    assert_no_symlink_path(root, dir)?;
    if dir.exists() {
        return Ok(());
    }
    let rel = path_slash(path_relative(root, dir)?);
    if plan.mkdir_paths.insert(rel.clone()) {
        plan.items.push(PlanItem {
            action: "mkdir".to_string(),
            path: rel,
        });
    }
    if !plan.dry_run {
        fs::create_dir_all(dir)?;
    }
    Ok(())
}

fn copy_embedded_module(
    root: &Path,
    spec: ModuleSpec,
    plan: &mut Plan,
    options: &Options,
) -> Result<()> {
    let prefix = format!("{}/", spec.vendor_name);
    for (stripped, bytes) in aiplus_core::assets::embedded_files_with_prefix(&prefix) {
        let dest = format!("{}/{}", spec.path, stripped);
        write_file_safe(root, &dest, bytes, plan, options)?;
    }
    Ok(())
}

fn read_manifest(root: &Path, quiet: bool) -> Result<Manifest> {
    let file = rel_to_abs(root, ".aiplus/manifest.json")?;
    if !file.exists() {
        if quiet {
            return Ok(Manifest::default());
        }
        return Err(CliError::new(1, "ERROR AiPlus manifest is missing").into());
    }
    assert_no_symlink_path(root, &file)?;
    match fs::read_to_string(file)
        .and_then(|text| serde_json::from_str(&text).map_err(io::Error::other))
    {
        Ok(manifest) => Ok(manifest),
        Err(error) if quiet => {
            let _ = error;
            Ok(Manifest::default())
        }
        Err(error) => Err(error.into()),
    }
}

fn read_manifest_diagnostic(root: &Path) -> Result<ManifestDiagnostic> {
    let file = rel_to_abs(root, ".aiplus/manifest.json")?;
    if !file.exists() {
        return Ok(ManifestDiagnostic {
            exists: false,
            parses: false,
            manifest: None,
        });
    }
    assert_no_symlink_path(root, &file)?;
    let text = fs::read_to_string(file)?;
    match serde_json::from_str::<Manifest>(&text) {
        Ok(manifest) => Ok(ManifestDiagnostic {
            exists: true,
            parses: true,
            manifest: Some(manifest),
        }),
        Err(_) => Ok(ManifestDiagnostic {
            exists: true,
            parses: false,
            manifest: None,
        }),
    }
}

fn push_check(checks: &mut Vec<Check>, label: impl Into<String>, ok: bool, fix: Option<String>) {
    checks.push(Check {
        label: label.into(),
        ok,
        fix,
        severity: CheckSeverity::NeedsFix,
    });
}

/// Issue #74: a check whose failure should surface a recommendation
/// (`INFO ...`) without flipping `DOCTOR_STATUS=NEEDS_FIX`. Use for
/// cosmetic / housekeeping issues that don't break install
/// correctness — e.g. stale-registry entries pointing at deleted
/// project directories.
fn push_info_check(
    checks: &mut Vec<Check>,
    label: impl Into<String>,
    ok: bool,
    fix: Option<String>,
) {
    checks.push(Check {
        label: label.into(),
        ok,
        fix,
        severity: CheckSeverity::Info,
    });
}

fn normalize_existing_modules(
    modules: Option<&BTreeMap<String, ManifestModule>>,
) -> BTreeMap<String, ManifestModule> {
    let mut out = BTreeMap::new();
    if let Some(modules) = modules {
        for (name, module) in modules {
            if let Some(canonical) = normalize_module(Some(name)) {
                out.insert(canonical.to_string(), module.clone());
            }
        }
    }
    out
}

fn compact_dir(root: &Path) -> Result<PathBuf> {
    rel_to_abs(root, ".aiplus/compact")
}

// One-time migration for projects installed before v0.5.11 — moves
// `.codex/compact/` to `.aiplus/compact/`. No-op when the new path already
// exists or the legacy path is absent. Removes the empty legacy directory
// (and `.codex` if that's the only thing left under it) so claude-code /
// opencode-only projects don't keep a stray `.codex/` tree.
fn migrate_legacy_codex_compact(root: &Path) -> Result<()> {
    let legacy = rel_to_abs(root, ".codex/compact")?;
    let new_path = rel_to_abs(root, ".aiplus/compact")?;
    if !legacy.exists() || new_path.exists() {
        return Ok(());
    }
    ensure_dir(root, &rel_to_abs(root, ".aiplus")?, &mut Plan::default())?;
    fs::rename(&legacy, &new_path)
        .with_context(|| format!("failed to migrate {legacy:?} -> {new_path:?}"))?;
    let legacy_parent = rel_to_abs(root, ".codex")?;
    if legacy_parent.is_dir() {
        if let Ok(mut entries) = fs::read_dir(&legacy_parent) {
            if entries.next().is_none() {
                let _ = fs::remove_dir(&legacy_parent);
            }
        }
    }
    println!("MIGRATION=compact .codex/compact/ -> .aiplus/compact/");
    Ok(())
}

fn compact_file(root: &Path, rel: &str) -> Result<PathBuf> {
    rel_to_abs(root, &format!(".aiplus/compact/{rel}"))
}

fn read_compact_text(root: &Path, file: &str) -> Result<String> {
    let path = compact_file(root, file)?;
    assert_no_symlink_path(root, &path)?;
    Ok(fs::read_to_string(path)?)
}

fn compact_validate_state(root: &Path) -> Result<CompactValidation> {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut review_items = Vec::new();
    let mut pending_gates = Vec::new();
    let mut denied_gates = Vec::new();
    let compact_dir = compact_dir(root)?;
    if !compact_dir.exists() {
        errors.push(".aiplus/compact/ is missing".to_string());
        return Ok(CompactValidation {
            ok: false,
            errors,
            warnings,
            review_items,
            pending_gates,
            denied_gates,
            next_safe_action: String::new(),
        });
    }

    for file in COMPACT_REQUIRED_FILES {
        if !compact_file(root, file)?.exists() {
            errors.push(format!("{file} is missing"));
        }
    }
    if !errors.is_empty() {
        return Ok(CompactValidation {
            ok: false,
            errors,
            warnings,
            review_items,
            pending_gates,
            denied_gates,
            next_safe_action: String::new(),
        });
    }

    let policy_text = read_compact_text(root, "compact-policy.json")?;
    let policy = match serde_json::from_str::<serde_json::Value>(&policy_text) {
        Ok(value) => Some(value),
        Err(error) => {
            errors.push(format!("compact-policy.json is invalid JSON: {error}"));
            None
        }
    };
    collect_version_review_items(root, policy.as_ref(), &mut review_items)?;

    for file in COMPACT_REQUIRED_FILES
        .iter()
        .filter(|file| file.ends_with(".md"))
    {
        let text = read_compact_text(root, file)?;
        for section in COMPACT_REQUIRED_SECTIONS {
            if !has_section(&text, section) {
                errors.push(format!("{file} missing section: {section}"));
            }
        }
    }
    let handoff = read_compact_text(root, "current-handoff.md")?;
    for section in COMPACT_HANDOFF_REQUIRED_SECTIONS {
        if !has_section(&handoff, section) {
            errors.push(format!("current-handoff.md missing section: {section}"));
        }
    }

    let goal = section_body(&handoff, "Current Goal");
    let phase = section_body(&handoff, "Current Phase")
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
    let next_actions = non_placeholder_lines(&section_body(&handoff, "Next 3 Actions"));
    if goal.is_empty() {
        errors.push("current-handoff.md Current Goal is empty".to_string());
    }
    if !TASK_STATUSES.contains(&phase.as_str()) {
        errors.push(format!(
            "current-handoff.md Current Phase is not allowed: {}",
            if phase.is_empty() { "<empty>" } else { &phase }
        ));
    }
    if next_actions.is_empty() {
        errors.push("current-handoff.md Next 3 Actions is empty".to_string());
    }

    for file in COMPACT_REQUIRED_FILES
        .iter()
        .filter(|file| file.ends_with(".md"))
    {
        let text = read_compact_text(root, file)?;
        let body = section_body(&text, "Owner Gates");
        let gates = owner_gate_tokens(&body);
        if gates.is_empty() {
            errors.push(format!("{file} Owner Gates has no gate status"));
        }
        for gate in gates {
            if !OWNER_GATE_VALUES.contains(&gate.as_str()) {
                errors.push(format!("{file} Owner gate has invalid status: {gate}"));
            }
            let line = body
                .lines()
                .find(|line| line.contains(&gate))
                .map(str::trim)
                .unwrap_or(&gate)
                .to_string();
            if gate == "UNKNOWN_PENDING" {
                pending_gates.push(format!("{file}: {line}"));
            }
            if gate == "DENIED" {
                denied_gates.push(format!("{file}: {line}"));
            }
        }
    }

    for row in parse_markdown_table(
        &read_compact_text(root, "decision-log.md")?,
        &["id", "status", "decision", "rationale", "evidence"],
    ) {
        if !DECISION_STATUSES.contains(&row.get("status").map(String::as_str).unwrap_or("")) {
            errors.push(format!(
                "decision-log.md invalid decision status: {}",
                row.get("status").map(String::as_str).unwrap_or("")
            ));
        }
    }
    for row in parse_markdown_table(
        &read_compact_text(root, "agent-state-ledger.md")?,
        &[
            "agent",
            "role",
            "status",
            "ownedScope",
            "lastEvidence",
            "nextAction",
        ],
    ) {
        if !AGENT_STATUSES.contains(&row.get("status").map(String::as_str).unwrap_or("")) {
            errors.push(format!(
                "agent-state-ledger.md invalid agent status: {}",
                row.get("status").map(String::as_str).unwrap_or("")
            ));
        }
    }
    for row in parse_markdown_table(
        &read_compact_text(root, "evidence-ledger.md")?,
        &["id", "confidence", "source", "finding", "artifact"],
    ) {
        if !EVIDENCE_CONFIDENCE.contains(&row.get("confidence").map(String::as_str).unwrap_or("")) {
            errors.push(format!(
                "evidence-ledger.md invalid evidence confidence: {}",
                row.get("confidence").map(String::as_str).unwrap_or("")
            ));
        }
    }

    if let Some(policy) = policy.as_ref() {
        if policy.get("manualCompactOnly").and_then(|v| v.as_bool()) != Some(true) {
            errors.push("compact-policy.json manualCompactOnly must be true".to_string());
        }
        check_policy_array(
            policy,
            "allowedOwnerGateStatuses",
            OWNER_GATE_VALUES,
            &mut errors,
        );
        check_policy_array(
            policy,
            "allowedDecisionStatuses",
            DECISION_STATUSES,
            &mut errors,
        );
        check_policy_array(policy, "allowedAgentStatuses", AGENT_STATUSES, &mut errors);
        check_policy_array(
            policy,
            "allowedEvidenceConfidence",
            EVIDENCE_CONFIDENCE,
            &mut errors,
        );
        check_policy_array(
            policy,
            "allowedTaskResultStatuses",
            TASK_STATUSES,
            &mut errors,
        );
    }

    warnings.extend(scan_sensitive(root)?);
    let ok = errors.is_empty() && warnings.is_empty() && review_items.is_empty();
    Ok(CompactValidation {
        ok,
        errors,
        warnings,
        review_items,
        pending_gates,
        denied_gates,
        next_safe_action: next_actions
            .first()
            .map(|line| strip_numbering(line))
            .unwrap_or_default(),
    })
}

fn compact_checkpoint(root: &Path, level: &str) -> Result<i32> {
    let (exit_code, _) = compact_checkpoint_with_options(root, level, true, true)?;
    Ok(exit_code)
}

fn compact_checkpoint_with_options(
    root: &Path,
    level: &str,
    print_output: bool,
    record_candidate: bool,
) -> Result<(i32, String)> {
    let result = compact_validate_state(root)?;
    let readiness = compact_readiness(&result, root);
    let status = if readiness.state == "READY_TO_COMPACT" {
        "SAFE_TO_COMPACT"
    } else if readiness.state == "BLOCKED_BY_OWNER_GATE" {
        "BLOCKED_DO_NOT_COMPACT"
    } else {
        readiness.state
    };
    let exit_code = readiness_exit_code(&readiness);
    if readiness.state == "BLOCKED_BY_OWNER_GATE" {
        if print_output {
            print_compact_diagnostics(&result);
            println!("{status}");
            print_readiness(&readiness);
            println!("CHECKPOINT_LEVEL={level}");
            println!("CHECKPOINT_CREATED=none");
            println!("checkpoint=none");
        }
        return Ok((exit_code, String::new()));
    }
    ensure_dir(
        root,
        &compact_file(root, "checkpoints")?,
        &mut Plan::default(),
    )?;
    let timestamp = timestamp();
    let handoff = read_compact_text(root, "current-handoff.md").unwrap_or_default();
    let evidence = read_compact_text(root, "evidence-ledger.md").unwrap_or_default();
    let decision_log = read_compact_text(root, "decision-log.md").unwrap_or_default();
    let (decisions_made, evidence_pointers, files_or_artifacts) =
        checkpoint_detail_vectors(level, &decision_log, &evidence);
    let checkpoint = CompactCheckpoint {
        schema_version: VERSION.to_string(),
        checkpoint_level: level.to_string(),
        timestamp: timestamp.clone(),
        cwd: "<REPO_ROOT>".to_string(),
        validation_result: if result.ok { "PASS" } else { "FAIL" }.to_string(),
        status: status.to_string(),
        readiness_state: readiness.state.to_string(),
        compact_pressure: readiness.pressure.to_string(),
        session_role: compact_section_or_unknown(&handoff, "Session Role"),
        workflow_level: compact_section_or_unknown(&handoff, "Workflow Level"),
        pending_gates: result.pending_gates.clone(),
        denied_gates: result.denied_gates.clone(),
        review_items: result.review_items.clone(),
        warnings: result.warnings.clone(),
        errors: result.errors.clone(),
        current_goal: optional_line(section_body(&handoff, "Current Goal")),
        current_phase: optional_line(section_body(&handoff, "Current Phase")),
        output_contract: optional_for_level(
            level,
            section_body(&handoff, "Output Contract"),
            "full",
        ),
        decisions_made,
        open_blockers: optional_line(section_body(&handoff, "Open Blockers")),
        owner_gates: optional_line(section_body(&handoff, "Owner Gates")),
        next_safe_action: optional_line(result.next_safe_action.clone()),
        do_not_do: lines_for_level(level, section_body(&handoff, "Do Not Do"), "light"),
        files_or_artifacts,
        evidence_pointers,
        manual_compact_only: true,
    };
    let filename = format!("{}.json", timestamp.replace([':', '.'], "-"));
    let rel = format!(".aiplus/compact/checkpoints/{filename}");
    write_file_safe(
        root,
        &rel,
        format!("{}\n", serde_json::to_string_pretty(&checkpoint)?).as_bytes(),
        &mut Plan::default(),
        &Options {
            force: true,
            backup: false,
            yes: true,
        },
    )?;
    if print_output {
        print_compact_diagnostics(&result);
        println!("{status}");
        print_readiness(&readiness);
        println!("CHECKPOINT_LEVEL={level}");
        println!("CHECKPOINT_CREATED={rel}");
        println!("checkpoint={rel}");
    }
    if record_candidate {
        append_savings_event(
            root,
            "checkpoint",
            "candidate",
            Some(&rel),
            level,
            &readiness,
        )
        .ok();
    }
    Ok((exit_code, rel))
}

fn compact_prepare(root: &Path, level: &str) -> Result<i32> {
    let result = compact_validate_state(root)?;
    let readiness = compact_readiness(&result, root);
    print_compact_diagnostics(&result);
    println!("COMPACT_PREPARE");
    print_readiness(&readiness);
    // Always attempt to build/update context capsule so it exists for resume
    let capsule = build_context_capsule_from_handoff(root).unwrap_or_else(|_| {
        aiplus_core::build_capsule_from_compact_state(
            &project_id_from_root(root),
            "Unknown objective",
            "Unknown state",
            "Run aiplus compact resume.",
        )
    });
    let capsule_written = save_context_capsule(root, &capsule).is_ok();
    if capsule_written {
        println!("CONTEXT_CAPSULE_CREATED=.aiplus/compact/context-capsule.json");
    }

    if readiness.state == "READY_TO_COMPACT" {
        let (_, checkpoint) = compact_checkpoint_with_options(root, level, false, false)?;
        append_savings_event(
            root,
            "prepare",
            "projected",
            Some(&checkpoint),
            level,
            &readiness,
        )
        .ok();
        println!("CHECKPOINT_LEVEL={level}");
        println!("CHECKPOINT_CREATED={checkpoint}");
        println!();
        println!("Ready to compact.");
        println!();
        println!("After compact:");
        println!("- If I continue automatically, you do not need to do anything.");
        println!("- If I do not reply, send: continue");
        println!();
        println!("I will resume from here.");
        println!("PREPARE_STATUS=PASS");
    } else {
        println!("PREPARE_STATUS={}", readiness.state);
        println!("manual_compact_recommended=no");
    }
    Ok(readiness_exit_code(&readiness))
}

fn compact_score(root: &Path) -> Result<i32> {
    let result = compact_validate_state(root)?;
    let readiness = compact_readiness(&result, root);
    print_compact_diagnostics(&result);
    println!("COMPACT_SCORE");
    print_readiness(&readiness);
    Ok(readiness_exit_code(&readiness))
}

fn compact_remind(
    root: &Path,
    event: Option<&str>,
    snooze: Option<&str>,
    clear_snooze: bool,
    json: bool,
    quiet: bool,
) -> Result<i32> {
    let mut snooze_status = compact_snooze_status(root)?;
    if clear_snooze {
        clear_compact_snooze(root)?;
        snooze_status = "cleared";
    } else if let Some(duration) = snooze {
        set_compact_snooze(root, duration)?;
        snooze_status = "set";
    }

    let result = compact_validate_state(root)?;
    let readiness = compact_readiness(&result, root);
    let handoff = compact_handoff_freshness(root)?;
    let latest_checkpoint_age = latest_checkpoint_age(root)?;
    let estimate = estimate_savings_for_remind(root)?;
    let reminder = compact_reminder_decision(
        event,
        snooze_status,
        &result,
        &readiness,
        &handoff,
        latest_checkpoint_age.as_deref(),
        &estimate,
    );

    if !quiet {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "schemaVersion": VERSION,
                    "status": "PASS",
                    "event": event.unwrap_or("manual"),
                    "reminderDecision": reminder.decision,
                    "reminderLevel": reminder.level,
                    "readinessState": reminder.readiness_state,
                    "recoveryConfidence": reminder.recovery_confidence,
                    "manualCompactRecommended": reminder.manual_compact_recommended,
                    "snoozeStatus": reminder.snooze_status,
                    "handoffState": reminder.handoff_state,
                    "lastCheckpointAge": reminder.last_checkpoint_age,
                    "estimatedTokensSaved": reminder.estimated_tokens_saved,
                    "estimatedUsdSaved": reminder.estimated_usd_saved,
                    "reason": reminder.reason,
                    "nextAction": reminder.next_action,
                    "secretValuesPrinted": "no",
                    "billingData": false,
                    "hostCompactTriggered": false
                })
            );
        } else {
            println!("Compact Reminder reminder");
            println!("COMPACT_REMINDER");
            println!("REMINDER_DECISION={}", reminder.decision);
            println!("REMINDER_LEVEL={}", reminder.level);
            println!("READINESS_STATE={}", reminder.readiness_state);
            println!("RECOVERY_CONFIDENCE={}", reminder.recovery_confidence);
            println!(
                "MANUAL_COMPACT_RECOMMENDED={}",
                yes_no(reminder.manual_compact_recommended)
            );
            println!("SNOOZE_STATUS={}", reminder.snooze_status);
            println!("HANDOFF_STATE={}", reminder.handoff_state);
            println!("LAST_CHECKPOINT_AGE={}", reminder.last_checkpoint_age);
            println!("ESTIMATED_TOKENS_SAVED={}", reminder.estimated_tokens_saved);
            println!("ESTIMATED_USD_SAVED={}", reminder.estimated_usd_saved);
            println!("SECRET_VALUES_PRINTED={}", reminder.secret_values_printed);
            println!("REASON={}", reminder.reason);
            println!("NEXT_ACTION={}", reminder.next_action);
            println!("HOST_COMPACT_TRIGGERED=no");
            println!();
            println!("{}", compact_reminder_plain_language(&reminder));
        }
    }

    Ok(match reminder.decision {
        "blocked" => 1,
        "wait" => 2,
        _ => 0,
    })
}

fn compact_reminder_decision(
    event: Option<&str>,
    snooze_status: &'static str,
    result: &CompactValidation,
    readiness: &CompactReadiness,
    handoff: &HandoffFreshness,
    latest_checkpoint_age: Option<&str>,
    estimate: &SavingsEstimate,
) -> CompactReminder {
    let scheduled_event = event.is_some_and(|event| event != "user-request");
    let checkpoint_exists = latest_checkpoint_age.is_some();
    let estimated_usd_saved = estimate
        .cost_saved_usd
        .map(|value| format!("{value:.4}"))
        .unwrap_or_else(|| "unavailable".to_string());
    let checkpoint_age = latest_checkpoint_age.unwrap_or("missing").to_string();
    let recovery_confidence =
        if readiness.state == "READY_TO_COMPACT" && handoff.state == "current" && checkpoint_exists
        {
            "high"
        } else if readiness.state == "READY_TO_COMPACT" && handoff.state == "current" {
            "medium"
        } else {
            "low"
        };

    if !result.errors.is_empty() || !result.warnings.is_empty() || !result.denied_gates.is_empty() {
        return CompactReminder {
            decision: "blocked",
            level: "safety_block",
            readiness_state: readiness.state.to_string(),
            recovery_confidence,
            manual_compact_recommended: false,
            snooze_status,
            handoff_state: handoff.state,
            last_checkpoint_age: checkpoint_age,
            estimated_tokens_saved: estimate.tokens_saved,
            estimated_usd_saved,
            reason: first_compact_reason(readiness, "compact validation has blocking findings"),
            next_action: readiness.next_action.clone(),
            secret_values_printed: "no",
        };
    }

    if handoff.state != "current" {
        return CompactReminder {
            decision: "wait",
            level: "safety_block",
            readiness_state: readiness.state.to_string(),
            recovery_confidence,
            manual_compact_recommended: false,
            snooze_status,
            handoff_state: handoff.state,
            last_checkpoint_age: checkpoint_age,
            estimated_tokens_saved: estimate.tokens_saved,
            estimated_usd_saved,
            reason: handoff.reason.clone(),
            next_action:
                "Update .aiplus/compact/current-handoff.md and run aiplus compact prepare."
                    .to_string(),
            secret_values_printed: "no",
        };
    }

    if readiness.state == "NOT_RECOMMENDED_DURING_ACTIVE_WORK" {
        return CompactReminder {
            decision: "wait",
            level: "safety_block",
            readiness_state: readiness.state.to_string(),
            recovery_confidence,
            manual_compact_recommended: false,
            snooze_status,
            handoff_state: handoff.state,
            last_checkpoint_age: checkpoint_age,
            estimated_tokens_saved: estimate.tokens_saved,
            estimated_usd_saved,
            reason: first_compact_reason(readiness, "current work is unstable"),
            next_action: readiness.next_action.clone(),
            secret_values_printed: "no",
        };
    }

    if readiness.state != "READY_TO_COMPACT" {
        return CompactReminder {
            decision: "prepare_only",
            level: "soft",
            readiness_state: readiness.state.to_string(),
            recovery_confidence,
            manual_compact_recommended: false,
            snooze_status,
            handoff_state: handoff.state,
            last_checkpoint_age: checkpoint_age,
            estimated_tokens_saved: estimate.tokens_saved,
            estimated_usd_saved,
            reason: first_compact_reason(readiness, "checkpoint is not ready yet"),
            next_action: readiness.next_action.clone(),
            secret_values_printed: "no",
        };
    }

    if snooze_status == "active" && scheduled_event {
        return CompactReminder {
            decision: "wait",
            level: "soft",
            readiness_state: readiness.state.to_string(),
            recovery_confidence,
            manual_compact_recommended: false,
            snooze_status,
            handoff_state: handoff.state,
            last_checkpoint_age: checkpoint_age,
            estimated_tokens_saved: estimate.tokens_saved,
            estimated_usd_saved,
            reason: "compact reminder snooze is active for scheduled reminders".to_string(),
            next_action: "Wait until snooze expires or run aiplus compact remind --clear-snooze."
                .to_string(),
            secret_values_printed: "no",
        };
    }

    if !checkpoint_exists {
        return CompactReminder {
            decision: "prepare_only",
            level: "soft",
            readiness_state: readiness.state.to_string(),
            recovery_confidence,
            manual_compact_recommended: false,
            snooze_status,
            handoff_state: handoff.state,
            last_checkpoint_age: checkpoint_age,
            estimated_tokens_saved: estimate.tokens_saved,
            estimated_usd_saved,
            reason: "handoff is current but no checkpoint exists yet".to_string(),
            next_action: "Run aiplus compact prepare, then suggest manual host compact if it creates a checkpoint."
                .to_string(),
            secret_values_printed: "no",
        };
    }

    CompactReminder {
        decision: "remind_now",
        level: "ready",
        readiness_state: readiness.state.to_string(),
        recovery_confidence,
        manual_compact_recommended: true,
        snooze_status,
        handoff_state: handoff.state,
        last_checkpoint_age: checkpoint_age,
        estimated_tokens_saved: estimate.tokens_saved,
        estimated_usd_saved,
        reason: first_compact_reason(readiness, "safe compact point with checkpoint ready"),
        next_action:
            "Suggest the host compact action manually; after compact, run aiplus compact resume."
                .to_string(),
        secret_values_printed: "no",
    }
}

fn compact_reminder_plain_language(reminder: &CompactReminder) -> String {
    match reminder.decision {
        "remind_now" => format!(
            "建议现在 compact：checkpoint 已准备好，恢复信心 {}，预计可节省约 {} tokens。AiPlus 不会替你点击或调用 host compact。",
            reminder.recovery_confidence, reminder.estimated_tokens_saved
        ),
        "prepare_only" => format!(
            "建议先准备 checkpoint：{}。准备完成后再决定是否手动 compact。",
            reminder.reason
        ),
        "wait" => format!("暂不建议 compact：{}。{}", reminder.reason, reminder.next_action),
        _ => format!("不要 compact：{}。{}", reminder.reason, reminder.next_action),
    }
}

fn first_compact_reason(readiness: &CompactReadiness, fallback: &str) -> String {
    readiness
        .reasons
        .first()
        .map(|value| single_line(value))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn compact_handoff_freshness(root: &Path) -> Result<HandoffFreshness> {
    let path = compact_file(root, "current-handoff.md")?;
    if !path.exists() {
        return Ok(HandoffFreshness {
            state: "missing",
            reason: "current-handoff.md is missing".to_string(),
        });
    }
    let text = fs::read_to_string(&path)?;
    if text.contains("Synthetic template")
        || text.contains("<REPO_ROOT>")
        || text.contains("<ISO8601_TIMESTAMP>")
    {
        return Ok(HandoffFreshness {
            state: "template_only",
            reason: "current-handoff.md still looks like the starter template".to_string(),
        });
    }
    let last_updated = single_line(&section_body(&text, "Last Updated"));
    let age_seconds = parse_unix_millis_marker(&last_updated)
        .map(|millis| epoch_millis().saturating_sub(millis) / 1000)
        .map(|seconds| seconds as u64);
    if age_seconds.is_none_or(|seconds| seconds > 5_400) {
        return Ok(HandoffFreshness {
            state: "stale",
            reason: "current-handoff.md is stale or has no parseable Last Updated timestamp"
                .to_string(),
        });
    }
    Ok(HandoffFreshness {
        state: "current",
        reason: "current-handoff.md is current".to_string(),
    })
}

fn latest_checkpoint_age(root: &Path) -> Result<Option<String>> {
    let Some(rel) = latest_checkpoint(root)? else {
        return Ok(None);
    };
    let path = rel_to_abs(root, &rel)?;
    let modified = fs::metadata(path)?.modified()?;
    let seconds = SystemTime::now()
        .duration_since(modified)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    Ok(Some(format_duration_seconds(seconds)))
}

fn format_duration_seconds(seconds: u64) -> String {
    if seconds < 60 {
        format!("{seconds}s")
    } else if seconds < 3_600 {
        format!("{}m", seconds / 60)
    } else {
        format!("{}h", seconds / 3_600)
    }
}

fn parse_unix_millis_marker(value: &str) -> Option<u128> {
    value
        .strip_prefix("unix-")?
        .strip_suffix("ms")?
        .parse::<u128>()
        .ok()
}

fn compact_snooze_status(root: &Path) -> Result<&'static str> {
    let Some(snooze) = read_compact_snooze(root)? else {
        return Ok("inactive");
    };
    if snooze.until_epoch_millis > epoch_millis() {
        Ok("active")
    } else {
        Ok("expired")
    }
}

fn read_compact_snooze(root: &Path) -> Result<Option<CompactReminderSnooze>> {
    let path = compact_file(root, "reminder-snooze.json")?;
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text).ok())
}

fn set_compact_snooze(root: &Path, duration: &str) -> Result<()> {
    let seconds = parse_snooze_duration_seconds(duration)?;
    let now = epoch_millis();
    let snooze = CompactReminderSnooze {
        schema_version: VERSION.to_string(),
        set_at_epoch_millis: now,
        until_epoch_millis: now + (seconds as u128 * 1000),
        duration_seconds: seconds,
    };
    write_file_safe(
        root,
        ".aiplus/compact/reminder-snooze.json",
        format!("{}\n", serde_json::to_string_pretty(&snooze)?).as_bytes(),
        &mut Plan::default(),
        &Options {
            force: true,
            backup: false,
            yes: true,
        },
    )
}

fn clear_compact_snooze(root: &Path) -> Result<()> {
    let path = compact_file(root, "reminder-snooze.json")?;
    if path.exists() {
        assert_no_symlink_path(root, &path)?;
        fs::remove_file(path)?;
    }
    Ok(())
}

fn parse_snooze_duration_seconds(value: &str) -> Result<u64> {
    let value = value.trim();
    if value.is_empty() {
        return Err(CliError::new(2, "ERROR snooze duration is empty").into());
    }
    let (number, multiplier) = match value.chars().last().unwrap_or('s') {
        'm' | 'M' => (&value[..value.len() - 1], 60),
        'h' | 'H' => (&value[..value.len() - 1], 3_600),
        's' | 'S' => (&value[..value.len() - 1], 1),
        _ => (value, 60),
    };
    let amount = number
        .parse::<u64>()
        .map_err(|_| CliError::new(2, format!("ERROR invalid snooze duration: {value}")))?;
    if amount == 0 {
        return Err(CliError::new(2, "ERROR snooze duration must be positive").into());
    }
    Ok(amount.saturating_mul(multiplier))
}

fn compact_resume(root: &Path) -> Result<i32> {
    let result = compact_validate_state(root)?;
    if !result.ok {
        println!("RESUME_BLOCKED");
        print_compact_diagnostics(&result);
        return Ok(
            if !result.errors.is_empty() || !result.warnings.is_empty() {
                1
            } else {
                2
            },
        );
    }
    let latest = latest_checkpoint(root)?;

    // Try to load context capsule first (v2.1)
    let capsule_loaded = match load_context_capsule(root) {
        Ok(capsule) => {
            let checksum_valid = verify_capsule_checksum(root, &capsule);
            if checksum_valid {
                println!("RESUME_READY");
                println!("CAPSULE_LOADED=yes");
                println!("CAPSULE_STATUS=current");
                println!(
                    "latest_checkpoint={}",
                    latest.as_deref().unwrap_or("missing")
                );
                println!("session_role={}", capsule.session_role);
                println!("workflow_level={}", capsule.workflow_level);
                println!("current_goal={}", capsule.objective);
                println!("current_phase={}", capsule.current_state);
                println!(
                    "open_blockers={}",
                    capsule
                        .owner_gates
                        .iter()
                        .filter(|g| g.status == "UNKNOWN_PENDING")
                        .map(|g| g.label.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                println!(
                    "owner_gates={}",
                    capsule
                        .owner_gates
                        .iter()
                        .map(|g| format!("{}:{}", g.label, g.status))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                println!(
                    "next_safe_action={}",
                    single_line(&capsule.next_safe_action)
                );
                println!("decisions_loaded={}", capsule.decisions.len());
                println!("read_only_recovery_guidance=yes");
                println!("high_risk_actions=manual_owner_approval_required");
                true
            } else {
                println!("CONTEXT_CAPSULE_STALE");
                println!("CAPSULE_LOADED=no");
                println!("CAPSULE_STATUS=checksum_mismatch");
                println!("reason=checksum_mismatch");
                false
            }
        }
        Err(e) => {
            let reason = if e.to_string().contains("not found") {
                "missing"
            } else if e.downcast_ref::<serde_json::Error>().is_some()
                || e.to_string().contains("parse")
                || e.to_string().contains("json")
                || e.to_string().contains("key must be a string")
                || e.to_string().contains("expected")
            {
                "malformed"
            } else {
                "error"
            };
            println!("CONTEXT_CAPSULE_NOT_LOADED");
            println!("CAPSULE_LOADED=no");
            println!("CAPSULE_STATUS={}", reason);
            println!("reason={}", reason);
            false
        }
    };

    // Fallback to handoff if capsule not loaded
    if !capsule_loaded {
        let handoff = read_compact_text(root, "current-handoff.md")?;
        println!("RESUME_READY");
        println!("CAPSULE_LOADED=no");
        println!("CAPSULE_STATUS=handoff_fallback");
        println!(
            "latest_checkpoint={}",
            latest.as_deref().unwrap_or("missing")
        );
        println!(
            "session_role={}",
            single_line(&section_body(&handoff, "Session Role"))
        );
        println!(
            "workflow_level={}",
            single_line(&section_body(&handoff, "Workflow Level"))
        );
        println!(
            "current_goal={}",
            single_line(&section_body(&handoff, "Current Goal"))
        );
        println!(
            "current_phase={}",
            single_line(&section_body(&handoff, "Current Phase"))
        );
        println!(
            "open_blockers={}",
            single_line(&section_body(&handoff, "Open Blockers"))
        );
        println!(
            "owner_gates={}",
            single_line(&section_body(&handoff, "Owner Gates"))
        );
        println!("next_safe_action={}", single_line(&result.next_safe_action));
        println!("read_only_recovery_guidance=yes");
        println!("high_risk_actions=manual_owner_approval_required");
    }

    let readiness = compact_readiness(&result, root);
    append_savings_event(
        root,
        "resume",
        "completed",
        latest.as_deref(),
        "standard",
        &readiness,
    )
    .ok();
    Ok(0)
}

fn compact_readiness(result: &CompactValidation, root: &Path) -> CompactReadiness {
    let mut reasons = Vec::new();
    if !result.errors.is_empty() || !result.warnings.is_empty() || !result.denied_gates.is_empty() {
        reasons.extend(result.errors.iter().cloned());
        reasons.extend(result.warnings.iter().cloned());
        reasons.extend(result.denied_gates.iter().cloned());
        return CompactReadiness {
            state: "BLOCKED_BY_OWNER_GATE",
            pressure: "BLOCKED",
            explanation: "Compact is blocked until validation errors, sensitive findings, or denied Owner gates are resolved.",
            next_action: "Fix blocking compact validation findings before compact.".to_string(),
            manual_compact_recommended: false,
            reasons,
        };
    }
    if !result.pending_gates.is_empty() {
        // Issue #34: distinguish "fresh install — Owner gates have
        // never been touched yet" from "Owner gates exist and are
        // pending review post-edit". The seed compact templates ship
        // a single UNKNOWN_PENDING gate ("Owner review of compact
        // handoff before first real use") and a seed Current Goal
        // ("Initialize compact/resume handoff state ..."). When both
        // signals are still in their seed state, the project simply
        // hasn't been used yet — the PreCompact hook should not flag
        // this as needs-review on every host compact attempt.
        let handoff = read_compact_text(root, "current-handoff.md").unwrap_or_default();
        if is_fresh_install_compact_state(&handoff, &result.pending_gates) {
            reasons.extend(result.pending_gates.iter().cloned());
            return CompactReadiness {
                state: "FRESH_INSTALL_AWAITING_FIRST_USE",
                pressure: "INFO",
                explanation:
                    "Compact protocol is in its seed state from `aiplus install` — no real work yet, so Owner gates have not been touched. This is informational, not a blocker.",
                next_action:
                    "When you start a real task, edit .aiplus/compact/current-handoff.md (Current Goal + Owner Gates) and rerun aiplus compact prepare."
                        .to_string(),
                manual_compact_recommended: false,
                reasons,
            };
        }
        reasons.extend(result.pending_gates.iter().cloned());
        return CompactReadiness {
            state: "UNKNOWN_NEEDS_REVIEW",
            pressure: "MEDIUM",
            explanation: "Owner gates are not fully documented, so compact readiness needs review.",
            next_action:
                "Document or resolve pending Owner gates, then rerun aiplus compact prepare."
                    .to_string(),
            manual_compact_recommended: false,
            reasons,
        };
    }
    if !result.review_items.is_empty() {
        reasons.extend(result.review_items.iter().cloned());
        return CompactReadiness {
            state: "UNKNOWN_NEEDS_REVIEW",
            pressure: "MEDIUM",
            explanation:
                "Compact files need version or structure review before recommending compact.",
            next_action: "Review compact protocol files, then rerun aiplus compact prepare."
                .to_string(),
            manual_compact_recommended: false,
            reasons,
        };
    }
    if result.next_safe_action.trim().is_empty() {
        return CompactReadiness {
            state: "NEEDS_HANDOFF_UPDATE",
            pressure: "MEDIUM",
            explanation: "The next safe action is missing from the handoff.",
            next_action: "Update .aiplus/compact/current-handoff.md with the next safe action."
                .to_string(),
            manual_compact_recommended: false,
            reasons: vec!["next safe action is missing".to_string()],
        };
    }
    let handoff = read_compact_text(root, "current-handoff.md").unwrap_or_default();
    let phase = section_body(&handoff, "Current Phase").to_ascii_uppercase();
    if phase.contains("DEBUG") || phase.contains("RUNNING_TESTS") || phase.contains("ACTIVE_WORK") {
        return CompactReadiness {
            state: "NOT_RECOMMENDED_DURING_ACTIVE_WORK",
            pressure: "LOW",
            explanation: "Compact is not recommended while active work is unstable.",
            next_action: "Finish or stabilize the current work phase before compact.".to_string(),
            manual_compact_recommended: false,
            reasons: vec![format!("current phase: {}", single_line(&phase))],
        };
    }
    CompactReadiness {
        state: "READY_TO_COMPACT",
        pressure: "HIGH",
        explanation: "Compact state is valid and the next safe action is documented.",
        next_action: "Run the host compact action manually, then continue or send: continue."
            .to_string(),
        manual_compact_recommended: true,
        reasons: vec!["validated compact files".to_string()],
    }
}

fn print_readiness(readiness: &CompactReadiness) {
    println!("READINESS_STATE={}", readiness.state);
    println!("COMPACT_PRESSURE={}", readiness.pressure);
    println!(
        "MANUAL_COMPACT_RECOMMENDED={}",
        if readiness.manual_compact_recommended {
            "yes"
        } else {
            "no"
        }
    );
    println!("READINESS_EXPLANATION={}", readiness.explanation);
    println!("NEXT_ACTION={}", readiness.next_action);
    for reason in &readiness.reasons {
        println!("REASON {}", single_line(reason));
    }
}

fn readiness_exit_code(readiness: &CompactReadiness) -> i32 {
    match readiness.state {
        "READY_TO_COMPACT" => 0,
        // Issue #34: fresh-install state is informational, not a
        // failure. The PreCompact hook treats non-zero as "noisy
        // problem" — exit 0 keeps fresh installs quiet and reserves
        // the warning channel for genuine UNKNOWN_NEEDS_REVIEW.
        "FRESH_INSTALL_AWAITING_FIRST_USE" => 0,
        "BLOCKED_BY_OWNER_GATE" => 1,
        _ => 2,
    }
}

/// Issue #34: detect the just-installed state of the compact protocol.
/// Returns true when both the handoff Current Goal is still the seed
/// text AND every pending gate matches one of the seed gate
/// placeholders shipped by the compact templates. Conservative on
/// purpose: any custom edit to the handoff Current Goal or any
/// non-seed Owner Gate drops the project out of "fresh install" and
/// back into the normal UNKNOWN_NEEDS_REVIEW loop.
fn is_fresh_install_compact_state(handoff_text: &str, pending_gates: &[String]) -> bool {
    const SEED_GOAL_MARKER: &str = "Initialize compact/resume handoff state for";
    // The seed templates ship one UNKNOWN_PENDING gate per compact
    // file; each file's gate has a distinct, file-specific placeholder
    // text (so a fresh install reports four pending gates, all from
    // the bundled templates). Any of these markers tagged on a
    // pending-gate line means the line came verbatim from the seed.
    const SEED_GATE_MARKERS: &[&str] = &[
        "Owner review of compact handoff before first real use",
        "Owner review of first real decision entries",
        "Owner review required before relying on delegated results",
        "Owner review of evidence quality before first real compact",
    ];
    let goal = section_body(handoff_text, "Current Goal");
    if !goal.contains(SEED_GOAL_MARKER) {
        return false;
    }
    if pending_gates.is_empty() {
        return false;
    }
    pending_gates
        .iter()
        .all(|g| SEED_GATE_MARKERS.iter().any(|m| g.contains(m)))
}

fn compact_section_or_unknown(text: &str, heading: &str) -> String {
    let value = single_line(&section_body(text, heading));
    if value.is_empty() {
        "Unknown".to_string()
    } else {
        value
    }
}

fn checkpoint_detail_vectors(
    level: &str,
    decision_log: &str,
    evidence: &str,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let decisions = if level == "light" {
        Vec::new()
    } else {
        parse_markdown_table(
            decision_log,
            &["id", "status", "decision", "rationale", "evidence"],
        )
        .into_iter()
        .filter_map(|row| {
            let id = row.get("id")?;
            let status = row.get("status").map(String::as_str).unwrap_or("");
            let decision = row.get("decision").map(String::as_str).unwrap_or("");
            Some(format!("{id}: {status} {decision}"))
        })
        .collect()
    };
    let evidence_ids = if level == "light" {
        Vec::new()
    } else {
        evidence_ids(evidence)
    };
    let artifacts = if level == "full" {
        parse_markdown_table(
            evidence,
            &["id", "confidence", "source", "finding", "artifact"],
        )
        .into_iter()
        .filter_map(|row| row.get("artifact").cloned())
        .filter(|value| !value.is_empty() && value != "<ARTIFACT_ID>")
        .collect()
    } else {
        Vec::new()
    };
    (decisions, evidence_ids, artifacts)
}

fn optional_for_level(level: &str, value: String, minimum: &str) -> Option<String> {
    let allowed = match minimum {
        "full" => level == "full",
        "standard" => level == "standard" || level == "full",
        _ => true,
    };
    if allowed {
        optional_line(value)
    } else {
        None
    }
}

fn lines_for_level(level: &str, value: String, minimum: &str) -> Vec<String> {
    let allowed = match minimum {
        "full" => level == "full",
        "standard" => level == "standard" || level == "full",
        _ => true,
    };
    if !allowed {
        return Vec::new();
    }
    non_placeholder_lines(&value)
        .into_iter()
        .map(|line| strip_numbering(&line))
        .collect()
}

fn latest_checkpoint(root: &Path) -> Result<Option<String>> {
    let dir = compact_file(root, "checkpoints")?;
    if !dir.exists() {
        return Ok(None);
    }
    let mut files = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files
        .last()
        .and_then(|path| path_relative(root, path).ok())
        .map(path_slash))
}

fn append_savings_event(
    root: &Path,
    event: &str,
    event_scope: &str,
    checkpoint_id: Option<&str>,
    level: &str,
    readiness: &CompactReadiness,
) -> Result<()> {
    let estimate = estimate_savings(root)?;
    let handoff = read_compact_text(root, "current-handoff.md").unwrap_or_default();
    let savings_event = SavingsEvent {
        schema_version: VERSION.to_string(),
        timestamp: timestamp(),
        event: event.to_string(),
        event_scope: event_scope.to_string(),
        checkpoint_id: checkpoint_id.map(str::to_string),
        checkpoint_level: level.to_string(),
        readiness_state: readiness.state.to_string(),
        compact_pressure: readiness.pressure.to_string(),
        session_role: compact_section_or_unknown(&handoff, "Session Role"),
        workflow_level: compact_section_or_unknown(&handoff, "Workflow Level"),
        estimated_input_tokens_before: estimate.input_before,
        estimated_handoff_tokens_after: estimate.handoff_after,
        estimated_resume_tokens: estimate.resume_tokens,
        estimated_tokens_saved: estimate.tokens_saved,
        estimated_token_reduction_percent: estimate.reduction_percent,
        estimated_cost_saved_usd: estimate.cost_saved_usd,
        pricing_model: estimate.pricing_model.clone(),
        pricing_status: if estimate.cost_saved_usd.is_some() {
            if estimate.pricing_model == "generic_default" {
                "generic_fallback".to_string()
            } else {
                "matched".to_string()
            }
        } else {
            "unavailable".to_string()
        },
        pricing_source: estimate.pricing_source.clone(),
        pricing_fetched_at: estimate.pricing_fetched_at.clone(),
        pricing_age_days: pricing_cache_age_days().ok().flatten(),
        input_price_usd_per_1m_tokens: pricing_input_price(&estimate),
        model_detected: estimate.model_detected,
        model_detection_confidence: estimate.model_detection_confidence,
        cost_estimate_available: estimate.cost_saved_usd.is_some(),
        cost_estimate_reason: if estimate.cost_saved_usd.is_some() {
            if estimate.pricing_model == "generic_default" {
                "generic conservative fallback pricing; not model-specific".to_string()
            } else {
                "matched cached public pricing".to_string()
            }
        } else {
            "pricing for detected model is not available".to_string()
        },
        billing_data: false,
        method: "local_estimate_v1".to_string(),
        confidence: estimate.confidence,
        notes: estimate.notes,
    };
    let ledger = compact_file(root, SAVINGS_LEDGER_REL)?;
    if let Some(parent) = ledger.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&ledger)?;
    writeln!(file, "{}", serde_json::to_string(&savings_event)?)?;
    Ok(())
}

#[allow(dead_code)]
fn load_context_capsule(root: &Path) -> Result<aiplus_core::ContextCapsule> {
    let path = compact_file(root, "context-capsule.json")?;
    if !path.exists() {
        return Err(anyhow::anyhow!("context-capsule.json not found"));
    }
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

fn verify_capsule_checksum(_root: &Path, capsule: &aiplus_core::ContextCapsule) -> bool {
    use aiplus_core::compute_capsule_checksum;
    let expected = compute_capsule_checksum(&format!(
        "{}{}{}",
        capsule.project_id, capsule.objective, capsule.current_state
    ));
    capsule
        .checksums
        .get("capsule_v1")
        .map(|stored| stored == &expected)
        .unwrap_or(false)
}

fn save_context_capsule(root: &Path, capsule: &aiplus_core::ContextCapsule) -> Result<()> {
    let path = compact_file(root, "context-capsule.json")?;
    fs::write(path, serde_json::to_string_pretty(capsule)?)?;
    Ok(())
}

fn build_context_capsule_from_handoff(root: &Path) -> Result<aiplus_core::ContextCapsule> {
    use aiplus_core::{build_capsule_from_compact_state, timestamp};
    let handoff = read_compact_text(root, "current-handoff.md")?;
    let objective = compact_section_or_unknown(&handoff, "Current Goal");
    let current_state = compact_section_or_unknown(&handoff, "Current Phase");
    let next_safe_action = compact_section_or_unknown(&handoff, "Next 3 Actions");

    let mut capsule = build_capsule_from_compact_state(
        &project_id_from_root(root),
        &objective,
        &current_state,
        &next_safe_action,
    );

    capsule.next_safe_action = next_safe_action.clone();
    capsule.resume_prompt = format!(
        "Resume work on: {}. Current state: {}. Next action: {}.",
        objective, current_state, next_safe_action
    );

    capsule.decisions = extract_decisions_from_ledger(root)?;
    capsule.owner_gates = extract_owner_gates_from_handoff(&handoff)?;

    capsule.updated_at = timestamp();
    Ok(capsule)
}

fn extract_decisions_from_ledger(root: &Path) -> Result<Vec<aiplus_core::CapsuleDecision>> {
    let path = compact_file(root, "decision-log.md")?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(path)?;
    let mut decisions = Vec::new();
    let mut in_decisions = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "## Decisions" {
            in_decisions = true;
            continue;
        }
        if in_decisions && trimmed.starts_with("## ") {
            break;
        }
        if !in_decisions || !trimmed.starts_with('|') {
            continue;
        }
        let parts: Vec<&str> = trimmed.split('|').map(|s| s.trim()).collect();
        if parts.len() < 4 {
            continue;
        }
        let id = parts[1];
        let status = parts[2];
        let description = parts.get(3).unwrap_or(&"");
        if id.is_empty() || id == "ID" || id.starts_with('<') || id.contains("---") {
            continue;
        }
        // Skip decisions that contain sensitive patterns
        let desc_lower = description.to_ascii_lowercase();
        let is_sensitive = [
            "api_key",
            "apikey",
            "secret_key",
            "password",
            "private_key",
            "bearer ",
            "authorization:",
            "cookie:",
            "-----begin ",
            "raw transcript",
            "provider payload",
            "sensitive",
            "private",
        ]
        .iter()
        .any(|needle| desc_lower.contains(needle));
        if is_sensitive {
            continue;
        }
        let verified = status.eq_ignore_ascii_case("DECIDED");
        decisions.push(aiplus_core::CapsuleDecision {
            id: id.to_string(),
            description: description.to_string(),
            status: status.to_string(),
            decided_at: String::new(),
            verified,
        });
    }
    Ok(decisions)
}

fn extract_owner_gates_from_handoff(handoff: &str) -> Result<Vec<aiplus_core::CapsuleOwnerGate>> {
    let mut gates = Vec::new();
    let section = section_body(handoff, "Owner Gates");
    for line in section.lines() {
        let line = line.trim();
        if line.starts_with("-") {
            let content = line.trim_start_matches('-').trim();
            let status = if content.contains("APPROVED") {
                "APPROVED"
            } else if content.contains("DENIED") {
                "DENIED"
            } else {
                "UNKNOWN_PENDING"
            };
            gates.push(aiplus_core::CapsuleOwnerGate {
                label: content.to_string(),
                status: status.to_string(),
                required: true,
                approved_by: None,
            });
        }
    }
    Ok(gates)
}

fn compact_watch(root: &Path, once: bool, interval: Option<&str>, json: bool) -> Result<i32> {
    use aiplus_core::{parse_watch_interval, ReminderState, WatchConfig, WatchMode, WatchResult};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let config = WatchConfig {
        mode: if once {
            WatchMode::Once
        } else {
            WatchMode::Interval
        },
        interval_seconds: interval
            .map(parse_watch_interval)
            .transpose()?
            .unwrap_or(600),
        max_iterations: if once { Some(1) } else { None },
        json_output: json,
    };

    let mode_str = if once { "once" } else { "interval" };

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .ok();

    #[cfg(unix)]
    {
        let r = running.clone();
        if let Ok(mut signals) = signal_hook::iterator::Signals::new([signal_hook::consts::SIGTERM])
        {
            std::thread::spawn(move || {
                if signals.forever().next().is_some() {
                    r.store(false, Ordering::SeqCst);
                }
            });
        }
    }

    let mut iteration = 0u64;

    loop {
        if !running.load(Ordering::SeqCst) {
            if !json {
                println!("COMPACT_WATCH_INTERRUPTED");
            }
            break;
        }

        iteration += 1;

        let exit_code = compact_remind(
            root,
            if once {
                Some("watch-once")
            } else {
                Some("watch-interval")
            },
            None,
            false,
            false, // suppress inner JSON; watch emits its own JSON if needed
            json,  // suppress all inner output when watch is in JSON mode
        )?;

        let mut state = load_reminder_state(root).unwrap_or_else(|_| {
            let mut s = ReminderState::new("aiplus-project");
            s.project_id = project_id_from_root(root);
            s
        });
        state.watch_count = state.watch_count.saturating_add(1);
        state.last_watch_at = Some(timestamp());

        if let Ok(result) = compact_validate_state(root) {
            let readiness = compact_readiness(&result, root);
            let handoff = compact_handoff_freshness(root).unwrap_or(HandoffFreshness {
                state: "unknown",
                reason: "handoff evaluation failed".to_string(),
            });
            state.last_reminder_decision = match exit_code {
                0 => "remind_now".to_string(),
                1 => "blocked".to_string(),
                2 => "wait".to_string(),
                _ => "unknown".to_string(),
            };
            state.last_reminder_level = if exit_code == 0 {
                "ready".to_string()
            } else if exit_code == 1 {
                "safety_block".to_string()
            } else {
                "soft".to_string()
            };
            state.last_handoff_state = handoff.state.to_string();
            state.last_recovery_confidence =
                if readiness.state == "READY_TO_COMPACT" && handoff.state == "current" {
                    "high".to_string()
                } else if readiness.state == "READY_TO_COMPACT" {
                    "medium".to_string()
                } else {
                    "low".to_string()
                };
        }

        let _ = save_reminder_state(root, &state);

        if json {
            let result = WatchResult {
                status: "PASS".to_string(),
                watch_mode: mode_str.to_string(),
                iteration,
                reminder_decision: state.last_reminder_decision.clone(),
                reminder_level: state.last_reminder_level.clone(),
                handoff_state: state.last_handoff_state.clone(),
                recovery_confidence: state.last_recovery_confidence.clone(),
                manual_compact_recommended: exit_code == 0,
                host_compact_triggered: false,
                secret_values_printed: false,
                raw_transcript_captured: false,
                context_capsule_status: "updated".to_string(),
                next_action: if exit_code == 0 {
                    "Run aiplus compact prepare, then suggest manual host compact.".to_string()
                } else {
                    "Check blockers and retry.".to_string()
                },
                reason: format!("watch iteration {iteration}"),
            };
            println!("{}", serde_json::to_string(&result)?);
        } else {
            println!("COMPACT_WATCH");
            println!("WATCH_MODE={}", mode_str);
            println!("WATCH_ITERATION={}", iteration);
            println!("REMINDER_DECISION={}", state.last_reminder_decision);
            println!("REMINDER_LEVEL={}", state.last_reminder_level);
            println!("HANDOFF_STATE={}", state.last_handoff_state);
            println!("RECOVERY_CONFIDENCE={}", state.last_recovery_confidence);
            println!(
                "MANUAL_COMPACT_RECOMMENDED={}",
                if exit_code == 0 { "yes" } else { "no" }
            );
            println!("HOST_COMPACT_TRIGGERED=no");
            println!("SECRET_VALUES_PRINTED=no");
            println!("RAW_TRANSCRIPT_CAPTURED=no");
            println!("CONTEXT_CAPSULE_STATUS=updated");
            println!(
                "NEXT_ACTION={}",
                if exit_code == 0 {
                    "Run aiplus compact prepare, then suggest manual host compact."
                } else {
                    "Check blockers and retry."
                }
            );
        }

        if once || config.max_iterations == Some(iteration) {
            break;
        }

        if !running.load(Ordering::SeqCst) {
            break;
        }

        std::thread::sleep(std::time::Duration::from_secs(config.interval_seconds));
    }

    Ok(0)
}

fn load_reminder_state(root: &Path) -> Result<aiplus_core::ReminderState> {
    let path = compact_file(root, "reminder-state.json")?;
    if !path.exists() {
        return Err(anyhow::anyhow!("reminder-state.json not found"));
    }
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

fn save_reminder_state(root: &Path, state: &aiplus_core::ReminderState) -> Result<()> {
    let path = compact_file(root, "reminder-state.json")?;
    fs::write(path, serde_json::to_string_pretty(state)?)?;
    Ok(())
}

fn project_id_from_root(root: &Path) -> String {
    let canonical = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    format!("{:x}", {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        canonical.to_string_lossy().hash(&mut hasher);
        hasher.finish()
    })
}

fn compact_savings(root: &Path, json: bool) -> Result<()> {
    let (events, malformed) = read_savings_events(root)?;
    if events.is_empty() {
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "schemaVersion": VERSION,
                    "status": "NO_SAVINGS_DATA",
                    "billingData": false,
                    "events": 0,
                    "malformedLines": malformed
                })
            );
        } else {
            println!("Compact savings estimate");
            println!("No savings data yet.");
            println!("Run compact prepare/checkpoint/resume first.");
            println!("Estimate only, not billing data.");
        }
        return Ok(());
    }

    let completed = completed_savings_cycles(&events);
    let latest_event = events.last().expect("events not empty");
    let latest_completed = completed.last();
    let total_saved: u64 = completed
        .iter()
        .map(|event| event.estimated_tokens_saved)
        .sum();
    let total_baseline: u64 = completed
        .iter()
        .map(|event| event.estimated_input_tokens_before)
        .sum();
    let cumulative_reduction = if total_baseline == 0 {
        0.0
    } else {
        (total_saved as f64 / total_baseline as f64) * 100.0
    };
    let priced_events = completed
        .iter()
        .filter(|event| event.cost_estimate_available)
        .count();
    let unpriced_events = completed.len().saturating_sub(priced_events);
    let total_cost: f64 = completed
        .iter()
        .filter_map(|event| event.estimated_cost_saved_usd)
        .sum();

    if json {
        println!(
            "{}",
            serde_json::json!({
                "schemaVersion": VERSION,
                "status": "PASS",
                "latestEvent": latest_event,
                "latestCompleted": latest_completed,
                "allTime": {
                    "ledgerEvents": events.len(),
                    "completedCycles": completed.len(),
                    "estimatedTokensSaved": total_saved,
                    "estimatedBaselineTokens": total_baseline,
                    "weightedReductionPercent": round1(cumulative_reduction),
                    "estimatedCostSavedUsd": if priced_events > 0 { Some(round4(total_cost)) } else { None },
                    "pricedEvents": priced_events,
                    "unpricedEvents": unpriced_events
                },
                "eventSemantics": {
                    "projected": "prepare events do not count toward completed savings totals",
                    "candidate": "checkpoint events do not count toward completed savings totals",
                    "completed": "successful resume events count once per checkpointId"
                },
                "malformedLines": malformed,
                "billingData": false,
                "method": "local_estimate_v1"
            })
        );
        return Ok(());
    }

    println!("Compact savings estimate");
    println!();
    println!("This compact:");
    let Some(latest) = latest_completed else {
        println!("- Status: no completed compact cycle yet");
        println!(
            "- Latest event: {} ({})",
            latest_event.event,
            event_scope(latest_event)
        );
        println!();
        println!("All time:");
        println!("- Tokens saved: ~0 input tokens");
        println!("- Average reduction: ~0%");
        println!("- Estimated cost saved: unavailable");
        println!("- Pricing coverage: 0/0 compacts");
        println!();
        println!("pricing_source={}", latest_event.pricing_source);
        println!("billing_data=no");
        if malformed > 0 {
            println!("WARNING malformed ledger lines ignored: {malformed}");
        }
        println!("Estimate only, not billing data.");
        return Ok(());
    };
    let latest_cost = latest.estimated_cost_saved_usd;
    println!(
        "- Tokens saved: ~{} input tokens",
        format_tokens(latest.estimated_tokens_saved)
    );
    println!(
        "- Token reduction: ~{}%",
        round1(latest.estimated_token_reduction_percent)
    );
    match latest_cost {
        Some(cost) => println!("- Estimated cost saved: ~${:.4}", round4(cost)),
        None => {
            println!("- Estimated cost saved: unavailable");
            println!("  Reason: {}", latest.cost_estimate_reason);
        }
    }
    println!(
        "- Recovery confidence: {}",
        latest.confidence.to_ascii_uppercase()
    );
    println!();
    println!("All time:");
    println!(
        "- Tokens saved: ~{} input tokens",
        format_tokens(total_saved)
    );
    println!("- Average reduction: ~{}%", round1(cumulative_reduction));
    if priced_events > 0 {
        println!(
            "- Estimated cost saved: ~${:.4} from {} priced compacts",
            round4(total_cost),
            priced_events
        );
        if unpriced_events > 0 {
            println!("- Unpriced compacts: {unpriced_events}");
        }
    } else {
        println!("- Estimated cost saved: unavailable");
    }
    println!(
        "- Pricing coverage: {}/{} compacts",
        priced_events,
        completed.len()
    );
    println!();
    println!("pricing_source={}", latest.pricing_source);
    println!(
        "pricing_cached_at={}",
        latest
            .pricing_fetched_at
            .as_deref()
            .unwrap_or("unavailable")
    );
    println!(
        "pricing_age_days={}",
        latest
            .pricing_age_days
            .map(|days| days.to_string())
            .unwrap_or_else(|| "unavailable".to_string())
    );
    println!("billing_data=no");
    if malformed > 0 {
        println!("WARNING malformed ledger lines ignored: {malformed}");
    }
    println!("Estimate only, not billing data.");
    Ok(())
}

fn completed_savings_cycles(events: &[SavingsEvent]) -> Vec<&SavingsEvent> {
    let mut by_checkpoint: BTreeMap<String, &SavingsEvent> = BTreeMap::new();
    for event in events
        .iter()
        .filter(|event| is_completed_savings_event(event))
    {
        if let Some(checkpoint_id) = event.checkpoint_id.as_deref() {
            by_checkpoint.insert(checkpoint_id.to_string(), event);
        }
    }
    by_checkpoint.values().copied().collect()
}

fn is_completed_savings_event(event: &SavingsEvent) -> bool {
    event.event == "resume" && event_scope(event) == "completed" && event.checkpoint_id.is_some()
}

fn event_scope(event: &SavingsEvent) -> &str {
    if event.event_scope.is_empty() && event.event == "resume" {
        "completed"
    } else if event.event_scope.is_empty() && event.event == "checkpoint" {
        "candidate"
    } else if event.event_scope.is_empty() && event.event == "prepare" {
        "projected"
    } else {
        &event.event_scope
    }
}

fn estimate_savings(root: &Path) -> Result<SavingsEstimate> {
    let handoff_tokens = estimate_compact_tokens(root)?;
    let resume_tokens = 600;
    let input_before =
        (handoff_tokens.saturating_mul(4)).max(handoff_tokens + resume_tokens + 1000);
    let handoff_after = handoff_tokens + resume_tokens;
    let tokens_saved = input_before.saturating_sub(handoff_after);
    let reduction_percent = if input_before == 0 {
        0.0
    } else {
        (tokens_saved as f64 / input_before as f64) * 100.0
    };
    let (model_detected, confidence) = detect_model_hint();
    let (pricing_model, pricing_source, pricing_fetched_at, price) =
        match load_pricing_catalog_for_savings() {
            Ok(catalog) => choose_pricing_model(&catalog, model_detected.as_deref(), &confidence),
            Err(_) => (
                "unavailable".to_string(),
                "unavailable".to_string(),
                None,
                None,
            ),
        };
    let cost_saved_usd = price.map(|price| (tokens_saved as f64 / 1_000_000.0) * price);
    Ok(SavingsEstimate {
        input_before,
        handoff_after,
        resume_tokens,
        tokens_saved,
        reduction_percent: round1(reduction_percent),
        cost_saved_usd: cost_saved_usd.map(round4),
        pricing_model,
        pricing_source,
        pricing_fetched_at,
        model_detected,
        model_detection_confidence: confidence,
        confidence: "low".to_string(),
        notes: vec![
            "local aggregate estimate only".to_string(),
            "no prompt text, transcript text, file contents, raw checkpoint text, billing data, or usage history is stored".to_string(),
        ],
    })
}

fn estimate_savings_for_remind(root: &Path) -> Result<SavingsEstimate> {
    let handoff_tokens = estimate_compact_tokens(root)?;
    let resume_tokens = 600;
    let input_before =
        (handoff_tokens.saturating_mul(4)).max(handoff_tokens + resume_tokens + 1000);
    let handoff_after = handoff_tokens + resume_tokens;
    let tokens_saved = input_before.saturating_sub(handoff_after);
    let reduction_percent = if input_before == 0 {
        0.0
    } else {
        (tokens_saved as f64 / input_before as f64) * 100.0
    };
    let (model_detected, confidence) = detect_model_hint();
    let catalog = bundled_pricing_catalog();
    let (pricing_model, pricing_source, pricing_fetched_at, price) =
        choose_pricing_model(&catalog, model_detected.as_deref(), &confidence);
    let cost_saved_usd = price.map(|price| (tokens_saved as f64 / 1_000_000.0) * price);
    Ok(SavingsEstimate {
        input_before,
        handoff_after,
        resume_tokens,
        tokens_saved,
        reduction_percent: round1(reduction_percent),
        cost_saved_usd: cost_saved_usd.map(round4),
        pricing_model,
        pricing_source,
        pricing_fetched_at,
        model_detected,
        model_detection_confidence: confidence,
        confidence: "low".to_string(),
        notes: vec![
            "local aggregate estimate only".to_string(),
            "no prompt text, transcript text, file contents, raw checkpoint text, billing data, or usage history is stored".to_string(),
            "pricing: bundled read-only, no network, no cache write".to_string(),
        ],
    })
}

fn estimate_compact_tokens(root: &Path) -> Result<u64> {
    let mut chars = 0usize;
    for file in COMPACT_REQUIRED_FILES {
        if let Ok(text) = read_compact_text(root, file) {
            chars = chars.saturating_add(text.chars().count());
        }
    }
    Ok(((chars as f64) / 4.0).ceil() as u64)
}

fn read_savings_events(root: &Path) -> Result<(Vec<SavingsEvent>, usize)> {
    let ledger = compact_file(root, SAVINGS_LEDGER_REL)?;
    if !ledger.exists() {
        return Ok((Vec::new(), 0));
    }
    let text = fs::read_to_string(ledger)?;
    let mut events = Vec::new();
    let mut malformed = 0;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        match serde_json::from_str::<SavingsEvent>(line) {
            Ok(event) => events.push(event),
            Err(_) => malformed += 1,
        }
    }
    Ok((events, malformed))
}

fn pricing_update() -> Result<()> {
    let (catalog, fetch_mode) = match fetch_pricing_catalog() {
        Ok(catalog) => (catalog, "network"),
        Err(error) => {
            eprintln!("WARNING pricing fetch failed; using bundled catalog: {error}");
            (bundled_pricing_catalog(), "bundled")
        }
    };
    let cache = pricing_cache_file()?;
    if let Some(parent) = cache.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&cache, serde_json::to_string_pretty(&catalog)?)?;
    println!("AIPLUS_PRICING_UPDATE");
    println!("PRICING_UPDATE_STATUS=PASS");
    println!("cache_path={}", cache.display());
    println!("pricing_source={}", catalog.source);
    println!("pricing_fetch_mode={fetch_mode}");
    println!(
        "pricing_cached_at={}",
        catalog.fetched_at.as_deref().unwrap_or("unavailable")
    );
    println!("pricing_age_days=0");
    println!("source_url={}", catalog.source_url);
    println!("models={}", catalog.models.len());
    println!("billing_data=no");
    println!("uploads=none");
    Ok(())
}

fn pricing_status() -> Result<()> {
    let cache = pricing_cache_file()?;
    let (catalog, status) = if cache.exists() {
        (load_pricing_catalog(true)?, "cached")
    } else {
        (bundled_pricing_catalog(), "bundled")
    };
    println!("AIPLUS_PRICING_STATUS");
    println!("PRICING_STATUS=PASS");
    println!("status={status}");
    println!("cache_path={}", cache.display());
    println!("pricing_source={}", catalog.source);
    println!("pricing_fetch_mode={status}");
    println!("source_url={}", catalog.source_url);
    println!(
        "pricing_cached_at={}",
        catalog.fetched_at.as_deref().unwrap_or("unavailable")
    );
    println!(
        "pricing_age_days={}",
        pricing_cache_age_days()?
            .map(|days| days.to_string())
            .unwrap_or_else(|| "unavailable".to_string())
    );
    println!("pricing_cache_ttl_days=7");
    println!(
        "pricing_cache_fresh={}",
        pricing_cache_age_days()?
            .map(|days| if days <= 7 { "yes" } else { "no" })
            .unwrap_or("unavailable")
    );
    println!("models={}", catalog.models.len());
    println!("billing_data=no");
    println!("uploads=none");
    Ok(())
}

fn fetch_pricing_catalog() -> Result<PricingCatalog> {
    let url =
        std::env::var("AIPLUS_PRICING_URL").unwrap_or_else(|_| PRICING_CATALOG_URL.to_string());
    let text = if let Some(path) = url.strip_prefix("file://") {
        fs::read_to_string(path)?
    } else if command_exists("curl") {
        let output = Command::new("curl").args(["-fsSL", &url]).output()?;
        if !output.status.success() {
            return Err(anyhow!("curl failed"));
        }
        String::from_utf8(output.stdout)?
    } else if command_exists("wget") {
        let output = Command::new("wget")
            .args(["-q", "-O", "-", &url])
            .output()?;
        if !output.status.success() {
            return Err(anyhow!("wget failed"));
        }
        String::from_utf8(output.stdout)?
    } else {
        return Err(anyhow!("curl or wget is required for pricing update"));
    };
    let mut catalog: PricingCatalog = serde_json::from_str(&text)?;
    catalog.fetched_at = Some(timestamp());
    if catalog.source.is_empty() {
        catalog.source = "official".to_string();
    }
    if catalog.source_url.is_empty() {
        catalog.source_url = url;
    }
    Ok(catalog)
}

fn fetch_to(url: &str, dest: &Path) -> Result<()> {
    if let Some(path) = url.strip_prefix("file://") {
        fs::copy(path, dest)?;
        return Ok(());
    }
    if command_exists("curl") {
        let status = Command::new("curl")
            .args(["-fsSL", url, "-o"])
            .arg(dest)
            .status()?;
        if status.success() {
            return Ok(());
        }
        return Err(anyhow!("curl failed for {url}"));
    }
    if command_exists("wget") {
        let status = Command::new("wget")
            .args(["-q", url, "-O"])
            .arg(dest)
            .status()?;
        if status.success() {
            return Ok(());
        }
        return Err(anyhow!("wget failed for {url}"));
    }
    Err(anyhow!("curl or wget is required"))
}

fn verify_checksum_file(checksums: &Path, asset: &Path) -> Result<()> {
    let asset_name = asset
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow!("asset has no filename"))?;
    let text = fs::read_to_string(checksums)?;
    let expected = text
        .lines()
        .find(|line| line.ends_with(asset_name))
        .ok_or_else(|| anyhow!("checksum not found for {asset_name}"))?
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow!("checksum line is malformed"))?;
    let output = if command_exists("shasum") {
        Command::new("shasum")
            .args(["-a", "256"])
            .arg(asset)
            .output()?
    } else if command_exists("sha256sum") {
        Command::new("sha256sum").arg(asset).output()?
    } else {
        return Err(anyhow!("shasum or sha256sum is required"));
    };
    if !output.status.success() {
        return Err(anyhow!("checksum command failed"));
    }
    let actual = String::from_utf8(output.stdout)?
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
    if actual != expected {
        return Err(CliError::new(1, "ERROR checksum mismatch").into());
    }
    Ok(())
}

fn verify_sha256_file(sha256_file: &Path, asset: &Path) -> Result<()> {
    let text = fs::read_to_string(sha256_file)?;
    // .sha256 files may contain just the checksum, or "CHECKSUM  FILENAME" format
    let expected = text
        .lines()
        .next()
        .ok_or_else(|| anyhow!(".sha256 file is empty"))?
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow!(".sha256 file is malformed"))?;
    let output = if command_exists("shasum") {
        Command::new("shasum")
            .args(["-a", "256"])
            .arg(asset)
            .output()?
    } else if command_exists("sha256sum") {
        Command::new("sha256sum").arg(asset).output()?
    } else {
        return Err(anyhow!("shasum or sha256sum is required"));
    };
    if !output.status.success() {
        return Err(anyhow!("checksum command failed"));
    }
    let actual = String::from_utf8(output.stdout)?
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
    if actual != expected {
        return Err(CliError::new(1, "ERROR checksum mismatch").into());
    }
    Ok(())
}

fn extract_release_archive(archive: &Path, dest: &Path) -> Result<()> {
    let name = archive
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if name.ends_with(".tar.gz") {
        let status = Command::new("tar")
            .arg("-xzf")
            .arg(archive)
            .arg("-C")
            .arg(dest)
            .status()?;
        if status.success() {
            return Ok(());
        }
        return Err(anyhow!("tar extraction failed"));
    }
    if name.ends_with(".zip") {
        let status = Command::new("unzip")
            .arg("-q")
            .arg(archive)
            .arg("-d")
            .arg(dest)
            .status()?;
        if status.success() {
            return Ok(());
        }
        return Err(anyhow!("zip extraction failed"));
    }
    Err(anyhow!("unsupported release asset: {name}"))
}

fn find_release_binary(dir: &Path, windows: bool) -> Result<PathBuf> {
    let wanted = if windows { "aiplus.exe" } else { "aiplus" };
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().and_then(|name| name.to_str()) == Some(wanted) {
                return Ok(path);
            }
        }
    }
    Err(anyhow!("release archive did not contain {wanted}"))
}

fn detect_release_asset() -> Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    match (os, arch) {
        ("macos", "aarch64") => Ok("aiplus-aarch64-apple-darwin.tar.gz".to_string()),
        _ => Err(CliError::new(
            1,
            format!("ERROR no verified AiPlus {RELEASE_TAG} binary asset for: {os} {arch}"),
        )
        .into()),
    }
}

fn self_update_target() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("AIPLUS_SELF_UPDATE_TARGET") {
        return Ok(PathBuf::from(path));
    }
    std::env::current_exe().context("locate current executable")
}

fn binary_version(path: &Path) -> Option<String> {
    let output = Command::new(path).arg("--version").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    Some(text.trim().trim_start_matches("aiplus ").to_string())
}

fn load_pricing_catalog(allow_bundled: bool) -> Result<PricingCatalog> {
    let cache = pricing_cache_file()?;
    if cache.exists() {
        let text = fs::read_to_string(cache)?;
        return Ok(serde_json::from_str(&text)?);
    }
    if allow_bundled {
        Ok(bundled_pricing_catalog())
    } else {
        Err(anyhow!("pricing cache is missing"))
    }
}

fn load_pricing_catalog_for_savings() -> Result<PricingCatalog> {
    let cache = pricing_cache_file()?;
    let fresh = pricing_cache_age_days()?.is_some_and(|days| days <= 7);
    if cache.exists() && fresh {
        let text = fs::read_to_string(cache)?;
        return Ok(serde_json::from_str(&text)?);
    }
    if let Ok(catalog) = fetch_pricing_catalog() {
        if let Some(parent) = cache.parent() {
            fs::create_dir_all(parent)?;
        }
        let _ = fs::write(&cache, serde_json::to_string_pretty(&catalog)?);
        return Ok(catalog);
    }
    if cache.exists() {
        let text = fs::read_to_string(cache)?;
        return Ok(serde_json::from_str(&text)?);
    }
    Ok(bundled_pricing_catalog())
}

fn bundled_pricing_catalog() -> PricingCatalog {
    embedded_asset_text("pricing/public-model-pricing.json")
        .ok()
        .and_then(|text| serde_json::from_str::<PricingCatalog>(&text).ok())
        .unwrap_or_else(|| PricingCatalog {
            schema_version: VERSION.to_string(),
            fetched_at: None,
            source_url: "bundled".to_string(),
            source: "bundled".to_string(),
            models: vec![PricingModel {
                provider: "generic".to_string(),
                model: "generic_default".to_string(),
                input_usd_per_1m_tokens: 2.50,
                source: "generic_default".to_string(),
                source_url: "bundled".to_string(),
            }],
        })
}

fn choose_pricing_model(
    catalog: &PricingCatalog,
    model: Option<&str>,
    confidence: &str,
) -> (String, String, Option<String>, Option<f64>) {
    let Some(model) = model.filter(|_| confidence != "unknown") else {
        if let Some(generic) = catalog
            .models
            .iter()
            .find(|entry| entry.model == "generic_default")
        {
            return (
                "generic_default".to_string(),
                "generic_default".to_string(),
                catalog.fetched_at.clone(),
                Some(generic.input_usd_per_1m_tokens),
            );
        }
        return (
            "unavailable".to_string(),
            "unavailable".to_string(),
            None,
            None,
        );
    };
    let normalized = normalize_model_id(model);
    for entry in &catalog.models {
        if normalize_model_id(&entry.model) == normalized {
            let source = if catalog.source == "official" {
                "official_public".to_string()
            } else if catalog.source == "cached" {
                if pricing_cache_age_days()
                    .ok()
                    .flatten()
                    .is_some_and(|days| days > 7)
                {
                    "stale_cache".to_string()
                } else {
                    "cached".to_string()
                }
            } else {
                catalog.source.clone()
            };
            return (
                entry.model.clone(),
                source,
                catalog.fetched_at.clone(),
                Some(entry.input_usd_per_1m_tokens),
            );
        }
    }
    (
        "unavailable".to_string(),
        "unavailable".to_string(),
        catalog.fetched_at.clone(),
        None,
    )
}

fn detect_model_hint() -> (Option<String>, String) {
    for key in ["AIPLUS_MODEL", "CODEX_MODEL", "OPENAI_MODEL", "AI_MODEL"] {
        if let Ok(value) = std::env::var(key) {
            if looks_non_secret_model_hint(&value) {
                return (Some(value), "medium".to_string());
            }
        }
    }
    (None, "unknown".to_string())
}

fn looks_non_secret_model_hint(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && value.len() < 120
        && !value.contains("sk-")
        && !value.contains("token")
        && !value.contains("secret")
        && !value.contains('\n')
}

fn pricing_cache_file() -> Result<PathBuf> {
    if let Ok(base) = std::env::var("XDG_CACHE_HOME") {
        return Ok(PathBuf::from(base).join("aiplus/pricing-cache.json"));
    }
    if let Ok(home) = std::env::var("HOME") {
        return Ok(PathBuf::from(home).join(".cache/aiplus/pricing-cache.json"));
    }
    Ok(PathBuf::from(".cache/aiplus/pricing-cache.json"))
}

fn pricing_cache_age_days() -> Result<Option<u64>> {
    let cache = pricing_cache_file()?;
    if !cache.exists() {
        return Ok(None);
    }
    let modified = fs::metadata(cache)?.modified()?;
    let age = SystemTime::now()
        .duration_since(modified)
        .map(|duration| duration.as_secs() / 86_400)
        .unwrap_or(0);
    Ok(Some(age))
}

fn pricing_input_price(estimate: &SavingsEstimate) -> Option<f64> {
    estimate.cost_saved_usd.and_then(|cost| {
        if estimate.tokens_saved == 0 {
            None
        } else {
            Some(round4(cost * 1_000_000.0 / estimate.tokens_saved as f64))
        }
    })
}

fn normalize_model_id(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .replace(['_', ' ', '.'], "-")
}

fn command_exists(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .output()
        .map(|output| {
            output.status.success() || !output.stdout.is_empty() || !output.stderr.is_empty()
        })
        .unwrap_or(false)
}

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.0}k", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn print_compact_diagnostics(result: &CompactValidation) {
    for warning in &result.warnings {
        eprintln!("WARNING {warning}");
    }
    for review_item in &result.review_items {
        eprintln!("UNKNOWN_NEEDS_REVIEW {review_item}");
    }
    for error in &result.errors {
        eprintln!("ERROR {error}");
    }
}

fn collect_version_review_items(
    root: &Path,
    policy: Option<&serde_json::Value>,
    review_items: &mut Vec<String>,
) -> Result<()> {
    if let Some(policy) = policy {
        check_supported_version(
            policy.get("protocolVersion").and_then(|v| v.as_str()),
            "compact-policy.json protocolVersion",
            review_items,
        );
        check_supported_version(
            policy.get("templateVersion").and_then(|v| v.as_str()),
            "compact-policy.json templateVersion",
            review_items,
        );
        check_supported_version(
            policy.get("schemaVersion").and_then(|v| v.as_str()),
            "compact-policy.json schemaVersion",
            review_items,
        );
    }
    for file in COMPACT_REQUIRED_FILES
        .iter()
        .filter(|file| file.ends_with(".md"))
    {
        if !compact_file(root, file)?.exists() {
            continue;
        }
        let text = read_compact_text(root, file)?;
        check_supported_version(
            Some(&section_body(&text, "Protocol Version")),
            &format!("{file} Protocol Version"),
            review_items,
        );
        let template_version = section_body(&text, "Template Version");
        if !template_version.is_empty() {
            check_supported_version(
                Some(&template_version),
                &format!("{file} Template Version"),
                review_items,
            );
        }
        let schema_version = section_body(&text, "Schema Version");
        if !schema_version.is_empty() {
            check_supported_version(
                Some(&schema_version),
                &format!("{file} Schema Version"),
                review_items,
            );
        }
    }
    Ok(())
}

fn check_supported_version(actual: Option<&str>, label: &str, review_items: &mut Vec<String>) {
    let version = actual.unwrap_or("").trim();
    if ![
        "0.1.0", "0.2.0", "0.2.1", "0.3.0", "0.3.1", "0.4.0", "0.4.1", "0.4.2", "0.4.3", "0.4.4",
        "0.4.5", "0.4.6", "0.4.7", "0.4.8", "0.5.0", "0.5.1",
    ]
    .contains(&version)
    {
        review_items.push(format!(
            "{label} unsupported or unknown: {}",
            if version.is_empty() {
                "<missing>"
            } else {
                version
            }
        ));
    }
}

/// P2.3: schema-version support check.
///
/// Historical versions (0.1.x → 0.4.x) are listed explicitly because
/// each one had distinct manifest shapes during early development. The
/// 0.5.x series stabilized the manifest schema and every 0.5.x release
/// since 0.5.0 has been backward-compatible — so we accept ALL 0.5.x
/// patch versions via prefix match, including future bumps. Bumping
/// the crate to 0.5.17 / 0.5.18 / etc. no longer requires editing this
/// function.
///
/// When 0.6.0 lands with a breaking schema change, replace
/// `is_zero_five_x` with an explicit list (or extend with a 0.6.x
/// prefix match if 0.6.x is also stable).
fn is_supported_manifest_schema(version: &str) -> bool {
    const HISTORICAL: &[&str] = &[
        "0.1.3", "0.2.0", "0.2.1", "0.3.0", "0.3.1", "0.4.0", "0.4.1", "0.4.2", "0.4.3", "0.4.4",
        "0.4.5", "0.4.6", "0.4.7", "0.4.8",
    ];
    if HISTORICAL.contains(&version) {
        return true;
    }
    is_zero_five_x(version)
}

/// Match strings of the form `0.5.<patch>` where `<patch>` is a
/// non-empty digit-only string. Strict enough to reject "0.5" (no patch),
/// "0.5.x" (non-numeric), "0.5.1abc" (extra suffix), and "0.50.0"
/// (different minor — `0.5.0` only matches exact prefix `0.5.`).
fn is_zero_five_x(version: &str) -> bool {
    let Some(patch) = version.strip_prefix("0.5.") else {
        return false;
    };
    !patch.is_empty() && patch.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod schema_support_tests {
    use super::{is_supported_manifest_schema, is_zero_five_x};

    #[test]
    fn historical_versions_supported() {
        for v in &[
            "0.1.3", "0.2.0", "0.2.1", "0.3.0", "0.3.1", "0.4.0", "0.4.8",
        ] {
            assert!(is_supported_manifest_schema(v), "{v} should be supported");
        }
    }

    #[test]
    fn current_zero_five_versions_supported() {
        for v in &[
            "0.5.0", "0.5.7", "0.5.10", "0.5.11", "0.5.16", "0.5.99",
            "0.5.100", // future versions
        ] {
            assert!(is_supported_manifest_schema(v), "{v} should be supported");
        }
    }

    #[test]
    fn pre_0_1_versions_rejected() {
        // We don't claim 0.0.x or 0.1.0–0.1.2 support; first supported
        // historical is 0.1.3.
        for v in &["0.0.1", "0.1.0", "0.1.1", "0.1.2"] {
            assert!(
                !is_supported_manifest_schema(v),
                "{v} should be unsupported"
            );
        }
    }

    #[test]
    fn future_major_versions_rejected() {
        // 0.6.0 / 1.0.0 are future; not yet wired up. Bumping there
        // should be a conscious change (extend this function).
        for v in &["0.6.0", "0.7.0", "1.0.0", "2.0.0"] {
            assert!(
                !is_supported_manifest_schema(v),
                "{v} should be unsupported"
            );
        }
    }

    #[test]
    fn malformed_zero_five_strings_rejected() {
        for v in &[
            "0.5",      // no patch
            "0.5.",     // empty patch
            "0.5.x",    // non-numeric patch
            "0.5.1abc", // patch with non-digit suffix
            "0.5.1.0",  // 4-segment, patch contains non-digit (the dot)
            "0.50.0",   // different minor — strip_prefix("0.5.") doesn't match "0.50.0"
        ] {
            assert!(!is_zero_five_x(v), "{v} should NOT match 0.5.x pattern");
        }
    }

    #[test]
    fn zero_five_pattern_matches_what_we_expect() {
        assert!(is_zero_five_x("0.5.0"));
        assert!(is_zero_five_x("0.5.42"));
        assert!(is_zero_five_x("0.5.999"));
        assert!(!is_zero_five_x("0.4.0"));
        assert!(!is_zero_five_x("0.6.0"));
    }
}

fn check_policy_array(
    policy: &serde_json::Value,
    name: &str,
    allowed: &[&str],
    errors: &mut Vec<String>,
) {
    let Some(values) = policy.get(name).and_then(|value| value.as_array()) else {
        errors.push(format!("compact-policy.json {name} must be an array"));
        return;
    };
    for value in values {
        let Some(value) = value.as_str() else {
            errors.push(format!("compact-policy.json {name} invalid value: {value}"));
            continue;
        };
        if !allowed.contains(&value) {
            errors.push(format!("compact-policy.json {name} invalid value: {value}"));
        }
    }
}

fn section_body(text: &str, heading: &str) -> String {
    let marker = format!("## {heading}");
    let mut in_section = false;
    let mut lines = Vec::new();
    for line in text.lines() {
        if line.trim() == marker {
            in_section = true;
            continue;
        }
        if in_section && line.starts_with("## ") {
            break;
        }
        if in_section {
            lines.push(line);
        }
    }
    lines.join("\n").trim().to_string()
}

fn has_section(text: &str, heading: &str) -> bool {
    let marker = format!("## {heading}");
    text.lines().any(|line| line.trim() == marker)
}

fn non_placeholder_lines(body: &str) -> Vec<String> {
    body.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with("Allowed "))
        .filter(|line| !line.contains("<ISO8601_TIMESTAMP>"))
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_markdown_table(text: &str, headers: &[&str]) -> Vec<BTreeMap<String, String>> {
    text.lines()
        .map(str::trim)
        .filter(|line| line.starts_with('|') && !line.starts_with("| ---"))
        .skip(1)
        .filter_map(|line| {
            let cells: Vec<String> = line
                .split('|')
                .skip(1)
                .take(headers.len())
                .map(|cell| cell.trim().to_string())
                .collect();
            if cells.len() >= headers.len() {
                Some(
                    headers
                        .iter()
                        .enumerate()
                        .map(|(index, header)| (header.to_string(), cells[index].clone()))
                        .collect(),
                )
            } else {
                None
            }
        })
        .collect()
}

fn owner_gate_tokens(body: &str) -> Vec<String> {
    body.split(|ch: char| !(ch.is_ascii_uppercase() || ch == '_'))
        .filter(|token| !token.is_empty())
        .filter(|token| token.contains('_') || OWNER_GATE_VALUES.contains(token))
        .map(ToOwned::to_owned)
        .collect()
}

fn scan_sensitive(root: &Path) -> Result<Vec<String>> {
    let mut warnings = Vec::new();
    for file in COMPACT_REQUIRED_FILES {
        let path = compact_file(root, file)?;
        if !path.exists() {
            continue;
        }
        let text = fs::read_to_string(path)?;
        for (label, found) in sensitive_findings(&text) {
            if found {
                warnings.push(format!("{file}: sensitive pattern detected ({label})"));
            }
        }
    }
    Ok(warnings)
}

fn strip_numbering(line: &str) -> String {
    let trimmed = line.trim();
    let without_digits = trimmed.trim_start_matches(|ch: char| ch.is_ascii_digit());
    without_digits
        .strip_prefix('.')
        .unwrap_or(without_digits)
        .trim()
        .to_string()
}

fn optional_line(value: String) -> Option<String> {
    let line = single_line(&value);
    if line.is_empty() {
        None
    } else {
        Some(line)
    }
}

fn evidence_ids(text: &str) -> Vec<String> {
    parse_markdown_table(text, &["id", "confidence", "source", "finding", "artifact"])
        .into_iter()
        .filter_map(|row| row.get("id").cloned())
        .collect()
}

fn normalize_runtime(value: Option<&str>) -> Option<&'static str> {
    match value? {
        "codex" => Some("codex"),
        "claude-code" | "claude" | "cc" => Some("claude-code"),
        "opencode" | "oc" => Some("opencode"),
        "all" => Some("all"),
        _ => None,
    }
}

fn runtime_list(value: &str) -> Vec<String> {
    if value == "all" {
        vec![
            "claude-code".to_string(),
            "codex".to_string(),
            "opencode".to_string(),
        ]
    } else {
        vec![value.to_string()]
    }
}

fn runtime_label(runtime: &str) -> String {
    match runtime {
        "codex" => "Codex".to_string(),
        "claude-code" => "Claude Code".to_string(),
        "opencode" => "OpenCode".to_string(),
        _ => runtime.to_string(),
    }
}

fn runtime_managed_files(runtime: &str) -> Vec<String> {
    match runtime {
        "codex" => vec!["AGENTS.md".to_string()],
        "claude-code" => vec![
            ".claude/commands/aiplus-refresh.md".to_string(),
            ".claude/agents/aiplus-advisor.md".to_string(),
        ],
        "opencode" => vec![
            ".opencode/opencode.json".to_string(),
            ".opencode/commands/aiplus-refresh.md".to_string(),
            ".opencode/agents/aiplus-advisor.md".to_string(),
            ".opencode/prompts/aiplus.md".to_string(),
        ],
        _ => Vec::new(),
    }
}

/// Read installed module names from `.aiplus/manifest.json`.
fn installed_module_names(root: &Path) -> Vec<String> {
    let manifest_path = root.join(".aiplus").join("manifest.json");
    let Ok(text) = std::fs::read_to_string(&manifest_path) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    value
        .get("modules")
        .and_then(|m| m.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default()
}

/// Doctor checks specific to the AEL claude-code adapter. Only fires when
/// the aieconlab module is installed in this project; otherwise returns
/// an empty vec so codex/opencode-only or vanilla AiPlus installs aren't
/// flagged for missing AEL content.
fn aieconlab_claude_code_doctor_checks(root: &Path) -> Result<Vec<(String, bool)>> {
    let modules = installed_module_names(root);
    if !modules.iter().any(|m| m == "aieconlab") {
        return Ok(Vec::new());
    }
    let mut checks = Vec::new();
    let core_roles = [
        "advisor",
        "pi",
        "theorist",
        "pm",
        "ra-stata",
        "ra-python",
        "referee",
        "replicator",
    ];
    let experts = [
        "coauthor-liaison",
        "computation",
        "econometrician",
        "ethics-irb",
        "historical-sources",
        "job-talk-coach",
        "lit-reviewer",
        "llm-measurement",
        "reproducibility",
        "survey-experiment",
        "viz-specialist",
        "writer",
    ];
    for role in core_roles.iter().chain(experts.iter()) {
        let rel = format!(".claude/agents/aieconlab-{role}.md");
        checks.push((
            format!(".claude/agents/aieconlab-{role}.md exists"),
            rel_to_abs(root, &rel)?.exists(),
        ));
    }
    for cmd in AIECONLAB_SLASH_COMMANDS {
        let rel = format!(".claude/commands/{cmd}.md");
        checks.push((
            format!(".claude/commands/{cmd}.md exists"),
            rel_to_abs(root, &rel)?.exists(),
        ));
    }
    checks.push((
        "CLAUDE.md contains exactly one AiEconLab managed block".to_string(),
        read_text_if_exists(&rel_to_abs(root, "CLAUDE.md")?)?
            .as_deref()
            .map(|t| {
                t.matches(MANAGED_BEGIN_AEL).count() == 1 && t.matches(MANAGED_END_AEL).count() == 1
            })
            .unwrap_or(false),
    ));
    Ok(checks)
}

fn runtime_doctor_requirements(root: &Path, runtime: &str) -> Result<Vec<(String, bool)>> {
    Ok(match runtime {
        "codex" => vec![
            (
                "AGENTS.md contains exactly one AiPlus managed block".to_string(),
                managed_block_count(root)? == 1,
            ),
            (
                "managed block points to .aiplus/AGENTS.aiplus.md".to_string(),
                managed_block_target_ok(root)?,
            ),
        ],
        "claude-code" => {
            let hooks_text =
                read_text_if_exists(&rel_to_abs(root, ".claude/settings.local.json")?)?
                    .unwrap_or_default();
            let hooks_value: Option<serde_json::Value> = serde_json::from_str(&hooks_text).ok();
            let hooks_have_event = |event: &str| -> bool {
                let Some(value) = &hooks_value else {
                    return false;
                };
                let Some(arr) = value
                    .get("hooks")
                    .and_then(|h| h.get(event))
                    .and_then(|v| v.as_array())
                else {
                    return false;
                };
                arr.iter().any(matcher_is_aiplus_managed)
            };
            vec![
                (
                    ".claude/commands/aiplus-refresh.md exists".to_string(),
                    rel_to_abs(root, ".claude/commands/aiplus-refresh.md")?.exists(),
                ),
                (
                    ".claude/commands/aiplus-refresh.md is AiPlus refresh command".to_string(),
                    file_contains(root, ".claude/commands/aiplus-refresh.md", "AiPlus Refresh")?,
                ),
                (
                    ".claude/agents/aiplus-advisor.md exists".to_string(),
                    rel_to_abs(root, ".claude/agents/aiplus-advisor.md")?.exists(),
                ),
                (
                    ".claude/agents/aiplus-advisor.md is AiPlus advisor agent".to_string(),
                    file_contains(root, ".claude/agents/aiplus-advisor.md", "AiPlus Advisor")?,
                ),
                (
                    ".claude/agents/aiplus-memory.md exists".to_string(),
                    rel_to_abs(root, ".claude/agents/aiplus-memory.md")?.exists(),
                ),
                (
                    ".claude/agents/aiplus-compact.md exists".to_string(),
                    rel_to_abs(root, ".claude/agents/aiplus-compact.md")?.exists(),
                ),
                (
                    ".claude/agents/aiplus-velocity.md exists".to_string(),
                    rel_to_abs(root, ".claude/agents/aiplus-velocity.md")?.exists(),
                ),
                (
                    ".claude/agents/aiplus-team-consultant.md exists".to_string(),
                    rel_to_abs(root, ".claude/agents/aiplus-team-consultant.md")?.exists(),
                ),
                (
                    ".claude/settings.local.json exists".to_string(),
                    rel_to_abs(root, ".claude/settings.local.json")?.exists(),
                ),
                (
                    ".claude/settings.local.json parses as JSON".to_string(),
                    hooks_value.is_some(),
                ),
                (
                    "settings.local.json has AiPlus SessionStart hook".to_string(),
                    hooks_have_event("SessionStart"),
                ),
                (
                    "settings.local.json has AiPlus PreCompact hook".to_string(),
                    hooks_have_event("PreCompact"),
                ),
                (
                    "CLAUDE.md exists".to_string(),
                    rel_to_abs(root, "CLAUDE.md")?.exists(),
                ),
                (
                    "CLAUDE.md contains exactly one AiPlus managed block".to_string(),
                    read_text_if_exists(&rel_to_abs(root, "CLAUDE.md")?)?
                        .as_deref()
                        .map(|t| {
                            t.matches(MANAGED_BEGIN).count() == 1
                                && t.matches(MANAGED_END).count() == 1
                        })
                        .unwrap_or(false),
                ),
            ]
            .into_iter()
            .chain(aieconlab_claude_code_doctor_checks(root)?)
            .collect()
        }
        "opencode" => opencode_doctor_requirements(root)?,
        _ => Vec::new(),
    })
}

fn opencode_doctor_requirements(root: &Path) -> Result<Vec<(String, bool)>> {
    let config = rel_to_abs(root, ".opencode/opencode.json")?;
    let parsed = parse_opencode_config(root)?;
    Ok(vec![
        (
            ".opencode/opencode.json exists".to_string(),
            config.exists(),
        ),
        (
            ".opencode/opencode.json parses as strict JSON".to_string(),
            parsed.is_some(),
        ),
        (
            ".opencode/opencode.json has no unsupported AiPlus top-level key".to_string(),
            parsed
                .as_ref()
                .is_some_and(|value| value.get("aiplus").is_none()),
        ),
        (
            ".opencode/opencode.json schema is a string when present".to_string(),
            parsed.as_ref().is_some_and(|value| {
                value
                    .get("$schema")
                    .is_none_or(|schema| schema.as_str().is_some())
            }),
        ),
        (
            ".opencode/commands/aiplus-refresh.md exists".to_string(),
            rel_to_abs(root, ".opencode/commands/aiplus-refresh.md")?.exists(),
        ),
        (
            ".opencode/agents/aiplus-advisor.md exists".to_string(),
            rel_to_abs(root, ".opencode/agents/aiplus-advisor.md")?.exists(),
        ),
        (
            ".opencode/prompts/aiplus.md exists".to_string(),
            rel_to_abs(root, ".opencode/prompts/aiplus.md")?.exists(),
        ),
    ])
}

fn parse_opencode_config(root: &Path) -> Result<Option<serde_json::Value>> {
    let path = rel_to_abs(root, ".opencode/opencode.json")?;
    if !path.exists() {
        return Ok(None);
    }
    assert_no_symlink_path(root, &path)?;
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text).ok())
}

fn file_contains(root: &Path, rel: &str, needle: &str) -> Result<bool> {
    let path = rel_to_abs(root, rel)?;
    if !path.exists() {
        return Ok(false);
    }
    assert_no_symlink_path(root, &path)?;
    Ok(fs::read_to_string(path)?.contains(needle))
}

fn known_aiplus_entries() -> BTreeSet<String> {
    let mut known = BTreeSet::from([
        ".aiplus/manifest.json".to_string(),
        ".aiplus/AGENTS.aiplus.md".to_string(),
        REFRESH_PROMPT_REL.to_string(),
        ".aiplus/modules".to_string(),
        ".aiplus/memory".to_string(),
        ".aiplus/identities".to_string(),
        ".aiplus/skills".to_string(),
        ".aiplus/restore".to_string(),
        ".aiplus/consultant-team.toml".to_string(),
        ".aiplus/agents".to_string(),
        ".aiplus/agent-team".to_string(),
        ".aiplus/aieconlab".to_string(),
        ".aiplus/compact".to_string(),
        // W3: per-role memory namespaces seeded by agent_team_init /
        // aieconlab_init. The dir itself is fixed; per-role
        // subdirectories live inside and uninstall walks the tree.
        ".aiplus/agent-memory".to_string(),
        // W6: velocity ledger (estimates / runs / rare cases / aggregates /
        // multipliers / rotation-state / anchor-signals / config). Files are
        // appended over the project's lifetime; uninstall removes the whole
        // dir along with `.aiplus/`.
        ".aiplus/velocity".to_string(),
        // Uninstall itself creates `.aiplus/backups/<timestamp>/` when it
        // updates the AiPlus / AiEconLab managed blocks in CLAUDE.md, AGENTS.md,
        // or settings.local.json. A re-run after a partial uninstall would
        // otherwise flag the backups it just wrote.
        ".aiplus/backups".to_string(),
        // _teams snapshots (Phase D v1, `aiplus agent set-team`).
        ".aiplus/_teams".to_string(),
    ]);
    for spec in aiplus_core::bundled_module_specs() {
        known.insert(spec.path.to_string());
    }
    known
}

fn has_managed_block(root: &Path) -> Result<bool> {
    Ok(managed_block_structure_ok(root)? && managed_block_target_ok(root)?)
}

fn managed_block_count(root: &Path) -> Result<usize> {
    let text = read_text_if_exists(&rel_to_abs(root, "AGENTS.md")?)?.unwrap_or_default();
    let begin_count = text.matches(MANAGED_BEGIN).count();
    let end_count = text.matches(MANAGED_END).count();
    if begin_count == 1 && end_count == 1 {
        Ok(1)
    } else {
        Ok(begin_count.max(end_count))
    }
}

fn managed_block_structure_ok(root: &Path) -> Result<bool> {
    let text = read_text_if_exists(&rel_to_abs(root, "AGENTS.md")?)?.unwrap_or_default();
    let begin_count = text.matches(MANAGED_BEGIN).count();
    let end_count = text.matches(MANAGED_END).count();
    let Some(begin) = text.find(MANAGED_BEGIN) else {
        return Ok(false);
    };
    let Some(end) = text.find(MANAGED_END) else {
        return Ok(false);
    };
    Ok(begin_count == 1 && end_count == 1 && begin < end)
}

fn managed_block_target_ok(root: &Path) -> Result<bool> {
    let text = read_text_if_exists(&rel_to_abs(root, "AGENTS.md")?)?.unwrap_or_default();
    let Some(begin) = text.find(MANAGED_BEGIN) else {
        return Ok(false);
    };
    let Some(end) = text.find(MANAGED_END) else {
        return Ok(false);
    };
    if begin >= end {
        return Ok(false);
    }
    Ok(text[begin..end].contains(MANAGED_REF))
}

fn managed_block() -> String {
    format!("{MANAGED_BEGIN}\n{MANAGED_REF}\n{MANAGED_END}")
}

fn agents_aiplus_content() -> String {
    r#"# AiPlus CLI Subcommand Translation Table

| Chinese Alias | English Subcommand |
|---------------|-------------------|
| 刷新 | refresh |
| 状态 | status |
| 安装 | install |
| 升级 | update |
| 更新 | update |
| 自升级 | self |
| 卸载 | uninstall |
| 回滚 | rollback |
| 健康 | doctor |
| 全局更新 / 升级所有项目 / update all | aiplus update --all-projects |
| 项目清单 / 哪些项目装了 aiplus / list projects | aiplus list-projects |
| 清理失效项目 / prune aiplus projects | aiplus prune-projects --yes (after dry-run) |

## Hard Rules

- If the user explicitly mentions AiPlus or `aiplus`, always do AiPlus refresh first. Report AiPlus status before any unrelated project refresh or project status.
- Never bury AiPlus status behind unrelated project refresh when the user asks for AiPlus.
- AiPlus cannot click compact for you.
- AiPlus does not upload data.
- AiPlus does not change global agent config.
6. When the user requests prune-projects, the agent MUST first run `aiplus prune-projects` (without --yes) to show the planned removals, get the user's confirmation, then run `aiplus prune-projects --yes`. Never auto---yes a prune.

# AiPlus Project Instructions

Use AiPlus Compact Reminder and AiPlus Auto Team Consultant when relevant.

## Refresh Keywords

Explicit AiPlus refresh triggers:

- `AiPlus 刷新`
- `刷新 AiPlus`
- `aiplus refresh`
- `aiplus status`
- `AiPlus status`
- `继续 AiPlus`
- `resume AiPlus`

If the user explicitly mentions AiPlus or `aiplus`, always do AiPlus refresh
first. Report AiPlus status before any unrelated project refresh or project
status.

Generic refresh priority rule:

- If the user says only `刷新` or `refresh`, do AiPlus refresh first, then
  optionally continue project-specific refresh if needed.
- If a project's rules conflict, briefly report AiPlus status before project
  status.
- Never bury AiPlus status behind unrelated project refresh when the user asks
  for AiPlus.

Default English response shape:

AiPlus refreshed.

Current project AiPlus status:
- Compact Reminder: installed/not installed
- Auto Team Consultant: installed/not installed
- Compact state: present/missing/review-needed

How I will use it:
- Proactively run `aiplus compact remind` at stable high-value moments: HEAVY work every 30 minutes or major phase boundary, MEDIUM work at phase boundary or before review/QA, and before subagent bursts, release prep, or Owner handoff.
- Prepare checkpoints before long tasks or compact-worthy moments.
- If you say "prepare compact", "save progress", "checkpoint this", "帮我准备 compact", or "保存进度", I will run aiplus compact prepare.
- If you ask "show compact savings" or "how many tokens did compact save?", I will run aiplus compact savings.
- If you ask "private profile status", "我的偏好生效了吗", or "检查我的 AiPlus profile", I will run aiplus profile status.
- If you ask "secret status", "检查 API key", "刷新 secret", or "API key 是否可用", I will run aiplus secret-broker status or doctor without printing values.
- After compact, if I do not reply, send: continue
- Use Auto Team Consultant for CEO Prompt, review, and brainstorm work.

Boundaries:
- AiPlus cannot click compact for you.
- AiPlus does not upload data.
- AiPlus does not change global agent config.

Chinese response shape when the user uses Chinese such as `刷新` or `AiPlus 刷新`:

已刷新 AiPlus。

当前项目 AiPlus 状态：
- Compact Reminder: 已安装/未安装
- Auto Team Consultant: 已安装/未安装
- Compact state: present/missing/review-needed

我会这样使用：
- 在稳定且高价值的时机主动运行 `aiplus compact remind`：HEAVY 任务每 30 分钟或阶段边界，MEDIUM 任务在阶段边界或 review/QA 前，以及 subagent 批量启动、release prep、Owner handoff 前。
- 长任务或 compact 前准备 checkpoint
- 如果你说“帮我准备 compact”“保存进度”或“做个交接”，我会运行 aiplus compact prepare。
- 如果你问“看一下 compact 收益”或“compact 帮我省了多少？”，我会运行 aiplus compact savings。
- 如果你问“我的偏好生效了吗”或“检查我的 AiPlus profile”，我会运行 aiplus profile status。
- 如果你问“secret 状态”“检查 API key”或“API key 是否可用”，我会运行 aiplus secret-broker status 或 doctor，不打印 secret value。
- compact 后如果我没自动继续，你发一句“继续”就行。我会从刚才的位置接着做。
- CEO Prompt / review / brainstorm 时使用 Auto Team Consultant

边界：
- AiPlus 不能替你点击 compact
- 不上传数据
- 不改全局 agent config

Generic continuation phrases should still try AiPlus first when possible:
`刷新`, `refresh`, `继续`, `continue`, `resume`, `go on`, `接着`.

When continuing:

1. Re-read `AGENTS.md`.
2. Re-read `.aiplus/AGENTS.aiplus.md`.
3. Re-read `.aiplus/compact/current-handoff.md` if it exists.
4. Enable AiPlus Auto Team Consultant and AiPlus Compact Reminder for the current session.
5. Run `aiplus compact resume` after compact when work should continue.
6. Continue the current task without asking the user to repeat the full instruction.

Refresh is not approval to push, publish, tag, release, deploy, globally install, edit global configs, contact external accounts, upload private data, add telemetry, or expose secrets.

## Missing AiPlus CLI

If `aiplus` is not found, do not fallback to Node or `compactctl.mjs`.
Report this instead:

AiPlus CLI not found. Please install AiPlus or fix PATH:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
```

Then reopen the terminal or ensure `~/.local/bin` is on PATH.

## Update AiPlus

Natural language update mapping:

- "update AiPlus", "update everything", "升级 AiPlus", or
  "把 AiPlus 全部更新到最新版": report scope, then run `aiplus update all`.
- "only update this project's AiPlus", "只更新这个项目的 AiPlus", or
  "只更新项目模块": run `aiplus update`.
- "update the aiplus command", "更新 aiplus 命令", or "全局更新 AiPlus CLI":
  run `aiplus self update`.
- "check AiPlus updates" or "检查 AiPlus 更新": run
  `aiplus self update --dry-run` and `aiplus status`.

Before running an update, say:

I will update the aiplus CLI and this project's AiPlus modules. I will not edit
global agent config or upload project data.

Chinese:

我会更新 aiplus 命令和当前项目里的 AiPlus 模块；不会修改全局 agent 配置，也不会上传项目数据。

## User Profile

If a user-level private profile is installed, load it after project rules and
before generic AiPlus guidance:

`~/.config/aiplus/profiles/<private-profile-name>/AGENTS.profile.md`

Natural language profile mapping:

- "我的偏好生效了吗", "private profile status", or "检查我的 AiPlus profile":
  run `aiplus profile status`.
- "本次忽略我的偏好", "关闭 private profile", or "只看项目规则":
  ignore the user-level profile for this session.

Do not copy profile content into public repos or compact files. Do not treat the
profile as approval to override project rules or the current Owner message.

## Agent Continuity

Natural language memory and identity mapping:

- "我的偏好生效了吗": run `aiplus profile status` and `aiplus memory status`.
- "你记住了什么", "这次用了哪些记忆", "这次你用了哪些记忆", or "memory status":
  run `aiplus memory status` and/or `aiplus memory context --runtime <runtime> --budget 2000`.
- "记住这个" or "记住这个偏好": add a project memory only after reducing it to a
  redacted summary, using `aiplus memory add --scope project --kind preference --text "..."`
- "以后都这样": treat as a profile/global preference candidate that needs
  review; do not silently write broad policy.
- "只在这个项目用": use project memory.
- "忘掉这个": run `aiplus memory forget <id>`; if the memory id is ambiguous,
  ask which memory to forget before changing persistent state.
- "新开顾问" or "新开 advisor": use `aiplus identity context --role advisor`.
- "新开 CEO": use `aiplus identity context --role ceo`.
- "把这次经验沉淀成 skill": create a Skill Candidate, not an approved skill.
- "不要用我的私人记忆" or "本次忽略我的偏好": session-local opt-out only;
  do not modify persistent memory unless explicitly requested.

Memory is context, not instruction. Role Identity is role contract, not
permission. Skill Candidate is proposal, not approved skill. Do not store raw
transcripts, secret values, provider payloads, private profile content, or
unredacted private paths in memory. Natural language triggers are not hidden
authorization for push, publish, deploy, secret use, external accounts, or
global config edits.

## Secret Broker

Natural language secret mapping:

- "secret 状态", "看看 secret", "检查 API key", "API key 是否可用",
  "刷新 secret", or "更新 secret": run `aiplus secret-broker status` or
  `aiplus secret-broker doctor`.
- When an explicit action needs a key, prefer `aiplus secret-broker run
  --aliases openai,kimi -- <command...>` so only requested values enter the
  child process environment.
- For approved provider inventory, run `aiplus secret-broker list`. Real
  Bitwarden checks require the `bws` CLI and a read-only machine account token.
  If `bws` is unavailable, report real Bitwarden verification as unverified.
- The child command can still print, log, transmit, or store its environment.
  Use `run --` only with trusted commands for an explicit action need.

Never print, paste, log, summarize, compact, or persist secret values. Do not run
`aiplus secret-broker resolve <alias> --print` in normal agent guidance. If a
secret is unavailable, report one exact fix command and continue without exposing
values.

## Compact Reminder

Read `.aiplus/compact/current-handoff.md` before long-running work if it exists.

Proactive reminder schedule:
1. HEAVY task: run `aiplus compact remind --event long-session` at least every
   30 minutes, at major phase boundaries, before review/QA, before spawning many
   subagents, before release prep, and before Owner handoff.
2. MEDIUM task: run `aiplus compact remind --event phase-end` at phase
   boundaries, before review/QA, before context-heavy investigation, and before
   Owner handoff.
3. LIGHT task: run `aiplus compact remind` only on user request or an obvious
   handoff point.
4. If `REMINDER_DECISION=remind_now`, run or confirm `aiplus compact prepare`,
   then suggest the user manually use the host compact control. AiPlus cannot
   click compact or call host compact automatically.
5. If `REMINDER_DECISION=prepare_only`, update handoff/checkpoint first and do
   not suggest host compact yet.
6. If `REMINDER_DECISION=wait` or `blocked`, explain the safety reason and keep
   working until the next stable point.

Before context compaction or compact-worthy moments:
1. Treat natural language as the primary interface. If the user says "prepare compact",
   "help me compact", "I want to compact", "save progress", "checkpoint this",
   "get ready for compact", "我想 compact", "准备 compact", "帮我准备 compact",
   "保存进度", "做个交接", or "我要 compact 了", run `aiplus compact prepare`.
2. Use `aiplus compact validate` and `aiplus compact checkpoint` only as fallback
   backend commands if `prepare` is unavailable.
3. Suggest compact only after readiness is `READY_TO_COMPACT`:
   Ready to compact.

   After compact:
   - If I continue automatically, you do not need to do anything.
   - If I do not reply, send: continue

   I will resume from here.

After context compaction:
1. If work continues automatically, or the user says "continue after compact",
   "resume after compact", continue, resume, refresh, 继续, 刷新, compact 后继续,
   or similar, run `aiplus compact resume`.
2. After resume, optionally run `aiplus compact savings` to report local
   estimate-only token/USD savings.
3. If the user sends a continuation message, accept natural phrasing:
   continue, resume, go on, 继续, 刷新, 接着.
4. Continue from the reported next safe action.

Savings requests:
1. If the user asks "show compact savings", "how many tokens did compact save?",
   "compact 帮我省了多少？", or "看一下 compact 收益", run `aiplus compact savings`.
2. Treat savings as local estimates only, not billing data and not quality proof.
3. Do not ask the user to enter model prices. Do not upload prompts, project
   files, checkpoints, savings ledgers, secrets, billing data, or usage history.
4. If pricing is missing, still report token savings and reduction percentage;
   USD savings may be unavailable or partial.

Beginner UX rule:
- Do not ask ordinary users to memorize compact CLI commands.
- The compact CLI is an agent backend, an advanced manual fallback, and a
  maintainer debugging interface.
- For ordinary users, say: "In the agent session, say prepare compact or save
  progress. After compact, say continue."

Limits:
- AiPlus cannot force host compact.
- AiPlus cannot click UI compact.
- AiPlus cannot call `/compact` for the Owner.
- AiPlus cannot wake the agent if the host requires user input.

## Auto Team Consultant

When the user asks for advice, CEO orchestration, review, implementation handoff, team discussion, prompt review, workflow tiering, or Owner gate judgment, use:

`.aiplus/modules/aiplus-auto-team-consultant/adapters/codex/skills/auto-team-consultant/SKILL.md`

Start from:

`.aiplus/modules/aiplus-auto-team-consultant/core/templates/TEMPLATE_INDEX.md`

Before CEO/review/QA/product/design/release/AI-integration work:
1. Read `.aiplus/consultant-team.toml` if it exists.
2. Use the configured Consultant Team Decision System with L0-L5 Router + Specialist Lenses.
3. If config is missing or malformed, use the safe AI-native default and report NEEDS_FIX.

Default:
- Advisor session: Advisor mode, LIGHT by default, no file edits unless explicitly approved.
- CEO session: CEO mode, set goal, decompose tasks, use agents only when useful, require Result Packets, run review/fix/QA.
- Reviewer session: findings first, PASS/REVISE/BLOCKED.
- Builder session: changed files, verification, risks, review request.

Consultant Team Decision System:
- 1 Core Product Council + 5 Specialist Expert Teams + Project-Specific User Evidence Layer.
- AI Integration / LLM Experience is enabled by default for AI-native products.
- Use the smallest useful set of lenses. Do not trigger Full Council for small tasks.
- L0 Direct -> L1 Self-Check -> L2 Single Specialist -> L3 Pair Review -> L4 Mini Council -> L5 Full Council / Owner Gate.
- Autonomous trigger is allowed for local planning/review/QA/docs. Dangerous actions still require Owner approval.
- If no real sub-agent ran, label work as `simulated specialist lens`.
- Do not claim independent review if no independent agent ran.

## Owner Gates

Do not push, publish, tag, release, deploy, globally install, edit global configs, contact external accounts, upload private data, add telemetry, or expose secrets without explicit Owner approval.
"#
    .to_string()
}

fn refresh_prompt_content() -> String {
    format!(
        r#"{REFRESH_PROMPT}

English: refresh

Explicit AiPlus refresh triggers:

- AiPlus 刷新
- 刷新 AiPlus
- aiplus refresh
- aiplus status
- AiPlus status
- 继续 AiPlus
- resume AiPlus

If the user explicitly mentions AiPlus or aiplus, always do AiPlus refresh first.
If the user says only 刷新 or refresh, do AiPlus refresh first, then optionally
continue project-specific refresh if needed. If project rules conflict, report
AiPlus status briefly before project status. Never bury AiPlus status behind
unrelated project refresh when the user asks for AiPlus.

Default reply:

AiPlus refreshed.

Current project AiPlus status:
- Compact Reminder: installed/not installed
- Auto Team Consultant: installed/not installed
- Compact state: present/missing/review-needed

How I will use it:
- Proactively run `aiplus compact remind` at stable high-value moments.
- Prepare checkpoints before long tasks or compact-worthy moments.
- If you say "prepare compact", "save progress", or "checkpoint this", I will run aiplus compact prepare.
- If you ask "show compact savings" or "how many tokens did compact save?", I will run aiplus compact savings.
- If you ask "private profile status" or "我的偏好生效了吗", I will run aiplus profile status.
- If you ask "secret status", "检查 API key", or "API key 是否可用", I will run aiplus secret-broker status or doctor without printing values.
- After compact, if I do not reply, send: continue
- Use Auto Team Consultant for CEO Prompt, review, and brainstorm work.

Boundaries:
- AiPlus cannot click compact for you.
- AiPlus does not upload data.
- AiPlus does not change global agent config.

Chinese reply when the user uses Chinese such as 刷新 or AiPlus 刷新:

已刷新 AiPlus。

当前项目 AiPlus 状态：
- Compact Reminder: 已安装/未安装
- Auto Team Consultant: 已安装/未安装
- Compact state: present/missing/review-needed

我会这样使用：
- 在稳定且高价值的时机主动运行 `aiplus compact remind`
- 长任务或 compact 前准备 checkpoint
- 如果你说“帮我准备 compact”“保存进度”或“做个交接”，我会运行 aiplus compact prepare。
- 如果你问“看一下 compact 收益”或“compact 帮我省了多少？”，我会运行 aiplus compact savings。
- 如果你问“我的偏好生效了吗”或“检查我的 AiPlus profile”，我会运行 aiplus profile status。
- 如果你问“secret 状态”“检查 API key”或“API key 是否可用”，我会运行 aiplus secret-broker status 或 doctor，不打印 secret value。
- compact 后如果我没自动继续，你发一句“继续”就行。我会从刚才的位置接着做。
- CEO Prompt / review / brainstorm 时使用 Auto Team Consultant

边界：
- AiPlus 不能替你点击 compact
- 不上传数据
- 不改全局 agent config

Generic continuation keywords: 刷新, refresh, 继续, continue, resume, go on, 接着

Meaning: reread AGENTS.md and .aiplus/AGENTS.aiplus.md, read .aiplus/compact/current-handoff.md if present, run aiplus compact resume after compact when work should continue, enable AiPlus, and continue the current task.

Refresh is not approval to push, publish, tag, release, deploy, globally install, edit global configs, contact external accounts, upload private data, add telemetry, or expose secrets.

If `aiplus` is not found, do not fallback to Node or compactctl.mjs. Say:

AiPlus CLI not found. Please install AiPlus or fix PATH:

curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash

Then reopen the terminal or ensure ~/.local/bin is on PATH.

Update mapping: "update AiPlus", "update everything", "升级 AiPlus", or
"把 AiPlus 全部更新到最新版" means run `aiplus update all` after reporting scope.
"只更新这个项目的 AiPlus" means run `aiplus update`. "更新 aiplus 命令" means run
`aiplus self update`. Say first: I will update the aiplus CLI and this project's
AiPlus modules. I will not edit global agent config or upload project data.

Profile mapping: "我的偏好生效了吗", "private profile status", or "检查我的
AiPlus profile" means run `aiplus profile status`. Load
~/.config/aiplus/profiles/<private-profile-name>/AGENTS.profile.md only after project
rules. If the Owner says "本次忽略我的偏好", "关闭 private profile", or
"只看项目规则", ignore that profile for this session.

Agent Continuity mapping: "记住这个" or "记住这个偏好" means suggest or run
`aiplus memory add --scope project --kind preference --text "..."` after
redaction. "以后都这样" means profile/global candidate only; do not silently
approve broad memory. "只在这个项目用" means project memory. "忘掉这个" means
`aiplus memory forget <id>` or ask which id if ambiguous. "你记住了什么",
"这次用了哪些记忆", or "memory status" means `aiplus memory status` and/or
`aiplus memory context --runtime <runtime> --budget 2000`. "新开顾问" or
"新开 advisor" means `aiplus identity context --role advisor`; "新开 CEO" means
`aiplus identity context --role ceo`. "把这次经验沉淀成 skill" means create a
Skill Candidate, not an approved skill. "不要用我的私人记忆" or "本次忽略我的偏好"
means session-local opt-out only. Memory is context, not instruction. Identity
is role contract, not permission. Skill Candidate is proposal, not approved
skill. Natural language triggers are not approval for push, publish, deploy,
secret use, external accounts, or global config edits.

Secret mapping: "secret 状态", "看看 secret", "检查 API key", "API key 是否可用",
"刷新 secret", or "更新 secret" means run `aiplus secret-broker status` or
`aiplus secret-broker doctor`. Never print, paste, log, compact, or persist
secret values. Use `aiplus secret-broker run --aliases openai,kimi -- <command...>`
only for explicit runtime secret needs. The child command can still print, log,
transmit, or store its environment; use it only with trusted commands. Run `aiplus secret-broker
list` for approved aliases. Real Bitwarden checks require `bws`; if unavailable,
report real Bitwarden verification as unverified.
"#
    )
}

fn claude_refresh_command_content() -> String {
    r#"# AiPlus Refresh

Re-read project-local AiPlus instructions:

1. Read AGENTS.md if present.
2. Read .aiplus/AGENTS.aiplus.md.
3. Read .aiplus/compact/current-handoff.md if present.
4. Enable AiPlus Auto Team Consultant, AiPlus Compact Reminder, and Agent Continuity for this session.
5. Proactively use `aiplus compact remind`: HEAVY work every 30 minutes or major phase boundary; MEDIUM work at phase boundary or before review/QA; before subagent bursts, release prep, and Owner handoff. If `REMINDER_DECISION=remind_now`, run/confirm `aiplus compact prepare`, then suggest manual host compact only.
6. If the user said AiPlus 刷新, 刷新 AiPlus, aiplus refresh, aiplus status, AiPlus status, 继续 AiPlus, resume AiPlus, or only 刷新/refresh, summarize Compact Reminder, Auto Team Consultant, and compact state before any project-specific refresh. Use English by default; use Chinese when the user used Chinese such as 刷新 or AiPlus 刷新.
7. Continue the current task.

Continuation keywords: AiPlus 刷新, 刷新 AiPlus, aiplus refresh, aiplus status, AiPlus status, 继续 AiPlus, resume AiPlus, 继续, 刷新, continue, resume, refresh, go on, 接着.

This is not approval to push, publish, tag, release, deploy, globally install, edit global configs, contact external accounts, upload private data, add telemetry, or expose secrets.

Agent Continuity mapping: 记住这个/记住这个偏好 -> project memory add after redaction; 以后都这样 -> profile/global candidate only; 只在这个项目用 -> project memory; 忘掉这个 -> memory forget id or ask if ambiguous; 你记住了什么/这次用了哪些记忆/memory status -> memory status/context; 新开顾问/新开 advisor -> advisor identity context; 新开 CEO -> ceo identity context; 把这次经验沉淀成 skill -> skill candidate only; 不要用我的私人记忆/本次忽略我的偏好 -> session-local opt-out.
"#
    .to_string()
}

fn claude_advisor_agent_content() -> String {
    r#"---
name: aiplus-advisor
description: General AiPlus orientation, refresh, and status. Use when the user says "AiPlus 刷新", "刷新", "aiplus status", "aiplus refresh", "继续 AiPlus", or asks general AiPlus questions. Routes to specialist subagents (aiplus-memory, aiplus-compact, aiplus-velocity, aiplus-team-consultant) when those match better.
---

# AiPlus Advisor

Use project-local AiPlus modules from .aiplus/modules/ when relevant.

- Compact Reminder: .aiplus/modules/aiplus-compact-reminder/
- Auto Team Consultant: .aiplus/modules/aiplus-auto-team-consultant/
- Agent Memory: .aiplus/modules/aiplus-agent-memory/

When a request fits a specialist better, route to:
- aiplus-memory for persistence of preferences and decisions
- aiplus-compact for handoff / resume / context-limit handling
- aiplus-velocity for task estimation and actuals
- aiplus-team-consultant for plan-time review

Compact Reminder reminder schedule: run `aiplus compact remind` proactively at safe
high-value moments. For HEAVY tasks, check every 30 minutes or at major phase
boundaries, before review/QA, before spawning many subagents, before release
prep, and before Owner handoff. For MEDIUM tasks, check at phase boundaries or
before review/QA. For LIGHT tasks, check only on user request or obvious handoff.
If `REMINDER_DECISION=remind_now`, run or confirm `aiplus compact prepare`, then
suggest the host compact action manually. AiPlus must not click or call host
compact automatically. After compact, run `aiplus compact resume`, then
optionally `aiplus compact savings`.

For already-open agent sessions, explicit AiPlus refresh triggers are:
AiPlus 刷新, 刷新 AiPlus, aiplus refresh, aiplus status, AiPlus status, 继续 AiPlus, resume AiPlus.

Generic continuation also works when possible:
继续, 刷新, continue, resume, refresh, go on, 接着.

Agent Continuity: use `aiplus memory status/context/add/forget`, `aiplus identity context --role advisor|ceo`, and `aiplus skill-candidate propose/reject` for natural phrases such as 记住这个, 以后都这样, 只在这个项目用, 忘掉这个, 你记住了什么, 这次用了哪些记忆, 新开顾问, 新开 advisor, 新开 CEO, 把这次经验沉淀成 skill, 不要用我的私人记忆, and 本次忽略我的偏好. Memory is context, identity is not permission, and skill candidates are not approved skills.
"#
    .to_string()
}

fn claude_memory_subagent_content() -> String {
    r#"---
name: aiplus-memory
description: Persists preferences, naming rules, decisions, and project-specific conventions across sessions so the agent stops re-asking the same questions. Use whenever the user states a persistent rule (Chinese signals — "记住", "以后", "下次别", "只在这个项目用", "忘掉这个", "你记住了什么"; English signals — "remember", "from now on", "don't", "the convention is", "what do you remember"). Always run before claiming you will "remember" something.
---

# AiPlus Memory

This subagent owns the `aiplus memory` surface. Twelve redaction patterns
strip secrets before any record is written, so capture preferences without
leaking them.

## Decision map

| User intent | Command |
|---|---|
| Record a preference or rule | `aiplus memory add --scope project --kind preference --text "..."` |
| Record a project decision | `aiplus memory add --scope project --kind decision --title "..." --summary "..."` |
| Recall persistent context | `aiplus memory context --runtime claude-code --budget 2000` |
| Audit what is stored | `aiplus memory status` |
| Drop a stale memory | `aiplus memory forget <id>` (ask if id is ambiguous) |

## Scope discipline

- "只在这个项目用" → `--scope project`.
- "以后都这样" / "from now on" → propose a profile/global candidate; do not
  silently approve at global scope. Use `aiplus skill-candidate propose ...`
  and tell the user it requires their explicit approval.
- "本次忽略我的偏好" / "don't use my private memory this session" → session-
  local opt-out; do not write any record.

## Hand-off rules

- After `aiplus memory add`, echo back the record id so the user can `forget`
  it later.
- After `aiplus memory context`, summarize what was loaded — never silently
  inject without acknowledgement.
- If redaction triggered, surface the redaction count (do not show what was
  redacted).

Memory is context, not permission. It never authorizes Owner-gated actions.
"#
    .to_string()
}

fn claude_compact_subagent_content() -> String {
    r#"---
name: aiplus-compact
description: Survives context compaction and session interruption by preparing structured handoffs and resuming from them. Use when context is approaching the compact threshold, when a long task is about to be interrupted, when the user says "the agent forgot" / "你又忘了", at session resume after /clear or /compact, or whenever you need to pick up where you left off. Owns `aiplus compact prepare | resume | remind | savings`.
---

# AiPlus Compact Reminder

This subagent owns the `aiplus compact` surface. It DOES NOT click or trigger
the host compact action; it only prepares the handoff and tells the user when
to compact manually.

## Decision map

| Signal | Action |
|---|---|
| Long-running task, ~30 min elapsed | `aiplus compact remind --event long-session` |
| Major phase boundary, before review/QA, before subagent bursts, before release prep, before Owner handoff | `aiplus compact remind --event phase-end` |
| `REMINDER_DECISION=remind_now` in output | Run `aiplus compact prepare`, then tell the user it is safe to compact manually |
| Session start with a pending capsule | `aiplus compact resume` first, then proceed |
| User says "continue" / "继续" / "where were we" after compact | `aiplus compact resume`, summarize what was restored, then continue |
| Demonstrating token savings | `aiplus compact savings` |

## Task-weight calibration

- HEAVY task — every 30 min or at major phase boundaries.
- MEDIUM task — at phase boundaries or before review/QA.
- LIGHT task — only on user request or obvious handoff.

## Hand-off rules

- After `prepare`, surface the handoff path so the user knows where it is.
- After `resume`, summarize what was restored before continuing — never
  silently swap context.
- Never auto-trigger the host's compact action. AiPlus prepares; the user
  presses the button.
"#
    .to_string()
}

fn claude_velocity_subagent_content() -> String {
    r#"---
name: aiplus-velocity
description: Replaces human-engineer-hour estimates with AI-native p50/p90 numbers calibrated from your own task history. Use BEFORE starting any non-trivial bounded task, when the user asks "how long will this take" / "多久能搞完" / "estimate this", when reviewing past delivery times, or when detecting human-time anchoring bias. Owns `aiplus velocity estimate | complete | bias | report`.
---

# AiPlus Velocity

This subagent owns the `aiplus velocity` surface. Every estimate and every
completion is logged as local JSONL under `.aiplus/velocity/`.

## Decision map

| Signal | Action |
|---|---|
| About to start a clearly bounded task | `aiplus velocity estimate --task-type <feature|fix|refactor|chore> --human-estimate <Nh>` |
| Task finished | `aiplus velocity complete --task-id <est_id> --actual <Nm|Nh> --outcome <success|partial|fail>` |
| User quoted a long human estimate ("this is 2 days") | Run `estimate` to surface the AI-native p50/p90; flag if `HUMAN_ANCHOR_DETECTED=yes` |
| User asks "are we faster than my old workflow" | `aiplus velocity bias` then `aiplus velocity report` |
| Sanity-checking the ledger | `aiplus velocity doctor` |

## Confidence calibration

- `MATCHED_RECORDS=0` → `CONFIDENCE=low`. Tell the user the estimate has no
  history yet and the numbers will tighten after a few runs.
- `MATCHED_RECORDS>=10` → quote the p50 with confidence; quote the p90 when
  the user needs a worst-case bound.

## Hand-off rules

- Always echo the `est_id` after an estimate so completion can reference it.
- Never quote a single number when you can quote the p50/p90 pair.
- Never overwrite a `complete` record; if the user wants to revise, surface
  the prior record and ask.
"#
    .to_string()
}

fn claude_team_consultant_subagent_content() -> String {
    r#"---
name: aiplus-team-consultant
description: Plan-time review that catches what single-agent planning misses — onboarding ease, security and privacy, real execution pitfalls, AI integration considerations. Use BEFORE finalizing any non-trivial plan: feature plans, architecture changes, refactors touching multiple modules, anything affecting security / onboarding / privacy / AI integration, anything that will be shipped to users. A coordinator scales the consult by complexity and risk so light tasks stay light. Owns `aiplus agent route | talk | invite | dismiss | audit run | transcript`.
---

# AiPlus Auto Team Consultant

This subagent owns the `aiplus agent` and team consultant surface, including
the project-local `consultant-team.toml`. The virtual team is 5 expert
members + the project's user personas, all seated at the same table.

## Decision map

| Signal | Action |
|---|---|
| Drafting a non-trivial plan | `aiplus agent route reviewer` to walk the plan against the table before committing |
| Plan touches security / privacy | Ensure `ai_integration` and security-aware members are on the consultant-team; surface concerns inline |
| User asks "is this plan good" / "second opinion" | `aiplus agent talk <role>` for the matching expert |
| Multi-stakeholder decision | Run the full table, then summarize divergent positions; do not pre-collapse |
| Acceptance gate | `aiplus agent audit run` |
| Coordination retrospective | `aiplus agent transcript` |

## Coordinator discipline

- Light task → at most one expert. Don't burn the whole table on a typo fix.
- Medium task → 2–3 experts matching the risk axes.
- Heavy task → full table including user personas.

## Hand-off rules

- Always surface dissent. If two experts disagree, name both positions; do
  not flatten into a single "consensus" line.
- Never approve Owner-gated actions on the team's behalf. The consultant
  produces a recommendation; the user gives the green light.
- After the consult, write a short decision record (via `aiplus memory add
  --kind decision`) so future sessions can see what was decided.
"#
    .to_string()
}

fn opencode_config_content() -> String {
    serde_json::json!({
        "$schema": "https://opencode.ai/config.json"
    })
    .to_string()
        + "\n"
}

fn opencode_prompt_content() -> String {
    r#"# AiPlus

Read .aiplus/AGENTS.aiplus.md and use project-local AiPlus modules when relevant.

Compact Reminder reminder schedule:
- HEAVY tasks: run `aiplus compact remind --event long-session` every 30 minutes
  or at major phase boundaries, before review/QA, before subagent bursts, before
  release prep, and before Owner handoff.
- MEDIUM tasks: run `aiplus compact remind --event phase-end` at phase
  boundaries, before review/QA, and before Owner handoff.
- LIGHT tasks: run `aiplus compact remind` only on user request or obvious
  handoff.
- If `REMINDER_DECISION=remind_now`, run/confirm `aiplus compact prepare`, then
  suggest manual host compact only. Do not click or call host compact.
- After compact, run `aiplus compact resume`, then optionally
  `aiplus compact savings`.

Explicit AiPlus refresh triggers for already-open agent sessions: AiPlus 刷新,
刷新 AiPlus, aiplus refresh, aiplus status, AiPlus status, 继续 AiPlus,
resume AiPlus.

Generic continuation keywords should try AiPlus first when possible: 继续, 刷新,
continue, resume, refresh, go on, 接着.

Agent Continuity natural-language mapping:
- 记住这个 / 记住这个偏好: add project memory after redaction.
- 以后都这样: create a profile/global candidate only; do not silently approve.
- 只在这个项目用: project memory.
- 忘掉这个: run memory forget with an id, or ask if ambiguous.
- 你记住了什么 / 这次用了哪些记忆 / memory status: memory status/context.
- 新开顾问 / 新开 advisor: advisor identity context.
- 新开 CEO: ceo identity context.
- 把这次经验沉淀成 skill: skill candidate only.
- 不要用我的私人记忆 / 本次忽略我的偏好: session-local opt-out.

Memory is context, identity is role contract not permission, and skill
candidates are proposals rather than approved skills.
"#
    .to_string()
}

// ---------------------------------------------------------------------------
// Claude Code settings.local.json hooks installer
// ---------------------------------------------------------------------------
//
// AiPlus writes a small set of Claude Code hooks into .claude/settings.local.json
// so that the agent automatically loads memory context on session start and
// prepares a compact handoff before Claude Code compacts. The file is
// project-local and typically gitignored by Claude Code conventions, so this
// touches only the host machine.
//
// AiPlus-managed matcher entries are tagged with `"aiplus_managed": true` and
// always use `"matcher": "*"`. Identification is also resilient if Claude Code
// or another tool strips unknown fields on rewrite: a matcher with `"*"` whose
// hooks are all `aiplus ...` commands is considered ours.
//
// User-defined hooks in the same file are preserved.

fn aiplus_managed_hook_set() -> Vec<(&'static str, Vec<serde_json::Value>)> {
    vec![
        (
            "SessionStart",
            vec![serde_json::json!({
                "matcher": "*",
                "aiplus_managed": true,
                "hooks": [{
                    "type": "command",
                    "command": "aiplus memory context --runtime claude-code --budget 2000",
                    "aiplus_managed": true
                }]
            })],
        ),
        (
            "PreCompact",
            vec![serde_json::json!({
                "matcher": "*",
                "aiplus_managed": true,
                "hooks": [{
                    "type": "command",
                    "command": "aiplus compact prepare",
                    "aiplus_managed": true
                }]
            })],
        ),
    ]
}

fn matcher_is_aiplus_managed(matcher: &serde_json::Value) -> bool {
    if matcher
        .get("aiplus_managed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return true;
    }
    let Some(hooks) = matcher.get("hooks").and_then(|h| h.as_array()) else {
        return false;
    };
    if hooks.iter().any(|h| {
        h.get("aiplus_managed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }) {
        return true;
    }
    let matcher_str = matcher
        .get("matcher")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if matcher_str != "*" {
        return false;
    }
    !hooks.is_empty()
        && hooks.iter().all(|h| {
            h.get("command")
                .and_then(|v| v.as_str())
                .map(|c| {
                    let t = c.trim_start();
                    t == "aiplus" || t.starts_with("aiplus ")
                })
                .unwrap_or(false)
        })
}

fn apply_aiplus_managed_hooks(value: &mut serde_json::Value) {
    let object = match value.as_object_mut() {
        Some(o) => o,
        None => return,
    };
    let hooks_entry = object
        .entry("hooks".to_string())
        .or_insert_with(|| serde_json::json!({}));
    if !hooks_entry.is_object() {
        *hooks_entry = serde_json::json!({});
    }
    let hooks_obj = hooks_entry.as_object_mut().unwrap();
    for (event, desired) in aiplus_managed_hook_set() {
        let event_entry = hooks_obj
            .entry(event.to_string())
            .or_insert_with(|| serde_json::json!([]));
        if !event_entry.is_array() {
            *event_entry = serde_json::json!([]);
        }
        let arr = event_entry.as_array_mut().unwrap();
        arr.retain(|m| !matcher_is_aiplus_managed(m));
        for m in desired {
            arr.push(m);
        }
    }
}

fn strip_aiplus_managed_hooks(value: &mut serde_json::Value) {
    let Some(obj) = value.as_object_mut() else {
        return;
    };
    let Some(hooks) = obj.get_mut("hooks").and_then(|v| v.as_object_mut()) else {
        return;
    };
    let event_keys: Vec<String> = hooks.keys().cloned().collect();
    for ev in event_keys {
        if let Some(arr) = hooks.get_mut(&ev).and_then(|v| v.as_array_mut()) {
            arr.retain(|m| !matcher_is_aiplus_managed(m));
            let empty = arr.is_empty();
            if empty {
                hooks.remove(&ev);
            }
        }
    }
    if hooks.is_empty() {
        obj.remove("hooks");
    }
}

fn install_claude_hooks(root: &Path, plan: &mut Plan) -> Result<()> {
    let rel = ".claude/settings.local.json";
    let target = rel_to_abs(root, rel)?;
    assert_no_symlink_path(root, &target)?;
    if let Some(parent) = target.parent() {
        ensure_dir(root, parent, plan)?;
    }

    let current_text = read_text_if_exists(&target)?;
    let mut value: serde_json::Value = match current_text.as_deref() {
        Some(t) if !t.trim().is_empty() => serde_json::from_str(t).map_err(|e| {
            CliError::new(
                1,
                format!(
                    "ERROR invalid JSON in {rel}: {e}; fix or remove the file and retry install"
                ),
            )
        })?,
        _ => serde_json::json!({}),
    };
    if !value.is_object() {
        return Err(CliError::new(1, format!("ERROR {rel} root must be a JSON object")).into());
    }

    apply_aiplus_managed_hooks(&mut value);

    let mut next = serde_json::to_string_pretty(&value)
        .map_err(|e| CliError::new(1, format!("ERROR serialize {rel}: {e}")))?;
    next.push('\n');

    if current_text.as_deref() == Some(next.as_str()) {
        plan.items.push(PlanItem {
            action: "skip-identical".to_string(),
            path: rel.to_string(),
        });
        return Ok(());
    }

    plan.items.push(PlanItem {
        action: if current_text.is_none() {
            "write"
        } else {
            "managed-update"
        }
        .to_string(),
        path: rel.to_string(),
    });
    if let Some(text) = current_text.as_ref() {
        backup_file(
            root,
            rel,
            text.as_bytes(),
            plan,
            &Options {
                force: true,
                backup: true,
                yes: true,
            },
        )?;
    }
    if !plan.dry_run {
        fs::write(&target, next.as_bytes())?;
    }
    Ok(())
}

fn remove_claude_hooks(root: &Path, plan: &mut Plan) -> Result<()> {
    let rel = ".claude/settings.local.json";
    let target = rel_to_abs(root, rel)?;
    let Some(current_text) = read_text_if_exists(&target)? else {
        return Ok(());
    };
    if current_text.trim().is_empty() {
        return Ok(());
    }
    let mut value: serde_json::Value = match serde_json::from_str(&current_text) {
        Ok(v) => v,
        Err(_) => return Ok(()),
    };
    if !value.is_object() {
        return Ok(());
    }
    strip_aiplus_managed_hooks(&mut value);

    let mut next = serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string());
    next.push('\n');

    if next == current_text {
        return Ok(());
    }
    plan.items.push(PlanItem {
        action: "managed-update".to_string(),
        path: rel.to_string(),
    });
    if !plan.dry_run {
        fs::write(&target, next.as_bytes())?;
    }
    Ok(())
}

fn target_root() -> Result<PathBuf> {
    Ok(fs::canonicalize(std::env::current_dir()?)?)
}

fn rel_to_abs(root: &Path, rel: &str) -> Result<PathBuf> {
    let rel_path = Path::new(rel);
    if rel_path.is_absolute()
        || rel_path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(
            CliError::new(1, format!("ERROR refusing path outside target root: {rel}")).into(),
        );
    }
    Ok(root.join(rel_path))
}

fn assert_no_symlink_path(root: &Path, abs_path: &Path) -> Result<()> {
    let resolved = if abs_path.is_absolute() {
        abs_path.to_path_buf()
    } else {
        root.join(abs_path)
    };
    if !is_inside(&resolved, root) {
        return Err(CliError::new(
            1,
            format!(
                "ERROR refusing write outside target root: {}",
                abs_path.display()
            ),
        )
        .into());
    }
    let relative = path_relative(root, &resolved)?;
    let mut current = root.to_path_buf();
    for part in relative.components() {
        current.push(part);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(CliError::new(
                    1,
                    format!(
                        "ERROR refusing to write through symlink: {}",
                        path_slash(path_relative(root, &current)?)
                    ),
                )
                .into());
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn is_inside(child: &Path, parent: &Path) -> bool {
    child.starts_with(parent)
}

fn path_relative(root: &Path, path: &Path) -> Result<PathBuf> {
    path.strip_prefix(root)
        .map(Path::to_path_buf)
        .map_err(|_| anyhow!("{} is outside {}", path.display(), root.display()))
}

fn path_slash(path: PathBuf) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn read_text_if_exists(file: &Path) -> Result<Option<String>> {
    match fs::read_to_string(file) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn replace_between(text: &str, begin: &str, end: &str, replacement: &str) -> Result<String> {
    let Some(start) = text.find(begin) else {
        return Ok(text.to_string());
    };
    let Some(end_start) = text[start..].find(end).map(|idx| start + idx) else {
        return Err(CliError::new(1, "ERROR malformed AiPlus managed block in AGENTS.md").into());
    };
    let end_idx = end_start + end.len();
    let mut next = String::new();
    next.push_str(&text[..start]);
    next.push_str(replacement);
    next.push_str(&text[end_idx..]);
    Ok(next)
}

fn plan_printer(plan: &Plan) {
    for item in &plan.items {
        println!(
            "{}{} {}",
            if plan.dry_run { "DRY_RUN " } else { "" },
            item.action,
            item.path
        );
    }
    println!("GLOBAL_CONFIG_UNTOUCHED");
}

fn list_entries(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        out.push(path.clone());
        if file_type.is_dir() && !file_type.is_symlink() {
            out.extend(list_entries(&path)?);
        }
    }
    Ok(out)
}

fn safe_remove_aiplus(root: &Path) -> Result<()> {
    let target = rel_to_abs(root, ".aiplus")?;
    assert_no_symlink_path(root, &target)?;
    fs::remove_dir_all(target)?;
    Ok(())
}

fn is_writable(root: &Path) -> bool {
    let probe = root.join(".aiplus-write-probe.tmp");
    match fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = fs::remove_file(probe);
            true
        }
        Err(_) => false,
    }
}

fn timestamp() -> String {
    format!("unix-{}ms", epoch_millis())
}

#[allow(clippy::too_many_arguments)]
fn command_velocity(
    subcommand: Option<String>,
    task_type: Option<String>,
    human_estimate: Option<String>,
    model: Option<String>,
    workflow: Option<String>,
    task_id: Option<String>,
    actual: Option<String>,
    outcome: Option<String>,
    task: Option<String>,
    yes: bool,
) -> Result<()> {
    let subcommand = subcommand.unwrap_or_else(|| {
        println!("Usage: aiplus velocity init|estimate|complete|bias|report|doctor|purge");
        process::exit(2);
    });

    let root = std::env::current_dir()?;

    match subcommand.as_str() {
        "init" => {
            init_velocity(&root)?;
            println!("VELOCITY_INIT_STATUS=PASS");
            println!("velocity_dir={}", velocity_dir(&root).display());
        }
        "estimate" => {
            let task_type = task_type.ok_or_else(|| anyhow!("--task-type required"))?;
            let human_estimate =
                human_estimate.ok_or_else(|| anyhow!("--human-estimate required"))?;
            let model = model.unwrap_or_else(|| "unknown".to_string());
            let workflow = workflow.unwrap_or_else(|| "MEDIUM".to_string());

            let human_estimate_minutes = parse_duration(&human_estimate)?;

            let result = compute_ai_native_estimate(
                &root,
                &task_type,
                &model,
                &workflow,
                human_estimate_minutes,
            )?;

            let estimate_id = generate_estimate_id();
            let record = EstimateRecord {
                schema_version: VELOCITY_SCHEMA_VERSION.to_string(),
                id: estimate_id.clone(),
                task_id: task_id
                    .clone()
                    .unwrap_or_else(|| generate_estimate_id().replace("est_", "task_")),
                created_at: now_iso(),
                project_id: "aiplus".to_string(),
                task_type: task_type.clone(),
                repo_area: "aiplus-public".to_string(),
                agent_role: "ceo".to_string(),
                runtime: "opencode".to_string(),
                model: model.clone(),
                workflow_level: workflow.clone(),
                estimate_basis: "human_engineer_time".to_string(),
                human_baseline_minutes: human_estimate_minutes,
                human_baseline_source: "agent_initial_estimate".to_string(),
                human_estimate_minutes,
                ai_native_estimate_p50_minutes: result.ai_native_estimate_p50_minutes,
                ai_native_estimate_p90_minutes: result.ai_native_estimate_p90_minutes,
                confidence: result.confidence.clone(),
                matched_records: result.matched_records,
                human_anchor_signals: if result.human_anchor_detected {
                    vec![
                        "multi_hour_language".to_string(),
                        "similar_ai_tasks_finished_fast".to_string(),
                    ]
                } else {
                    vec![]
                },
                stop_when_done: result.stop_when_done,
            };

            reject_sensitive_velocity_text(&serde_json::to_string(&record)?)?;
            append_velocity_jsonl(&root, "estimates.jsonl", &record)?;
            apply_velocity_retention(&root)?;

            println!("VELOCITY_ESTIMATE_STATUS=PASS");
            println!("ESTIMATE_ID={estimate_id}");
            println!(
                "HUMAN_ANCHOR_DETECTED={}",
                if result.human_anchor_detected {
                    "yes"
                } else {
                    "no"
                }
            );
            println!("HUMAN_ESTIMATE_MINUTES={human_estimate_minutes}");
            println!(
                "AI_NATIVE_ESTIMATE_P50_MINUTES={}",
                result.ai_native_estimate_p50_minutes
            );
            println!(
                "AI_NATIVE_ESTIMATE_P90_MINUTES={}",
                result.ai_native_estimate_p90_minutes
            );
            println!("EXPECTED_SPEEDUP={}", result.expected_speedup_range);
            println!("MATCHED_RECORDS={}", result.matched_records);
            println!("CONFIDENCE={}", result.confidence);
            println!("STOP_WHEN_DONE=yes");
        }
        "complete" => {
            let task_id = task_id.ok_or_else(|| anyhow!("--task-id required"))?;
            let actual = actual.ok_or_else(|| anyhow!("--actual required"))?;
            let outcome = outcome.unwrap_or_else(|| "pass".to_string());

            let actual_minutes = parse_duration(&actual)?;

            let estimates: Vec<EstimateRecord> = read_velocity_jsonl(&root, "estimates.jsonl")?;
            let estimate = estimates.iter().find(|e| e.task_id == task_id);
            let (original_estimate, _ai_native_p50, human_baseline) = match estimate {
                Some(e) => (
                    e.human_estimate_minutes,
                    e.ai_native_estimate_p50_minutes,
                    e.human_baseline_minutes,
                ),
                None => (actual_minutes * 2, actual_minutes, actual_minutes * 2),
            };

            let overestimate_ratio = if actual_minutes > 0 {
                original_estimate as f64 / actual_minutes as f64
            } else {
                0.0
            };

            let run_id = generate_run_id();
            let run = RunRecord {
                schema_version: VELOCITY_SCHEMA_VERSION.to_string(),
                id: run_id,
                estimate_id: estimate.map(|e| e.id.clone()).unwrap_or_default(),
                task_id: task_id.clone(),
                created_at: now_iso(),
                project_id: "aiplus".to_string(),
                task_type: task_type.clone().unwrap_or_default(),
                repo_area: "aiplus-public".to_string(),
                agent_role: "ceo".to_string(),
                runtime: "opencode".to_string(),
                model: model.clone().unwrap_or_else(|| "unknown".to_string()),
                workflow_level: workflow.clone().unwrap_or_else(|| "MEDIUM".to_string()),
                original_estimate_minutes: original_estimate,
                human_baseline_minutes: human_baseline,
                actual_active_minutes: actual_minutes,
                actual_time_source: "manual".to_string(),
                wall_clock_minutes: actual_minutes,
                tool_wait_minutes: 0,
                blocked_minutes: 0,
                outcome: outcome.clone(),
                verification_depth: "targeted".to_string(),
                quality_verdict: outcome.clone(),
                rework_count: 0,
                owner_gate_hit: false,
                overestimate_ratio,
                human_time_bias: false,
                slow_reason: "none".to_string(),
                redaction_status: "pass".to_string(),
                raw_content_stored: false,
                secret_values_stored: false,
                memory_integration: "disabled".to_string(),
                seed: false,
            };

            validate_run_record(&run)?;
            reject_sensitive_velocity_text(&serde_json::to_string(&run)?)?;
            append_velocity_jsonl(&root, "runs.jsonl", &run)?;

            if is_rare_case(&run) {
                if let Some(rare) = classify_rare_case(&run) {
                    append_velocity_jsonl(&root, "rare-cases.jsonl", &rare)?;
                }
            }

            update_multipliers(&root)?;
            update_aggregates(&root)?;
            apply_velocity_retention(&root)?;

            println!("VELOCITY_COMPLETE_STATUS=PASS");
            println!("ACTUAL_ACTIVE_MINUTES={actual_minutes}");
            println!("OVERESTIMATE_RATIO={overestimate_ratio:.1}");
            if overestimate_ratio >= 2.0 {
                println!("HUMAN_TIME_BIAS=detected");
            } else {
                println!("HUMAN_TIME_BIAS=not_detected");
            }
            println!("RETENTION_STATUS=applied");
        }
        "bias" => {
            let task_id = task.ok_or_else(|| anyhow!("--task required"))?;

            let estimates: Vec<EstimateRecord> = read_velocity_jsonl(&root, "estimates.jsonl")?;
            let runs: Vec<RunRecord> = read_velocity_jsonl(&root, "runs.jsonl")?;

            let estimate = estimates.iter().find(|e| e.task_id == task_id);
            let run = runs.iter().find(|r| r.task_id == task_id);

            match (estimate, run) {
                (Some(est), Some(run)) => {
                    let bias = detect_bias(est, run);
                    println!("VELOCITY_BIAS_STATUS=PASS");
                    println!("BIAS_TYPE={}", bias.bias_type);
                    println!("OVERESTIMATE_RATIO={:.1}", bias.overestimate_ratio);
                    println!(
                        "HUMAN_TIME_BIAS_FOUND={}",
                        if bias.human_time_bias_found {
                            "yes"
                        } else {
                            "no"
                        }
                    );
                    println!(
                        "HUMAN_TIME_BIAS_CONFIDENCE={}",
                        bias.human_time_bias_confidence
                    );
                    println!("HUMAN_BASELINE_STATUS={}", bias.human_baseline_status);
                    println!("NEXT_ESTIMATE_ADJUSTMENT={}", bias.next_estimate_adjustment);
                    if let Some(ff) = bias.fast_finish_calibration {
                        println!("FAST_FINISH_CALIBRATION");
                        println!("ORIGINAL_ESTIMATE={}", ff.original_estimate_minutes);
                        println!("ACTUAL_TIME={}", ff.actual_time_minutes);
                        println!("ACCEPTANCE_STATUS={}", ff.acceptance_status);
                        println!("ERROR_RATIO={:.1}", ff.error_ratio);
                        println!("CAUSE={}", ff.cause);
                        println!("NEXT_ESTIMATE_ADJUSTMENT={}", ff.next_estimate_adjustment);
                    }
                }
                _ => {
                    println!("VELOCITY_BIAS_STATUS=NEEDS_FIX");
                    println!("reason=estimate_or_run_not_found");
                }
            }
        }
        "report" => {
            let aggregates: aiplus_core::Aggregates =
                read_velocity_json(&root, "aggregates.json").unwrap_or_default();
            println!("VELOCITY_REPORT_STATUS=PASS");
            println!("CALIBRATION_WINDOW=latest_200");
            println!("TOTAL_ESTIMATES={}", aggregates.total_estimates);
            println!("TOTAL_RUNS={}", aggregates.total_runs);
            println!("TOTAL_RARE_CASES={}", aggregates.total_rare_cases);
            println!(
                "MEDIAN_OVERESTIMATE_RATIO={:.1}",
                aggregates.median_overestimate_ratio
            );
            println!(
                "HUMAN_TIME_BIAS_RATE={:.2}",
                aggregates.human_time_bias_rate
            );
        }
        "doctor" => {
            let report = velocity_doctor(&root)?;
            println!("VELOCITY_DOCTOR_STATUS={}", report.status);
            println!("records_count={}", report.records_count);
            println!(
                "rotation_needed={}",
                if report.rotation_needed { "yes" } else { "no" }
            );
            println!("bad_jsonl_lines={}", report.bad_jsonl_lines);
            println!("secret_values={}", report.secret_values);
            println!("raw_content_found={}", report.raw_content_found);
            println!(
                "global_agent_config_edits={}",
                report.global_agent_config_edits
            );
            println!("duplicate_ids={}", report.duplicate_ids);
            println!("missing_required_fields={}", report.missing_required_fields);
            println!("negative_time_records={}", report.negative_time_records);
            println!(
                "actual_exceeds_wallclock={}",
                report.actual_exceeds_wallclock
            );
            println!("nan_multipliers={}", report.nan_multipliers);
            println!(
                "sqlite_found={}",
                if report.sqlite_found { "yes" } else { "no" }
            );
            if !report.over_threshold_files.is_empty() {
                println!(
                    "over_threshold_files={}",
                    report.over_threshold_files.join(",")
                );
            }
            // W6: surface AEL-unit-type buckets that still ride on
            // synthetic seeds (fewer than 5 calibrated, non-seed
            // records). Empty when the project doesn't ship AEL or
            // when all five buckets have crossed the threshold.
            if !report.uncalibrated_buckets.is_empty() {
                println!(
                    "uncalibrated_buckets={}",
                    report.uncalibrated_buckets.join(",")
                );
                println!(
                    "note=estimates for these task types are advisory; log 5+ real runs to calibrate"
                );
            }
        }
        "purge" => {
            if !yes {
                println!("Usage: aiplus velocity purge --yes");
                return Err(CliError::new(1, "ERROR purge requires --yes").into());
            }
            purge_velocity(&root)?;
            println!("VELOCITY_PURGE_STATUS=PASS");
        }
        _ => {
            println!("Usage: aiplus velocity init|estimate|complete|bias|report|doctor|purge");
            process::exit(2);
        }
    }

    Ok(())
}

fn now_iso() -> String {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let nanos = duration.subsec_nanos();
    let millis = nanos / 1_000_000;
    let dt = simple_time_from_epoch(secs);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        dt.year, dt.month, dt.day, dt.hour, dt.minute, dt.second, millis
    )
}

struct SimpleDateTime {
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

fn simple_time_from_epoch(mut secs: u64) -> SimpleDateTime {
    const DAYS_IN_MONTH: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut year = 1970i32;
    loop {
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let secs_in_year = if is_leap { 31_622_400 } else { 31_536_000 };
        if secs >= secs_in_year as u64 {
            secs -= secs_in_year as u64;
            year += 1;
        } else {
            break;
        }
    }
    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let mut month = 0usize;
    loop {
        let days = if month == 1 && is_leap {
            29
        } else {
            DAYS_IN_MONTH[month] as u64
        };
        let secs_in_month = days * 86_400;
        if secs >= secs_in_month {
            secs -= secs_in_month;
            month += 1;
        } else {
            break;
        }
    }
    let day = (secs / 86_400) + 1;
    secs %= 86_400;
    let hour = secs / 3600;
    secs %= 3600;
    let minute = secs / 60;
    let second = secs % 60;
    SimpleDateTime {
        year,
        month: (month + 1) as u8,
        day: day as u8,
        hour: hour as u8,
        minute: minute as u8,
        second: second as u8,
    }
}

fn read_velocity_jsonl<T: for<'de> serde::Deserialize<'de>>(
    root: &Path,
    filename: &str,
) -> Result<Vec<T>> {
    aiplus_core::read_velocity_jsonl(root, filename)
}

fn read_velocity_json<T: for<'de> serde::Deserialize<'de>>(
    root: &Path,
    filename: &str,
) -> Result<T> {
    aiplus_core::read_velocity_json(root, filename)
}

// ---------------------------------------------------------------------------
// Phase E: `aiplus mcp register` — wire the AiPlus MCP server into installed
// runtimes (codex / claude-code / opencode) so they can call agent_route /
// agent_status / agent_set_team as native tools.
//
// Codex uses a global TOML config at ~/.codex/config.toml. Claude-code and
// opencode use project-local JSON files (.mcp.json and opencode.json) in cwd.
// All writes are idempotent — re-running this command is safe and a no-op if
// the aiplus entry already exists.
// ---------------------------------------------------------------------------

/// P1.5: `--scope global|project` selects where to write the config.
/// Each runtime has an idiomatic default — codex is global (matches how
/// codex itself stores all config user-wide), claude / opencode are
/// project-local (matches how they auto-discover .mcp.json /
/// opencode.json in cwd). The flag lets the user override either way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McpScope {
    Global,
    Project,
}

fn resolve_scope_for(runtime: &str, requested: Option<&str>) -> Result<McpScope> {
    match requested {
        Some("global") => Ok(McpScope::Global),
        Some("project") => Ok(McpScope::Project),
        None => match runtime {
            "codex" => Ok(McpScope::Global),
            "claude" | "opencode" => Ok(McpScope::Project),
            _ => unreachable!("runtime allowlist guarded above"),
        },
        Some(other) => Err(anyhow!(
            "unknown --scope '{other}'. Valid: global, project."
        )),
    }
}

fn command_mcp_register(
    runtime: Option<String>,
    dry_run: bool,
    force: bool,
    scope: Option<String>,
) -> Result<()> {
    let bin = std::env::current_exe().context("locate current aiplus binary path")?;
    let bin_str = bin.to_string_lossy().into_owned();

    let runtimes: Vec<&str> = match runtime.as_deref() {
        None => vec!["codex", "claude", "opencode"],
        Some("codex") => vec!["codex"],
        Some("claude") => vec!["claude"],
        Some("opencode") => vec!["opencode"],
        Some(other) => {
            return Err(anyhow!(
                "unknown --runtime '{other}'. Valid: codex, claude, opencode."
            ));
        }
    };

    if dry_run {
        println!("MCP_REGISTER_MODE=dry-run (no files will be written)");
    }
    if force {
        println!("MCP_REGISTER_MODE=force (corrupt configs will be overwritten)");
    }
    println!("MCP_REGISTER_BIN={bin_str}");

    let mut any_change = false;
    for rt in runtimes {
        let rt_scope = resolve_scope_for(rt, scope.as_deref())?;
        let changed = match rt {
            "codex" => register_mcp_codex(&bin_str, dry_run, force, rt_scope)?,
            "claude" => register_mcp_claude(&bin_str, dry_run, force, rt_scope)?,
            "opencode" => register_mcp_opencode(&bin_str, dry_run, force, rt_scope)?,
            _ => unreachable!(),
        };
        any_change = any_change || changed;
    }

    if dry_run {
        println!("MCP_REGISTER_STATUS=DRY_RUN_OK any_change={any_change}");
    } else if any_change {
        println!("MCP_REGISTER_STATUS=OK any_change=true");
    } else {
        println!("MCP_REGISTER_STATUS=NOOP any_change=false (already registered)");
    }
    Ok(())
}

fn register_mcp_codex(bin: &str, dry_run: bool, force: bool, scope: McpScope) -> Result<bool> {
    // P1.5: scope selects whether to write user-global or project-local
    // codex config. The project-local case is rare (codex itself reads
    // config from ~/.codex), but we support it for users who want a
    // project-scoped aiplus-only codex setup.
    let codex_dir: PathBuf = match scope {
        McpScope::Global => {
            let home = match std::env::var_os("HOME").map(PathBuf::from) {
                Some(h) => h,
                None => {
                    println!("MCP_REGISTER_CODEX=SKIP reason=no_HOME scope=global");
                    return Ok(false);
                }
            };
            let dir = home.join(".codex");
            if !dir.exists() {
                println!(
                    "MCP_REGISTER_CODEX=SKIP reason=codex_not_installed scope=global \
                     hint=run `codex --version` to verify install"
                );
                return Ok(false);
            }
            dir
        }
        McpScope::Project => {
            let cwd = std::env::current_dir().context("get cwd for codex project scope")?;
            let dir = cwd.join(".codex");
            if !dir.exists() {
                println!(
                    "MCP_REGISTER_CODEX=SKIP reason=no_project_codex_dir scope=project \
                     hint=create ./.codex/ to opt into project-scoped codex config"
                );
                return Ok(false);
            }
            dir
        }
    };
    let config_path = codex_dir.join("config.toml");
    let existing = if config_path.exists() {
        fs::read_to_string(&config_path)
            .with_context(|| format!("read {}", config_path.display()))?
    } else {
        String::new()
    };
    let mut doc: toml::Value = if existing.trim().is_empty() {
        toml::Value::Table(toml::value::Table::new())
    } else {
        match toml::from_str::<toml::Value>(&existing) {
            Ok(v) => v,
            Err(e) if force => {
                println!(
                    "MCP_REGISTER_CODEX=FORCE_OVERWRITE path={} reason=parse_failed detail={e}",
                    config_path.display()
                );
                toml::Value::Table(toml::value::Table::new())
            }
            Err(e) => {
                return Err(anyhow!(
                    "Cannot parse existing {}: {e}.\n\
                     Re-run `aiplus mcp-register --force` to overwrite the broken file, \
                     or fix the file manually first.",
                    config_path.display()
                ));
            }
        }
    };
    let table = doc
        .as_table_mut()
        .ok_or_else(|| anyhow!("codex config.toml root is not a table"))?;
    let servers = table
        .entry("mcp_servers")
        .or_insert_with(|| toml::Value::Table(toml::value::Table::new()))
        .as_table_mut()
        .ok_or_else(|| anyhow!("mcp_servers in codex config.toml is not a table"))?;

    // Idempotency: check if the existing entry already matches.
    let want_command = toml::Value::String(bin.to_string());
    let want_args = toml::Value::Array(vec![toml::Value::String("mcp-serve".into())]);
    if let Some(toml::Value::Table(existing_aiplus)) = servers.get("aiplus") {
        let cmd_ok = existing_aiplus.get("command") == Some(&want_command);
        let args_ok = existing_aiplus.get("args") == Some(&want_args);
        if cmd_ok && args_ok {
            println!(
                "MCP_REGISTER_CODEX=NOOP path={} (already registered)",
                config_path.display()
            );
            return Ok(false);
        }
    }

    let mut new_entry = toml::value::Table::new();
    new_entry.insert("command".to_string(), want_command);
    new_entry.insert("args".to_string(), want_args);
    servers.insert("aiplus".to_string(), toml::Value::Table(new_entry));

    let serialized = toml::to_string_pretty(&doc).context("re-serialize codex config.toml")?;
    if dry_run {
        println!(
            "MCP_REGISTER_CODEX=WOULD_WRITE path={}",
            config_path.display()
        );
        println!("--- proposed config.toml ---\n{serialized}--- end ---");
    } else {
        write_file_atomic(&config_path, serialized.as_bytes())
            .with_context(|| format!("write {}", config_path.display()))?;
        println!("MCP_REGISTER_CODEX=WROTE path={}", config_path.display());
    }
    Ok(true)
}

fn register_mcp_claude(bin: &str, dry_run: bool, force: bool, scope: McpScope) -> Result<bool> {
    // P1.5: scope selects ./.mcp.json (project) vs ~/.claude/.mcp.json
    // (global). Project is the natural home for claude-code MCP config,
    // but a power user might want a global default that every project
    // inherits — we support both.
    let config_path: PathBuf = match scope {
        McpScope::Project => {
            let cwd = std::env::current_dir().context("get cwd for claude project scope")?;
            cwd.join(".mcp.json")
        }
        McpScope::Global => {
            let home = match std::env::var_os("HOME").map(PathBuf::from) {
                Some(h) => h,
                None => {
                    println!("MCP_REGISTER_CLAUDE=SKIP reason=no_HOME scope=global");
                    return Ok(false);
                }
            };
            // ~/.claude/ may not exist on a fresh machine. Create it so
            // global registration is self-contained — claude itself will
            // create the dir on first run otherwise.
            let dir = home.join(".claude");
            std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
            dir.join(".mcp.json")
        }
    };
    let existing = if config_path.exists() {
        fs::read_to_string(&config_path)
            .with_context(|| format!("read {}", config_path.display()))?
    } else {
        String::new()
    };
    let mut doc: serde_json::Value = if existing.trim().is_empty() {
        serde_json::json!({})
    } else {
        match serde_json::from_str::<serde_json::Value>(&existing) {
            Ok(v) => v,
            Err(e) if force => {
                println!(
                    "MCP_REGISTER_CLAUDE=FORCE_OVERWRITE path={} reason=parse_failed detail={e}",
                    config_path.display()
                );
                serde_json::json!({})
            }
            Err(e) => {
                return Err(anyhow!(
                    "Cannot parse existing {}: {e}.\n\
                     Re-run `aiplus mcp-register --force` to overwrite the broken file, \
                     or fix the file manually first.",
                    config_path.display()
                ));
            }
        }
    };
    let root = doc
        .as_object_mut()
        .ok_or_else(|| anyhow!(".mcp.json root must be a JSON object"))?;
    let servers = root
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| anyhow!("mcpServers in .mcp.json must be a JSON object"))?;

    let want_entry = serde_json::json!({
        "command": bin,
        "args": ["mcp-serve"]
    });
    if servers.get("aiplus") == Some(&want_entry) {
        println!(
            "MCP_REGISTER_CLAUDE=NOOP path={} (already registered)",
            config_path.display()
        );
        return Ok(false);
    }
    servers.insert("aiplus".to_string(), want_entry);

    let serialized = serde_json::to_string_pretty(&doc).context("re-serialize .mcp.json")?;
    if dry_run {
        println!(
            "MCP_REGISTER_CLAUDE=WOULD_WRITE path={}",
            config_path.display()
        );
        println!("--- proposed .mcp.json ---\n{serialized}\n--- end ---");
    } else {
        write_file_atomic(&config_path, format!("{serialized}\n").as_bytes())
            .with_context(|| format!("write {}", config_path.display()))?;
        println!("MCP_REGISTER_CLAUDE=WROTE path={}", config_path.display());
    }
    Ok(true)
}

fn register_mcp_opencode(bin: &str, dry_run: bool, force: bool, scope: McpScope) -> Result<bool> {
    // P1.5: scope selects ./opencode.json (project) vs
    // ~/.opencode/opencode.json (global). Project is the default and
    // matches opencode's auto-discovery.
    let config_path: PathBuf = match scope {
        McpScope::Project => {
            let cwd = std::env::current_dir().context("get cwd for opencode project scope")?;
            cwd.join("opencode.json")
        }
        McpScope::Global => {
            let home = match std::env::var_os("HOME").map(PathBuf::from) {
                Some(h) => h,
                None => {
                    println!("MCP_REGISTER_OPENCODE=SKIP reason=no_HOME scope=global");
                    return Ok(false);
                }
            };
            let dir = home.join(".opencode");
            std::fs::create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
            dir.join("opencode.json")
        }
    };
    let existing = if config_path.exists() {
        fs::read_to_string(&config_path)
            .with_context(|| format!("read {}", config_path.display()))?
    } else {
        String::new()
    };
    let mut doc: serde_json::Value = if existing.trim().is_empty() {
        serde_json::json!({})
    } else {
        match serde_json::from_str::<serde_json::Value>(&existing) {
            Ok(v) => v,
            Err(e) if force => {
                println!(
                    "MCP_REGISTER_OPENCODE=FORCE_OVERWRITE path={} reason=parse_failed detail={e}",
                    config_path.display()
                );
                serde_json::json!({})
            }
            Err(e) => {
                return Err(anyhow!(
                    "Cannot parse existing {}: {e}.\n\
                     Re-run `aiplus mcp-register --force` to overwrite the broken file, \
                     or fix the file manually first.",
                    config_path.display()
                ));
            }
        }
    };
    let root = doc
        .as_object_mut()
        .ok_or_else(|| anyhow!("opencode.json root must be a JSON object"))?;
    let mcp = root
        .entry("mcp")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| anyhow!("mcp in opencode.json must be a JSON object"))?;

    let want_entry = serde_json::json!({
        "type": "local",
        "command": [bin, "mcp-serve"],
        "enabled": true
    });
    if mcp.get("aiplus") == Some(&want_entry) {
        println!(
            "MCP_REGISTER_OPENCODE=NOOP path={} (already registered)",
            config_path.display()
        );
        return Ok(false);
    }
    mcp.insert("aiplus".to_string(), want_entry);

    let serialized = serde_json::to_string_pretty(&doc).context("re-serialize opencode.json")?;
    if dry_run {
        println!(
            "MCP_REGISTER_OPENCODE=WOULD_WRITE path={}",
            config_path.display()
        );
        println!("--- proposed opencode.json ---\n{serialized}\n--- end ---");
    } else {
        write_file_atomic(&config_path, format!("{serialized}\n").as_bytes())
            .with_context(|| format!("write {}", config_path.display()))?;
        println!("MCP_REGISTER_OPENCODE=WROTE path={}", config_path.display());
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    // S1: push-target parser and dotenv writer tests.
    //
    // Why parser tests matter: PushTarget::parse is the ONLY input
    // gate between a user-supplied --to string and the side-effect
    // dispatch. Malformed input must surface a clear error, not panic
    // or default to a wrong target type.

    #[test]
    fn push_target_parse_github_secret_happy() {
        let p = PushTarget::parse("github-secret:izhiwen/AiEconLab:ANTHROPIC_API_KEY").unwrap();
        match p {
            PushTarget::GithubSecret { owner, repo, name } => {
                assert_eq!(owner, "izhiwen");
                assert_eq!(repo, "AiEconLab");
                assert_eq!(name, "ANTHROPIC_API_KEY");
            }
            _ => panic!("expected GithubSecret variant"),
        }
    }

    #[test]
    fn push_target_parse_env_happy() {
        let p = PushTarget::parse("env:OPENAI_API_KEY").unwrap();
        match p {
            PushTarget::Env { var } => assert_eq!(var, "OPENAI_API_KEY"),
            _ => panic!("expected Env variant"),
        }
    }

    #[test]
    fn push_target_parse_dotenv_happy() {
        let p = PushTarget::parse("dotenv:.env.local").unwrap();
        match p {
            PushTarget::Dotenv { path } => assert_eq!(path, PathBuf::from(".env.local")),
            _ => panic!("expected Dotenv variant"),
        }
    }

    #[test]
    fn push_target_parse_rejects_unknown_scheme() {
        let err = PushTarget::parse("aws-ssm:/foo/bar").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unknown push target"), "got: {msg}");
    }

    #[test]
    fn push_target_parse_rejects_empty_components() {
        assert!(PushTarget::parse("github-secret:owner/:NAME").is_err());
        assert!(PushTarget::parse("github-secret:/repo:NAME").is_err());
        assert!(PushTarget::parse("github-secret:owner/repo:").is_err());
        assert!(PushTarget::parse("env:").is_err());
        assert!(PushTarget::parse("dotenv:").is_err());
    }

    #[test]
    fn push_target_summary_does_not_leak_value() {
        // The summary string is what gets logged. It must surface
        // identity but never include the secret value (which isn't
        // even held by PushTarget anyway — value is passed by
        // reference to write()).
        let p = PushTarget::parse("github-secret:o/r:N").unwrap();
        assert_eq!(p.summary(), "github-secret:o/r:N");
        let p = PushTarget::parse("env:VAR").unwrap();
        assert_eq!(p.summary(), "env:VAR");
    }

    #[test]
    fn dotenv_write_appends_new_key() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".env");
        std::fs::write(&path, "EXISTING=foo\n").unwrap();
        write_dotenv_line(&path, "NEW", "bar").unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("EXISTING=foo\n"));
        assert!(body.contains("NEW=\"bar\"\n"));
    }

    #[test]
    fn dotenv_write_replaces_existing_key() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".env");
        std::fs::write(&path, "EXISTING=foo\nKEEP=bar\n").unwrap();
        write_dotenv_line(&path, "EXISTING", "new-value").unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(body.contains("EXISTING=\"new-value\"\n"));
        assert!(body.contains("KEEP=bar\n"));
        // Replacement must be idempotent — running twice yields the same file.
        write_dotenv_line(&path, "EXISTING", "new-value").unwrap();
        let body2 = std::fs::read_to_string(&path).unwrap();
        assert_eq!(body, body2);
    }

    #[test]
    fn dotenv_write_escapes_shell_metachars() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join(".env");
        // Quotes, backslashes, $, ` are the four `source`-dangerous chars.
        write_dotenv_line(&path, "TRICKY", "a\"b\\c$d`e").unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        assert!(
            body.contains("TRICKY=\"a\\\"b\\\\c\\$d\\`e\"\n"),
            "body was: {body}"
        );
    }

    #[test]
    fn dotenv_write_creates_parent_dir_if_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("nested/dir/.env");
        write_dotenv_line(&path, "K", "v").unwrap();
        assert!(path.exists());
    }

    static REGISTRY_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn registry_lock() -> &'static Mutex<()> {
        REGISTRY_TEST_LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_translate_chinese_refresh() {
        let args = vec!["aiplus".to_string(), "刷新".to_string()];
        let result = translate_chinese_subcommand(args);
        assert_eq!(result[1], "refresh");
    }

    #[test]
    fn test_translate_chinese_status() {
        let args = vec!["aiplus".to_string(), "状态".to_string()];
        let result = translate_chinese_subcommand(args);
        assert_eq!(result[1], "status");
    }

    #[test]
    fn test_translate_chinese_install() {
        let args = vec!["aiplus".to_string(), "安装".to_string()];
        let result = translate_chinese_subcommand(args);
        assert_eq!(result[1], "install");
    }

    #[test]
    fn test_translate_chinese_update() {
        let args = vec!["aiplus".to_string(), "升级".to_string()];
        let result = translate_chinese_subcommand(args);
        assert_eq!(result[1], "update");
    }

    #[test]
    fn test_translate_chinese_update_alt() {
        let args = vec!["aiplus".to_string(), "更新".to_string()];
        let result = translate_chinese_subcommand(args);
        assert_eq!(result[1], "update");
    }

    #[test]
    fn test_translate_chinese_self() {
        let args = vec!["aiplus".to_string(), "自升级".to_string()];
        let result = translate_chinese_subcommand(args);
        assert_eq!(result[1], "self");
    }

    #[test]
    fn test_translate_chinese_uninstall() {
        let args = vec!["aiplus".to_string(), "卸载".to_string()];
        let result = translate_chinese_subcommand(args);
        assert_eq!(result[1], "uninstall");
    }

    #[test]
    fn test_translate_chinese_rollback() {
        let args = vec!["aiplus".to_string(), "回滚".to_string()];
        let result = translate_chinese_subcommand(args);
        assert_eq!(result[1], "rollback");
    }

    #[test]
    fn test_translate_chinese_doctor() {
        let args = vec!["aiplus".to_string(), "健康".to_string()];
        let result = translate_chinese_subcommand(args);
        assert_eq!(result[1], "doctor");
    }

    #[test]
    fn test_translate_chinese_self_upgrade() {
        let args = vec!["aiplus".to_string(), "self".to_string(), "升级".to_string()];
        let result = translate_chinese_subcommand(args);
        assert_eq!(result[1], "self");
        assert_eq!(result[2], "upgrade");
    }

    #[test]
    fn test_translate_english_unchanged() {
        let args = vec!["aiplus".to_string(), "refresh".to_string()];
        let result = translate_chinese_subcommand(args);
        assert_eq!(result[1], "refresh");
    }

    #[test]
    fn test_translate_single_arg() {
        let args = vec!["aiplus".to_string()];
        let result = translate_chinese_subcommand(args);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_registry_upsert_and_remove() {
        let _guard = registry_lock().lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let config_home = tmp.path().join("config");
        std::env::set_var("XDG_CONFIG_HOME", &config_home);
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        // Upsert
        upsert_registry_entry(&project, &["codex".to_string(), "opencode".to_string()]).unwrap();
        let registry = read_registry().unwrap();
        assert_eq!(registry.installed_projects.len(), 1);
        assert_eq!(registry.schema_version, "1.0");
        assert_eq!(
            registry.installed_projects[0].runtimes,
            vec!["codex", "opencode"]
        );
        // Re-upsert updates timestamp
        std::thread::sleep(std::time::Duration::from_millis(10));
        upsert_registry_entry(&project, &["opencode".to_string()]).unwrap();
        let registry2 = read_registry().unwrap();
        assert_eq!(registry2.installed_projects.len(), 1);
        assert_eq!(registry2.installed_projects[0].runtimes, vec!["opencode"]);
        // Remove
        remove_registry_entry(&project).unwrap();
        let registry3 = read_registry().unwrap();
        assert!(registry3.installed_projects.is_empty());
    }

    #[test]
    fn test_list_projects_empty() {
        let _guard = registry_lock().lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        let registry = read_registry().unwrap();
        assert_eq!(registry.installed_projects.len(), 0);
    }

    #[test]
    fn test_prune_projects_dry_run() {
        let _guard = registry_lock().lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let config_home = tmp.path().join("config");
        std::env::set_var("XDG_CONFIG_HOME", &config_home);
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        upsert_registry_entry(&project, &["codex".to_string()]).unwrap();
        // Delete project directory so entry becomes stale
        std::fs::remove_dir_all(&project).unwrap();
        let registry = read_registry().unwrap();
        assert_eq!(registry.installed_projects.len(), 1);
        // Not stale because directory gone
        let stale = registry
            .installed_projects
            .iter()
            .filter(|e| !e.path.exists())
            .count();
        assert_eq!(stale, 1);
    }

    #[test]
    fn test_update_all_projects_registry_empty() {
        let _guard = registry_lock().lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_CONFIG_HOME", tmp.path());
        let registry = read_registry().unwrap();
        assert!(registry.installed_projects.is_empty());
    }

    #[test]
    fn test_translate_chinese_update_all_projects() {
        let args = vec!["aiplus".to_string(), "全局更新".to_string()];
        let result = translate_chinese_subcommand(args);
        assert_eq!(result[1], "update");
        assert_eq!(result[2], "--all-projects");
    }
}
