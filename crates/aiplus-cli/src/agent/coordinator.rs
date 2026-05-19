#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinatorTier {
    LightNoCode,
    LightCode,
    Medium,
    Heavy,
}

impl CoordinatorTier {
    pub fn as_str(self) -> &'static str {
        match self {
            CoordinatorTier::LightNoCode => "LIGHT_NO_CODE",
            CoordinatorTier::LightCode => "LIGHT_CODE",
            CoordinatorTier::Medium => "MEDIUM",
            CoordinatorTier::Heavy => "HEAVY",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoordinatorScore {
    pub complexity: u8,
    pub risk: f32,
    pub requires_code_change: bool,
    pub design_impact: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoordinatorPlan {
    pub score: CoordinatorScore,
    pub tier: CoordinatorTier,
    pub fire_consultant: bool,
    pub staffing_roles: Vec<String>,
    pub forced_by_risk: Vec<String>,
    pub auto_summoned: Vec<String>,
}

pub fn plan_task(task: &str) -> CoordinatorPlan {
    let score = score_task(task);
    let tier = classify_tier(score.complexity, score.risk, score.requires_code_change);
    let fire_consultant = matches!(tier, CoordinatorTier::Medium | CoordinatorTier::Heavy);
    let mut staffing_roles = staffing_roles(tier, score.design_impact);
    let forced_by_risk = apply_risk_forcing(&mut staffing_roles, score.risk);
    CoordinatorPlan {
        score,
        tier,
        fire_consultant,
        staffing_roles,
        forced_by_risk,
        auto_summoned: Vec::new(),
    }
}

pub fn plan_task_for_project(project_root: &Path, task: &str) -> Result<CoordinatorPlan> {
    let mut plan = plan_task(task);
    apply_auto_summon(project_root, task, &mut plan)?;
    Ok(plan)
}

pub fn score_task(task: &str) -> CoordinatorScore {
    let normalized = task.to_ascii_lowercase();
    let requires_code_change = contains_any(
        &normalized,
        &[
            "implement",
            "build",
            "add ",
            "create",
            "fix",
            "patch",
            "refactor",
            "rewrite",
            "update",
            "delete",
            "remove",
            "migrate",
            "wire",
            "integrate",
            "endpoint",
            "api",
            "code",
            "test",
            "实现",
            "新增",
            "添加",
            "修复",
            "改",
            "重构",
            "删除",
            "迁移",
            "集成",
            "接口",
        ],
    );
    let design_impact = contains_any(
        &normalized,
        &[
            "architecture",
            "design",
            "schema",
            "migration",
            "migrate",
            "cross-module",
            "cross module",
            "database",
            "payment",
            "auth",
            "api",
            "架构",
            "设计",
            "模式",
            "迁移",
            "数据库",
            "支付",
            "认证",
            "权限",
            "接口",
        ],
    );

    let risk = if contains_any(
        &normalized,
        &[
            "payment", "billing", "charge", "refund", "pci", "支付", "账单", "扣款", "退款",
        ],
    ) {
        0.85
    } else if contains_any(
        &normalized,
        &[
            "auth",
            "permission",
            "security",
            "secret",
            "privacy",
            "compliance",
            "key",
            "token",
            "认证",
            "权限",
            "安全",
            "密钥",
            "隐私",
            "合规",
        ],
    ) {
        0.75
    } else if contains_any(
        &normalized,
        &[
            "deploy",
            "release",
            "production",
            "prod",
            "rollback",
            "migration",
            "database",
            "data loss",
            "发布",
            "部署",
            "生产",
            "回滚",
            "迁移",
            "数据库",
            "数据丢失",
        ],
    ) {
        0.65
    } else if design_impact {
        0.55
    } else if requires_code_change {
        0.35
    } else {
        0.15
    };

    let complexity = if risk >= 0.80
        || contains_any(
            &normalized,
            &[
                "payment",
                "billing",
                "auth",
                "security",
                "production",
                "deploy",
                "release",
                "支付",
                "账单",
                "认证",
                "安全",
                "生产",
                "部署",
                "发布",
            ],
        ) {
        5
    } else if contains_any(
        &normalized,
        &[
            "cross-module",
            "cross module",
            "migration",
            "database",
            "architecture",
            "integration",
            "integrate",
            "api",
            "endpoint",
            "重构",
            "迁移",
            "数据库",
            "架构",
            "集成",
            "接口",
        ],
    ) {
        4
    } else if contains_any(
        &normalized,
        &[
            "implement",
            "build",
            "refactor",
            "rewrite",
            "tests",
            "workflow",
            "实现",
            "构建",
            "重写",
            "测试",
            "流程",
        ],
    ) {
        3
    } else if requires_code_change {
        2
    } else {
        1
    };

    CoordinatorScore {
        complexity,
        risk,
        requires_code_change,
        design_impact,
    }
}

pub fn classify_tier(complexity: u8, risk: f32, requires_code_change: bool) -> CoordinatorTier {
    if complexity >= 5 || risk >= 0.70 {
        CoordinatorTier::Heavy
    } else if (3..=4).contains(&complexity) {
        CoordinatorTier::Medium
    } else if requires_code_change {
        CoordinatorTier::LightCode
    } else {
        CoordinatorTier::LightNoCode
    }
}

pub fn staffing_roles(tier: CoordinatorTier, design_impact: bool) -> Vec<String> {
    match tier {
        CoordinatorTier::LightNoCode => Vec::new(),
        CoordinatorTier::LightCode => str_vec(&["engineer-a"]),
        CoordinatorTier::Medium => {
            if design_impact {
                str_vec(&["architect", "engineer-a", "reviewer"])
            } else {
                str_vec(&["engineer-a", "reviewer"])
            }
        }
        CoordinatorTier::Heavy => str_vec(&[
            "pm",
            "architect",
            "engineer-a",
            "engineer-b",
            "reviewer",
            "qa",
        ]),
    }
}

pub fn apply_risk_forcing(staffing_roles: &mut Vec<String>, risk: f32) -> Vec<String> {
    let mut forced = Vec::new();
    if risk >= 0.70 {
        forced.push("reviewer".to_string());
        if !contains_role(staffing_roles, "reviewer") {
            staffing_roles.push("reviewer".to_string());
        }
    }
    if risk >= 0.85 {
        forced.push("qa".to_string());
        if !contains_role(staffing_roles, "qa") {
            staffing_roles.push("qa".to_string());
        }
    }
    forced
}

fn apply_auto_summon(project_root: &Path, task: &str, plan: &mut CoordinatorPlan) -> Result<()> {
    let state = load_team_config(project_root)?;
    let cap = cluster_cap(plan.tier);
    let mut candidates = Vec::new();

    for config in state.agents.values() {
        let Some(autosummon) = config.autosummon.as_ref() else {
            continue;
        };
        let intent_hint = autosummon.intent_hint.trim();
        if intent_hint.is_empty()
            || config.stub
            || contains_role(&plan.staffing_roles, &config.role)
        {
            continue;
        }

        if expert_intent_match(task, intent_hint) {
            candidates.push((autosummon.priority, config.role.clone()));
        }
    }

    candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    for (_, role) in candidates {
        if plan.staffing_roles.len() >= cap {
            break;
        }
        if contains_role(&plan.staffing_roles, &role) {
            continue;
        }
        plan.staffing_roles.push(role.clone());
        plan.auto_summoned.push(role);
    }
    Ok(())
}

fn expert_intent_match(task: &str, intent_hint: &str) -> bool {
    let key = intent_cache_key(task, intent_hint);
    if let Some(value) = intent_cache_get(&key) {
        return value;
    }

    let value = classify_intent_match(task, intent_hint).unwrap_or(false);
    intent_cache_put(key, value);
    value
}

fn classify_intent_match(task: &str, intent_hint: &str) -> Option<bool> {
    if env::var("AIPLUS_AUTOSUMMON_INTENT_MOCK").ok().as_deref() == Some("1") {
        return Some(mock_intent_match(task, intent_hint));
    }

    let api_key = env::var("ANTHROPIC_API_KEY")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())?;
    let model = env::var("AIPLUS_AUTOSUMMON_INTENT_MODEL")
        .unwrap_or_else(|_| "claude-haiku-4-5-20251001".to_string());
    let prompt = intent_prompt(task, intent_hint);
    let body = serde_json::json!({
        "model": model,
        "max_tokens": 4,
        "temperature": 0,
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ]
    });
    let output = Command::new("curl")
        .args([
            "-fsS",
            "https://api.anthropic.com/v1/messages",
            "-H",
            "content-type: application/json",
            "-H",
            "anthropic-version: 2023-06-01",
            "-H",
            &format!("x-api-key: {api_key}"),
            "-d",
            &body.to_string(),
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let response: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let answer = response
        .get("content")
        .and_then(serde_json::Value::as_array)?
        .iter()
        .find_map(|part| part.get("text").and_then(serde_json::Value::as_str))?
        .trim()
        .to_ascii_uppercase();
    if answer.starts_with("YES") {
        Some(true)
    } else if answer.starts_with("NO") {
        Some(false)
    } else {
        None
    }
}

fn intent_prompt(task: &str, intent_hint: &str) -> String {
    format!(
        "You are classifying whether a software task matches an intent description.\n\nTask: \"{}\"\nIntent: \"{}\"\n\nDoes this task match this intent? Reply with a single word: YES or NO.",
        task.replace('"', "\\\""),
        intent_hint.replace('"', "\\\"")
    )
}

fn mock_intent_match(task: &str, intent_hint: &str) -> bool {
    let task = task.to_ascii_lowercase();
    let intent = intent_hint.to_ascii_lowercase();
    if intent.contains("credentials") || intent.contains("安全") {
        return contains_any(
            &task,
            &[
                "payment",
                "billing",
                "auth",
                "security",
                "secure",
                "secret",
                "token",
                "credential",
                "privacy",
                "vulnerability",
                "csrf",
                "xss",
                "encryption",
                "支付",
                "认证",
                "敏感",
                "凭据",
                "密钥",
                "安全",
                "隐私",
            ],
        );
    }
    if intent.contains("readme") || intent.contains("文档") {
        return contains_any(
            &task,
            &[
                "docs",
                "documentation",
                "readme",
                "guide",
                "manual",
                "onboarding",
                "api docs",
                "release notes",
                "tutorial",
                "文档",
                "说明",
                "指南",
                "教程",
            ],
        );
    }
    if intent.contains("llm") || intent.contains("大模型") {
        return contains_any(
            &task,
            &[
                "llm",
                "ai",
                "model",
                "prompt",
                "rag",
                "embedding",
                "openai",
                "anthropic",
                "claude",
                "codex",
                "agent",
                "token budget",
                "大模型",
                "模型",
                "提示词",
                "智能体",
            ],
        );
    }
    false
}

fn intent_cache_key(task: &str, intent_hint: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(task.as_bytes());
    hasher.update(b"\n");
    hasher.update(intent_hint.as_bytes());
    hex::encode(hasher.finalize())
}

fn intent_cache_get(key: &str) -> Option<bool> {
    let mut cache = intent_cache().lock().ok()?;
    let value = cache.entries.get(key).copied();
    if value.is_some() {
        cache.hits += 1;
    }
    value
}

fn intent_cache_put(key: String, value: bool) {
    let Ok(mut cache) = intent_cache().lock() else {
        return;
    };
    if !cache.entries.contains_key(&key) {
        if cache.order.len() >= INTENT_CACHE_CAP {
            if let Some(oldest) = cache.order.pop_front() {
                cache.entries.remove(&oldest);
            }
        }
        cache.order.push_back(key.clone());
    }
    cache.entries.insert(key, value);
    cache.misses += 1;
}

fn intent_cache() -> &'static Mutex<IntentCache> {
    INTENT_CACHE.get_or_init(|| Mutex::new(IntentCache::default()))
}

#[derive(Debug, Default)]
struct IntentCache {
    entries: HashMap<String, bool>,
    order: VecDeque<String>,
    hits: usize,
    misses: usize,
}

const INTENT_CACHE_CAP: usize = 1000;
static INTENT_CACHE: OnceLock<Mutex<IntentCache>> = OnceLock::new();

#[cfg(test)]
fn reset_autosummon_intent_cache_for_tests() {
    if let Ok(mut cache) = intent_cache().lock() {
        *cache = IntentCache::default();
    }
}

#[cfg(test)]
fn autosummon_intent_cache_metrics_for_tests() -> (usize, usize, usize) {
    let cache = intent_cache().lock().expect("intent cache lock");
    (cache.entries.len(), cache.hits, cache.misses)
}

fn cluster_cap(tier: CoordinatorTier) -> usize {
    match tier {
        CoordinatorTier::LightNoCode => 2,
        CoordinatorTier::LightCode => 3,
        CoordinatorTier::Medium => 5,
        CoordinatorTier::Heavy => 8,
    }
}

fn contains_role(roles: &[String], role: &str) -> bool {
    roles.iter().any(|candidate| candidate == role)
}

fn str_vec(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

pub fn thresholds_match_design() -> bool {
    classify_tier(2, 0.69, true) == CoordinatorTier::LightCode
        && classify_tier(3, 0.69, true) == CoordinatorTier::Medium
        && classify_tier(4, 0.69, true) == CoordinatorTier::Medium
        && classify_tier(5, 0.69, true) == CoordinatorTier::Heavy
        && classify_tier(4, 0.69, true) == CoordinatorTier::Medium
        && classify_tier(4, 0.70, true) == CoordinatorTier::Heavy
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complexity_boundaries_classify_per_design() {
        assert_eq!(classify_tier(2, 0.69, true), CoordinatorTier::LightCode);
        assert_eq!(classify_tier(3, 0.69, true), CoordinatorTier::Medium);
        assert_eq!(classify_tier(4, 0.69, true), CoordinatorTier::Medium);
        assert_eq!(classify_tier(5, 0.69, true), CoordinatorTier::Heavy);
    }

    #[test]
    fn risk_boundary_is_inclusive_at_point_seven() {
        assert_eq!(classify_tier(4, 0.69, true), CoordinatorTier::Medium);
        assert_eq!(classify_tier(4, 0.70, true), CoordinatorTier::Heavy);
    }

    #[test]
    fn d5_payment_task_scores_heavy() {
        let plan = plan_task("实现支付接口");
        assert_eq!(plan.score.complexity, 5);
        assert!((0.7..=0.9).contains(&plan.score.risk));
        assert_eq!(plan.tier, CoordinatorTier::Heavy);
        assert_eq!(
            plan.staffing_roles,
            str_vec(&[
                "pm",
                "architect",
                "engineer-a",
                "engineer-b",
                "reviewer",
                "qa"
            ])
        );
        assert!(plan.fire_consultant);
    }

    #[test]
    fn thresholds_self_check_matches_design() {
        assert!(thresholds_match_design());
    }

    #[test]
    fn risk_forcing_records_threshold_roles_and_dedupes_staffing() {
        let mut light = str_vec(&["engineer-a"]);
        let forced = apply_risk_forcing(&mut light, 0.85);
        assert_eq!(light, str_vec(&["engineer-a", "reviewer", "qa"]));
        assert_eq!(forced, str_vec(&["reviewer", "qa"]));

        let mut medium = str_vec(&["engineer-a", "reviewer"]);
        let forced = apply_risk_forcing(&mut medium, 0.85);
        assert_eq!(medium, str_vec(&["engineer-a", "reviewer", "qa"]));
        assert_eq!(forced, str_vec(&["reviewer", "qa"]));

        let mut heavy = str_vec(&[
            "pm",
            "architect",
            "engineer-a",
            "engineer-b",
            "reviewer",
            "qa",
        ]);
        let forced = apply_risk_forcing(&mut heavy, 0.85);
        assert_eq!(forced, str_vec(&["reviewer", "qa"]));
    }

    #[test]
    fn risk_forcing_does_not_fire_below_boundary() {
        let mut light = str_vec(&["engineer-a"]);
        let forced = apply_risk_forcing(&mut light, 0.50);
        assert_eq!(light, str_vec(&["engineer-a"]));
        assert!(forced.is_empty());
    }

    #[test]
    fn intent_match_cache_hits_on_repeat_task_and_intent() {
        reset_autosummon_intent_cache_for_tests();
        std::env::set_var("AIPLUS_AUTOSUMMON_INTENT_MOCK", "1");

        let task = "实现支付接口";
        let intent = "支付、认证、敏感数据、credentials、凭据、安全漏洞或隐私相关的软件工作";
        assert!(expert_intent_match(task, intent));
        assert!(expert_intent_match(task, intent));

        let (entries, hits, misses) = autosummon_intent_cache_metrics_for_tests();
        assert_eq!(entries, 1);
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
        std::env::remove_var("AIPLUS_AUTOSUMMON_INTENT_MOCK");
    }
}
use crate::agent::core::load_team_config;
use anyhow::Result;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::env;
use std::path::Path;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
