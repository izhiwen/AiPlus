//! Phase E: `aiplus mcp-serve` — minimal stdio-based MCP server exposing
//! AiPlus agent operations as MCP tools that codex / claude-code / opencode
//! invoke directly during conversations.
//!
//! Wire protocol: JSON-RPC 2.0 over stdin/stdout, one message per line. The
//! supported methods are the three MCP-required ones for tool-only servers:
//!
//! - `initialize` → capabilities + serverInfo
//! - `tools/list` → tool registry
//! - `tools/call` → invoke a tool by name with arguments
//!
//! Notifications (`notifications/initialized`, etc.) are accepted and
//! ignored. The server is intentionally minimal — anything more would be a
//! future-Phase concern.
//!
//! Why MCP and not a custom protocol: codex, claude-code, and opencode all
//! already implement MCP client logic. Choosing MCP means the PI can call
//! `agent_route` as a real tool with no copy-paste, no fenced bash, no "run
//! that" — the LLM invokes a function and gets a structured result.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

const PROTOCOL_VERSION: &str = "2024-11-05";
const SERVER_NAME: &str = "aiplus-mcp";

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

pub fn run_server() -> Result<()> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    let project_root = std::env::current_dir()?;

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("aiplus-mcp: read stdin error: {e}");
                return Err(e.into());
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("aiplus-mcp: parse error: {e} on line: {line}");
                write_response(
                    &mut stdout,
                    error_response(Value::Null, -32700, &format!("parse error: {e}")),
                )?;
                continue;
            }
        };

        if request.id.is_none() {
            continue;
        }
        let id = request.id.clone().unwrap();

        let response = match request.method.as_str() {
            "initialize" => handle_initialize(id),
            "tools/list" => handle_tools_list(id),
            "tools/call" => handle_tools_call(id, request.params, &project_root),
            "resources/list" => empty_array_response(id, "resources"),
            "prompts/list" => empty_array_response(id, "prompts"),
            other => error_response(id, -32601, &format!("method not implemented: {other}")),
        };
        write_response(&mut stdout, response)?;
    }

    Ok(())
}

fn write_response(stdout: &mut impl Write, response: JsonRpcResponse) -> Result<()> {
    let serialized = serde_json::to_string(&response)?;
    writeln!(stdout, "{serialized}")?;
    stdout.flush()?;
    Ok(())
}

fn ok_response(id: Value, result: Value) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(result),
        error: None,
    }
}

fn error_response(id: Value, code: i64, message: &str) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0",
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message: message.to_string(),
            data: None,
        }),
    }
}

fn empty_array_response(id: Value, key: &str) -> JsonRpcResponse {
    ok_response(id, json!({ key: [] }))
}

fn handle_initialize(id: Value) -> JsonRpcResponse {
    ok_response(
        id,
        json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": SERVER_NAME,
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
}

fn handle_tools_list(id: Value) -> JsonRpcResponse {
    ok_response(id, json!({ "tools": tool_definitions() }))
}

fn tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "agent_route",
            "description": "Dispatch a task to a role on the active AiPlus virtual team. Writes an entry to .aiplus/agents/dispatch-log.jsonl, marks the role active in .aiplus/agents/active-roles.json, and provisions a git worktree if the role's config calls for one. Use this whenever the PI/CEO would say 'I'm dispatching <role> to do <task>' — call this tool instead, and the dispatch becomes a real persistent artifact.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "role": {
                        "type": "string",
                        "description": "Role slug (e.g. ra-stata, ra-python, theorist, referee, replicator for AEL; engineer-a, architect, qa for agent-team)."
                    },
                    "task": {
                        "type": "string",
                        "description": "Concrete task description. The route command will surface a HEAVY/MEDIUM/LIGHT tier badge based on keyword heuristics (submit / structural / robustness / etc.)."
                    }
                },
                "required": ["role", "task"]
            }
        }),
        json!({
            "name": "agent_status",
            "description": "Return the current state of the AiPlus virtual team in this project: which team is active (agent-team / aieconlab), which roles have been dispatched, total agents available, and worktree provisioning state. Use this to read project state before deciding a dispatch.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "agent_set_team",
            "description": "Switch which virtual team is active in this project (agent-team for software engineering, aieconlab for applied-economics research). Both team snapshots are preserved under .aiplus/agents/_teams/; switching is a file-copy from snapshot to active layout, no re-install.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "team": {
                        "type": "string",
                        "enum": ["agent-team", "aieconlab"],
                        "description": "Target team."
                    }
                },
                "required": ["team"]
            }
        }),
        json!({
            "name": "agent_list",
            "description": "List every role configured on the active virtual team, including stub roles (placeholders) and disabled roles. Use this when you need the complete roster before deciding who to invite or dispatch.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "functional": {
                        "type": "boolean",
                        "description": "If true, list only functional (non-stub, non-disabled) roles. Default false."
                    }
                }
            }
        }),
        json!({
            "name": "agent_doctor",
            "description": "Run health checks specific to the agent layer: team config integrity, persona files present, runtime adapter mirroring up to date. Use this when something looks off before deciding to escalate to the Owner.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "agent_invite",
            "description": "Mark a stub / dormant role as 'invited' so it can receive dispatches. Use when bringing an expert (e.g. job-talk-coach, structural-modeler) into the team for a specific scope. Logged to audit.jsonl as an Owner-visible event.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "role": {
                        "type": "string",
                        "description": "Role slug to invite (typically a stub or v0.2 expert)."
                    }
                },
                "required": ["role"]
            }
        }),
        json!({
            "name": "agent_dismiss",
            "description": "Reverse an invite — return an expert role to dormant state. Use when the scope they were invited for is done. Logged to audit.jsonl.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "role": {
                        "type": "string",
                        "description": "Role slug to dismiss."
                    }
                },
                "required": ["role"]
            }
        }),
        json!({
            "name": "agent_disable",
            "description": "Disable a role so it cannot be dispatched. Different from dismiss — disable is for problem cases (consistently underperforming, identity unclear, identification concerns) and the role stays disabled across sessions until explicitly enabled.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "role": {
                        "type": "string",
                        "description": "Role slug to disable."
                    }
                },
                "required": ["role"]
            }
        }),
        json!({
            "name": "agent_enable",
            "description": "Re-enable a previously disabled role. Inverse of agent_disable.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "role": {
                        "type": "string",
                        "description": "Role slug to enable."
                    }
                },
                "required": ["role"]
            }
        }),
        json!({
            "name": "agent_integrate",
            "description": "Merge a role's worktree branch (agent/<role>) back into the project's main branch and prune the worktree. Use after a role reports their dispatched task is done AND the work has passed Replicator / Referee gates.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "role": {
                        "type": "string",
                        "description": "Role slug whose branch should be integrated."
                    }
                },
                "required": ["role"]
            }
        }),
        json!({
            "name": "agent_talk",
            "description": "Surface the shell command for opening an interactive runtime session focused on a specific role's persona. Note: this tool does NOT spawn a nested runtime (that would conflict with the current MCP host session); it returns the command the Owner should run in a separate terminal to enter a role-focused session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "role": {
                        "type": "string",
                        "description": "Role slug to surface a talk command for."
                    }
                },
                "required": ["role"]
            }
        }),
    ]
}

fn handle_tools_call(id: Value, params: Value, project_root: &std::path::Path) -> JsonRpcResponse {
    let Some(name) = params.get("name").and_then(|v| v.as_str()) else {
        return error_response(id, -32602, "missing 'name' in tool call");
    };
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    let result = match name {
        "agent_route" => call_agent_route(&args, project_root),
        "agent_status" => call_agent_status(project_root),
        "agent_set_team" => call_agent_set_team(&args, project_root),
        "agent_list" => call_agent_list(&args, project_root),
        "agent_doctor" => call_agent_doctor(project_root),
        "agent_invite" => call_agent_single_role(&args, "invite", project_root),
        "agent_dismiss" => call_agent_single_role(&args, "dismiss", project_root),
        "agent_disable" => call_agent_single_role(&args, "disable", project_root),
        "agent_enable" => call_agent_single_role(&args, "enable", project_root),
        "agent_integrate" => call_agent_single_role(&args, "integrate", project_root),
        "agent_talk" => call_agent_talk(&args, project_root),
        other => Err(anyhow!("unknown tool: {other}")),
    };
    match result {
        Ok(text) => ok_response(
            id,
            json!({
                "content": [{ "type": "text", "text": text }],
                "isError": false
            }),
        ),
        Err(err) => ok_response(
            id,
            json!({
                "content": [{ "type": "text", "text": format!("ERROR: {err}") }],
                "isError": true
            }),
        ),
    }
}

fn call_agent_route(args: &Value, project_root: &std::path::Path) -> Result<String> {
    let role = args
        .get("role")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("agent_route requires 'role' (string)"))?;
    let task = args
        .get("task")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("agent_route requires 'task' (string)"))?;
    let self_exe = std::env::current_exe()?;
    let output = std::process::Command::new(self_exe)
        .args(["agent", "route", role, task])
        .current_dir(project_root)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() {
        return Err(anyhow!(
            "aiplus agent route exited non-zero:\nstdout:\n{stdout}\nstderr:\n{stderr}"
        ));
    }
    Ok(if stderr.trim().is_empty() {
        stdout
    } else {
        format!("{stdout}\n--- stderr ---\n{stderr}")
    })
}

fn call_agent_status(project_root: &std::path::Path) -> Result<String> {
    let self_exe = std::env::current_exe()?;
    let output = std::process::Command::new(self_exe)
        .args(["agent", "status"])
        .current_dir(project_root)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if !output.status.success() {
        return Err(anyhow!(
            "aiplus agent status exited non-zero: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(stdout)
}

fn call_agent_set_team(args: &Value, project_root: &std::path::Path) -> Result<String> {
    let team = args
        .get("team")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("agent_set_team requires 'team' (string)"))?;
    run_cli(project_root, &["agent", "set-team", team])
}

fn call_agent_list(args: &Value, project_root: &std::path::Path) -> Result<String> {
    let functional = args
        .get("functional")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if functional {
        run_cli(project_root, &["agent", "list", "--functional"])
    } else {
        run_cli(project_root, &["agent", "list"])
    }
}

fn call_agent_doctor(project_root: &std::path::Path) -> Result<String> {
    run_cli(project_root, &["agent", "doctor"])
}

/// Shared helper for agent subcommands that take a single `role` arg and
/// no other options: invite / dismiss / disable / enable / integrate.
fn call_agent_single_role(
    args: &Value,
    subcommand: &str,
    project_root: &std::path::Path,
) -> Result<String> {
    let role = args
        .get("role")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("agent_{subcommand} requires 'role' (string)"))?;
    run_cli(project_root, &["agent", subcommand, role])
}

/// agent_talk is intentionally NOT a shell-out — `aiplus agent talk <role>`
/// spawns the runtime CLI (codex/claude/opencode) interactively, which would
/// conflict with the MCP host's own runtime session. Return the command the
/// Owner should run in a separate terminal instead.
fn call_agent_talk(args: &Value, _project_root: &std::path::Path) -> Result<String> {
    let role = args
        .get("role")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("agent_talk requires 'role' (string)"))?;
    Ok(format!(
        "agent_talk does not spawn a nested runtime session (would conflict with \
         the current MCP host). To enter a role-focused session, the Owner should \
         run in a separate terminal:\n\n  aiplus agent talk {role}\n"
    ))
}

/// Shell out to the same aiplus binary, in the project root, with the given
/// args. Centralises the success/failure plumbing so each call_agent_X stays
/// a 2-line wrapper.
fn run_cli(project_root: &std::path::Path, args: &[&str]) -> Result<String> {
    let self_exe = std::env::current_exe()?;
    let output = std::process::Command::new(self_exe)
        .args(args)
        .current_dir(project_root)
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() {
        return Err(anyhow!(
            "aiplus {} exited non-zero:\nstdout:\n{stdout}\nstderr:\n{stderr}",
            args.join(" ")
        ));
    }
    Ok(if stderr.trim().is_empty() {
        stdout
    } else {
        format!("{stdout}\n--- stderr ---\n{stderr}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_returns_protocol_and_server_info() {
        let resp = handle_initialize(json!(1));
        let result = resp.result.expect("initialize must return result");
        assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(result["serverInfo"]["name"], SERVER_NAME);
        assert!(result["serverInfo"]["version"].is_string());
        assert!(result["capabilities"]["tools"].is_object());
    }

    #[test]
    fn tools_list_advertises_eleven_tools_with_required_fields() {
        let resp = handle_tools_list(json!(1));
        let result = resp.result.expect("tools/list must return result");
        let tools = result["tools"].as_array().expect("tools is array");
        assert_eq!(
            tools.len(),
            11,
            "expected 3 original + 8 new agent tools = 11"
        );
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        // Original 3.
        assert!(names.contains(&"agent_route"));
        assert!(names.contains(&"agent_status"));
        assert!(names.contains(&"agent_set_team"));
        // New 8 (P0.3 of v0.5.11 close-out).
        assert!(names.contains(&"agent_list"));
        assert!(names.contains(&"agent_doctor"));
        assert!(names.contains(&"agent_invite"));
        assert!(names.contains(&"agent_dismiss"));
        assert!(names.contains(&"agent_disable"));
        assert!(names.contains(&"agent_enable"));
        assert!(names.contains(&"agent_integrate"));
        assert!(names.contains(&"agent_talk"));
        for tool in tools {
            assert!(tool["description"].is_string());
            assert!(tool["inputSchema"]["type"].as_str() == Some("object"));
        }
    }

    #[test]
    fn agent_route_required_args_validated() {
        let tmp = std::env::temp_dir();
        let err = call_agent_route(&json!({}), &tmp).unwrap_err();
        assert!(err.to_string().contains("role"));
        let err = call_agent_route(&json!({"role": "ra-stata"}), &tmp).unwrap_err();
        assert!(err.to_string().contains("task"));
    }

    #[test]
    fn agent_single_role_tools_validate_role_arg() {
        let tmp = std::env::temp_dir();
        for sub in ["invite", "dismiss", "disable", "enable", "integrate"] {
            let err = call_agent_single_role(&json!({}), sub, &tmp).unwrap_err();
            assert!(
                err.to_string().contains("role"),
                "expected 'role' validation error for {sub}, got: {err}"
            );
        }
    }

    #[test]
    fn agent_talk_validates_role_and_does_not_spawn_subprocess() {
        let tmp = std::env::temp_dir();
        // Missing role → error.
        let err = call_agent_talk(&json!({}), &tmp).unwrap_err();
        assert!(err.to_string().contains("role"));
        // With role → returns instructional text instead of shelling out.
        let out = call_agent_talk(&json!({"role": "ra-stata"}), &tmp).unwrap();
        assert!(out.contains("aiplus agent talk ra-stata"));
        assert!(
            out.contains("separate terminal") || out.contains("MCP host"),
            "agent_talk should explain why it doesn't spawn nested runtime: {out}"
        );
    }

    #[test]
    fn agent_list_functional_flag_recognized() {
        // We can't actually run the underlying CLI from a unit test (no
        // .aiplus directory), but we can verify the args path is taken via
        // the public dispatch — handle_tools_call returns isError=true if
        // the shell-out fails, which it will here because no project is
        // initialized. What we're checking: the arg parsing doesn't panic
        // and the response shape is well-formed.
        let resp = handle_tools_call(
            json!(1),
            json!({"name": "agent_list", "arguments": {"functional": true}}),
            &std::env::temp_dir(),
        );
        let result = resp.result.expect("tools/call always returns result");
        // Either ok or isError — but always a structured envelope.
        assert!(result["content"].is_array());
    }

    #[test]
    fn agent_set_team_required_args_validated() {
        let tmp = std::env::temp_dir();
        let err = call_agent_set_team(&json!({}), &tmp).unwrap_err();
        assert!(err.to_string().contains("team"));
    }

    #[test]
    fn unknown_tool_returns_is_error_envelope() {
        let resp = handle_tools_call(
            json!(1),
            json!({"name": "nonexistent"}),
            &std::env::temp_dir(),
        );
        let result = resp
            .result
            .expect("tools/call always returns result envelope");
        assert_eq!(result["isError"], json!(true));
        let text = result["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("unknown tool"));
    }

    #[test]
    fn tools_call_missing_name_returns_error() {
        let resp = handle_tools_call(json!(1), json!({}), &std::env::temp_dir());
        assert!(resp.error.is_some());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32602);
    }
}
