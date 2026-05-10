pub mod assets;
pub mod auto_write;
pub mod capsule;
pub mod compact_state;
pub mod error;
pub mod identity;
pub mod install_plan;
pub mod manifest;
pub mod memory;
pub mod memory_conflict;
pub mod memory_context;
pub mod module_manifest;
pub mod paths;
pub mod profile_sync;
pub mod redaction;
pub mod rollback;
pub mod session;
pub mod skill_candidate;
pub mod snapshot;
pub mod velocity;

pub use assets::{embedded_asset_bytes, embedded_asset_paths, embedded_asset_text};
pub use auto_write::{AutoWriteConfig, AutoWriteResult, AutoWriter, RiskLevel};
pub use capsule::{
    build_capsule_from_compact_state, compute_capsule_checksum, timestamp, CapsuleDecision,
    CapsuleItem, CapsuleOwnerGate, CapsuleRisk, CapsuleTier, CapsuleVerification, ContextCapsule,
    RedactionSummary, CAPSULE_SCHEMA_VERSION,
};
pub use compact_state::{
    parse_watch_interval, RedactionInfo, ReminderState, WatchConfig, WatchMode, WatchResult,
    REMINDER_STATE_SCHEMA_VERSION,
};
pub use identity::{read_identity, RoleIdentity, IDENTITY_SCHEMA_VERSION_V2};
pub use manifest::{ProjectManifest, ProjectManifestModule};
pub use memory::{
    append_jsonl_atomic, epoch_millis, find_by_id, find_by_query, read_active,
    read_all as read_memory_records, read_all_including_rejected, rewrite_jsonl_atomic,
    single_line, slugify, stable_hash, write_file_atomic, FileLock, MemoryRecord, MemoryStore,
};
pub use memory_conflict::{
    detect_conflicts, detect_stale, ConflictDetector, ConflictReport, StaleReport,
};
pub use memory_context::{select_records, ContextBudget};
pub use module_manifest::{
    available_modules_text, bundled_module_specs, default_module_names, module_spec,
    normalize_module, validate_bundled_module_manifests, ModuleManifest, ModuleSpec,
};
pub use paths::{identity_dir, memory_dir, skills_dir};
pub use profile_sync::{ProfileSync, SyncResult};
pub use redaction::{
    has_phone_like, has_secret_assignment, is_jwt_like, reject_sensitive_memory_text,
    sensitive_findings,
};
pub use rollback::{RollbackEntry, RollbackPlan};
pub use session::{SessionIndex, SessionRecord, SESSION_SCHEMA_VERSION};
pub use skill_candidate::{
    read_all as read_skill_candidates, SkillCandidate, SkillRegistry, SKILL_SCHEMA_VERSION_V2,
};
pub use snapshot::{SnapshotBuilder, SNAPSHOT_SCHEMA_VERSION};
pub use velocity::{
    append_jsonl as append_velocity_jsonl, apply_retention as apply_velocity_retention,
    classify_rare_case, compute_ai_native_estimate, detect_bias, doctor as velocity_doctor,
    generate_estimate_id, generate_run_id, init_velocity, is_rare_case, now_iso, parse_duration,
    purge_velocity, read_config as read_velocity_config, read_json as read_velocity_json,
    read_jsonl as read_velocity_jsonl, reject_sensitive_velocity_text,
    rewrite_jsonl as rewrite_velocity_jsonl, update_aggregates, update_multipliers,
    validate_run_record, velocity_dir, write_config as write_velocity_config,
    write_json as write_velocity_json, Aggregates, AnchorSignalRecord, BiasResult, BucketData,
    DoctorReport, EstimateRecord, EstimateResult, FastFinishCalibration, MultiplierBucket,
    RareCaseRecord, RotationState, RunRecord, VelocityConfig, VELOCITY_SCHEMA_VERSION,
};
