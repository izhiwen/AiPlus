use anyhow::{anyhow, Context, Result};
use clap::{ArgAction, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{SystemTime, UNIX_EPOCH};

include!(concat!(env!("OUT_DIR"), "/asset_files.rs"));

const VERSION: &str = "0.4.6";
const RELEASE_TAG: &str = "v0.4.6";
const INSTALLER: &str = "aiplus";
const REFRESH_PROMPT: &str = "刷新";
const REFRESH_PROMPT_REL: &str = ".aiplus/REFRESH_PROMPT.txt";
const SAVINGS_LEDGER_REL: &str = "savings-ledger.jsonl";
const PRICING_CATALOG_URL: &str =
    "https://raw.githubusercontent.com/izhiwen/aiplus/main/assets/pricing/public-model-pricing.json";
const MANAGED_BEGIN: &str = "<!-- BEGIN AIPLUS MANAGED BLOCK -->";
const MANAGED_END: &str = "<!-- END AIPLUS MANAGED BLOCK -->";
const MANAGED_REF: &str = "@./.aiplus/AGENTS.aiplus.md";
const SECRET_BROKER_SERVICE: &str = "aiplus/bws-access-token";
const SECRET_BROKER_ACCOUNT: &str = "aiplus-secret-broker";
const DEFAULT_BWS_PROJECT_ID: &str = "ddd15408-b7bd-4230-8df3-b44401403ce3";

#[derive(Parser)]
#[command(
    name = "aiplus",
    version = VERSION,
    disable_version_flag = true,
    after_help = "Safety:\n  Project-local project writes are limited to .aiplus/, .codex/compact/, and\n  the AiPlus managed block in AGENTS.md. User-level profile writes are limited to\n  ~/.config/aiplus and never include secret values. `aiplus pricing update`,\n  `aiplus self update`, and `aiplus secret-broker` may fetch public release/pricing\n  data or read approved Bitwarden secrets at runtime. No npm publish, global install,\n  telemetry, user-data upload, secret persistence, or global config edits are implemented."
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
    },
    Update {
        module: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        dry_run: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        verbose: bool,
    },
    Add {
        module: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        dry_run: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        verbose: bool,
    },
    Doctor,
    Status,
    Refresh {
        trigger: Vec<String>,
    },
    Uninstall {
        #[arg(long, action = ArgAction::SetTrue)]
        dry_run: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        yes: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        force: bool,
    },
    Compact {
        subcommand: Option<String>,
        #[arg(long, default_value = "standard", value_parser = ["light", "standard", "full"])]
        level: String,
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
        #[arg(long, action = ArgAction::SetTrue)]
        force: bool,
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
    #[command(name = "secret-broker")]
    SecretBroker {
        subcommand: Option<String>,
        arg: Option<String>,
        #[arg(long, action = ArgAction::SetTrue)]
        print: bool,
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
    },
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

#[derive(Clone, Copy)]
struct ModuleSpec {
    name: &'static str,
    vendor_name: &'static str,
    version: &'static str,
    path: &'static str,
    required_files: &'static [&'static str],
}

const MODULES: &[ModuleSpec] = &[
    ModuleSpec {
        name: "auto-compact",
        vendor_name: "aiplus-auto-compact",
        version: "0.4.6",
        path: ".aiplus/modules/aiplus-auto-compact",
        required_files: &[
            "LICENSE",
            "core/templates/current-handoff.md",
            "core/templates/compact-policy.json",
        ],
    },
    ModuleSpec {
        name: "auto-team-consultant",
        vendor_name: "aiplus-auto-team-consultant",
        version: "0.4.6",
        path: ".aiplus/modules/aiplus-auto-team-consultant",
        required_files: &[
            "LICENSE",
            "adapters/codex/skills/auto-team-consultant/SKILL.md",
            "core/templates/TEMPLATE_INDEX.md",
        ],
    },
];

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
}

struct PlanItem {
    action: String,
    path: String,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema_version: Option<String>,
    installer: Option<String>,
    installer_version: Option<String>,
    installed_at: Option<String>,
    updated_at: Option<String>,
    target_root: Option<String>,
    runtime_adapters: Option<Vec<String>>,
    modules: Option<BTreeMap<String, ManifestModule>>,
    managed_files: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
struct ManifestModule {
    version: Option<String>,
    source: Option<String>,
    path: Option<String>,
    installed_at: Option<String>,
    updated_at: Option<String>,
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
    "FAIL",
    "IN_PROGRESS",
    "NEEDS_VERIFICATION",
    "BLOCKED_OWNER_GATE",
    "BLOCKED_MISSING_FILES",
    "BLOCKED_EXTERNAL_ACCESS",
    "BLOCKED_UNCLEAR_GOAL",
];

fn main() {
    let cli = Cli::parse();
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
        eprintln!("INTERNAL_ERROR {error:?}");
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
        } => command_install(
            runtime,
            runtime_opt,
            all_runtimes,
            dry_run,
            verbose,
            Options { force, backup, yes },
        ),
        Commands::Update {
            module,
            dry_run,
            verbose,
        } => command_update(module, dry_run, verbose),
        Commands::Add {
            module,
            dry_run,
            verbose,
        } => command_add(module, dry_run, verbose),
        Commands::Doctor => command_doctor(),
        Commands::Status => command_status(),
        Commands::Refresh { trigger } => command_refresh(trigger),
        Commands::Uninstall {
            dry_run,
            yes,
            force,
        } => command_uninstall(dry_run, yes, force),
        Commands::Compact {
            subcommand,
            level,
            json,
            force,
        } => command_compact(subcommand, &level, json, force),
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
        Commands::SecretBroker {
            subcommand,
            arg,
            print,
            command,
        } => command_secret_broker(subcommand, arg, print, command),
        Commands::SelfCommand {
            subcommand,
            dry_run,
            yes,
        } => command_self(subcommand, dry_run, yes),
    }
}

fn print_usage() {
    println!(
        "AiPlus CLI {VERSION}\n\nUsage:\n  aiplus <command> [options]\n\nCommands:\n  install codex|claude-code|opencode|all [--dry-run] [--verbose] [--force --backup --yes]\n  update [all|auto-compact|auto-team-consultant] [--dry-run] [--verbose]\n  add auto-compact|auto-team-consultant [--dry-run] [--verbose]\n  doctor\n  status\n  refresh\n  uninstall --dry-run\n  uninstall --yes [--force]\n  compact init|validate|prepare|score|checkpoint|resume|savings [--json] [--level light|standard|full]\n  pricing update|status\n  profile status|install|update|link|disable|uninstall|migrate|cleanup|doctor\n  secret-broker status|doctor|list|resolve|run|token\n  self update [--dry-run] [--yes]\n\nSafety:\n  Project-local project writes are limited to .aiplus/, .codex/compact/, and\n  the AiPlus managed block in AGENTS.md. User-level profile writes are limited to\n  ~/.config/aiplus and never include secret values. `aiplus pricing update`,\n  `aiplus self update`, and `aiplus secret-broker` may fetch public release/pricing\n  data or read approved Bitwarden secrets at runtime. No npm publish, global install,\n  telemetry, user-data upload, secret persistence, or global config edits are implemented."
    );
}

fn command_install(
    runtime: Option<String>,
    runtime_opt: Option<String>,
    all_runtimes: bool,
    dry_run: bool,
    verbose: bool,
    options: Options,
) -> Result<()> {
    if options.force && !options.yes {
        return Err(CliError::new(1, "ERROR --force requires --yes").into());
    }
    if options.force && !options.backup {
        return Err(CliError::new(1, "ERROR --force requires --backup --yes").into());
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
    install_base(&root, &mut plan, &effective_options, default_module_names())?;
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
    command_self_update(dry_run, true)?;
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
        if requested == "auto-compact" {
            compact_init(&root, &mut plan, false)?;
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

fn command_status() -> Result<()> {
    let root = target_root()?;
    let manifest = read_manifest(&root, true).unwrap_or_default();
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
        if rel_to_abs(&root, ".codex/compact")?.exists() {
            "present"
        } else {
            "missing"
        }
    );
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
    println!("STATUS=PASS");
    Ok(())
}

fn command_refresh(trigger: Vec<String>) -> Result<()> {
    let root = target_root()?;
    let manifest = read_manifest(&root, true).unwrap_or_default();
    let modules = normalize_existing_modules(manifest.modules.as_ref());
    let compact_state = if rel_to_abs(&root, ".codex/compact")?.exists() {
        "present"
    } else {
        "missing"
    };

    if prefers_chinese_refresh(&trigger) {
        let auto_compact = module_refresh_status_zh(&modules, "auto-compact");
        let auto_team = module_refresh_status_zh(&modules, "auto-team-consultant");
        println!("已刷新 AiPlus。");
        println!();
        println!("当前项目 AiPlus 状态：");
        println!("- Auto Compact: {auto_compact}");
        println!("- Auto Team Consultant: {auto_team}");
        println!("- Compact state: {compact_state}");
        println!();
        println!("我会这样使用：");
        println!("- 长任务或 compact 前准备 checkpoint");
        println!(
            "- 如果你说“帮我准备 compact”“保存进度”或“做个交接”，我会运行 aiplus compact prepare。"
        );
        println!(
            "- 如果你问“看一下 compact 收益”或“compact 帮我省了多少？”，我会运行 aiplus compact savings。"
        );
        println!("- compact 后如果我没自动继续，你发一句“继续”就行。我会从刚才的位置接着做。");
        println!("- CEO Prompt / review / brainstorm 时使用 Auto Team Consultant");
        println!();
        println!("边界：");
        println!("- AiPlus 不能替你点击 compact");
        println!("- 不上传数据");
        println!("- 不改全局 agent config");
    } else {
        let auto_compact = module_refresh_status_en(&modules, "auto-compact");
        let auto_team = module_refresh_status_en(&modules, "auto-team-consultant");
        println!("AiPlus refreshed.");
        println!();
        println!("Current project AiPlus status:");
        println!("- Auto Compact: {auto_compact}");
        println!("- Auto Team Consultant: {auto_team}");
        println!("- Compact state: {compact_state}");
        println!();
        println!("How I will use it:");
        println!("- Prepare checkpoints before long tasks or compact-worthy moments.");
        println!("- If you say \"prepare compact\", \"save progress\", or \"checkpoint this\", I will run aiplus compact prepare.");
        println!("- If you ask \"show compact savings\" or \"how many tokens did compact save?\", I will run aiplus compact savings.");
        println!("- After compact, if I do not reply, send: continue");
        println!("- Use Auto Team Consultant for CEO Prompt, review, and brainstorm work.");
        println!();
        println!("Boundaries:");
        println!("- AiPlus cannot click compact for you.");
        println!("- AiPlus does not upload data.");
        println!("- AiPlus does not change global agent config.");
    }
    println!("AIPLUS_REFRESH_STATUS=PASS");
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
    if modules.contains_key("auto-compact") {
        push_check(
            &mut checks,
            ".codex/compact/ exists".to_string(),
            rel_to_abs(&root, ".codex/compact")?.exists(),
            Some("run compact init".to_string()),
        );
    }
    push_check(
        &mut checks,
        "no global configs were touched by installer".to_string(),
        true,
        None,
    );
    let pass = checks.iter().all(|item| item.ok);
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
    println!("refreshPrompt={REFRESH_PROMPT}");
    println!("globalConfig=untouched");
    println!("target={}", root.display());
    println!(
        "next={}",
        if pass {
            "send AiPlus 刷新, 刷新 AiPlus, aiplus refresh, or aiplus status to the current agent session".to_string()
        } else {
            "run install, then rerun doctor".to_string()
        }
    );
    println!();
    for item in &checks {
        if item.ok {
            println!("PASS {}", item.label);
        } else if let Some(fix) = item.fix.as_ref() {
            println!("NEEDS_FIX {} ({fix})", item.label);
        } else {
            println!("NEEDS_FIX {}", item.label);
        }
    }
    println!("DOCTOR_STATUS={}", if pass { "PASS" } else { "NEEDS_FIX" });
    Ok(())
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

fn command_compact(subcommand: Option<String>, level: &str, json: bool, force: bool) -> Result<()> {
    let Some(subcommand) = subcommand else {
        print_usage();
        process::exit(2);
    };
    if ![
        "init",
        "validate",
        "checkpoint",
        "resume",
        "prepare",
        "score",
        "savings",
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
        _ => {
            println!(
                "Usage: aiplus profile status|install|update|link|disable|uninstall|migrate|cleanup|doctor"
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
    }
    println!("secret_values=none");
    println!("global_agent_config_edits=none");
    println!("PROFILE_STATUS=PASS");
    Ok(())
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
    if source.join("secret-aliases.tsv").exists() {
        let broker_dir = config_home()?
            .join("aiplus")
            .join("secret-broker")
            .join("profiles")
            .join(&profile);
        fs::create_dir_all(&broker_dir)?;
        install_profile_file(&source, &broker_dir, "secret-aliases.tsv")?;
    }
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
    let installed = if profile == "all" {
        let profiles_root = config_home()?.join("aiplus").join("profiles");
        !installed_profile_names(&profiles_root)?.is_empty()
    } else {
        validate_profile_name(&profile)?;
        let dir = profile_dir(&profile)?;
        dir.join("profile.toml").exists() && dir.join("AGENTS.profile.md").exists()
    };
    println!("PROFILE_DOCTOR");
    println!("profile={profile}");
    println!("installed={}", yes_no(installed));
    println!("private_content_in_public_assets=not_checked_by_command");
    println!("secret_values=none");
    println!("global_agent_config_edits=none");
    println!(
        "PROFILE_DOCTOR_STATUS={}",
        if installed { "PASS" } else { "NEEDS_INSTALL" }
    );
    Ok(())
}

fn command_secret_broker(
    subcommand: Option<String>,
    arg: Option<String>,
    print_secret: bool,
    command: Vec<String>,
) -> Result<()> {
    match subcommand.as_deref() {
        Some("status") => secret_broker_status(),
        Some("doctor") => secret_broker_doctor(),
        Some("list") => secret_broker_list(),
        Some("resolve") => secret_broker_resolve(arg, print_secret),
        Some("run") => secret_broker_run(command),
        Some("token") => secret_broker_token(arg),
        _ => {
            println!("Usage: aiplus secret-broker status|doctor|list|resolve <alias>|run -- <command...>|token set|delete");
            process::exit(2);
        }
    }
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
    println!("token_source={source}");
    if source == "not_configured" {
        println!("next=run aiplus secret-broker token set in Terminal");
    }
    println!("keychain_supported={}", yes_no(cfg!(target_os = "macos")));
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
    println!("provider_status=PASS");
    println!("secret_value_printed={}", yes_no(print_secret));
    if print_secret {
        println!("{}", value.expose_for_explicit_print());
    }
    println!("SECRET_RESOLVE_STATUS=PASS");
    Ok(())
}

fn secret_broker_run(command: Vec<String>) -> Result<()> {
    if command.is_empty() {
        return Err(CliError::new(2, "ERROR secret-broker run requires a command after --").into());
    }
    let provider = load_secret_provider()?;
    let mut child = Command::new(&command[0]);
    if command.len() > 1 {
        child.args(&command[1..]);
    }
    for alias in secret_aliases()? {
        let value = provider.resolve(&alias)?;
        child.env(alias.env_var, value.value);
    }
    let status = child.status().context("run child command")?;
    if !status.success() {
        return Err(CliError::new(status.code().unwrap_or(1), "ERROR child command failed").into());
    }
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
                "SECRET_RESOLVE_STATUS=FAIL provider=bws reason=missing_value",
            )
        })?;
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

impl BwsProvider {
    fn lookup_secret_id(&self, alias: &SecretAlias) -> Result<String> {
        let project_id = bitwarden_project_id();
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

fn load_secret_provider() -> Result<Box<dyn SecretsProvider>> {
    if provider_name() == "mock" {
        return Ok(Box::new(MockProvider));
    }
    let token = get_bws_token()?;
    Ok(Box::new(BwsProvider { token }))
}

fn provider_name() -> String {
    std::env::var("AIPLUS_SECRET_PROVIDER").unwrap_or_else(|_| "bws".to_string())
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

fn read_keychain_token() -> Result<Option<String>> {
    if !cfg!(target_os = "macos") {
        return Ok(None);
    }
    let output = Command::new("security")
        .args([
            "find-generic-password",
            "-a",
            SECRET_BROKER_ACCOUNT,
            "-s",
            SECRET_BROKER_SERVICE,
            "-w",
        ])
        .output()
        .context("read macOS keychain")?;
    if !output.status.success() {
        return Ok(None);
    }
    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if token.is_empty() {
        Ok(None)
    } else {
        Ok(Some(token))
    }
}

fn write_keychain_token(token: &str) -> Result<()> {
    if !cfg!(target_os = "macos") {
        return Err(CliError::new(
            1,
            "ERROR keychain token storage is only implemented on macOS",
        )
        .into());
    }
    let status = Command::new("security")
        .args([
            "add-generic-password",
            "-U",
            "-a",
            SECRET_BROKER_ACCOUNT,
            "-s",
            SECRET_BROKER_SERVICE,
            "-w",
            token,
        ])
        .status()
        .context("write macOS keychain")?;
    if !status.success() {
        return Err(CliError::new(1, "TOKEN_SET_STATUS=FAIL reason=keychain_write_failed").into());
    }
    Ok(())
}

fn delete_keychain_token() -> Result<()> {
    if !cfg!(target_os = "macos") {
        return Err(CliError::new(
            1,
            "ERROR keychain token storage is only implemented on macOS",
        )
        .into());
    }
    let _ = Command::new("security")
        .args([
            "delete-generic-password",
            "-a",
            SECRET_BROKER_ACCOUNT,
            "-s",
            SECRET_BROKER_SERVICE,
        ])
        .status();
    Ok(())
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
    let home = std::env::var("HOME").context("HOME is required")?;
    Ok(PathBuf::from(home).join(".config"))
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn write_file_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension(format!("tmp-{}", epoch_millis()));
    fs::write(&temp, bytes)?;
    fs::rename(temp, path)?;
    Ok(())
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

fn command_self(subcommand: Option<String>, dry_run: bool, yes: bool) -> Result<()> {
    match subcommand.as_deref() {
        Some("update") => command_self_update(dry_run, yes),
        _ => {
            println!("Usage: aiplus self update [--dry-run] [--yes]");
            process::exit(2);
        }
    }
}

fn command_self_update(dry_run: bool, yes: bool) -> Result<()> {
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
    let archive = temp.join(&asset);
    fetch_to(&format!("{base_url}/checksums.txt"), &checksums)?;
    fetch_to(&format!("{base_url}/{asset}"), &archive)?;
    verify_checksum_file(&checksums, &archive)?;
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
    if module_names.iter().any(|name| name == "auto-compact") {
        compact_init(root, plan, false)?;
    }
    Ok(())
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
            write_file_safe(
                root,
                ".claude/commands/aiplus-refresh.md",
                claude_refresh_command_content().as_bytes(),
                plan,
                options,
            )?;
            write_file_safe(
                root,
                ".claude/agents/aiplus-advisor.md",
                claude_advisor_agent_content().as_bytes(),
                plan,
                options,
            )
        }
        "opencode" => {
            write_file_safe(
                root,
                ".opencode/opencode.json",
                opencode_config_content().as_bytes(),
                plan,
                options,
            )?;
            write_file_safe(
                root,
                ".opencode/commands/aiplus-refresh.md",
                opencode_prompt_content().as_bytes(),
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
            )
        }
        _ => Ok(()),
    }
}

fn compact_init(root: &Path, plan: &mut Plan, force: bool) -> Result<()> {
    if plan.dry_run {
        plan.items.push(PlanItem {
            action: "compact-init".to_string(),
            path: ".codex/compact/".to_string(),
        });
        return Ok(());
    }
    ensure_dir(root, &rel_to_abs(root, ".codex/compact")?, plan)?;
    ensure_dir(root, &rel_to_abs(root, ".codex/compact/checkpoints")?, plan)?;
    for file in [
        "current-handoff.md",
        "decision-log.md",
        "agent-state-ledger.md",
        "evidence-ledger.md",
        "compact-policy.json",
    ] {
        let asset_path = format!("aiplus-auto-compact/core/templates/{file}");
        let content =
            embedded_asset_text(&asset_path)?.replace("<ISO8601_TIMESTAMP>", &timestamp());
        write_compact_template(
            root,
            &format!(".codex/compact/{file}"),
            content.as_bytes(),
            plan,
            force,
        )?;
    }
    plan.items.push(PlanItem {
        action: "compact-init".to_string(),
        path: ".codex/compact/".to_string(),
    });
    Ok(())
}

fn migrate_compact_handoff_if_needed(root: &Path, plan: &mut Plan) -> Result<bool> {
    let rel = ".codex/compact/current-handoff.md";
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
    let manifest = Manifest {
        schema_version: Some(VERSION.to_string()),
        installer: Some(INSTALLER.to_string()),
        installer_version: Some(VERSION.to_string()),
        installed_at: Some(existing.installed_at.unwrap_or_else(|| now.clone())),
        updated_at: Some(now),
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
        println!("- .codex/compact/");
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
        println!(".codex/compact/ state was preserved.");
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
    [
        ".aiplus/AGENTS.aiplus.md",
        REFRESH_PROMPT_REL,
        ".aiplus/modules/aiplus-auto-compact",
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
    let stamp = timestamp().replace([':', '.'], "-");
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
    }
    Ok(())
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
    for (rel, bytes) in ASSET_FILES {
        let Some(stripped) = rel.strip_prefix(&prefix) else {
            continue;
        };
        let dest = format!("{}/{}", spec.path, stripped);
        write_file_safe(root, &dest, bytes, plan, options)?;
    }
    Ok(())
}

fn embedded_asset_text(rel: &str) -> Result<String> {
    let bytes = ASSET_FILES
        .iter()
        .find_map(|(path, bytes)| (*path == rel).then_some(*bytes))
        .ok_or_else(|| anyhow!("missing embedded asset: {rel}"))?;
    String::from_utf8(bytes.to_vec()).with_context(|| format!("decode embedded asset {rel}"))
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
    rel_to_abs(root, ".codex/compact")
}

fn compact_file(root: &Path, rel: &str) -> Result<PathBuf> {
    rel_to_abs(root, &format!(".codex/compact/{rel}"))
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
        errors.push(".codex/compact/ is missing".to_string());
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
    let rel = format!(".codex/compact/checkpoints/{filename}");
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
    let handoff = read_compact_text(root, "current-handoff.md")?;
    let latest = latest_checkpoint(root)?;
    println!("RESUME_READY");
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
            next_action: "Update .codex/compact/current-handoff.md with the next safe action."
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
        "BLOCKED_BY_OWNER_GATE" => 1,
        _ => 2,
    }
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
        "0.4.5", "0.4.6",
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

fn is_supported_manifest_schema(version: &str) -> bool {
    matches!(
        version,
        "0.1.3"
            | "0.2.0"
            | "0.2.1"
            | "0.3.0"
            | "0.3.1"
            | "0.4.0"
            | "0.4.1"
            | "0.4.2"
            | "0.4.3"
            | "0.4.4"
            | "0.4.5"
            | "0.4.6"
    )
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

fn sensitive_findings(text: &str) -> Vec<(&'static str, bool)> {
    let lower = text.to_ascii_lowercase();
    vec![
        (
            "authorization header",
            lower.contains("authorization: bearer") || lower.contains("authorization: basic"),
        ),
        (
            "private key",
            text.contains("-----BEGIN ") && text.contains("PRIVATE KEY-----"),
        ),
        ("jwt", text.split_whitespace().any(is_jwt_like)),
        ("cookie", lower.contains("cookie:") && text.contains('=')),
        (
            "private path",
            text.contains("/Users/")
                || text.contains("/home/")
                || lower.contains("dropbox/")
                || text.contains("iCloud"),
        ),
        ("email pii", text.contains('@') && text.contains('.')),
        ("phone pii", has_phone_like(text)),
        (
            "raw audio/transcript payload",
            lower.contains("begin transcript")
                || lower.contains("webvtt")
                || lower.contains("provider request body")
                || lower.contains("provider response body"),
        ),
        (
            "har/webrtc dump",
            lower.contains(".har") || lower.contains(".webrtcdump"),
        ),
        ("api key", has_secret_assignment(&lower)),
    ]
}

fn is_jwt_like(token: &str) -> bool {
    let parts: Vec<&str> = token
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '.' && ch != '_' && ch != '-')
        .split('.')
        .collect();
    parts.len() == 3 && parts[0].starts_with("eyJ")
}

fn has_phone_like(text: &str) -> bool {
    let digits: String = text.chars().filter(|ch| ch.is_ascii_digit()).collect();
    digits.len() >= 10 && digits.len() <= 15
}

fn has_secret_assignment(lower: &str) -> bool {
    [
        "api_key",
        "apikey",
        "api-key",
        "secret_key",
        "secret-key",
        "access_token",
        "access-token",
    ]
    .iter()
    .any(|needle| lower.contains(needle) && (lower.contains('=') || lower.contains(':')))
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

fn single_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
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

fn normalize_module(value: Option<&str>) -> Option<&'static str> {
    match value? {
        "auto-compact" | "aiplus-auto-compact" | "compact" => Some("auto-compact"),
        "auto-team-consultant" | "aiplus-auto-team-consultant" | "team" => {
            Some("auto-team-consultant")
        }
        _ => None,
    }
}

fn module_spec(name: &str) -> Option<ModuleSpec> {
    MODULES.iter().copied().find(|spec| spec.name == name)
}

fn default_module_names() -> Vec<String> {
    vec![
        "auto-compact".to_string(),
        "auto-team-consultant".to_string(),
    ]
}

fn available_modules_text() -> String {
    MODULES
        .iter()
        .map(|spec| spec.name)
        .collect::<Vec<_>>()
        .join(", ")
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

fn runtime_doctor_requirements(root: &Path, runtime: &str) -> Result<Vec<(String, bool)>> {
    Ok(match runtime {
        "codex" => vec![
            (
                "AGENTS.md contains managed block".to_string(),
                has_managed_block(root)?,
            ),
            (
                "managed block points to .aiplus/AGENTS.aiplus.md".to_string(),
                has_managed_block(root)?,
            ),
        ],
        "claude-code" => vec![
            (
                ".claude/commands/aiplus-refresh.md exists".to_string(),
                rel_to_abs(root, ".claude/commands/aiplus-refresh.md")?.exists(),
            ),
            (
                ".claude/agents/aiplus-advisor.md exists".to_string(),
                rel_to_abs(root, ".claude/agents/aiplus-advisor.md")?.exists(),
            ),
        ],
        "opencode" => vec![
            (
                ".opencode/opencode.json exists".to_string(),
                rel_to_abs(root, ".opencode/opencode.json")?.exists(),
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
        ],
        _ => Vec::new(),
    })
}

fn known_aiplus_entries() -> BTreeSet<String> {
    let mut known = BTreeSet::from([
        ".aiplus/manifest.json".to_string(),
        ".aiplus/AGENTS.aiplus.md".to_string(),
        REFRESH_PROMPT_REL.to_string(),
        ".aiplus/modules".to_string(),
    ]);
    for spec in MODULES {
        known.insert(spec.path.to_string());
    }
    known
}

fn has_managed_block(root: &Path) -> Result<bool> {
    let text = read_text_if_exists(&rel_to_abs(root, "AGENTS.md")?)?.unwrap_or_default();
    Ok(text.contains(MANAGED_BEGIN) && text.contains(MANAGED_END) && text.contains(MANAGED_REF))
}

fn managed_block() -> String {
    format!("{MANAGED_BEGIN}\n{MANAGED_REF}\n{MANAGED_END}")
}

fn agents_aiplus_content() -> String {
    r#"# AiPlus Project Instructions

Use AiPlus Auto Compact and AiPlus Auto Team Consultant when relevant.

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
- Auto Compact: installed/not installed
- Auto Team Consultant: installed/not installed
- Compact state: present/missing/review-needed

How I will use it:
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
- Auto Compact: 已安装/未安装
- Auto Team Consultant: 已安装/未安装
- Compact state: present/missing/review-needed

我会这样使用：
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
3. Re-read `.codex/compact/current-handoff.md` if it exists.
4. Enable AiPlus Auto Team Consultant and AiPlus Auto Compact for the current session.
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

## Secret Broker

Natural language secret mapping:

- "secret 状态", "看看 secret", "检查 API key", "API key 是否可用",
  "刷新 secret", or "更新 secret": run `aiplus secret-broker status` or
  `aiplus secret-broker doctor`.
- When an explicit action needs a key, prefer `aiplus secret-broker run --
  <command...>` so the value enters only the child process environment.
- For approved provider inventory, run `aiplus secret-broker list`. Real
  Bitwarden checks require the `bws` CLI and a read-only machine account token.
  If `bws` is unavailable, report real Bitwarden verification as unverified.
- The child command can still print, log, transmit, or store its environment.
  Use `run --` only with trusted commands for an explicit action need.

Never print, paste, log, summarize, compact, or persist secret values. Do not run
`aiplus secret-broker resolve <alias> --print` in normal agent guidance. If a
secret is unavailable, report one exact fix command and continue without exposing
values.

## Auto Compact

Read `.codex/compact/current-handoff.md` before long-running work if it exists.

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
2. If the user sends a continuation message, accept natural phrasing:
   continue, resume, go on, 继续, 刷新, 接着.
3. Continue from the reported next safe action.

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

Default:
- Advisor session: Advisor mode, LIGHT by default, no file edits unless explicitly approved.
- CEO session: CEO mode, set goal, decompose tasks, use agents only when useful, require Result Packets, run review/fix/QA.
- Reviewer session: findings first, PASS/REVISE/BLOCKED.
- Builder session: changed files, verification, risks, review request.

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
- Auto Compact: installed/not installed
- Auto Team Consultant: installed/not installed
- Compact state: present/missing/review-needed

How I will use it:
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
- Auto Compact: 已安装/未安装
- Auto Team Consultant: 已安装/未安装
- Compact state: present/missing/review-needed

我会这样使用：
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

Meaning: reread AGENTS.md and .aiplus/AGENTS.aiplus.md, read .codex/compact/current-handoff.md if present, run aiplus compact resume after compact when work should continue, enable AiPlus, and continue the current task.

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

Secret mapping: "secret 状态", "看看 secret", "检查 API key", "API key 是否可用",
"刷新 secret", or "更新 secret" means run `aiplus secret-broker status` or
`aiplus secret-broker doctor`. Never print, paste, log, compact, or persist
secret values. Use `aiplus secret-broker run -- <command...>` only for explicit
runtime secret needs. The child command can still print, log, transmit, or store
its environment; use it only with trusted commands. Run `aiplus secret-broker
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
3. Read .codex/compact/current-handoff.md if present.
4. Enable AiPlus Auto Team Consultant and AiPlus Auto Compact for this session.
5. If the user said AiPlus 刷新, 刷新 AiPlus, aiplus refresh, aiplus status, AiPlus status, 继续 AiPlus, resume AiPlus, or only 刷新/refresh, summarize Auto Compact, Auto Team Consultant, and compact state before any project-specific refresh. Use English by default; use Chinese when the user used Chinese such as 刷新 or AiPlus 刷新.
6. Continue the current task.

Continuation keywords: AiPlus 刷新, 刷新 AiPlus, aiplus refresh, aiplus status, AiPlus status, 继续 AiPlus, resume AiPlus, 继续, 刷新, continue, resume, refresh, go on, 接着.

This is not approval to push, publish, tag, release, deploy, globally install, edit global configs, contact external accounts, upload private data, add telemetry, or expose secrets.
"#
    .to_string()
}

fn claude_advisor_agent_content() -> String {
    r#"# AiPlus Advisor

Use project-local AiPlus modules from .aiplus/modules/ when relevant.

- Auto Compact: .aiplus/modules/aiplus-auto-compact/
- Auto Team Consultant: .aiplus/modules/aiplus-auto-team-consultant/

For already-open agent sessions, explicit AiPlus refresh triggers are:
AiPlus 刷新, 刷新 AiPlus, aiplus refresh, aiplus status, AiPlus status, 继续 AiPlus, resume AiPlus.

Generic continuation also works when possible:
继续, 刷新, continue, resume, refresh, go on, 接着.
"#
    .to_string()
}

fn opencode_config_content() -> String {
    serde_json::json!({
        "aiplus": {
            "localOnly": true,
            "refreshKeywords": ["AiPlus 刷新", "刷新 AiPlus", "aiplus refresh", "aiplus status", "AiPlus status", "继续 AiPlus", "resume AiPlus", "继续", "刷新", "continue", "resume", "refresh", "go on", "接着"],
            "instructions": ".aiplus/AGENTS.aiplus.md"
        }
    })
    .to_string()
        + "\n"
}

fn opencode_prompt_content() -> String {
    r#"# AiPlus

Read .aiplus/AGENTS.aiplus.md and use project-local AiPlus modules when relevant.

Explicit AiPlus refresh triggers for already-open agent sessions: AiPlus 刷新,
刷新 AiPlus, aiplus refresh, aiplus status, AiPlus status, 继续 AiPlus,
resume AiPlus.

Generic continuation keywords should try AiPlus first when possible: 继续, 刷新,
continue, resume, refresh, go on, 接着.
"#
    .to_string()
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

fn epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}
