# aiplus-token-cost

`aiplus-token-cost` reads `.aiplus/agents/dispatch-log.jsonl`, extracts token usage, and estimates USD spend across 1h, 8h, and 24h windows.

Pricing source order:

1. Project override: `.aiplus/pricing.toml`
2. Fresh local cache: `~/.cache/aiplus-token-cost/pricing.json` or `$XDG_CACHE_HOME/aiplus-token-cost/pricing.json`
3. LiteLLM `model_prices_and_context_window.json`
4. Embedded fallback constants

Override example:

```toml
[[price]]
provider = "anthropic"
model = "claude-sonnet-4-6"
input_usd_per_token = 0.000003
output_usd_per_token = 0.000015
```

Future embedded-price update flow: run `aiplus token-cost --refresh-embedded` to refresh constants from the current LiteLLM JSON. That command is documented for the next iteration and is not implemented in v1.
