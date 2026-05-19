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
            "description": "PREFERRED programmatic surface for dispatching a task to a role. Use this MCP tool instead of `aiplus agent route <role> \"<task>\"` CLI when the user asks to assign work. Writes dispatch history, active-role state, and any configured worktree artifacts.",
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
            "description": "PREFERRED programmatic surface for team status queries. Use this MCP tool instead of `aiplus agent status` CLI when the user asks about team configuration, active roles, or current state. Returns structured JSON for active team, role state, and worktrees.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "agent_set_team",
            "description": "PREFERRED programmatic surface for switching active teams. Use this MCP tool instead of `aiplus agent set-team <name>` CLI when the user asks to switch to agent-team or AiEconLab. Preserves team snapshots and returns structured JSON.",
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
            "description": "PREFERRED programmatic surface for role roster queries. Use this MCP tool instead of `aiplus agent list` CLI when the user asks who is available, invited, disabled, or functional. Returns structured JSON for active-team roles.",
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
            "description": "PREFERRED programmatic surface for agent health checks. Use this MCP tool instead of `aiplus agent doctor` CLI when the user asks whether the agent layer, personas, runtime mirrors, or catalogs are healthy. Returns structured JSON diagnostics.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "agent_audit_verify_log",
            "description": "PREFERRED programmatic surface for log integrity. Use this MCP tool instead of `aiplus agent audit verify-log` CLI when answering audit/tamper queries; MCP returns structured JSON. Verifies dispatch-log hash chain and reports PASS/FAIL with first bad line.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "agent_route_score_only",
            "description": "PREFERRED programmatic surface for planning. User says \"implement X\" -> call this with task=\"implement X\" before answering from training data. Use instead of `aiplus agent route --score-only`; MCP returns structured JSON.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "task": {
                        "type": "string",
                        "description": "Task description in natural language."
                    }
                },
                "required": ["task"]
            }
        }),
        json!({
            "name": "agent_token_cost",
            "description": "PREFERRED programmatic surface for token cost/spend. Use this MCP tool instead of `aiplus agent dispatch-history` or token-cost CLI when answering cost queries; MCP returns structured JSON for 1h/8h/24h windows, per-role, top tasks.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "window": {
                        "type": "string",
                        "enum": ["1h", "8h", "24h"],
                        "description": "Single window to restrict to. Omit to get all three."
                    },
                    "by_role": {
                        "type": "boolean",
                        "description": "Include per-role breakdown. Default false."
                    },
                    "top_n": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 50,
                        "description": "Top-N most expensive tasks per window. Default 5."
                    }
                },
                "required": []
            }
        }),
        json!({
            "name": "agent_invite",
            "description": "PREFERRED programmatic surface for inviting a role into the active team. Use this MCP tool instead of `aiplus agent invite <role>` CLI when the user asks to bring in an expert or dormant role. Logs an Owner-visible event.",
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
            "description": "PREFERRED programmatic surface for dismissing a role from the active team. Use this MCP tool instead of `aiplus agent dismiss <role>` CLI when the user asks to remove an invited expert after scope completion. Logs the event.",
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
            "description": "PREFERRED programmatic surface for disabling a role. Use this MCP tool instead of `aiplus agent disable <role>` CLI when the user asks to prevent dispatch to a problematic role. Persists until explicitly re-enabled.",
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
            "description": "PREFERRED programmatic surface for re-enabling a disabled role. Use this MCP tool instead of `aiplus agent enable <role>` CLI when the user asks to restore a role for dispatch. Returns structured JSON status.",
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
            "description": "PREFERRED programmatic surface for integrating a role worktree. Use this MCP tool instead of `aiplus agent integrate <role>` CLI when the user asks to merge a completed role branch back to main and prune its worktree.",
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
            "description": "PREFERRED programmatic surface for role-focused conversation setup. Use this MCP tool instead of `aiplus agent talk <role>` CLI when the user asks to talk with one role. Returns the command to run in a separate terminal.",
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
        "agent_audit_verify_log" => call_agent_audit_verify_log(project_root),
        "agent_route_score_only" => call_agent_route_score_only(&args, project_root),
        "agent_token_cost" => call_agent_token_cost(&args, project_root),
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

fn call_agent_audit_verify_log(project_root: &std::path::Path) -> Result<String> {
    let (stdout, stderr, success) =
        run_cli_capture(project_root, &["agent", "audit", "verify-log"])?;
    if let Some(value) = parse_audit_verify_log_output(&stdout)? {
        return Ok(value.to_string());
    }
    if success {
        Err(anyhow!(
            "aiplus agent audit verify-log output missing VERIFY_LOG line:\n{stdout}"
        ))
    } else {
        Err(anyhow!(
            "aiplus agent audit verify-log exited non-zero:\nstdout:\n{stdout}\nstderr:\n{stderr}"
        ))
    }
}

fn call_agent_route_score_only(args: &Value, project_root: &std::path::Path) -> Result<String> {
    let task = args
        .get("task")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("agent_route_score_only requires 'task' (non-empty string)"))?;
    let output = run_cli(project_root, &["agent", "route", "--score-only", task])?;
    Ok(parse_route_score_only_output(&output)?.to_string())
}

fn call_agent_token_cost(args: &Value, project_root: &std::path::Path) -> Result<String> {
    let mut command_args = vec![
        "agent".to_string(),
        "token-cost".to_string(),
        "--top-n".to_string(),
    ];
    let top_n = match args.get("top_n").and_then(|v| v.as_u64()) {
        Some(value @ 1..=50) => value,
        Some(_) => return Err(anyhow!("agent_token_cost 'top_n' must be between 1 and 50")),
        None => 5,
    };
    command_args.push(top_n.to_string());

    if let Some(window) = args.get("window").and_then(|v| v.as_str()) {
        if !["1h", "8h", "24h"].contains(&window) {
            return Err(anyhow!(
                "agent_token_cost 'window' must be one of: 1h, 8h, 24h"
            ));
        }
        command_args.push("--window".to_string());
        command_args.push(window.to_string());
    }

    let by_role = args
        .get("by_role")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if by_role {
        command_args.push("--by-role".to_string());
    }

    let output = run_cli_owned(project_root, &command_args)?;
    Ok(parse_token_cost_output(&output)?.to_string())
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
    let (stdout, stderr, success) = run_cli_capture(project_root, args)?;
    if !success {
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

fn run_cli_owned(project_root: &std::path::Path, args: &[String]) -> Result<String> {
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

fn run_cli_capture(
    project_root: &std::path::Path,
    args: &[&str],
) -> Result<(String, String, bool)> {
    let self_exe = std::env::current_exe()?;
    let output = std::process::Command::new(self_exe)
        .args(args)
        .current_dir(project_root)
        .output()?;
    Ok((
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.success(),
    ))
}

fn parse_audit_verify_log_output(output: &str) -> Result<Option<Value>> {
    let Some(line) = output.lines().find(|line| line.starts_with("VERIFY_LOG=")) else {
        return Ok(None);
    };
    if let Some(rest) = line.strip_prefix("VERIFY_LOG=PASS") {
        let checked_lines = find_key_value(rest, "checked_lines")
            .unwrap_or("0")
            .parse::<usize>()?;
        return Ok(Some(json!({
            "verdict": "PASS",
            "checked_lines": checked_lines,
            "first_bad_line": Value::Null,
            "reason": Value::Null
        })));
    }
    if let Some(rest) = line.strip_prefix("VERIFY_LOG=FAIL") {
        let line_number = find_key_value(rest, "line")
            .ok_or_else(|| anyhow!("VERIFY_LOG=FAIL missing line=N"))?
            .parse::<usize>()?;
        let reason = rest
            .split_once(" reason=")
            .map(|(_, value)| value.trim())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("VERIFY_LOG=FAIL missing reason"))?;
        return Ok(Some(json!({
            "verdict": "FAIL",
            "checked_lines": Value::Null,
            "first_bad_line": line_number,
            "reason": reason
        })));
    }
    Err(anyhow!("unsupported VERIFY_LOG line: {line}"))
}

fn parse_route_score_only_output(output: &str) -> Result<Value> {
    let line = output
        .lines()
        .find(|line| line.starts_with("Adaptive coordinator:"))
        .ok_or_else(|| anyhow!("score-only output missing Adaptive coordinator line"))?;
    let mut complexity = None;
    let mut risk = None;
    let mut tier = None;
    let mut code_change = None;
    let mut design_impact = None;
    let mut consultant = None;
    for part in line.split_whitespace() {
        if let Some(value) = part.strip_prefix("complexity=") {
            complexity = Some(value.parse::<u8>()?);
        } else if let Some(value) = part.strip_prefix("risk=") {
            risk = Some(value.parse::<f64>()?);
        } else if let Some(value) = part.strip_prefix("tier=") {
            tier = Some(value.to_string());
        } else if let Some(value) = part.strip_prefix("code_change=") {
            code_change = Some(parse_bool_text(value)?);
        } else if let Some(value) = part.strip_prefix("design_impact=") {
            design_impact = Some(parse_bool_text(value)?);
        } else if let Some(value) = part.strip_prefix("consultant=") {
            consultant = Some(value.to_string());
        }
    }
    let staffing_roles = output
        .lines()
        .find_map(|line| parse_bracket_list_line(line, "Would staff: "))
        .unwrap_or_default();
    let forced_by_risk = output
        .lines()
        .find_map(|line| parse_bracket_list_line(line, "Forced by risk: "))
        .unwrap_or_default();
    let auto_summoned = output
        .lines()
        .find_map(|line| parse_bracket_list_line(line, "Auto-summoned experts: "))
        .unwrap_or_default();

    Ok(json!({
        "complexity": complexity.ok_or_else(|| anyhow!("score-only output missing complexity"))?,
        "risk": risk.ok_or_else(|| anyhow!("score-only output missing risk"))?,
        "tier": tier.ok_or_else(|| anyhow!("score-only output missing tier"))?,
        "code_change": code_change.ok_or_else(|| anyhow!("score-only output missing code_change"))?,
        "design_impact": design_impact.ok_or_else(|| anyhow!("score-only output missing design_impact"))?,
        "consultant": consultant.ok_or_else(|| anyhow!("score-only output missing consultant"))?,
        "staffing_roles": staffing_roles,
        "forced_by_risk": forced_by_risk,
        "auto_summoned": auto_summoned
    }))
}

fn parse_token_cost_output(output: &str) -> Result<Value> {
    let mut pricing_source = None;
    let mut pricing_entries = None;
    let mut dispatch_log = None;
    let mut snapshot_path = None;
    let mut snapshot_written = None;
    let mut warnings: Vec<String> = Vec::new();
    let mut windows: Vec<Value> = Vec::new();
    let mut current: Option<serde_json::Map<String, Value>> = None;
    let mut section = TokenCostSection::None;

    for line in output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if line == "AIPLUS_TOKEN_COST" {
            continue;
        }
        if let Some(value) = line.strip_prefix("pricing_source=") {
            pricing_source = Some(value.to_string());
            continue;
        }
        if let Some(value) = line.strip_prefix("pricing_entries=") {
            pricing_entries = Some(value.parse::<usize>()?);
            continue;
        }
        if let Some(value) = line.strip_prefix("dispatch_log=") {
            dispatch_log = Some(value.to_string());
            continue;
        }
        if let Some(value) = line.strip_prefix("snapshot_path=") {
            snapshot_path = Some(value.to_string());
            continue;
        }
        if let Some(value) = line.strip_prefix("snapshot_written=") {
            snapshot_written = Some(parse_bool_text(value)?);
            continue;
        }
        if let Some(value) = line.strip_prefix("WARN ") {
            if let Some(window) = current.as_mut() {
                push_json_array_item(window, "warnings", json!(value));
            } else {
                warnings.push(value.to_string());
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("WINDOW ") {
            if let Some(window) = current.take() {
                windows.push(Value::Object(window));
            }
            current = Some(parse_token_cost_window(rest)?);
            section = TokenCostSection::None;
            continue;
        }
        if line == "TOP_TASKS" {
            section = TokenCostSection::TopTasks;
            continue;
        }
        if line == "BY_ROLE" {
            section = TokenCostSection::ByRole;
            continue;
        }
        if line == "(none)" {
            continue;
        }
        let Some(window) = current.as_mut() else {
            continue;
        };
        match section {
            TokenCostSection::TopTasks => {
                push_json_array_item(window, "top_tasks", parse_token_cost_top_task(line)?);
            }
            TokenCostSection::ByRole => {
                push_json_array_item(window, "by_role", parse_token_cost_role(line)?);
            }
            TokenCostSection::None => {}
        }
    }
    if let Some(window) = current.take() {
        windows.push(Value::Object(window));
    }

    Ok(json!({
        "pricing_source": pricing_source.ok_or_else(|| anyhow!("token-cost output missing pricing_source"))?,
        "pricing_entries": pricing_entries.ok_or_else(|| anyhow!("token-cost output missing pricing_entries"))?,
        "dispatch_log": dispatch_log.ok_or_else(|| anyhow!("token-cost output missing dispatch_log"))?,
        "snapshot_path": snapshot_path.ok_or_else(|| anyhow!("token-cost output missing snapshot_path"))?,
        "snapshot_written": snapshot_written.ok_or_else(|| anyhow!("token-cost output missing snapshot_written"))?,
        "warnings": warnings,
        "windows": windows
    }))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenCostSection {
    None,
    TopTasks,
    ByRole,
}

fn parse_token_cost_window(rest: &str) -> Result<serde_json::Map<String, Value>> {
    let mut parts = rest.split_whitespace();
    let label = parts
        .next()
        .ok_or_else(|| anyhow!("WINDOW line missing label"))?;
    let mut total_tokens = None;
    let mut total_usd = None;
    for part in parts {
        if let Some(value) = part.strip_prefix("total_tokens=") {
            total_tokens = Some(value.parse::<u64>()?);
        } else if let Some(value) = part.strip_prefix("total_usd=") {
            total_usd = Some(value.parse::<f64>()?);
        }
    }
    let mut window = serde_json::Map::new();
    window.insert("window".to_string(), json!(label));
    window.insert(
        "total_tokens".to_string(),
        json!(total_tokens.ok_or_else(|| anyhow!("WINDOW line missing total_tokens"))?),
    );
    window.insert(
        "total_usd".to_string(),
        json!(total_usd.ok_or_else(|| anyhow!("WINDOW line missing total_usd"))?),
    );
    window.insert("top_tasks".to_string(), json!([]));
    window.insert("by_role".to_string(), json!([]));
    window.insert("warnings".to_string(), json!([]));
    Ok(window)
}

fn parse_token_cost_top_task(line: &str) -> Result<Value> {
    let (rank_text, rest) = line
        .split_once(". ")
        .ok_or_else(|| anyhow!("TOP_TASKS row missing rank: {line}"))?;
    let rank = rank_text.parse::<usize>()?;
    let (fields, task) = rest
        .split_once(" task=\"")
        .ok_or_else(|| anyhow!("TOP_TASKS row missing task text: {line}"))?;
    let task = task
        .strip_suffix('"')
        .ok_or_else(|| anyhow!("TOP_TASKS task text missing closing quote: {line}"))?;
    Ok(json!({
        "rank": rank,
        "usd": required_field(fields, "usd")?.parse::<f64>()?,
        "tokens": required_field(fields, "tokens")?.parse::<u64>()?,
        "role": required_field(fields, "role")?,
        "provider": required_field(fields, "provider")?,
        "model": required_field(fields, "model")?,
        "key": required_field(fields, "key")?,
        "task": task
    }))
}

fn parse_token_cost_role(line: &str) -> Result<Value> {
    let (role, fields) = line
        .split_once(' ')
        .ok_or_else(|| anyhow!("BY_ROLE row missing fields: {line}"))?;
    Ok(json!({
        "role": role,
        "tokens": required_field(fields, "tokens")?.parse::<u64>()?,
        "input": required_field(fields, "input")?.parse::<u64>()?,
        "output": required_field(fields, "output")?.parse::<u64>()?,
        "usd": required_field(fields, "usd")?.parse::<f64>()?
    }))
}

fn push_json_array_item(map: &mut serde_json::Map<String, Value>, key: &str, item: Value) {
    if let Some(values) = map.get_mut(key).and_then(Value::as_array_mut) {
        values.push(item);
    }
}

fn parse_bool_text(value: &str) -> Result<bool> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(anyhow!("expected boolean true/false, got {value}")),
    }
}

fn parse_bracket_list_line(line: &str, prefix: &str) -> Option<Vec<String>> {
    let values = line.strip_prefix(prefix)?;
    let values = values.strip_prefix('[')?.strip_suffix(']')?;
    if values.trim().is_empty() {
        return Some(Vec::new());
    }
    Some(
        values
            .split(',')
            .map(|value| value.trim().to_string())
            .collect(),
    )
}

fn find_key_value<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.split_whitespace()
        .find_map(|part| part.strip_prefix(&format!("{key}=")))
}

fn required_field<'a>(text: &'a str, key: &str) -> Result<&'a str> {
    find_key_value(text, key).ok_or_else(|| anyhow!("missing {key}= field in: {text}"))
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
    fn tools_list_advertises_fourteen_tools_with_required_fields() {
        let resp = handle_tools_list(json!(1));
        let result = resp.result.expect("tools/list must return result");
        let tools = result["tools"].as_array().expect("tools is array");
        assert_eq!(
            tools.len(),
            14,
            "expected 11 existing + 3 agent-autoflow tools = 14"
        );
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        // Original 3.
        assert!(names.contains(&"agent_route"));
        assert!(names.contains(&"agent_status"));
        assert!(names.contains(&"agent_set_team"));
        // P0.3 of v0.5.11 close-out.
        assert!(names.contains(&"agent_list"));
        assert!(names.contains(&"agent_doctor"));
        assert!(names.contains(&"agent_invite"));
        assert!(names.contains(&"agent_dismiss"));
        assert!(names.contains(&"agent_disable"));
        assert!(names.contains(&"agent_enable"));
        assert!(names.contains(&"agent_integrate"));
        assert!(names.contains(&"agent_talk"));
        // Agent-autoflow MCP.
        assert!(names.contains(&"agent_audit_verify_log"));
        assert!(names.contains(&"agent_route_score_only"));
        assert!(names.contains(&"agent_token_cost"));
        for tool in tools {
            assert!(tool["description"].is_string());
            assert!(tool["inputSchema"]["type"].as_str() == Some("object"));
        }
    }

    #[test]
    fn autoflow_tool_descriptions_prefer_mcp_and_stay_compact() {
        let tools = tool_definitions();
        for name in [
            "agent_token_cost",
            "agent_audit_verify_log",
            "agent_route_score_only",
        ] {
            let tool = tools
                .iter()
                .find(|tool| tool["name"].as_str() == Some(name))
                .unwrap_or_else(|| panic!("missing tool {name}"));
            let description = tool["description"].as_str().expect("description string");
            assert!(
                description.starts_with("PREFERRED programmatic surface"),
                "{name} should explicitly prefer MCP:\n{description}"
            );
            assert!(
                description.contains("MCP returns structured JSON"),
                "{name} should say why MCP beats CLI:\n{description}"
            );
            assert!(
                description.chars().count() <= 400,
                "{name} description too long: {} chars\n{description}",
                description.chars().count()
            );
        }
    }

    #[test]
    fn coverage_tool_descriptions_prefer_mcp_and_name_cli_alternative() {
        let tools = tool_definitions();
        for (name, cli) in [
            ("agent_route", "aiplus agent route"),
            ("agent_status", "aiplus agent status"),
            ("agent_set_team", "aiplus agent set-team"),
            ("agent_list", "aiplus agent list"),
            ("agent_doctor", "aiplus agent doctor"),
            ("agent_invite", "aiplus agent invite"),
            ("agent_dismiss", "aiplus agent dismiss"),
            ("agent_disable", "aiplus agent disable"),
            ("agent_enable", "aiplus agent enable"),
            ("agent_integrate", "aiplus agent integrate"),
            ("agent_talk", "aiplus agent talk"),
        ] {
            let tool = tools
                .iter()
                .find(|tool| tool["name"].as_str() == Some(name))
                .unwrap_or_else(|| panic!("missing tool {name}"));
            let description = tool["description"].as_str().expect("description string");
            assert!(
                description.starts_with("PREFERRED programmatic surface"),
                "{name} should explicitly prefer MCP:\n{description}"
            );
            assert!(
                description.contains("Use this MCP tool instead of"),
                "{name} should direct agents away from CLI first:\n{description}"
            );
            assert!(
                description.contains(cli),
                "{name} should name CLI alternative {cli}:\n{description}"
            );
            assert!(
                description.chars().count() <= 400,
                "{name} description too long: {} chars\n{description}",
                description.chars().count()
            );
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
    fn agent_autoflow_args_validated_before_subprocess() {
        let tmp = std::env::temp_dir();
        let err = call_agent_route_score_only(&json!({}), &tmp).unwrap_err();
        assert!(err.to_string().contains("task"));
        let err = call_agent_route_score_only(&json!({"task": ""}), &tmp).unwrap_err();
        assert!(err.to_string().contains("task"));
        let err = call_agent_token_cost(&json!({"window": "2h"}), &tmp).unwrap_err();
        assert!(err.to_string().contains("window"));
        let err = call_agent_token_cost(&json!({"top_n": 0}), &tmp).unwrap_err();
        assert!(err.to_string().contains("top_n"));
        let err = call_agent_token_cost(&json!({"top_n": 51}), &tmp).unwrap_err();
        assert!(err.to_string().contains("top_n"));
    }

    #[test]
    fn audit_verify_log_output_parser_handles_pass_and_fail() {
        let pass = parse_audit_verify_log_output("VERIFY_LOG=PASS checked_lines=7\n")
            .unwrap()
            .unwrap();
        assert_eq!(pass["verdict"], "PASS");
        assert_eq!(pass["checked_lines"], 7);
        assert!(pass["first_bad_line"].is_null());

        let fail = parse_audit_verify_log_output("VERIFY_LOG=FAIL line=42 reason=hash mismatch\n")
            .unwrap()
            .unwrap();
        assert_eq!(fail["verdict"], "FAIL");
        assert_eq!(fail["first_bad_line"], 42);
        assert_eq!(fail["reason"], "hash mismatch");
    }

    #[test]
    fn route_score_only_output_parser_handles_optional_lists() {
        let parsed = parse_route_score_only_output(
            "Adaptive coordinator: complexity=5 risk=0.85 tier=HEAVY code_change=true design_impact=true consultant=fire\n\
             Plan step: would fire consultant for HEAVY task\n\
             Would staff: [pm,architect,engineer-a,reviewer,qa]\n\
             Forced by risk: [qa]\n\
             Auto-summoned experts: [security-reviewer,tech-writer]\n",
        )
        .unwrap();
        assert_eq!(parsed["complexity"], 5);
        assert_eq!(parsed["risk"], 0.85);
        assert_eq!(parsed["tier"], "HEAVY");
        assert_eq!(parsed["code_change"], true);
        assert_eq!(parsed["design_impact"], true);
        assert_eq!(parsed["consultant"], "fire");
        assert_eq!(
            parsed["staffing_roles"],
            json!(["pm", "architect", "engineer-a", "reviewer", "qa"])
        );
        assert_eq!(parsed["forced_by_risk"], json!(["qa"]));
        assert_eq!(
            parsed["auto_summoned"],
            json!(["security-reviewer", "tech-writer"])
        );
    }

    #[test]
    fn token_cost_output_parser_handles_windows_tasks_roles_and_warnings() {
        let parsed = parse_token_cost_output(
            "AIPLUS_TOKEN_COST\n\
             pricing_source=litellm_cache\n\
             pricing_entries=3967\n\
             dispatch_log=/tmp/project/.aiplus/agents/dispatch-log.jsonl\n\
             snapshot_path=/tmp/project/.aiplus/agents/token-cost-snapshots.jsonl\n\
             snapshot_written=true\n\
             WARN pricing stale\n\
             \n\
             WINDOW 1h total_tokens=110 total_usd=0.012300\n\
             TOP_TASKS\n\
             1. usd=0.012300 tokens=110 role=engineer-a provider=anthropic model=claude key=dispatch-1 task=\"implement payment\"\n\
             BY_ROLE\n\
             engineer-a tokens=110 input=100 output=10 usd=0.012300\n",
        )
        .unwrap();
        assert_eq!(parsed["pricing_source"], "litellm_cache");
        assert_eq!(parsed["pricing_entries"], 3967);
        assert_eq!(parsed["snapshot_written"], true);
        assert_eq!(parsed["warnings"], json!(["pricing stale"]));
        assert_eq!(parsed["windows"][0]["window"], "1h");
        assert_eq!(parsed["windows"][0]["total_tokens"], 110);
        assert_eq!(
            parsed["windows"][0]["top_tasks"][0]["task"],
            "implement payment"
        );
        assert_eq!(parsed["windows"][0]["by_role"][0]["role"], "engineer-a");
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
