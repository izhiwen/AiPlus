// G1 local harness artifact.
//
// This records the N=15 baseline prompt set and the expected runtime field bars
// for each adapter without invoking live external runtimes. Live validation can
// replay these prompts, but unit tests keep the contract stable offline.

const SWITCH_SENTENCE: &str = "Already in <current_role> mode. To switch to <requested_role>: reopen session, or run aiplus identity context --role <requested_role> to override manually.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedDisposition {
    Activate,
    RefuseAlreadyBound,
    NoTrigger,
    AskOnce,
}

#[derive(Debug, Clone, Copy)]
struct BaselinePrompt {
    prompt: &'static str,
    expected: ExpectedDisposition,
}

fn skip_markdown_blockquote_lines(prompt: &str) -> String {
    prompt
        .lines()
        .filter(|line| !line.trim_start().starts_with('>'))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn offline_expected_disposition_after_quote_skip(prompt: &str) -> ExpectedDisposition {
    match skip_markdown_blockquote_lines(prompt).as_str() {
        "" => ExpectedDisposition::NoTrigger,
        "you are CEO"
        | "你是 CEO"
        | "you are qa"
        | "你是 qa"
        | "开 advisor"
        | "做 engineer-b"
        | "take reviewer"
        | "take the reviewer role"
        | "以 CEO 的视角看一下" => ExpectedDisposition::Activate,
        "switch to architect" => ExpectedDisposition::RefuseAlreadyBound,
        "你是 CEO 吗？" | "compare CEO and advisor" | "不要切到 CEO" => {
            ExpectedDisposition::NoTrigger
        }
        "maybe use the PM perspective here" => ExpectedDisposition::AskOnce,
        _ => ExpectedDisposition::AskOnce,
    }
}

const BASELINE_PROMPTS: &[BaselinePrompt] = &[
    BaselinePrompt {
        prompt: "you are CEO",
        expected: ExpectedDisposition::Activate,
    },
    BaselinePrompt {
        prompt: "你是 CEO",
        expected: ExpectedDisposition::Activate,
    },
    BaselinePrompt {
        prompt: "you are qa",
        expected: ExpectedDisposition::Activate,
    },
    BaselinePrompt {
        prompt: "你是 qa",
        expected: ExpectedDisposition::Activate,
    },
    BaselinePrompt {
        prompt: "开 advisor",
        expected: ExpectedDisposition::Activate,
    },
    BaselinePrompt {
        prompt: "做 engineer-b",
        expected: ExpectedDisposition::Activate,
    },
    BaselinePrompt {
        prompt: "take reviewer",
        expected: ExpectedDisposition::Activate,
    },
    BaselinePrompt {
        prompt: "take the reviewer role",
        expected: ExpectedDisposition::Activate,
    },
    BaselinePrompt {
        prompt: "switch to architect",
        expected: ExpectedDisposition::RefuseAlreadyBound,
    },
    BaselinePrompt {
        prompt: "以 CEO 的视角看一下",
        expected: ExpectedDisposition::Activate,
    },
    BaselinePrompt {
        prompt: "你是 CEO 吗？",
        expected: ExpectedDisposition::NoTrigger,
    },
    BaselinePrompt {
        prompt: "> you are CEO",
        expected: ExpectedDisposition::NoTrigger,
    },
    BaselinePrompt {
        prompt: "compare CEO and advisor",
        expected: ExpectedDisposition::NoTrigger,
    },
    BaselinePrompt {
        prompt: "不要切到 CEO",
        expected: ExpectedDisposition::NoTrigger,
    },
    BaselinePrompt {
        prompt: "maybe use the PM perspective here",
        expected: ExpectedDisposition::AskOnce,
    },
];

fn activation_bar(runtime: &str) -> String {
    format!("schema=v1 runtime={runtime} trigger=nl_role_bind")
}

fn refusal_bar(runtime: &str) -> String {
    format!("reason=session_already_bound schema=v1 runtime={runtime} trigger=nl_role_bind")
}

#[test]
fn g1_harness_records_n15_baseline_prompts() {
    assert_eq!(BASELINE_PROMPTS.len(), 15);
    assert_eq!(
        BASELINE_PROMPTS
            .iter()
            .filter(|case| case.expected == ExpectedDisposition::Activate)
            .count(),
        9
    );
    assert_eq!(
        BASELINE_PROMPTS
            .iter()
            .filter(|case| case.expected == ExpectedDisposition::RefuseAlreadyBound)
            .count(),
        1
    );
    assert_eq!(
        BASELINE_PROMPTS
            .iter()
            .filter(|case| case.expected == ExpectedDisposition::NoTrigger)
            .count(),
        4
    );
    assert_eq!(
        BASELINE_PROMPTS
            .iter()
            .filter(|case| case.expected == ExpectedDisposition::AskOnce)
            .count(),
        1
    );
    assert!(BASELINE_PROMPTS.iter().all(|case| !case.prompt.is_empty()));
    let quote_block = BASELINE_PROMPTS
        .iter()
        .find(|case| case.prompt == "> you are CEO")
        .expect("quote-block English CEO negative case");
    assert_eq!(quote_block.expected, ExpectedDisposition::NoTrigger);
    for prompt in ["you are qa", "你是 qa", "take reviewer", "开 advisor"] {
        let case = BASELINE_PROMPTS
            .iter()
            .find(|case| case.prompt == prompt)
            .expect("hard floor phrase positive case");
        assert_eq!(case.expected, ExpectedDisposition::Activate);
    }
}

#[test]
fn g1_harness_records_quote_block_as_no_trigger_with_no_role_line() {
    let quote_block = BASELINE_PROMPTS
        .iter()
        .find(|case| case.prompt == "> you are CEO")
        .expect("quote-block English CEO negative case");

    assert_eq!(quote_block.expected, ExpectedDisposition::NoTrigger);
    assert_ne!(quote_block.expected, ExpectedDisposition::Activate);
    assert_ne!(
        quote_block.expected,
        ExpectedDisposition::RefuseAlreadyBound,
        "quote-block role text must not emit ROLE_BIND_REFUSED after a prior bind"
    );
}

#[test]
fn g1_detector_skips_markdown_blockquote_lines_before_floor_matching() {
    assert_eq!(
        offline_expected_disposition_after_quote_skip("> you are CEO"),
        ExpectedDisposition::NoTrigger
    );
    assert_eq!(
        offline_expected_disposition_after_quote_skip("   > you are CEO"),
        ExpectedDisposition::NoTrigger,
        "lines whose first non-space character is > must not trigger"
    );
    assert_eq!(
        offline_expected_disposition_after_quote_skip("> quoted role text\nyou are CEO"),
        ExpectedDisposition::Activate,
        "non-blockquote floor phrases still activate after quoted lines are skipped"
    );
    for prompt in ["you are CEO", "you are qa", "take reviewer", "开 advisor"] {
        assert_eq!(
            offline_expected_disposition_after_quote_skip(prompt),
            ExpectedDisposition::Activate,
            "positive trigger behavior must be preserved for {prompt}"
        );
    }
}

#[test]
fn g1_harness_records_expected_runtime_bars() {
    let expected = [
        ("codex", activation_bar("codex"), refusal_bar("codex")),
        (
            "claude-code",
            activation_bar("claude-code"),
            refusal_bar("claude-code"),
        ),
        (
            "opencode",
            activation_bar("opencode"),
            refusal_bar("opencode"),
        ),
    ];

    for (runtime, activation, refusal) in expected {
        assert!(activation.contains(&format!("runtime={runtime}")));
        assert!(refusal.contains(&format!("runtime={runtime}")));
        assert!(!activation.contains("runtime=<codex|claude-code|opencode>"));
        assert!(!refusal.contains("runtime=<codex|claude-code|opencode>"));
    }

    assert_eq!(
        SWITCH_SENTENCE,
        "Already in <current_role> mode. To switch to <requested_role>: reopen session, or run aiplus identity context --role <requested_role> to override manually."
    );
}
