# TEST-3: Headless `/aiel-talk` Execution

Issue: https://github.com/izhiwen/AiEconLab/issues/25

This test verifies behavior, not artifact installation. The harness installs a
fresh runtime-specific AEL project, invokes `/aiel-talk` headlessly where the
runtime exposes a real non-interactive command path, and asserts transcript
markers from the runtime output:

- role marker: `advisor`, `pi`, `ra-stata`/`stata`, or `referee`
- team marker: `AiEconLab`
- domain marker: `research`

## Runtime coverage

- Claude Code: `claude --print --dangerously-skip-permissions "/aiel-talk <role> ..."`
- OpenCode: `opencode run --command aiel-talk --model openai/gpt-4o-mini <role> ...`
- Codex: explicit skip. `aiplus agent talk --runtime codex <role>` currently
  launches top-level interactive `codex` with the persona prompt. The Codex CLI
  headless path is `codex exec`, but AiPlus does not expose a way to route
  `agent talk` through `codex exec` or pass the owner's follow-up prompt.

The Codex limitation is surfaced as a TEST-3 warning/skip instead of simulated
coverage because a fake `codex exec` wrapper around persona files would not test
the real `aiplus agent talk` path requested by TEST-3.

## Secrets

The workflow uses live provider CLIs and requires these GitHub secrets:

- `ANTHROPIC_API_KEY` for Claude Code
- `OPENAI_API_KEY` for OpenCode

Codex will also require `OPENAI_API_KEY` once AiPlus exposes a headless
`agent talk` invocation for Codex. The missing headless path is tracked
upstream as https://github.com/izhiwen/AiPlus/issues/97.
