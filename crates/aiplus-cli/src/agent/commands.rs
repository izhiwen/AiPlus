use crate::agent::audit;
use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct AgentArgs {
    #[command(subcommand)]
    pub subcommand: AgentSub,
}

#[derive(Subcommand)]
pub enum AgentSub {
    #[command(visible_aliases = ["团队", "团"])]
    Status {
        #[arg(long, action = clap::ArgAction::SetTrue)]
        verbose: bool,
        #[arg(long, action = clap::ArgAction::SetTrue)]
        json: bool,
    },

    #[command(visible_aliases = ["健康", "诊断"])]
    Doctor,

    #[command(visible_aliases = ["缓存", "暖"])]
    Cache {
        #[arg(long, action = clap::ArgAction::SetTrue)]
        enable_disk: bool,
        #[arg(long, action = clap::ArgAction::SetTrue)]
        disable_disk: bool,
        #[arg(long, action = clap::ArgAction::SetTrue)]
        clear: bool,
        #[arg(long, action = clap::ArgAction::SetTrue)]
        status: bool,
    },

    #[command(visible_aliases = ["列表", "清单"])]
    List {
        #[arg(long, action = clap::ArgAction::SetTrue)]
        functional: bool,
    },

    #[command(visible_aliases = ["跟", "找"])]
    Talk {
        /// Runtime to open: codex, claude-code, or opencode. Omit to keep
        /// the existing manifest/PATH auto-detection behavior.
        #[arg(long, value_name = "RUNTIME")]
        runtime: Option<String>,
        role: String,
    },

    #[command(visible_aliases = ["派单", "分"])]
    Route {
        /// Score and plan the task without consulting, staffing roles, or writing cache.
        /// For BWS-backed secrets, wrap live dispatch with `aiplus secret-broker run --aliases anthropic,openai --`.
        #[arg(long = "score-only", visible_alias = "打分", action = clap::ArgAction::SetTrue)]
        score_only: bool,
        /// Optional multi-phase workflow. Supported: author-critic-fixer.
        /// Must precede the positional role/task so it isn't swallowed
        /// by the trailing-var-arg task parser.
        #[arg(long, value_name = "WORKFLOW")]
        workflow: Option<String>,
        /// Approve one or more owner gates before dispatch. Repeatable.
        /// Must precede the positional role/task so it isn't swallowed
        /// by the trailing-var-arg task parser.
        #[arg(long = "owner-approved", action = clap::ArgAction::Append)]
        owner_approved: Vec<String>,
        role: Option<String>,
        #[arg(long)]
        role_opt: Option<String>,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        task: Vec<String>,
    },

    #[command(visible_aliases = ["重置", "复位"])]
    Reset,

    /// Switch which installed virtual team is the active one.
    /// `agent-team` (software-engineering) or `aieconlab` (econ research).
    /// Re-runs the chosen team's install hook so `.aiplus/agents/` reflects it.
    #[command(visible_aliases = ["切团队", "选团队"])]
    SetTeam { team: String },

    #[command(visible_aliases = ["召唤", "请"])]
    Invite { role: String },

    #[command(visible_aliases = ["让走", "解散"])]
    Dismiss { role: String },

    #[command(visible_aliases = ["禁用", "关闭"])]
    Disable { role: String },

    #[command(visible_aliases = ["启用", "打开"])]
    Enable { role: String },

    #[command(visible_aliases = ["合并", "集成"])]
    Integrate { role: String },

    #[command(visible_aliases = ["看活", "记录"])]
    Transcript,

    /// P1.3: dispatch-history view of `.aiplus/agents/dispatch-log.jsonl`.
    /// Shows success / fail / canceled outcomes with reason + detail.
    /// Filter by role, outcome, or time window.
    #[command(name = "dispatch-history", visible_aliases = ["派单史", "历史"])]
    DispatchHistory {
        /// Only show entries from this role (slug).
        #[arg(long)]
        role: Option<String>,
        /// Only show entries with this outcome: success / fail / canceled.
        #[arg(long)]
        outcome: Option<String>,
        /// Only show entries from the last N days (default: all).
        #[arg(long, value_name = "DAYS")]
        since_days: Option<u64>,
        /// Emit JSON instead of human-readable table.
        #[arg(long, action = clap::ArgAction::SetTrue)]
        json: bool,
    },

    #[command(name = "prune-worktrees", visible_aliases = ["清", "清理"])]
    PruneWorktrees {
        #[arg(long, action = clap::ArgAction::SetTrue)]
        yes: bool,
    },

    #[command(visible_aliases = ["审计", "查"])]
    Audit(audit::AuditArgs),

    #[command(name = "token-cost", visible_aliases = ["成本", "token成本"])]
    TokenCost {
        #[arg(long, action = clap::ArgAction::SetTrue)]
        by_role: bool,
        #[arg(long, value_name = "1h|8h|24h")]
        window: Option<String>,
        #[arg(long, default_value_t = 5)]
        top_n: usize,
    },
}
