use crate::agent::worktree::get_repo_name;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct AgentConfig {
    #[serde(default)]
    pub schema_version: String,
    #[serde(rename = "agent")]
    pub agent: AgentSection,
    #[serde(rename = "persona")]
    pub persona: Option<PersonaSection>,
    #[serde(rename = "workspace")]
    pub workspace: Option<WorkspaceSection>,
    #[serde(rename = "memory")]
    pub memory: Option<MemorySection>,
    #[serde(rename = "invocation")]
    pub invocation: Option<InvocationSection>,
    /// S5: secret-broker aliases this role needs to do its job.
    /// Used by `agent route` (S7) to auto-inject broker env vars
    /// and by `aiplus doctor` (S6) to flag missing aliases at
    /// install time so the user knows to provision them BEFORE the
    /// first dispatch fails.
    #[serde(rename = "secret_needs", default)]
    pub secret_needs: Option<SecretNeedsSection>,
    // Flattened convenience fields (derived from nested sections)
    #[serde(skip)]
    pub role: String,
    #[serde(skip)]
    pub display_name: String,
    #[serde(skip)]
    pub tier: String,
    #[serde(skip)]
    pub status: String,
    #[serde(skip)]
    pub warm_bench_ttl_seconds: u64,
    #[serde(skip)]
    pub stub: bool,
    #[serde(skip)]
    pub needs_worktree: bool,
    #[serde(skip)]
    pub worktree_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct AgentSection {
    pub role: String,
    pub display_name: String,
    #[serde(default)]
    pub tier: String,
    #[serde(default)]
    pub default_specialty: String,
    #[serde(default = "default_ttl")]
    pub warm_bench_ttl_seconds: u64,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct PersonaSection {
    #[serde(default)]
    pub system_prompt_file: String,
    #[serde(default)]
    pub voice: String,
    #[serde(default)]
    pub escalation_target: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct WorkspaceSection {
    #[serde(default)]
    pub needs_worktree: bool,
    #[serde(default)]
    pub worktree_branch: String,
    #[serde(default)]
    pub worktree_path: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct MemorySection {
    #[serde(default)]
    pub personal_dir: String,
    #[serde(default)]
    pub read_team_memory: bool,
    #[serde(default)]
    pub read_project_memory: bool,
    #[serde(default)]
    pub write_team_memory: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct InvocationSection {
    #[serde(default)]
    pub chinese_aliases: Vec<String>,
    #[serde(default)]
    pub english_aliases: Vec<String>,
}

/// S5: per-role declaration of secret-broker aliases needed for
/// the role to function. Schema:
///   [secret_needs]
///   aliases = ["anthropic", "openai"]
///
/// Empty list (or omitted section) means "no external secret needed";
/// the role runs entirely on local resources. Listed aliases get
/// pre-flight checked by `doctor` (S6) and pre-injected by `route` (S7).
#[derive(Debug, Clone, Deserialize, Default)]
#[allow(dead_code)]
pub struct SecretNeedsSection {
    #[serde(default)]
    pub aliases: Vec<String>,
}

fn default_ttl() -> u64 {
    1800
}

impl AgentConfig {
    pub fn flatten(&mut self) {
        self.role = self.agent.role.clone();
        self.display_name = self.agent.display_name.clone();
        self.tier = self.agent.tier.clone();
        self.status = self.agent.status.clone();
        self.warm_bench_ttl_seconds = self.agent.warm_bench_ttl_seconds;
        self.needs_worktree = self
            .workspace
            .as_ref()
            .map(|w| w.needs_worktree)
            .unwrap_or(false);
        self.worktree_path = self.workspace.as_ref().and_then(|w| {
            if w.worktree_path.is_empty() {
                None
            } else {
                Some(w.worktree_path.clone())
            }
        });
    }
}

#[derive(Debug, Default)]
pub struct TeamState {
    pub agents: HashMap<String, AgentConfig>,
    pub active_roles: Vec<String>,
    pub disabled_roles: Vec<String>,
    pub stub_roles: Vec<String>,
    pub worktree_paths: HashMap<String, PathBuf>,
}

/// The 8 core roles available in v0.1
const CORE_ROLES: &[&str] = &[
    "advisor",
    "ceo",
    "architect",
    "pm",
    "engineer-a",
    "engineer-b",
    "reviewer",
    "qa",
];

/// The 6 functional experts available in v0.1
const FUNCTIONAL_EXPERTS: &[&str] = &[
    "ai-integration",
    "security-reviewer",
    "tech-writer",
    "devops",
    "ui-designer",
    "researcher",
];

/// The 5 v0.2 stub experts
const STUB_EXPERTS: &[&str] = &[
    "data-analyst",
    "customer-researcher",
    "performance-engineer",
    "accessibility",
    "compliance-reviewer",
];

/// Issue #32: roster owned by each installed virtual team. When both
/// `agent-team` and `aieconlab` install into the same project, every
/// role's TOML/persona ends up under `.aiplus/agents/` (the init paths
/// don't pre-clear the other team), so naive `read_dir` returns ~37
/// roles and `aiplus agent status` no longer reflects the active team.
///
/// These constants are the source of truth for membership; the active
/// team's roster is what `load_team_config` retains.
const AGENT_TEAM_CORE: &[&str] = &[
    "advisor",
    "ceo",
    "architect",
    "pm",
    "engineer-a",
    "engineer-b",
    "reviewer",
    "qa",
];
const AGENT_TEAM_EXPERTS: &[&str] = &[
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
const AIECONLAB_CORE: &[&str] = &[
    "advisor",
    "pi",
    "theorist",
    "pm",
    "ra-stata",
    "ra-python",
    "referee",
    "replicator",
];
const AIECONLAB_EXPERTS: &[&str] = &[
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

/// Return the set of role IDs that belong to `team`, including experts.
/// `None` for unknown team names — callers should treat that as "do not
/// filter" (preserve the legacy unfiltered behavior).
fn team_roster(team: &str) -> Option<HashSet<String>> {
    let (core, experts): (&[&str], &[&str]) = match team {
        "agent-team" => (AGENT_TEAM_CORE, AGENT_TEAM_EXPERTS),
        "aieconlab" => (AIECONLAB_CORE, AIECONLAB_EXPERTS),
        _ => return None,
    };
    Some(
        core.iter()
            .chain(experts.iter())
            .map(|s| (*s).to_string())
            .collect(),
    )
}

/// Check if a role is a v0.2 stub (not yet functional in v0.1)
pub fn is_stub(role: &str) -> bool {
    STUB_EXPERTS.contains(&role)
}

/// Load all agent configurations from `.aiplus/agents/` and `.aiplus/agents/_experts/`
pub fn load_team_config(project_root: &Path) -> Result<TeamState> {
    let mut state = TeamState::default();
    let agents_dir = project_root.join(".aiplus").join("agents");

    if !agents_dir.exists() {
        return Ok(state);
    }

    // Load core team configs from .aiplus/agents/
    for entry in fs::read_dir(&agents_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            // Skip team-wide configs (different schema): agent-team.toml from
            // aiplus-agent-team, econ-team.toml from aieconlab.
            let file_name = path.file_name().and_then(|s| s.to_str());
            if matches!(file_name, Some("agent-team.toml" | "econ-team.toml")) {
                continue;
            }
            let content =
                fs::read_to_string(&path).with_context(|| format!("Failed to read {:?}", path))?;
            let mut config: AgentConfig = toml::from_str(&content)
                .with_context(|| format!("Failed to parse TOML {:?}", path))?;
            config.flatten();
            config.stub = is_stub(&config.role);
            if config.stub {
                state.stub_roles.push(config.role.clone());
            }
            if let Some(ref wt) = config.worktree_path {
                let repo_name = get_repo_name(project_root).unwrap_or_default();
                let resolved = wt.replace("{project_name}", &repo_name);
                state
                    .worktree_paths
                    .insert(config.role.clone(), PathBuf::from(resolved));
            }
            state.agents.insert(config.role.clone(), config);
        }
    }

    // Load expert configs from .aiplus/agents/experts/
    let experts_dir = agents_dir.join("experts");
    if experts_dir.exists() {
        for entry in fs::read_dir(&experts_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("toml") {
                let content = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read {:?}", path))?;
                let mut config: AgentConfig = toml::from_str(&content)
                    .with_context(|| format!("Failed to parse TOML {:?}", path))?;
                config.flatten();
                config.stub = is_stub(&config.role);
                if config.stub {
                    state.stub_roles.push(config.role.clone());
                }
                if let Some(ref wt) = config.worktree_path {
                    let repo_name = get_repo_name(project_root).unwrap_or_default();
                    let resolved = wt.replace("{project_name}", &repo_name);
                    state
                        .worktree_paths
                        .insert(config.role.clone(), PathBuf::from(resolved));
                }
                state.agents.insert(config.role.clone(), config);
            }
        }
    }

    // Issue #32: When both `agent-team` and `aieconlab` are installed
    // in the same project, every role's TOML lands under
    // `.aiplus/agents/` (the init paths don't pre-clear the other
    // team), so the raw read above produces ~37 mixed roles regardless
    // of which team is active. Filter the roster to the active team
    // before computing active/disabled/stub lists, so `aiplus agent
    // status` (and every other consumer of this function) reflects
    // only the team the project is currently using.
    if let Some(active_team) = crate::agent::set_team::read_active_team(project_root) {
        if let Some(roster) = team_roster(&active_team) {
            state.agents.retain(|role, _| roster.contains(role));
            state.worktree_paths.retain(|role, _| roster.contains(role));
            state
                .stub_roles
                .retain(|role| state.agents.contains_key(role));
        }
    }

    // Derive active/disabled lists from status field
    for (role, config) in &state.agents {
        match config.status.as_str() {
            "active" => state.active_roles.push(role.clone()),
            "disabled" => state.disabled_roles.push(role.clone()),
            _ => {}
        }
    }

    Ok(state)
}

/// List all roles, optionally filtering out stubs
pub fn list_roles(team: &TeamState, functional_only: bool) -> Vec<&AgentConfig> {
    let mut roles: Vec<&AgentConfig> = team.agents.values().collect();
    if functional_only {
        roles.retain(|c| !is_stub(&c.role));
    }
    roles
}

/// Load a specific role's configuration
pub fn get_role_config(role: &str) -> Result<AgentConfig> {
    let project_root = std::env::current_dir()?;
    let state = load_team_config(&project_root)?;

    if let Some(config) = state.agents.get(role) {
        return Ok(config.clone());
    }

    // Return a default config for known roles even if file doesn't exist
    if is_stub(role) {
        Ok(AgentConfig::with_role(role, "expert", "stub", true))
    } else if FUNCTIONAL_EXPERTS.contains(&role) || CORE_ROLES.contains(&role) {
        let tier = if role == "advisor" || role == "ceo" {
            "owner_facing"
        } else {
            "internal"
        };
        Ok(AgentConfig::with_role(role, tier, "inactive", false))
    } else {
        anyhow::bail!("Unknown role: {}", role)
    }
}

impl AgentConfig {
    pub fn with_role(role: &str, tier: &str, status: &str, stub: bool) -> Self {
        let mut config = Self {
            schema_version: "1.0".to_string(),
            agent: AgentSection {
                role: role.to_string(),
                display_name: role.to_string(),
                tier: tier.to_string(),
                default_specialty: String::new(),
                warm_bench_ttl_seconds: 1800,
                status: status.to_string(),
            },
            persona: None,
            workspace: None,
            memory: None,
            invocation: None,
            secret_needs: None,
            role: role.to_string(),
            display_name: role.to_string(),
            tier: tier.to_string(),
            status: status.to_string(),
            warm_bench_ttl_seconds: 1800,
            stub,
            needs_worktree: false,
            worktree_path: None,
        };
        config.flatten();
        config
    }
}
