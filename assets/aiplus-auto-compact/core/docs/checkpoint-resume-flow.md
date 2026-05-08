# Checkpoint And Resume Flow

AiPlus Auto Compact is local-first. It prepares evidence for a compact/resume
boundary; it does not press, call, or automate any host agent compact control.

## Installed Through AiPlus

Ordinary users do not need to remember compact commands. In the agent session,
say "prepare compact" or "save progress". The agent should use this backend
tool from the target repository:

```bash
aiplus compact prepare
```

If `prepare` is unavailable, use the closest supported sequence:

```bash
aiplus compact validate
aiplus compact checkpoint
```

`prepare` checks readiness, creates or recommends a checkpoint as appropriate,
and prints the next step. `validate` checks the local compact state.
`checkpoint` writes a redacted JSON checkpoint under `.codex/compact/checkpoints/`
with validation status, Owner gate state, review items, warnings, errors, role
context, and the next safe action.

Only recommend manual compact when checkpoint output is `SAFE_TO_COMPACT` and
all Owner gates are explicitly `APPROVED`.

## Recommended Compact Message

When checkpoint state is ready, the agent can say:

```text
Ready to compact.

After compact:
- If I continue automatically, you do not need to do anything.
- If I do not reply, send: continue

I will resume from here.
```

The user or host runtime still performs compact manually.

## Resume After Host Compact

After compact, the agent should run:

```bash
aiplus compact resume
```

and continue from the checkpoint and local handoff files.

If the agent does not reply, say:

```text
continue
```

Natural continuation phrases such as `continue`, `resume`, `refresh`, `go on`,
`继续`, `刷新`, and `接着` should restart the resume flow.

Use explicit AiPlus refresh triggers when a project has its own meaning for
`刷新` or `refresh`; the agent should report AiPlus status before unrelated
project refresh when AiPlus is mentioned.

This is best-effort automatic resume. AiPlus Auto Compact can prepare the
checkpoint and tell the agent how to resume, but it cannot wake a host runtime
that requires a user message.

## Module-Only Or Legacy Reference

Advanced users who adopt only AiPlus Auto Compact can inspect `core/templates/`
and `core/docs/` directly, but compact execution should still use Rust-native
AiPlus CLI:

```bash
aiplus compact prepare
aiplus compact score
aiplus compact validate
aiplus compact checkpoint
aiplus compact resume
```

The legacy Node helper is retained only as archived history and compatibility
test fixture. It is not an ordinary-user path and must not be used as a fallback.
If `aiplus` is missing, install AiPlus or fix PATH:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
```

Then reopen the terminal or ensure `~/.local/bin` is on PATH. Do not fallback to
Node.
