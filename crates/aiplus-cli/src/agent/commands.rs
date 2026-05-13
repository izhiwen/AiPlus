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

    #[command(visible_aliases = ["列表", "清单"])]
    List {
        #[arg(long, action = clap::ArgAction::SetTrue)]
        functional: bool,
    },

    #[command(visible_aliases = ["跟", "找"])]
    Talk { role: String },

    #[command(visible_aliases = ["派单", "分"])]
    Route {
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

    #[command(name = "prune-worktrees", visible_aliases = ["清", "清理"])]
    PruneWorktrees {
        #[arg(long, action = clap::ArgAction::SetTrue)]
        yes: bool,
    },

    #[command(visible_aliases = ["审计", "查"])]
    Audit(audit::AuditArgs),
}
