use crate::error::CoreError;

pub fn sensitive_findings(text: &str) -> Vec<(&'static str, bool)> {
    let lower = text.to_ascii_lowercase();
    vec![
        (
            "authorization header",
            lower.contains("authorization: bearer") || lower.contains("authorization: basic"),
        ),
        (
            "placeholder secret",
            text.contains("PENDING_OWNER_INPUT_DO_NOT_USE") || text.contains("BWS_ACCESS_TOKEN"),
        ),
        (
            "private key",
            text.contains("-----BEGIN ") && text.contains("PRIVATE KEY-----"),
        ),
        ("jwt", text.split_whitespace().any(is_jwt_like)),
        ("cookie", lower.contains("cookie:") && text.contains('=')),
        (
            "private path",
            text.contains("/Users/")
                || text.contains("/home/")
                || lower.contains("dropbox/")
                || text.contains("iCloud"),
        ),
        ("email pii", text.contains('@') && text.contains('.')),
        ("phone pii", has_phone_like(text)),
        (
            "raw audio/transcript payload",
            lower.contains("begin transcript")
                || lower.contains("webvtt")
                || lower.contains("provider request body")
                || lower.contains("provider response body")
                || lower.contains("raw transcript"),
        ),
        (
            "har/webrtc dump",
            lower.contains(".har") || lower.contains(".webrtcdump"),
        ),
        ("api key", has_secret_assignment(&lower)),
        ("password assignment", has_password_assignment(&lower)),
        ("raw chat transcript", has_chat_transcript(&lower)),
        (
            "secret assignment",
            (lower.contains("secret=") || lower.contains("secret:"))
                && !has_secret_assignment(&lower),
        ),
    ]
}

pub fn reject_sensitive_memory_text(text: &str) -> Result<(), CoreError> {
    let findings: Vec<&str> = sensitive_findings(text)
        .into_iter()
        .filter_map(|(label, found)| found.then_some(label))
        .collect();
    if !findings.is_empty() {
        return Err(CoreError::new(format!(
            "MEMORY_REDACTION_STATUS=BLOCKED reason=sensitive_pattern labels=[{}]",
            findings.join(",")
        )));
    }
    Ok(())
}

pub fn is_jwt_like(token: &str) -> bool {
    let parts: Vec<&str> = token
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '.' && ch != '_' && ch != '-')
        .split('.')
        .collect();
    parts.len() == 3 && parts[0].starts_with("eyJ")
}

pub fn has_phone_like(text: &str) -> bool {
    text.split_whitespace().any(|token| {
        if token.contains("unix-") {
            return false;
        }
        let candidate = token.trim_matches(|ch: char| {
            !(ch.is_ascii_digit() || matches!(ch, '+' | '-' | '(' | ')' | '.'))
        });
        // Reject candidates that still contain letters (e.g., JSON fragments like
        // `-local","contentHash":"hash:833454a6c08f432c`)
        if candidate.chars().any(|ch| ch.is_ascii_alphabetic()) {
            return false;
        }
        let digits = candidate.chars().filter(|ch| ch.is_ascii_digit()).count();
        let has_phone_separator = candidate.starts_with('+')
            || candidate.contains('-')
            || candidate.contains('(')
            || candidate.contains(')');
        has_phone_separator && (10..=15).contains(&digits)
    })
}

pub fn has_secret_assignment(lower: &str) -> bool {
    [
        "api_key",
        "apikey",
        "api-key",
        "secret_key",
        "secret-key",
        "access_token",
        "access-token",
    ]
    .iter()
    .any(|needle| lower.contains(needle) && (lower.contains('=') || lower.contains(':')))
}

fn has_password_assignment(lower: &str) -> bool {
    for key in ["password", "passwd", "pwd"] {
        if let Some(pos) = lower.find(key) {
            let after = &lower[pos + key.len()..];
            let after_trimmed = after.trim_start();
            if after_trimmed.starts_with('=') || after_trimmed.starts_with(':') {
                return true;
            }
        }
    }
    false
}

fn has_chat_transcript(lower: &str) -> bool {
    (lower.contains("user:") && (lower.contains("assistant:") || lower.contains("agent:")))
        || (lower.contains("human:") && lower.contains("ai:"))
        || lower.contains("\nassistant:")
        || lower.starts_with("assistant:")
        || lower.lines().any(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("user: ")
                || trimmed.starts_with("assistant: ")
                || trimmed.starts_with("human: ")
                || trimmed.starts_with("ai: ")
        })
        || has_qa_format(lower)
}

fn has_qa_format(lower: &str) -> bool {
    // Pattern 1: Q: ... A: ... (with colon)
    let has_q_colon = lower.contains("q:") || lower.contains("q.");
    let has_a_colon = lower.contains("a:") || lower.contains("a.");
    let pattern_1 = has_q_colon && has_a_colon;

    // Pattern 2: Question: ... Answer: ...
    let pattern_2 = lower.contains("question:") && lower.contains("answer:");

    // Pattern 3: User question: ... Assistant answer: ...
    let pattern_3 = lower.contains("user question:") && lower.contains("assistant answer:");

    // Pattern 4: Line-by-line Q/A at start of lines
    let lines: Vec<&str> = lower.lines().collect();
    let has_q_line = lines.iter().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("q ") || trimmed.starts_with("q:") || trimmed.starts_with("q.")
    });
    let has_a_line = lines.iter().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("a ") || trimmed.starts_with("a:") || trimmed.starts_with("a.")
    });
    let pattern_4 = has_q_line && has_a_line;

    // Pattern 5: Question/Answer as standalone words on separate lines
    let has_question_line = lines.iter().any(|line| {
        let trimmed = line.trim();
        trimmed == "question" || trimmed.starts_with("question ")
    });
    let has_answer_line = lines.iter().any(|line| {
        let trimmed = line.trim();
        trimmed == "answer" || trimmed.starts_with("answer ")
    });
    let pattern_5 = has_question_line && has_answer_line;

    pattern_1 || pattern_2 || pattern_3 || pattern_4 || pattern_5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwt_detection() {
        assert!(is_jwt_like("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c"));
        assert!(!is_jwt_like("not.a.jwt"));
        assert!(!is_jwt_like("hello"));
    }

    #[test]
    fn phone_detection() {
        assert!(has_phone_like("Call me at +1-234-567-8901"));
        assert!(has_phone_like("My number is (555)123-4567"));
        assert!(!has_phone_like("unix-timestamp-12345"));
        assert!(!has_phone_like("no phone here"));
    }

    #[test]
    fn secret_assignment_detection() {
        assert!(has_secret_assignment("api_key=secret123"));
        assert!(has_secret_assignment("api-key: secret123"));
        assert!(!has_secret_assignment("api key is not assigned"));
    }

    #[test]
    fn sensitive_findings_detection() {
        let findings = sensitive_findings("authorization: bearer token123");
        assert!(findings
            .iter()
            .any(|(label, found)| *label == "authorization header" && *found));

        let findings = sensitive_findings("-----BEGIN RSA PRIVATE KEY-----");
        assert!(findings
            .iter()
            .any(|(label, found)| *label == "private key" && *found));
    }

    #[test]
    fn reject_sensitive_blocks_jwt() {
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        assert!(reject_sensitive_memory_text(jwt).is_err());
    }

    #[test]
    fn reject_sensitive_allows_safe() {
        assert!(reject_sensitive_memory_text("This is a safe memory text.").is_ok());
    }

    #[test]
    fn password_assignment_detection() {
        assert!(has_password_assignment("password=secret123"));
        assert!(has_password_assignment("password: secret123"));
        assert!(has_password_assignment("passwd=secret123"));
        assert!(has_password_assignment("pwd: secret123"));
        assert!(has_password_assignment("my password = secret"));
        assert!(!has_password_assignment(
            "this is about passwords in general"
        ));
    }

    #[test]
    fn chat_transcript_detection() {
        assert!(has_chat_transcript("user: hello\nassistant: hi there"));
        assert!(has_chat_transcript("human: question\nai: answer"));
        assert!(has_chat_transcript("assistant: here is the code"));
        assert!(has_chat_transcript("  user: hello\n  assistant: hi"));
        assert!(!has_chat_transcript("no chat here"));
    }

    #[test]
    fn sensitive_findings_blocks_password_and_transcript() {
        let findings = sensitive_findings("password=secret123");
        assert!(findings
            .iter()
            .any(|(label, found)| *label == "password assignment" && *found));

        let findings = sensitive_findings("password: secret123");
        assert!(findings
            .iter()
            .any(|(label, found)| *label == "password assignment" && *found));

        let findings = sensitive_findings("User: Hello\nAssistant: Hi there");
        assert!(findings
            .iter()
            .any(|(label, found)| *label == "raw chat transcript" && *found));

        let findings = sensitive_findings("Human: question\nAI: answer");
        assert!(findings
            .iter()
            .any(|(label, found)| *label == "raw chat transcript" && *found));

        let findings = sensitive_findings("Assistant: Here is the code");
        assert!(findings
            .iter()
            .any(|(label, found)| *label == "raw chat transcript" && *found));
    }

    #[test]
    fn qa_format_detection() {
        assert!(has_qa_format("q: what is the password? a: supersecret123"));
        assert!(has_qa_format(
            "question: what is the capital? answer: paris"
        ));
        assert!(has_qa_format(
            "user question: how do i deploy? assistant answer: use cargo run"
        ));
        // Q. / A. variant
        assert!(has_qa_format("q. what is the password? a. supersecret123"));
        // Line-by-line Q/A
        assert!(has_qa_format("q what is the password\na supersecret123"));
        assert!(has_qa_format("  q: hello\n  a: world"));
        // Question / Answer as standalone lines
        assert!(has_qa_format(
            "question what is the password\nanswer supersecret123"
        ));
        assert!(has_qa_format(
            "question\nwhat is the password\nanswer\nsupersecret123"
        ));
        // Negative cases
        assert!(!has_qa_format("no question or answer here"));
        assert!(!has_qa_format("a is the first letter"));
        assert!(!has_qa_format("quality is important"));
    }

    #[test]
    fn reject_sensitive_blocks_qa_format() {
        assert!(
            reject_sensitive_memory_text("Q: What is the password? A: SuperSecret123").is_err()
        );
        assert!(reject_sensitive_memory_text("Question: What? Answer: Nothing").is_err());
        assert!(reject_sensitive_memory_text("User question: How? Assistant answer: Yes").is_err());
        // Q. / A. variant
        assert!(
            reject_sensitive_memory_text("Q. What is the password? A. SuperSecret123").is_err()
        );
        // Line-by-line Q/A
        assert!(
            reject_sensitive_memory_text("Q: What is the password?\nA: SuperSecret123").is_err()
        );
    }

    #[test]
    fn reject_sensitive_blocks_raw_transcript() {
        assert!(reject_sensitive_memory_text("raw transcript of conversation").is_err());
        assert!(reject_sensitive_memory_text("begin transcript").is_err());
        assert!(reject_sensitive_memory_text("provider response body").is_err());
    }
}
