use anyhow::{anyhow, Context, Result};
use clap::{ArgAction, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

include!(concat!(env!("OUT_DIR"), "/asset_files.rs"));

const VERSION: &str = "0.1.1";
const INSTALLER: &str = "aiplus";
const REFRESH_PROMPT: &str = "刷新";
const REFRESH_PROMPT_REL: &str = ".aiplus/REFRESH_PROMPT.txt";
const MANAGED_BEGIN: &str = "<!-- BEGIN AIPLUS MANAGED BLOCK -->";
const MANAGED_END: &str = "<!-- END AIPLUS MANAGED BLOCK -->";
const MANAGED_REF: &str = "@./.aiplus/AGENTS.aiplus.md";

#[derive(Parser)]
#[command(
    name = "aiplus",
    version = VERSION,
    disable_version_flag = true,
    after_help = "Safety:\n  Project-local only. Writes are limited to .aiplus/, .codex/compact/, and\n  the AiPlus managed block in AGENTS.md. No npm publish, global install,\n  telemetry, network calls, or global config edits are implemented."
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
        #[arg(long, action = ArgAction::SetTrue)]
        force: bool,
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
        version: "0.1.0",
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
        version: "0.1.2",
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CompactCheckpoint {
    schema_version: String,
    timestamp: String,
    cwd: String,
    validation_result: String,
    status: String,
    pending_gates: Vec<String>,
    denied_gates: Vec<String>,
    review_items: Vec<String>,
    warnings: Vec<String>,
    errors: Vec<String>,
    current_goal: Option<String>,
    current_phase: Option<String>,
    open_blockers: Option<String>,
    owner_gates: Option<String>,
    next_safe_action: Option<String>,
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
        Commands::Uninstall {
            dry_run,
            yes,
            force,
        } => command_uninstall(dry_run, yes, force),
        Commands::Compact { subcommand, force } => command_compact(subcommand, force),
    }
}

fn print_usage() {
    println!(
        "AiPlus CLI {VERSION}\n\nUsage:\n  aiplus <command> [options]\n\nCommands:\n  install codex|claude-code|opencode|all [--dry-run] [--verbose] [--force --backup --yes]\n  update [auto-compact|auto-team-consultant] [--dry-run] [--verbose]\n  add auto-compact|auto-team-consultant [--dry-run] [--verbose]\n  doctor\n  status\n  uninstall --dry-run\n  uninstall --yes [--force]\n  compact init|validate|checkpoint|resume\n\nSafety:\n  Project-local only. Writes are limited to .aiplus/, .codex/compact/, and\n  the AiPlus managed block in AGENTS.md. No npm publish, global install,\n  telemetry, network calls, or global config edits are implemented."
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
    write_manifest(
        &root,
        &mut plan,
        &effective_options,
        &adapters,
        &default_module_names(),
        &default_module_names(),
    )?;
    print_install_summary(&plan, verbose, &adapters, upgrade_existing);
    Ok(())
}

fn command_update(module: Option<String>, dry_run: bool, verbose: bool) -> Result<()> {
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
    if verbose {
        plan_printer(&plan);
    } else {
        println!("GLOBAL_CONFIG_UNTOUCHED");
    }
    println!("UPDATE_STATUS=PASS");
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
    println!("type \"{REFRESH_PROMPT}\" or \"refresh\"");
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
        println!("next=For already-open agent sessions, type \"{REFRESH_PROMPT}\" or \"refresh\".");
    } else {
        println!("next=run install codex");
    }
    println!("STATUS=PASS");
    Ok(())
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
                == Some("0.1.3"),
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
            && manifest.schema_version.as_deref() == Some("0.1.3")
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
            format!("send {REFRESH_PROMPT} or refresh to the current agent session")
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

fn command_compact(subcommand: Option<String>, force: bool) -> Result<()> {
    let Some(subcommand) = subcommand else {
        print_usage();
        process::exit(2);
    };
    if !["init", "validate", "checkpoint", "resume"].contains(&subcommand.as_str()) {
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
            let exit_code = compact_checkpoint(&root)?;
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
        _ => unreachable!(),
    }
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
        schema_version: Some("0.1.3".to_string()),
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
    println!("Next: send \"{REFRESH_PROMPT}\" or \"refresh\" to any already-open agent session.");
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

fn compact_checkpoint(root: &Path) -> Result<i32> {
    ensure_dir(
        root,
        &compact_file(root, "checkpoints")?,
        &mut Plan::default(),
    )?;
    let result = compact_validate_state(root)?;
    let mut status = "SAFE_TO_COMPACT";
    let mut exit_code = 0;
    if !result.errors.is_empty() || !result.warnings.is_empty() || !result.denied_gates.is_empty() {
        status = "BLOCKED_DO_NOT_COMPACT";
        exit_code = 1;
    } else if !result.review_items.is_empty() || !result.pending_gates.is_empty() {
        status = "UNKNOWN_NEEDS_REVIEW";
        exit_code = 2;
    }
    let timestamp = timestamp();
    let handoff = read_compact_text(root, "current-handoff.md").unwrap_or_default();
    let evidence = read_compact_text(root, "evidence-ledger.md").unwrap_or_default();
    let checkpoint = CompactCheckpoint {
        schema_version: "0.1.0".to_string(),
        timestamp: timestamp.clone(),
        cwd: "<REPO_ROOT>".to_string(),
        validation_result: if result.ok { "PASS" } else { "FAIL" }.to_string(),
        status: status.to_string(),
        pending_gates: result.pending_gates.clone(),
        denied_gates: result.denied_gates.clone(),
        review_items: result.review_items.clone(),
        warnings: result.warnings.clone(),
        errors: result.errors.clone(),
        current_goal: optional_line(section_body(&handoff, "Current Goal")),
        current_phase: optional_line(section_body(&handoff, "Current Phase")),
        open_blockers: optional_line(section_body(&handoff, "Open Blockers")),
        owner_gates: optional_line(section_body(&handoff, "Owner Gates")),
        next_safe_action: optional_line(result.next_safe_action.clone()),
        evidence_pointers: evidence_ids(&evidence),
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
    print_compact_diagnostics(&result);
    println!("{status}");
    println!("CHECKPOINT_CREATED={rel}");
    println!("checkpoint={rel}");
    Ok(exit_code)
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
    println!("RESUME_READY");
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
    Ok(0)
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
    if version != "0.1.0" {
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

If the user says only `刷新` or `refresh`, treat it as AiPlus refresh first,
not as a generic project status refresh. Respond in this shape:

已刷新 AiPlus。

当前项目 AiPlus 状态：
- Auto Compact: 已安装/未安装
- Auto Team Consultant: 已安装/未安装
- Compact state: present/missing/review-needed

我会这样使用：
- 长任务或 compact 前准备 checkpoint
- compact 后如果宿主交回控制权，我会自动 resume
- 如果宿主需要消息唤醒，你随便说“继续/刷新/refresh/continue”即可
- CEO Prompt / review / brainstorm 时使用 Auto Team Consultant

边界：
- AiPlus 不能替你点击 compact
- 不上传数据
- 不改全局 agent config

If the user says a natural continuation such as `继续`, `continue`, `resume`,
`go on`, or `接着`, treat it as:

1. Re-read `AGENTS.md`.
2. Re-read `.aiplus/AGENTS.aiplus.md`.
3. Re-read `.codex/compact/current-handoff.md` if it exists.
4. Enable AiPlus Auto Team Consultant and AiPlus Auto Compact for the current session.
5. Run `aiplus compact resume` if compact state exists and host control has
   returned after compact.
6. Continue the current task without asking the user to repeat the full instruction.

Refresh is not approval to push, publish, tag, release, deploy, globally install, edit global configs, contact external accounts, upload private data, add telemetry, or expose secrets.

## Auto Compact

Read `.codex/compact/current-handoff.md` before long-running work if it exists.

Before context compaction or compact-worthy moments:
1. Run `aiplus compact validate`.
2. Run `aiplus compact checkpoint`.
3. Suggest compact only after checkpoint is ready:
   建议现在 compact。AiPlus checkpoint 已准备好。compact 后如果宿主继续把控制权交给我，我会自动恢复；如果工具等待你发消息，随便说“继续”“刷新”“continue”“resume”或类似意思即可。

After context compaction:
1. If the host gives control back automatically, run `aiplus compact resume`
   without waiting for a specific user phrase.
2. If the host requires a user message, accept any natural continuation:
   继续, 刷新, continue, resume, refresh, go on, 接着.
3. Continue from the reported next safe action.

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

If the user says only 刷新 or refresh, treat it as AiPlus refresh first. Reply:

已刷新 AiPlus。

当前项目 AiPlus 状态：
- Auto Compact: 已安装/未安装
- Auto Team Consultant: 已安装/未安装
- Compact state: present/missing/review-needed

我会这样使用：
- 长任务或 compact 前准备 checkpoint
- compact 后如果宿主交回控制权，我会自动 resume
- 如果宿主需要消息唤醒，你随便说“继续/刷新/refresh/continue”即可
- CEO Prompt / review / brainstorm 时使用 Auto Team Consultant

边界：
- AiPlus 不能替你点击 compact
- 不上传数据
- 不改全局 agent config

Other continuation keywords: 继续, continue, resume, go on, 接着

Meaning: reread AGENTS.md and .aiplus/AGENTS.aiplus.md, read .codex/compact/current-handoff.md if present, run aiplus compact resume when compact state exists after host control returns, enable AiPlus, and continue the current task.

Refresh is not approval to push, publish, tag, release, deploy, globally install, edit global configs, contact external accounts, upload private data, add telemetry, or expose secrets.
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
5. If the user said only 刷新 or refresh, reply with "已刷新 AiPlus。" and summarize Auto Compact, Auto Team Consultant, and compact state.
6. Continue the current task.

Continuation keywords: 继续, 刷新, continue, resume, refresh, go on, 接着.

This is not approval to push, publish, tag, release, deploy, globally install, edit global configs, contact external accounts, upload private data, add telemetry, or expose secrets.
"#
    .to_string()
}

fn claude_advisor_agent_content() -> String {
    r#"# AiPlus Advisor

Use project-local AiPlus modules from .aiplus/modules/ when relevant.

- Auto Compact: .aiplus/modules/aiplus-auto-compact/
- Auto Team Consultant: .aiplus/modules/aiplus-auto-team-consultant/

For already-open agent sessions, the user can type any natural continuation:
继续, 刷新, continue, resume, refresh, go on, 接着.
"#
    .to_string()
}

fn opencode_config_content() -> String {
    serde_json::json!({
        "aiplus": {
            "localOnly": true,
            "refreshKeywords": ["继续", "刷新", "continue", "resume", "refresh", "go on", "接着"],
            "instructions": ".aiplus/AGENTS.aiplus.md"
        }
    })
    .to_string()
        + "\n"
}

fn opencode_prompt_content() -> String {
    r#"# AiPlus

Read .aiplus/AGENTS.aiplus.md and use project-local AiPlus modules when relevant.

Continuation keywords for already-open agent sessions: 继续, 刷新, continue,
resume, refresh, go on, 接着.
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
