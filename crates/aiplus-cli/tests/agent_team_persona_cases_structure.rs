// Track C.1: structural sanity test for the persona-behavior cases.
//
// The full LLM-based behavior suite (`tests/persona_behavior/`) only
// runs in the dedicated `agent-team-persona-behavior` workflow when
// `ANTHROPIC_API_KEY` is set. This test runs in every regular CI cycle
// to catch structural breakage in the cases file without needing API
// credentials — covers things like:
//   - All 8 agent-team core personas have at least one case.
//   - Each persona has all 3 case kinds (in_scope, boundary, stop_gate).
//   - Each case names a prompt + at least one expects_any OR
//     forbids_any assertion (so no silent always-pass cases).
//   - Each persona name in test_cases.toml maps to a persona file
//     that actually exists under
//     `assets/aiplus-agent-team/core/templates/personas/`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

#[derive(serde::Deserialize)]
struct Doc {
    cases: Vec<Case>,
}

#[derive(serde::Deserialize)]
struct Case {
    persona: String,
    kind: String,
    prompt: String,
    #[serde(default)]
    expects_any: Vec<String>,
    #[serde(default)]
    forbids_any: Vec<String>,
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root")
        .to_path_buf()
}

#[test]
fn all_eight_core_personas_have_cases() {
    let root = repo_root();
    let cases_path = root.join("tests/persona_behavior/test_cases.toml");
    let body = fs::read_to_string(&cases_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", cases_path.display()));
    let doc: Doc = toml::from_str(&body).unwrap_or_else(|e| panic!("parse cases: {e}"));

    let by_persona: BTreeMap<&str, Vec<&Case>> =
        doc.cases.iter().fold(BTreeMap::new(), |mut acc, c| {
            acc.entry(c.persona.as_str()).or_default().push(c);
            acc
        });

    let expected_personas = [
        "advisor",
        "ceo",
        "architect",
        "pm",
        "engineer-a",
        "engineer-b",
        "reviewer",
        "qa",
    ];
    for p in expected_personas {
        assert!(
            by_persona.contains_key(p),
            "no test cases for persona {p:?} — expected coverage of all 8 agent-team core roles"
        );
    }
}

#[test]
fn each_persona_has_three_case_kinds() {
    let root = repo_root();
    let body = fs::read_to_string(root.join("tests/persona_behavior/test_cases.toml")).unwrap();
    let doc: Doc = toml::from_str(&body).unwrap();
    let mut by_persona: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for c in &doc.cases {
        by_persona
            .entry(c.persona.clone())
            .or_default()
            .insert(c.kind.clone());
    }
    let expected_kinds = ["in_scope", "boundary", "stop_gate"];
    for (persona, kinds) in &by_persona {
        for k in expected_kinds {
            assert!(
                kinds.contains(k),
                "persona {persona:?} missing case kind {k:?}; have {kinds:?}"
            );
        }
    }
}

#[test]
fn every_case_has_a_prompt_and_at_least_one_assertion() {
    let root = repo_root();
    let body = fs::read_to_string(root.join("tests/persona_behavior/test_cases.toml")).unwrap();
    let doc: Doc = toml::from_str(&body).unwrap();
    for c in &doc.cases {
        assert!(
            !c.prompt.trim().is_empty(),
            "case {persona}/{kind} has empty prompt",
            persona = c.persona,
            kind = c.kind
        );
        assert!(
            !c.expects_any.is_empty() || !c.forbids_any.is_empty(),
            "case {persona}/{kind} has no expects_any or forbids_any — would silently always pass",
            persona = c.persona,
            kind = c.kind
        );
    }
}

#[test]
fn every_persona_in_cases_has_a_corresponding_persona_file() {
    let root = repo_root();
    let body = fs::read_to_string(root.join("tests/persona_behavior/test_cases.toml")).unwrap();
    let doc: Doc = toml::from_str(&body).unwrap();
    let personas_dir = root.join("assets/aiplus-agent-team/core/templates/personas");
    let referenced: BTreeSet<&str> = doc.cases.iter().map(|c| c.persona.as_str()).collect();
    for p in referenced {
        let f = personas_dir.join(format!("{p}.md"));
        assert!(
            f.exists(),
            "test case references persona {p:?} but {} does not exist",
            f.display()
        );
    }
}

#[test]
fn stop_gate_cases_assert_an_owner_gate_or_escalation_word() {
    // Defensive: a stop_gate case that doesn't expect any escalation
    // language is probably miscategorized. Catch it before the LLM
    // suite does (and burns API credits proving the obvious).
    let root = repo_root();
    let body = fs::read_to_string(root.join("tests/persona_behavior/test_cases.toml")).unwrap();
    let doc: Doc = toml::from_str(&body).unwrap();
    let escalation_signals = [
        "owner",
        "escalate",
        "gate",
        "stop",
        "production",
        "force-push",
        "secret rotation",
        "schema migration",
        "external api",
        "main",
    ];
    for c in doc.cases.iter().filter(|c| c.kind == "stop_gate") {
        let touches_signal = c.expects_any.iter().any(|e| {
            escalation_signals
                .iter()
                .any(|s| e.to_lowercase().contains(s))
        });
        assert!(
            touches_signal,
            "stop_gate case for {persona:?} has no escalation/gate signal in expects_any: {expects:?}",
            persona = c.persona,
            expects = c.expects_any,
        );
    }
}
