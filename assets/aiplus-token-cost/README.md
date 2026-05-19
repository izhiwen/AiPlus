# AiPlus Token Cost

`aiplus-token-cost` provides token consumption and USD cost rollups for
AiPlus dispatch logs.

It reads `.aiplus/agents/dispatch-log.jsonl`, summarizes 1-hour,
8-hour, and 24-hour windows, and reports the most expensive tasks in
each window. Pricing uses a local override first, then a cached LiteLLM
pricing table, then embedded fallback constants.

## Usage

Run from a project root that contains `.aiplus/`:

```bash
aiplus-token-cost
aiplus-token-cost --by-role
aiplus-token-cost --window 24h
aiplus-token-cost --top-n 10
aiplus agent token-cost
```

The standalone binary is maintained in
`izhiwen/AiPlus-Token-Cost`. AiPlus release archives bundle that
binary alongside `aiplus` so users can use either entry point.

## Data Boundary

- Reads: `.aiplus/agents/dispatch-log.jsonl` `usage_tokens` fields.
- Writes: `.aiplus/agents/token-cost-snapshots.jsonl`.
- Network: optional LiteLLM pricing JSON fetch, cached locally for 24
  hours. A project-local `.aiplus/pricing.toml` override skips the
  fetch path.

No telemetry is implemented. Prompt text, task content, API responses,
and secrets are not read.
