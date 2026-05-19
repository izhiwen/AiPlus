use aiplus_token_cost::pricing::{PricingLoadOptions, PricingTable};
use aiplus_token_cost::{format_report, run_token_cost, TokenCostOptions};
use chrono::{Duration, Utc};
use std::fs;

#[test]
fn token_cost_rolls_up_dispatch_log_and_honors_override() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let agents = root.join(".aiplus/agents");
    fs::create_dir_all(&agents).unwrap();
    fs::write(
        root.join(".aiplus/pricing.toml"),
        r#"
[[price]]
provider = "anthropic"
model = "claude-sonnet-4-6"
input_usd_per_token = 1.0
output_usd_per_token = 2.0
"#,
    )
    .unwrap();
    let line = serde_json::json!({
        "timestamp": Utc::now().to_rfc3339(),
        "dispatchId": "dispatch-expensive",
        "role": "engineer-a",
        "task": "implement payment",
        "provider": "anthropic",
        "model": "claude-sonnet-4-6",
        "usage_tokens": {"input_tokens": 2, "output_tokens": 3}
    });
    fs::write(agents.join("dispatch-log.jsonl"), format!("{line}\n")).unwrap();

    let report = run_token_cost(
        root,
        &TokenCostOptions {
            by_role: true,
            window: Some("1h".to_string()),
            top_n: 5,
        },
    )
    .unwrap();
    assert_eq!(report.windows[0].total_tokens, 5);
    assert_eq!(report.windows[0].total_usd, 8.0);
    assert!(report.snapshot_written);

    let output = format_report(&report, true);
    assert!(output.contains("WINDOW 1h total_tokens=5 total_usd=8.000000"));
    assert!(output.contains("BY_ROLE"));
    assert!(output.contains("engineer-a tokens=5"));
}

#[test]
fn embedded_fallback_loads_with_fetch_disabled_and_null_usage_is_zero() {
    let temp = tempfile::tempdir().unwrap();
    let pricing = PricingTable::load_with_options(
        temp.path(),
        PricingLoadOptions {
            fetch_enabled: false,
            cache_dir: Some(temp.path().join("cache")),
            pricing_url: None,
        },
    );
    assert!(pricing.lookup("openai", "gpt-5").is_some());

    let agents = temp.path().join(".aiplus/agents");
    fs::create_dir_all(&agents).unwrap();
    let line = serde_json::json!({
        "timestamp": Utc::now().to_rfc3339(),
        "decisionId": "coord-null",
        "event": "coordinator_decision",
        "taskExcerpt": "score only",
        "usage_tokens": null
    });
    fs::write(agents.join("dispatch-log.jsonl"), format!("{line}\n")).unwrap();
    let report = run_token_cost(temp.path(), &TokenCostOptions::default()).unwrap();
    assert_eq!(report.windows[0].total_tokens, 0);
    assert_eq!(report.windows[0].total_usd, 0.0);
}

#[test]
fn window_filter_excludes_old_rows() {
    let temp = tempfile::tempdir().unwrap();
    let agents = temp.path().join(".aiplus/agents");
    fs::create_dir_all(&agents).unwrap();
    let old = serde_json::json!({
        "timestamp": (Utc::now() - Duration::hours(2)).to_rfc3339(),
        "dispatchId": "dispatch-old",
        "role": "engineer-a",
        "provider": "openai",
        "model": "gpt-5",
        "usage_tokens": {"input_tokens": 10, "output_tokens": 10}
    });
    fs::write(agents.join("dispatch-log.jsonl"), format!("{old}\n")).unwrap();

    let report = run_token_cost(
        temp.path(),
        &TokenCostOptions {
            by_role: false,
            window: Some("1h".to_string()),
            top_n: 5,
        },
    )
    .unwrap();
    assert_eq!(report.windows.len(), 1);
    assert_eq!(report.windows[0].total_tokens, 0);
}
