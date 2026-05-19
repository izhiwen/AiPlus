# Token Cost Implementation Notes

## 1. Crate Structure

`crates/aiplus-token-cost/` ships as a library crate with focused modules:

- `lib.rs`: public API, CLI report formatter, project-root wiring.
- `pricing.rs`: provider/model price table, local override, cache, fetch, fallback.
- `rollup.rs`: dispatch-log JSONL walker, usage-token extraction, window aggregation.
- `snapshot.rs`: hourly JSONL snapshot writer.
- `embedded.rs`: embedded fallback pricing constants.
- `error.rs`: crate `Result` alias.

The `aiplus-cli` integration is a thin `aiplus agent token-cost` subcommand registered in `commands.rs`; `agent/mod.rs` only dispatches that new variant to the new crate. `crates/aiplus-cli/Cargo.toml` gets a path dependency on the new crate as required glue. No top-level `aiplus token-cost` command is added in this wave because the scope assigns CLI registration to `agent/commands.rs`.

## 2. Pricing Data Flow

Source precedence for each `(provider, model)` entry:

1. Project-local override: `.aiplus/pricing.toml`.
2. Fresh local cache: `$XDG_CACHE_HOME/aiplus-token-cost/pricing.json` or `~/.cache/aiplus-token-cost/pricing.json`, max age 24h.
3. LiteLLM JSON fetch from `https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json`, then cache write on success.
4. Embedded constants.

Local overrides are layered on top of the selected base table, so a project can override one model while still using fetched/cache/embedded prices for all other models. Network or parse failure falls back to cache if present, then embedded constants. Unknown model pairs count tokens but USD remains `0.0` and a warning is carried in the report.

Override TOML:

```toml
[[price]]
provider = "anthropic"
model = "claude-sonnet-4-6"
input_usd_per_token = 0.000003
output_usd_per_token = 0.000015
```

`input_usd_per_1m_tokens` / `output_usd_per_1m_tokens` are also accepted and converted to per-token USD.

## 3. Rollup Algorithm

Pseudocode:

```text
read dispatch-log.jsonl
for each JSON line:
  parse timestamp; skip malformed/outside requested windows
  extract usage_tokens or usageTokens
  if usage is missing/null: input=0, output=0, total=0
  extract provider/model from top-level fields or usage object
  infer provider from model as fallback
  lookup price table
  usd = input * input_usd + output * output_usd, or 0 if unknown
  task key = decisionId || dispatchId || taskExcerpt || task
  role = role || "coordinator"
  add to all windows whose duration contains timestamp
sort top tasks by USD desc; truncate top_n
```

Edge cases:

- Missing `usage_tokens`: count 0 tokens and 0 USD.
- Unknown provider/model: count tokens, USD 0, warning.
- Malformed JSON or malformed timestamp: skip the row and record a warning.
- Future-dated rows: outside all windows unless the caller supplies a matching `now`.

## 4. Subcommand UX

Command:

```text
aiplus agent token-cost [--by-role] [--window 1h|8h|24h] [--top-n N]
```

Default output shows all three windows and top 5 tasks:

```text
AIPLUS_TOKEN_COST
pricing_source=...
dispatch_log=.aiplus/agents/dispatch-log.jsonl
snapshot_written=true|false

WINDOW 1h total_tokens=1234 total_usd=0.012345
TOP_TASKS
1. usd=0.010000 tokens=1000 role=engineer-a key=dispatch-...
```

`--by-role` appends a `BY_ROLE` section for each window. `--top-n` defaults to 5. `--window` filters output to a single window.

## 5. Snapshot Format

Subcommand invocation triggers `.aiplus/agents/token-cost-snapshots.jsonl`. A line is written only if the last parseable snapshot timestamp is at least one hour old.

Each line:

```json
{"schemaVersion":"0.1.0","event":"token_cost_snapshot","timestamp":"...","windows":[...]}
```

No rotation is implemented in v1. Future policy: retain the last 30 days or compact to daily totals after 30 days.

## 6. Embedded Constants

Embedded constants were sampled from LiteLLM `model_prices_and_context_window.json` on 2026-05-19:

- Anthropic `claude-opus-4-7`: input `0.000005`, output `0.000025`
- Anthropic `claude-sonnet-4-6`: input `0.000003`, output `0.000015`
- Anthropic `claude-haiku-4-5-20251001`: input `0.000001`, output `0.000005`
- OpenAI `gpt-5`: input `0.00000125`, output `0.00001`
- OpenAI `gpt-5-mini`: input `0.00000025`, output `0.000002`
- OpenAI `gpt-5-nano`: input `0.00000005`, output `0.0000004`
- OpenAI `gpt-4o`: input `0.0000025`, output `0.00001`
- OpenAI `gpt-4o-mini`: input `0.00000015`, output `0.0000006`

README documents a future `aiplus token-cost --refresh-embedded` workflow, but that command is out of scope for this implementation.

## 7. CHANGELOG 0.6.5 Draft

```markdown
### Added
- Added `aiplus-token-cost`, a new workspace crate for rolling up dispatch-log token usage into USD estimates. The new `aiplus agent token-cost` command reports 1h/8h/24h totals, top-cost tasks, optional per-role breakdowns, hourly local snapshots, LiteLLM pricing cache support, embedded fallback pricing, and project-local `.aiplus/pricing.toml` overrides.
```

## 8. Test Plan

- New crate tests cover embedded fallback, local override precedence, unknown-model warnings, null/missing usage tokens, top-N task sorting, by-role aggregation, snapshot skip/write behavior, and CLI report formatting.
- `cargo test -p aiplus-token-cost`
- `cargo clippy -p aiplus-token-cost -- -D warnings`
- `cargo build --workspace`
- `cargo test --workspace` with retry-once for any failure before classification.
- Live smoke from a temp project containing `.aiplus/agents/dispatch-log.jsonl`.

## Phase 3 Evidence

Implementation verdict: `PASS_WITH_DEVIATIONS`.

Deviations:

- CLI landed as `aiplus agent token-cost`, not top-level `aiplus token-cost`, because this Wave 2-C scope explicitly owns `crates/aiplus-cli/src/agent/commands.rs` registration and forbids unrelated top-level CLI edits.
- Minimal glue also touched `crates/aiplus-cli/Cargo.toml` and `crates/aiplus-cli/src/agent/mod.rs` so the new subcommand can compile and call the new crate. No existing subcommand definitions were modified.

Verification:

- `cargo test -p aiplus-token-cost`: PASS, 9 tests.
- `cargo clippy -p aiplus-token-cost -- -D warnings`: PASS, no issues.
- `cargo build --workspace`: PASS.
- `cargo test --workspace`: PASS, 564 passed, 1 ignored, 43 suites.
- `./target/debug/aiplus agent token-cost --help`: PASS; flags `--by-role`, `--window <1h|8h|24h>`, `--top-n` present.
- `./target/debug/aiplus agent doctor`: PASS exit status; existing warning only for missing `.aiplus/agents` in the worktree root.
- Scope check: `route.rs`, `coordinator.rs`, and `doctor.rs` have empty diffs. `commands.rs` diff is only the appended `TokenCost` variant.

Live smoke:

```text
AIPLUS_TOKEN_COST
pricing_source=litellm_cache
pricing_entries=3967
WINDOW 1h total_tokens=1100 total_usd=0.004500
TOP_TASKS
1. usd=0.004500 tokens=1100 role=engineer-a provider=anthropic model=claude-sonnet-4-6 key=dispatch-live-engineer-a task="implement payment flow"
```

`--by-role` smoke:

```text
BY_ROLE
engineer-a tokens=1100 input=1000 output=100 usd=0.004500
```

Embedded fallback smoke with empty `XDG_CACHE_HOME` and invalid pricing URL:

```text
pricing_source=embedded_litellm_snapshot_2026-05-19
pricing_entries=8
WARN litellm fetch failed: No such file or directory (os error 2)
WINDOW 1h total_tokens=1100 total_usd=0.004500
```

Project-local override smoke:

```text
WINDOW 1h total_tokens=1100 total_usd=1200.000000
BY_ROLE
engineer-a tokens=1100 input=1000 output=100 usd=1200.000000
```
