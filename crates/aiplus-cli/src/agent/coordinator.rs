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
    pub staffing_roles: Vec<&'static str>,
}

pub fn plan_task(task: &str) -> CoordinatorPlan {
    let score = score_task(task);
    let tier = classify_tier(score.complexity, score.risk, score.requires_code_change);
    let fire_consultant = matches!(tier, CoordinatorTier::Medium | CoordinatorTier::Heavy);
    let staffing_roles = staffing_roles(tier, score.design_impact);
    CoordinatorPlan {
        score,
        tier,
        fire_consultant,
        staffing_roles,
    }
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

pub fn staffing_roles(tier: CoordinatorTier, design_impact: bool) -> Vec<&'static str> {
    match tier {
        CoordinatorTier::LightNoCode => Vec::new(),
        CoordinatorTier::LightCode => vec!["engineer-a"],
        CoordinatorTier::Medium => {
            if design_impact {
                vec!["architect", "engineer-a", "reviewer"]
            } else {
                vec!["engineer-a", "reviewer"]
            }
        }
        CoordinatorTier::Heavy => vec![
            "pm",
            "architect",
            "engineer-a",
            "engineer-b",
            "reviewer",
            "qa",
        ],
    }
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
            vec![
                "pm",
                "architect",
                "engineer-a",
                "engineer-b",
                "reviewer",
                "qa"
            ]
        );
        assert!(plan.fire_consultant);
    }

    #[test]
    fn thresholds_self_check_matches_design() {
        assert!(thresholds_match_design());
    }
}
