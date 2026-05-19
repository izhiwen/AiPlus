# Security Policy

`aiplus-token-cost` operates on local files only.

- Reads `.aiplus/agents/dispatch-log.jsonl` usage counters.
- Writes `.aiplus/agents/token-cost-snapshots.jsonl`.
- May fetch public LiteLLM pricing JSON when no local override or fresh
  cache is available.

No prompts, task content, API responses, or secret values are read.
Please report security issues through the AiPlus main repository's
private security advisory channel.
