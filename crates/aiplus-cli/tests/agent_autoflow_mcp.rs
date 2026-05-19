use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn start_mcp(
    project_root: &std::path::Path,
    home: &std::path::Path,
) -> (Child, ChildStdin, BufReader<std::process::ChildStdout>) {
    let cache = home.join(".cache");
    let mut child = Command::new(bin())
        .arg("mcp-serve")
        .current_dir(project_root)
        .env("HOME", home)
        .env("XDG_CACHE_HOME", &cache)
        .env(
            "AIPLUS_TOKEN_COST_PRICING_URL",
            "file:///nonexistent-pricing.json",
        )
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn aiplus mcp-serve");
    let stdin = child.stdin.take().expect("child stdin");
    let stdout = BufReader::new(child.stdout.take().expect("child stdout"));
    (child, stdin, stdout)
}

fn rpc(
    stdin: &mut ChildStdin,
    stdout: &mut BufReader<std::process::ChildStdout>,
    message: Value,
) -> Value {
    writeln!(stdin, "{message}").expect("write json-rpc line");
    stdin.flush().expect("flush json-rpc line");
    let mut line = String::new();
    stdout.read_line(&mut line).expect("read json-rpc response");
    serde_json::from_str(&line).unwrap_or_else(|error| panic!("parse response {line:?}: {error}"))
}

fn tool_text(response: &Value) -> Value {
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .expect("tool response text");
    serde_json::from_str(text)
        .unwrap_or_else(|error| panic!("tool text was not JSON {text:?}: {error}"))
}

#[test]
fn agent_autoflow_mcp_tools_list_and_call_live() {
    let project = tempfile::tempdir().expect("project tempdir");
    let home = tempfile::tempdir().expect("home tempdir");
    let (mut child, mut stdin, mut stdout) = start_mcp(project.path(), home.path());

    let initialize = rpc(
        &mut stdin,
        &mut stdout,
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
    );
    assert_eq!(initialize["result"]["serverInfo"]["name"], "aiplus-mcp");

    let list = rpc(
        &mut stdin,
        &mut stdout,
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
    );
    let names: Vec<&str> = list["result"]["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert!(names.contains(&"agent_token_cost"), "{names:?}");
    assert!(names.contains(&"agent_audit_verify_log"), "{names:?}");
    assert!(names.contains(&"agent_route_score_only"), "{names:?}");

    let audit = rpc(
        &mut stdin,
        &mut stdout,
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"agent_audit_verify_log","arguments":{}}}),
    );
    assert_eq!(audit["result"]["isError"], false);
    let audit_json = tool_text(&audit);
    assert_eq!(audit_json["verdict"], "PASS");

    let score = rpc(
        &mut stdin,
        &mut stdout,
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"agent_route_score_only","arguments":{"task":"implement a small docs update"}}}),
    );
    assert_eq!(score["result"]["isError"], false);
    let score_json = tool_text(&score);
    assert!(score_json["complexity"].as_u64().is_some(), "{score_json}");
    assert!(score_json["risk"].as_f64().is_some(), "{score_json}");
    assert!(score_json["staffing_roles"].is_array(), "{score_json}");

    let token_cost = rpc(
        &mut stdin,
        &mut stdout,
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"agent_token_cost","arguments":{"window":"1h","by_role":true,"top_n":1}}}),
    );
    assert_eq!(token_cost["result"]["isError"], false);
    let token_json = tool_text(&token_cost);
    assert_eq!(token_json["windows"][0]["window"], "1h");
    assert!(
        token_json["windows"][0]["top_tasks"].is_array(),
        "{token_json}"
    );

    let invalid = rpc(
        &mut stdin,
        &mut stdout,
        json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"agent_route_score_only","arguments":{}}}),
    );
    assert_eq!(invalid["result"]["isError"], true);
    let error_text = invalid["result"]["content"][0]["text"].as_str().unwrap();
    assert!(error_text.contains("task"), "{error_text}");

    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
}
