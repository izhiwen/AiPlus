use crate::capsule::timestamp;
use crate::memory::{epoch_millis, single_line, slugify, stable_hash, MemoryRecord};
use crate::redaction::reject_sensitive_memory_text;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

pub const SKILL_SCHEMA_VERSION_V2: &str = "0.2.0";

#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Default)]
pub struct SessionRecord {
    pub id: String,
    pub commands_run: Vec<String>,
    pub files_changed: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone)]
pub struct ConsolidationCandidate {
    pub pattern: String,
    pub occurrence_count: usize,
    pub related_memory_ids: Vec<String>,
    pub suggested_title: String,
    pub risk_level: RiskLevel,
}

pub struct ConsolidationEngine {
    #[allow(dead_code)]
    registry: SkillRegistry,
}

impl ConsolidationEngine {
    pub fn new(registry: SkillRegistry) -> Self {
        Self { registry }
    }

    pub fn find_consolidation_candidates(
        &self,
        records: &[MemoryRecord],
        sessions: &[SessionRecord],
    ) -> Vec<ConsolidationCandidate> {
        let mut grouped: HashMap<String, Vec<&MemoryRecord>> = HashMap::new();

        for record in records {
            if record.is_stale() || record.is_expired() {
                continue;
            }
            let pattern = extract_pattern(&record.summary);
            let key = format!("{}:{}", record.record_type, pattern);
            grouped.entry(key).or_default().push(record);
        }

        let mut candidates = Vec::new();
        for (_key, group) in grouped {
            if group.len() < 2 {
                continue;
            }

            let pattern = extract_pattern(&group[0].summary);
            let is_failure = group.iter().any(|r| {
                r.status.to_ascii_lowercase().contains("fail")
                    || r.tags
                        .iter()
                        .any(|t| t.to_ascii_lowercase().contains("fail"))
                    || r.tags
                        .iter()
                        .any(|t| t.to_ascii_lowercase().contains("error"))
            });

            let occurrence_count = group.len();
            let should_consolidate = if is_failure {
                occurrence_count >= 2
            } else {
                occurrence_count >= 3
            };

            if !should_consolidate && !has_keyword_trigger(&pattern) {
                continue;
            }

            let risk_level = if is_failure {
                RiskLevel::High
            } else if has_verification_evidence(&group) {
                RiskLevel::Low
            } else {
                RiskLevel::Medium
            };

            let related_memory_ids: Vec<String> = group.iter().map(|r| r.id.clone()).collect();

            let suggested_title = generate_title(&pattern, &group[0].record_type);

            candidates.push(ConsolidationCandidate {
                pattern,
                occurrence_count,
                related_memory_ids,
                suggested_title,
                risk_level,
            });
        }

        if !sessions.is_empty() {
            let session_candidates = self.find_session_patterns(records, sessions);
            candidates.extend(session_candidates);
        }

        candidates.sort_by(|a, b| {
            let a_score = score_candidate(a);
            let b_score = score_candidate(b);
            b_score.cmp(&a_score)
        });

        candidates
    }

    pub fn auto_consolidate(
        &self,
        root: &Path,
        pattern: &ConsolidationCandidate,
    ) -> Result<SkillCandidate> {
        reject_sensitive_memory_text(&pattern.suggested_title)?;
        let problem_pattern = format!(
            "Auto-detected repeated task pattern: {} ({} occurrences)",
            pattern.pattern, pattern.occurrence_count
        );
        reject_sensitive_memory_text(&problem_pattern)?;

        let id = format!(
            "skill-candidate/{}_{}",
            epoch_millis(),
            stable_hash(&pattern.pattern)
        );
        let now = timestamp();

        let mut allowed_actions = vec!["read".to_string()];
        if pattern.risk_level == RiskLevel::Low {
            allowed_actions.push("write".to_string());
        }

        let forbidden_actions = vec![
            "execute".to_string(),
            "network".to_string(),
            "shell".to_string(),
        ];

        let candidate = SkillCandidate {
            schema_version: SKILL_SCHEMA_VERSION_V2.to_string(),
            id: id.clone(),
            title: single_line(&pattern.suggested_title),
            status: "candidate_proposed".to_string(),
            source_memory_ids: pattern.related_memory_ids.clone(),
            problem_pattern,
            proposed_skill_name: slugify(&pattern.suggested_title),
            repeatability_evidence: RepeatabilityEvidence {
                occurrence_count: pattern.occurrence_count as u64,
                synthetic_replay_available: false,
            },
            trigger_design: TriggerDesign {
                positive_triggers: vec![pattern.pattern.clone()],
                negative_triggers: Vec::new(),
            },
            scope: SkillScope {
                allowed_actions,
                forbidden_actions,
            },
            privacy: SkillPrivacy {
                contains_secrets: false,
                contains_private_paths: false,
                redaction_required: false,
            },
            qa: SkillQa {
                required_checks: vec!["owner_review".to_string()],
                status: "pending".to_string(),
            },
            owner_gate: SkillOwnerGate {
                required_for_approval: true,
                approved_by: None,
                approved_at: None,
            },
            evidence_links: Vec::new(),
            rejection_reason: None,
            needs_evidence: pattern.risk_level == RiskLevel::High,
            proposed_at: now.clone(),
            updated_at: now,
        };

        let path = root.join(".aiplus/skills/candidates/candidates.jsonl");
        let line = serde_json::to_string(&candidate)?;
        append_jsonl_atomic(&path, &line)?;

        Ok(candidate)
    }

    pub fn should_consolidate(records: &[MemoryRecord], pattern_text: &str) -> bool {
        let normalized = normalize_summary(pattern_text);
        let pattern_lower = pattern_text.to_ascii_lowercase();
        let mut count = 0;
        let mut failure_count = 0;

        for record in records {
            if record.is_stale() || record.is_expired() {
                continue;
            }
            let record_normalized = normalize_summary(&record.summary);
            let record_lower = record.summary.to_ascii_lowercase();
            let matches = token_overlap(&normalized, &record_normalized) > 0.7
                || record_lower.contains(&pattern_lower)
                || pattern_lower.contains(&record_lower);
            if matches {
                count += 1;
                if record.status.to_ascii_lowercase().contains("fail")
                    || record
                        .tags
                        .iter()
                        .any(|t| t.to_ascii_lowercase().contains("fail"))
                {
                    failure_count += 1;
                }
            }
        }

        if failure_count >= 2 {
            return true;
        }

        count >= 3
    }

    fn find_session_patterns(
        &self,
        records: &[MemoryRecord],
        sessions: &[SessionRecord],
    ) -> Vec<ConsolidationCandidate> {
        let mut command_groups: HashMap<String, Vec<&SessionRecord>> = HashMap::new();

        for session in sessions {
            let key = session.commands_run.join(";");
            command_groups.entry(key).or_default().push(session);
        }

        let mut candidates = Vec::new();
        for (commands, group) in command_groups {
            if group.len() < 2 {
                continue;
            }

            let related_memory_ids: Vec<String> = group
                .iter()
                .filter_map(|s| {
                    records
                        .iter()
                        .find(|r| r.summary == s.summary)
                        .map(|r| r.id.clone())
                })
                .collect();

            let suggested_title =
                format!("Session workflow: {}", &commands[..commands.len().min(40)]);
            candidates.push(ConsolidationCandidate {
                pattern: commands.clone(),
                occurrence_count: group.len(),
                related_memory_ids,
                suggested_title,
                risk_level: RiskLevel::Medium,
            });
        }

        candidates
    }
}

fn normalize_summary(summary: &str) -> String {
    summary
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn token_overlap(a: &str, b: &str) -> f64 {
    let a_tokens: HashSet<&str> = a.split_whitespace().collect();
    let b_tokens: HashSet<&str> = b.split_whitespace().collect();

    if a_tokens.is_empty() || b_tokens.is_empty() {
        return 0.0;
    }

    let intersection: HashSet<&str> = a_tokens.intersection(&b_tokens).copied().collect();
    let union: HashSet<&str> = a_tokens.union(&b_tokens).copied().collect();

    intersection.len() as f64 / union.len() as f64
}

fn extract_pattern(summary: &str) -> String {
    let lower = summary.to_ascii_lowercase();
    let keywords = [
        "release checklist",
        "compact prepare",
        "doctor",
        "review",
        "test",
    ];

    for keyword in &keywords {
        if lower.contains(keyword) {
            return keyword.to_string();
        }
    }

    lower
        .split_whitespace()
        .take(5)
        .collect::<Vec<_>>()
        .join(" ")
}

fn has_keyword_trigger(pattern: &str) -> bool {
    let keywords = [
        "release checklist",
        "compact prepare",
        "doctor",
        "review",
        "test",
    ];
    let lower = pattern.to_ascii_lowercase();
    keywords.iter().any(|k| lower.contains(k))
}

fn has_verification_evidence(records: &[&MemoryRecord]) -> bool {
    records.iter().any(|r| {
        !r.evidence.is_empty()
            || r.tags
                .iter()
                .any(|t| t.to_ascii_lowercase().contains("verified"))
    })
}

fn generate_title(pattern: &str, record_type: &str) -> String {
    if pattern.len() > 40 {
        format!("{} skill: {}...", record_type, &pattern[..40])
    } else {
        format!("{} skill: {}", record_type, pattern)
    }
}

fn score_candidate(candidate: &ConsolidationCandidate) -> usize {
    let mut score = candidate.occurrence_count * 10;
    match candidate.risk_level {
        RiskLevel::Low => score += 5,
        RiskLevel::High => score += 3,
        RiskLevel::Medium => score += 1,
    }
    if has_verification_evidence_by_ids(&candidate.related_memory_ids) {
        score += 10;
    }
    score
}

fn has_verification_evidence_by_ids(_ids: &[String]) -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SkillCandidate {
    pub schema_version: String,
    pub id: String,
    pub title: String,
    pub status: String,
    pub source_memory_ids: Vec<String>,
    pub problem_pattern: String,
    pub proposed_skill_name: String,
    pub repeatability_evidence: RepeatabilityEvidence,
    pub trigger_design: TriggerDesign,
    pub scope: SkillScope,
    pub privacy: SkillPrivacy,
    pub qa: SkillQa,
    pub owner_gate: SkillOwnerGate,
    // v2 fields
    pub evidence_links: Vec<String>,
    pub rejection_reason: Option<String>,
    pub needs_evidence: bool,
    pub proposed_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RepeatabilityEvidence {
    pub occurrence_count: u64,
    pub synthetic_replay_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct TriggerDesign {
    pub positive_triggers: Vec<String>,
    pub negative_triggers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillScope {
    pub allowed_actions: Vec<String>,
    pub forbidden_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillPrivacy {
    pub contains_secrets: bool,
    pub contains_private_paths: bool,
    pub redaction_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillQa {
    pub required_checks: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillOwnerGate {
    pub required_for_approval: bool,
    pub approved_by: Option<String>,
    pub approved_at: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SkillRegistry {
    pub root: PathBuf,
}

impl SkillRegistry {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    pub fn read_all(&self) -> Result<Vec<SkillCandidate>> {
        read_all(&self.root)
    }

    pub fn propose(&self, title: &str, from_memory: Option<&str>) -> Result<SkillCandidate> {
        reject_sensitive_memory_text(title)?;
        let id = format!("skill-candidate/{}_{}", epoch_millis(), stable_hash(title));
        let source_memory_ids: Vec<String> =
            from_memory.into_iter().map(|s| s.to_string()).collect();
        let now = timestamp();
        let candidate = SkillCandidate {
            schema_version: SKILL_SCHEMA_VERSION_V2.to_string(),
            id: id.clone(),
            title: single_line(title),
            status: "candidate_proposed".to_string(),
            source_memory_ids,
            problem_pattern: "Owner-proposed repeatable workflow candidate".to_string(),
            proposed_skill_name: slugify(title),
            repeatability_evidence: RepeatabilityEvidence::default(),
            trigger_design: TriggerDesign::default(),
            scope: SkillScope::default(),
            privacy: SkillPrivacy::default(),
            qa: SkillQa {
                required_checks: Vec::new(),
                status: "pending".to_string(),
            },
            owner_gate: SkillOwnerGate {
                required_for_approval: true,
                approved_by: None,
                approved_at: None,
            },
            evidence_links: Vec::new(),
            rejection_reason: None,
            needs_evidence: false,
            proposed_at: now.clone(),
            updated_at: now,
        };

        let path = self.root.join(".aiplus/skills/candidates/candidates.jsonl");
        let line = serde_json::to_string(&candidate)?;
        append_jsonl_atomic(&path, &line)?;

        Ok(candidate)
    }

    pub fn reject(&self, id: &str, reason: Option<&str>) -> Result<()> {
        let path = self.root.join(".aiplus/skills/candidates/candidates.jsonl");
        let mut candidates = self.read_all()?;
        let mut found = false;
        for candidate in &mut candidates {
            if candidate.id == id {
                candidate.status = "rejected".to_string();
                candidate.rejection_reason = reason.map(|s| s.to_string());
                candidate.updated_at = timestamp();
                found = true;
            }
        }
        if !found {
            return Err(anyhow::anyhow!(
                "SkillCandidate reject: id={} not_found",
                id
            ));
        }
        rewrite_jsonl_atomic(&path, &candidates)
    }

    pub fn mark_needs_evidence(&self, id: &str) -> Result<()> {
        let path = self.root.join(".aiplus/skills/candidates/candidates.jsonl");
        let mut candidates = self.read_all()?;
        let mut found = false;
        for candidate in &mut candidates {
            if candidate.id == id {
                candidate.needs_evidence = true;
                candidate.updated_at = timestamp();
                found = true;
            }
        }
        if !found {
            return Err(anyhow::anyhow!(
                "SkillCandidate mark_needs_evidence: id={} not_found",
                id
            ));
        }
        rewrite_jsonl_atomic(&path, &candidates)
    }

    pub fn consolidate_from_memory(
        &mut self,
        records: &[MemoryRecord],
    ) -> Result<Vec<SkillCandidate>> {
        let engine = ConsolidationEngine::new(SkillRegistry::new(&self.root));
        let candidates = engine.find_consolidation_candidates(records, &[]);
        let mut created = Vec::new();

        for candidate in candidates {
            if ConsolidationEngine::should_consolidate(records, &candidate.pattern) {
                let skill = engine.auto_consolidate(&self.root, &candidate)?;
                created.push(skill);
            }
        }

        Ok(created)
    }
}

pub fn read_all(root: &Path) -> Result<Vec<SkillCandidate>> {
    let path = root.join(".aiplus/skills/candidates/candidates.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut candidates = Vec::new();
    for line in fs::read_to_string(&path)
        .with_context(|| format!("read skill candidates {}", path.display()))?
        .lines()
        .filter(|line| !line.trim().is_empty())
    {
        let candidate: SkillCandidate = serde_json::from_str(line)
            .with_context(|| format!("parse skill candidate in {}", path.display()))?;
        candidates.push(candidate);
    }
    Ok(candidates)
}

fn append_jsonl_atomic(path: &Path, line: &str) -> Result<()> {
    let _lock = FileLock::acquire(&path.with_extension("lock"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut current = fs::read_to_string(path).unwrap_or_default();
    if !current.is_empty() && !current.ends_with('\n') {
        current.push('\n');
    }
    current.push_str(line);
    current.push('\n');
    write_file_atomic(path, current.as_bytes())
}

fn rewrite_jsonl_atomic<T: Serialize>(path: &Path, rows: &[T]) -> Result<()> {
    let _lock = FileLock::acquire(&path.with_extension("lock"))?;
    let mut body = String::new();
    for row in rows {
        body.push_str(&serde_json::to_string(row)?);
        body.push('\n');
    }
    write_file_atomic(path, body.as_bytes())
}

struct FileLock {
    path: PathBuf,
}

impl FileLock {
    fn acquire(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        for _ in 0..200 {
            match fs::create_dir(path) {
                Ok(()) => {
                    return Ok(Self {
                        path: path.to_path_buf(),
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(error.into()),
            }
        }
        Err(anyhow::anyhow!("lock timeout: {}", path.display()))
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.path);
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn skill_candidate_propose() {
        let tmp = TempDir::new().unwrap();
        let registry = SkillRegistry::new(tmp.path());

        let candidate = registry.propose("Test Skill", None).unwrap();
        assert_eq!(candidate.status, "candidate_proposed");
        assert_eq!(candidate.title, "Test Skill");
        assert!(candidate.id.starts_with("skill-candidate/"));
    }

    #[test]
    fn skill_candidate_reject() {
        let tmp = TempDir::new().unwrap();
        let registry = SkillRegistry::new(tmp.path());

        let candidate = registry.propose("Test Skill", None).unwrap();
        registry.reject(&candidate.id, Some("Not needed")).unwrap();

        let candidates = registry.read_all().unwrap();
        assert_eq!(candidates[0].status, "rejected");
        assert_eq!(
            candidates[0].rejection_reason,
            Some("Not needed".to_string())
        );
    }

    #[test]
    fn skill_candidate_mark_needs_evidence() {
        let tmp = TempDir::new().unwrap();
        let registry = SkillRegistry::new(tmp.path());

        let candidate = registry.propose("Test Skill", None).unwrap();
        registry.mark_needs_evidence(&candidate.id).unwrap();

        let candidates = registry.read_all().unwrap();
        assert!(candidates[0].needs_evidence);
    }

    #[test]
    fn skill_candidate_backward_compat_v1() {
        let json = r#"{"schemaVersion":"0.1.0","id":"sc_1","title":"Old Skill","status":"candidate_proposed","sourceMemoryIds":[],"problemPattern":"test","proposedSkillName":"old-skill","repeatabilityEvidence":{"occurrenceCount":0,"syntheticReplayAvailable":false},"triggerDesign":{"positiveTriggers":[],"negativeTriggers":[]},"scope":{"allowedActions":[],"forbiddenActions":[]},"privacy":{"containsSecrets":false,"containsPrivatePaths":false,"redactionRequired":false},"qa":{"requiredChecks":[],"status":"pending"},"ownerGate":{"requiredForApproval":true,"approvedBy":null,"approvedAt":null}}"#;
        let candidate: SkillCandidate = serde_json::from_str(json).unwrap();
        assert_eq!(candidate.schema_version, "0.1.0");
        assert_eq!(candidate.title, "Old Skill");
        assert!(!candidate.needs_evidence);
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("Test--Value"), "test-value");
    }

    fn sample_memory(id: &str, record_type: &str, summary: &str) -> MemoryRecord {
        MemoryRecord {
            id: id.to_string(),
            record_type: record_type.to_string(),
            summary: summary.to_string(),
            status: "active".to_string(),
            ..MemoryRecord::default()
        }
    }

    #[test]
    fn consolidation_engine_find_candidates_basic() {
        let tmp = TempDir::new().unwrap();
        let registry = SkillRegistry::new(tmp.path());
        let engine = ConsolidationEngine::new(registry);

        let records = vec![
            sample_memory("mem_1", "task", "Run release checklist for deployment"),
            sample_memory("mem_2", "task", "Run release checklist for staging"),
            sample_memory("mem_3", "task", "Run release checklist for production"),
        ];

        let candidates = engine.find_consolidation_candidates(&records, &[]);
        assert!(!candidates.is_empty());
        assert_eq!(candidates[0].occurrence_count, 3);
        assert_eq!(candidates[0].pattern, "release checklist");
    }

    #[test]
    fn consolidation_engine_should_consolidate_threshold() {
        let records = vec![
            sample_memory("mem_1", "task", "Run tests before commit"),
            sample_memory("mem_2", "task", "Run tests before commit"),
            sample_memory("mem_3", "task", "Run tests before commit"),
        ];

        assert!(ConsolidationEngine::should_consolidate(
            &records,
            "Run tests before commit"
        ));

        let two_records = vec![
            sample_memory("mem_1", "task", "Run tests before commit"),
            sample_memory("mem_2", "task", "Run tests before commit"),
        ];

        assert!(!ConsolidationEngine::should_consolidate(
            &two_records,
            "Run tests before commit"
        ));
    }

    #[test]
    fn consolidation_engine_failure_pattern() {
        let mut r1 = sample_memory("mem_1", "task", "Doctor check failed");
        r1.status = "failed".to_string();
        let mut r2 = sample_memory("mem_2", "task", "Doctor check failed again");
        r2.status = "failed".to_string();

        let records = vec![r1, r2];
        assert!(ConsolidationEngine::should_consolidate(
            &records,
            "Doctor check failed"
        ));
    }

    #[test]
    fn consolidation_engine_auto_consolidate() {
        let tmp = TempDir::new().unwrap();
        let registry = SkillRegistry::new(tmp.path());
        let engine = ConsolidationEngine::new(registry);

        let candidate = ConsolidationCandidate {
            pattern: "release checklist".to_string(),
            occurrence_count: 3,
            related_memory_ids: vec!["mem_1".to_string(), "mem_2".to_string()],
            suggested_title: "Release Checklist Skill".to_string(),
            risk_level: RiskLevel::Low,
        };

        let skill = engine.auto_consolidate(tmp.path(), &candidate).unwrap();
        assert_eq!(skill.status, "candidate_proposed");
        assert_eq!(skill.title, "Release Checklist Skill");
        assert!(skill.owner_gate.required_for_approval);
        assert!(skill.scope.allowed_actions.contains(&"read".to_string()));
        assert!(skill
            .scope
            .forbidden_actions
            .contains(&"execute".to_string()));
    }

    #[test]
    fn consolidation_engine_high_risk() {
        let tmp = TempDir::new().unwrap();
        let registry = SkillRegistry::new(tmp.path());
        let engine = ConsolidationEngine::new(registry);

        let candidate = ConsolidationCandidate {
            pattern: "doctor check".to_string(),
            occurrence_count: 2,
            related_memory_ids: vec!["mem_1".to_string()],
            suggested_title: "Doctor Check Skill".to_string(),
            risk_level: RiskLevel::High,
        };

        let skill = engine.auto_consolidate(tmp.path(), &candidate).unwrap();
        assert!(skill.needs_evidence);
        assert!(!skill.scope.allowed_actions.contains(&"write".to_string()));
    }

    #[test]
    fn registry_consolidate_from_memory() {
        let tmp = TempDir::new().unwrap();
        let mut registry = SkillRegistry::new(tmp.path());

        let records = vec![
            sample_memory("mem_1", "workflow", "Compact prepare for release"),
            sample_memory("mem_2", "workflow", "Compact prepare for deploy"),
            sample_memory("mem_3", "workflow", "Compact prepare for staging"),
        ];

        let created = registry.consolidate_from_memory(&records).unwrap();
        assert!(!created.is_empty());

        let all = registry.read_all().unwrap();
        assert!(!all.is_empty());
        assert_eq!(all[0].status, "candidate_proposed");
    }

    #[test]
    fn consolidation_engine_verified_priority() {
        let tmp = TempDir::new().unwrap();
        let registry = SkillRegistry::new(tmp.path());
        let engine = ConsolidationEngine::new(registry);

        let mut r1 = sample_memory("mem_1", "task", "Run review process");
        r1.evidence = vec!["verified".to_string()];
        let r2 = sample_memory("mem_2", "task", "Run review process");
        let r3 = sample_memory("mem_3", "task", "Run review process");

        let records = vec![r1, r2, r3];
        let candidates = engine.find_consolidation_candidates(&records, &[]);
        assert!(!candidates.is_empty());
        assert_eq!(candidates[0].risk_level, RiskLevel::Low);
    }

    #[test]
    fn consolidation_engine_session_patterns() {
        let tmp = TempDir::new().unwrap();
        let registry = SkillRegistry::new(tmp.path());
        let engine = ConsolidationEngine::new(registry);

        let records = vec![
            sample_memory("mem_1", "session", "Build and test workflow"),
            sample_memory("mem_2", "session", "Build and test workflow"),
        ];

        let sessions = vec![
            SessionRecord {
                id: "sess_1".to_string(),
                commands_run: vec!["cargo build".to_string(), "cargo test".to_string()],
                files_changed: vec!["src/lib.rs".to_string()],
                summary: "Build and test workflow".to_string(),
            },
            SessionRecord {
                id: "sess_2".to_string(),
                commands_run: vec!["cargo build".to_string(), "cargo test".to_string()],
                files_changed: vec!["src/main.rs".to_string()],
                summary: "Build and test workflow".to_string(),
            },
        ];

        let candidates = engine.find_consolidation_candidates(&records, &sessions);
        assert!(!candidates.is_empty());
    }

    #[test]
    fn consolidation_engine_ignores_stale() {
        let tmp = TempDir::new().unwrap();
        let registry = SkillRegistry::new(tmp.path());
        let engine = ConsolidationEngine::new(registry);

        let mut r1 = sample_memory("mem_1", "task", "Run tests");
        r1.confidence = "stale".to_string();
        let mut r2 = sample_memory("mem_2", "task", "Run tests");
        r2.confidence = "stale".to_string();
        let mut r3 = sample_memory("mem_3", "task", "Run tests");
        r3.confidence = "stale".to_string();

        let records = vec![r1, r2, r3];
        let candidates = engine.find_consolidation_candidates(&records, &[]);
        assert!(candidates.is_empty());
    }

    #[test]
    fn consolidation_engine_no_auto_activate() {
        let tmp = TempDir::new().unwrap();
        let registry = SkillRegistry::new(tmp.path());
        let engine = ConsolidationEngine::new(registry);

        let candidate = ConsolidationCandidate {
            pattern: "test pattern".to_string(),
            occurrence_count: 5,
            related_memory_ids: vec!["mem_1".to_string()],
            suggested_title: "Test Skill".to_string(),
            risk_level: RiskLevel::Low,
        };

        let skill = engine.auto_consolidate(tmp.path(), &candidate).unwrap();
        assert_ne!(skill.status, "active");
        assert_eq!(skill.status, "candidate_proposed");
    }
}
