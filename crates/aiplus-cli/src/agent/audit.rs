use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct AuditArgs {
    #[command(subcommand)]
    pub subcommand: AuditSub,
}

#[derive(Subcommand)]
pub enum AuditSub {
    /// Run audit against acceptance schema
    #[command(visible_aliases = ["审", "审查"])]
    Run {
        #[arg(long)]
        deliverable: Option<String>,
        #[arg(long, default_value = "deterministic")]
        mode: String,
    },

    /// Trigger canary replay
    #[command(visible_aliases = ["金丝雀", "复查"])]
    Canary,

    /// Replay a specific audit run
    #[command(visible_aliases = ["重放", "回放"])]
    Replay {
        run_id: String,
    },

    /// First-run GPG setup wizard
    #[command(visible_aliases = ["配置签名", "初始化"])]
    SetupGpg,
}
