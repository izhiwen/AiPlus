# Agent Autoflow MCP Implementation Notes

Status: Phase 1 design, before implementation.

## 1. Subprocess Approach

Use the existing subprocess pattern in `crates/aiplus-cli/src/mcp_server.rs`.
Each new MCP tool shells out to the current `aiplus` executable via
`std::env::current_exe()` and runs the already-shipped CLI subcommand in the
MCP server's project root.

This keeps MCP behavior aligned with CLI parsing and avoids coupling the server
to token-cost, audit, or coordinator internals. No structural refactor is
planned.

The existing MCP response shape is:

```json
{
  "content": [{ "type": "text", "text": "..." }],
  "isError": false
}
```

To preserve compatibility with the existing 11 tools, the 3 new tools keep this
envelope and return stable JSON as the text payload.

## 2. Final JSON Tool Definitions

### agent_audit_verify_log

```json
{
  "name": "agent_audit_verify_log",
  "description": "Verify the integrity of .aiplus/agents/dispatch-log.jsonl hash chain. Reports PASS or FAIL with the first bad line and reason.",
  "inputSchema": {
    "type": "object",
    "properties": {}
  }
}
```

Text payload shape:

```json
{"verdict":"PASS","checked_lines":0,"first_bad_line":null,"reason":null}
```

or:

```json
{"verdict":"FAIL","checked_lines":null,"first_bad_line":42,"reason":"hash mismatch"}
```

### agent_route_score_only

```json
{
  "name": "agent_route_score_only",
  "description": "Pre-flight a task by running the adaptive coordinator scorer and tier classifier without dispatching roles.",
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
}
```

Text payload shape:

```json
{
  "complexity": 3,
  "risk": 0.35,
  "tier": "MEDIUM",
  "code_change": true,
  "design_impact": false,
  "consultant": "fire",
  "staffing_roles": ["engineer-a", "reviewer"],
  "forced_by_risk": [],
  "auto_summoned": []
}
```

### agent_token_cost

```json
{
  "name": "agent_token_cost",
  "description": "Show token consumption and USD cost rollups for AiPlus dispatch logs in 1-hour / 8-hour / 24-hour windows, with optional per-role breakdown and top-N most expensive tasks.",
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
}
```

Text payload shape:

```json
{
  "pricing_source": "litellm_cache",
  "pricing_entries": 3967,
  "dispatch_log": ".../.aiplus/agents/dispatch-log.jsonl",
  "snapshot_path": ".../.aiplus/agents/token-cost-snapshots.jsonl",
  "snapshot_written": true,
  "warnings": [],
  "windows": [
    {
      "window": "1h",
      "total_tokens": 0,
      "total_usd": 0.0,
      "top_tasks": [],
      "by_role": []
    }
  ]
}
```

## 3. Output Parsing Strategy

- `agent_audit_verify_log`: line-by-line parse `VERIFY_LOG=PASS checked_lines=N`
  or `VERIFY_LOG=FAIL line=N reason=...`. The CLI exits non-zero on FAIL, so the
  wrapper must still parse stdout and return a non-error tool result with
  `"verdict":"FAIL"` when that structured line is present.
- `agent_route_score_only`: line-by-line parse:
  - `Adaptive coordinator: complexity=N risk=F tier=T code_change=B design_impact=B consultant=fire|skip`
  - `Would staff: [role-a,role-b]`
  - optional `Forced by risk: [...]`
  - optional `Auto-summoned experts: [...]`
- `agent_token_cost`: line-by-line parse:
  - header key/value lines after `AIPLUS_TOKEN_COST`
  - `WINDOW <label> total_tokens=N total_usd=F`
  - `TOP_TASKS` rows
  - optional `BY_ROLE` rows
  - `WARN ...` lines

Parsing is intentionally conservative: malformed or missing required lines
return a clean MCP error instead of guessing.

## 4. Test Plan

- Unit tests in `mcp_server.rs`:
  - tools/list count and names include the 3 new tools
  - required arg validation for `agent_route_score_only`
  - optional arg validation for `agent_token_cost`
  - parser fixtures for audit PASS/FAIL, score-only output, and token-cost output
- Integration test in `crates/aiplus-cli/tests/agent_autoflow_mcp.rs`:
  - start `aiplus mcp-serve`
  - send JSON-RPC `initialize`
  - send `tools/list` and assert all 3 tools are present
  - call each of the 3 new tools on a temp project
  - call invalid args and assert `isError=true`
- Gates:
  - `cargo fmt --check`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`

## 5. CHANGELOG Draft

```markdown
## Unreleased

- Add MCP tools `agent_token_cost`, `agent_audit_verify_log`, and
  `agent_route_score_only` so registered AiPlus MCP clients can discover and
  invoke token-cost rollups, dispatch-log verification, and coordinator
  score-only preflight checks.
```

## Phase 3 Evidence

Implementation result:

- `_IMPL-OK_` `agent_audit_verify_log`: listed in MCP tools and wraps
  `aiplus agent audit verify-log`; returns JSON text payload with verdict,
  checked line count, first bad line, and reason.
- `_IMPL-OK_` `agent_route_score_only`: listed in MCP tools and wraps
  `aiplus agent route --score-only <task>`; returns JSON text payload with
  score, tier, consultant decision, staffing roles, forced-by-risk roles, and
  auto-summoned experts.
- `_IMPL-OK_` `agent_token_cost`: listed in MCP tools and wraps
  `aiplus agent token-cost`; supports optional `window`, `by_role`, and
  bounded `top_n`; returns JSON text payload with pricing metadata and window
  rollups.

Focused test evidence:

```text
cargo test -p aiplus-cli mcp_server -- --nocapture
cargo test: 13 passed, 358 filtered out (41 suites, 0.01s)

cargo test -p aiplus-cli --test agent_autoflow_mcp -- --nocapture
cargo test: 1 passed (1 suite, 0.74s)
```

Live MCP smoke evidence:

```text
LIVE_MCP_SMOKE=PASS
tools/list contained:
  agent_token_cost
  agent_audit_verify_log
  agent_route_score_only

agent_audit_verify_log:
  {"checked_lines":0,"first_bad_line":null,"reason":null,"verdict":"PASS"}

agent_route_score_only:
  {"complexity":3,"risk":0.35,"tier":"MEDIUM","staffing_roles":["engineer-a","reviewer"],...}

agent_token_cost:
  {"pricing_source":"embedded_litellm_snapshot_2026-05-19","windows":[{"window":"1h",...}],...}

invalid args:
  isError=true, text="ERROR: agent_route_score_only requires 'task' (non-empty string)"
```

Full gate evidence:

```text
cargo fmt --check
PASS

git diff --check
PASS

cargo test --workspace
cargo test: 574 passed, 1 ignored (46 suites, 60.06s)

cargo clippy --workspace --all-targets -- -D warnings
cargo clippy: No issues found
```
