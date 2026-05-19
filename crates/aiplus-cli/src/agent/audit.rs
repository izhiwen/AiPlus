use clap::{Parser, Subcommand};

pub mod bin_aliases;
pub mod canary;
pub mod force_skip;
pub mod owner_feedback;
pub mod owner_feedback_retract;
pub mod re_sign_manifest;
pub mod replay;
pub mod run;
pub mod setup_gpg;
pub mod status;
pub mod verify_log;
pub mod weekly_spot_check;

#[derive(Parser)]
pub struct AuditArgs {
    #[command(subcommand)]
    pub subcommand: AuditSub,
}

#[derive(Subcommand)]
pub enum AuditSub {
    /// Run audit against acceptance schema
    #[command(visible_aliases = ["跑", "执行"])]
    Run {
        #[arg(long)]
        deliverable: Option<String>,
        #[arg(long, default_value = "deterministic")]
        mode: String,
        #[arg(long, value_name = "PATH", hide = true)]
        schema_path: Option<String>,
    },

    /// Trigger canary replay
    #[command(visible_aliases = ["金丝雀", "复查"])]
    Canary,

    /// Replay a specific audit run
    #[command(visible_aliases = ["重放", "回放"])]
    Replay { run_id: String },

    /// Owner feedback on audit results
    #[command(visible_aliases = ["反馈", "评价"])]
    OwnerFeedback {
        run_id: String,
        #[arg(long)]
        actual_verdict: String,
        #[arg(long)]
        note: Option<String>,
    },

    /// Retract owner feedback
    #[command(visible_aliases = ["撤回", "取消"])]
    OwnerFeedbackRetract { run_id: String },

    /// Force skip an audit step
    #[command(visible_aliases = ["跳过", "强跳"])]
    ForceSkip {
        gate_id: String,
        #[arg(long)]
        reason: String,
    },

    /// Re-sign manifest
    #[command(visible_aliases = ["重签", "签名"])]
    ReSignManifest,

    /// First-run GPG setup wizard
    #[command(visible_aliases = ["配gpg", "密钥"])]
    SetupGpg,

    /// Verify tamper-evident dispatch-log hash chain
    #[command(name = "verify-log")]
    VerifyLog,

    /// Weekly spot check
    #[command(visible_aliases = ["周检", "抽查"])]
    WeeklySpotCheck,

    /// Audit status
    #[command(visible_aliases = ["状态", "情况"])]
    Status,
}

pub fn dispatch(args: AuditArgs) -> anyhow::Result<()> {
    match args.subcommand {
        AuditSub::Run {
            deliverable,
            mode,
            schema_path,
        } => run::handle_run(deliverable.as_deref(), &mode, schema_path.as_deref()),
        AuditSub::Canary => {
            println!("Canary command not yet implemented in v0.1");
            Ok(())
        }
        AuditSub::Replay { run_id } => replay::handle_replay(&run_id),
        AuditSub::OwnerFeedback {
            run_id,
            actual_verdict,
            note,
        } => owner_feedback::handle_owner_feedback(&run_id, &actual_verdict, note.as_deref()),
        AuditSub::OwnerFeedbackRetract { run_id } => {
            owner_feedback_retract::handle_owner_feedback_retract(&run_id)
        }
        AuditSub::ForceSkip { gate_id, reason } => force_skip::handle_force_skip(&gate_id, &reason),
        AuditSub::ReSignManifest => re_sign_manifest::handle_re_sign_manifest(),
        AuditSub::SetupGpg => setup_gpg::handle_setup_gpg(),
        AuditSub::VerifyLog => verify_log::handle_verify_log(),
        AuditSub::WeeklySpotCheck => weekly_spot_check::handle_weekly_spot_check(),
        AuditSub::Status => status::handle_status(),
    }
}
