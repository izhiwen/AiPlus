# Alias Naming Conventions

The broker config has three columns per row (alias, backend-path, env-var-name).
This guide is about choosing good values for each.

## TL;DR

```
openai	zhiwen/openai/api_key	OPENAI_API_KEY
```

- **Alias**: short, lowercase, single-word. The name *you* type.
- **Backend path**: scoped path in your Bitwarden Secrets Manager workspace. Decoupled from alias.
- **Env var name**: SDK convention (uppercase, what the official client library reads).

## Alias column — naming rules

The alias is what you type at the CLI:

```bash
aiplus secret-broker run --alias openai -- python agent.py
aiplus secret-broker resolve openai
```

**Rules:**

1. **Lowercase.** `openai`, not `OPENAI` or `OpenAI`.
2. **Single word, no separators.** `openrouter`, not `open-router` or `open_router`.
3. **Provider/service name, not "purpose" name.** `openai`, not `llm` or `mainmodel`.
4. **Stable across rotation.** If you rotate the underlying key, the alias does NOT change — only the backend value changes.

**Anti-patterns:**

| Bad | Why | Better |
|---|---|---|
| `key1`, `key2` | Numeric suffix encodes nothing | Use provider name |
| `mykey` | "My" is not a provider | `openai` |
| `openai-new` | "new" goes stale immediately | Just `openai`; rotate at backend |
| `OPENAI_API_KEY` | That is the *env var name*, not the alias | `openai` |

## Multi-account: one alias per (provider, account) pair

If you have multiple accounts at the same provider — personal vs work, sandbox vs prod — use a `<provider>_<account>` alias:

```
openai_personal	yourname/openai/personal/api_key	OPENAI_API_KEY
openai_work	yourname/openai/work/api_key	OPENAI_API_KEY
```

Both inject the same env var name (`OPENAI_API_KEY`); you pick which alias to invoke at runtime:

```bash
# personal work
aiplus secret-broker run --alias openai_personal -- python agent.py

# work account
aiplus secret-broker run --alias openai_work -- python agent.py
```

This is the right answer for the "wrong-account confusion" failure mode — the alias makes the account choice explicit at the call site.

## Backend path column — recommended scheme

```
<scope>/<provider>/<key-name>
```

- **`<scope>`** — usually your Bitwarden Secrets Manager username or a project tag.
- **`<provider>`** — the service that issued the credential.
- **`<key-name>`** — what kind of credential (`api_key`, `token`, `dsn`, etc.).

Examples:

```
zhiwen/openai/api_key
zhiwen/anthropic/api_key
zhiwen/github/token
zhiwen/postgres/prod/dsn
```

The scope isolates your secrets from teammates' secrets when sharing a Bitwarden workspace. Avoid putting environment names (`dev`, `prod`) at the top of the path; put them at the bottom so they sort naturally.

## Env var name column — match SDK conventions

The third column is what env var the child process will see. Choose **the name the official client library reads**, so consumer code does not need any code change:

| Provider | Conventional env var |
|---|---|
| OpenAI | `OPENAI_API_KEY` |
| Anthropic | `ANTHROPIC_API_KEY` |
| Google AI / Gemini | `GEMINI_API_KEY` |
| GitHub | `GITHUB_TOKEN` |
| Cloudflare | `CLOUDFLARE_API_TOKEN` |
| HuggingFace | `HUGGINGFACE_TOKEN` |
| Replicate | `REPLICATE_API_TOKEN` |
| Stability | `STABILITY_API_KEY` |
| Groq | `GROQ_API_KEY` |
| DeepSeek | `DEEPSEEK_API_KEY` |
| Mistral | `MISTRAL_API_KEY` |
| Cohere | `COHERE_API_KEY` |
| ElevenLabs | `ELEVENLABS_API_KEY` |
| Brave | `BRAVE_API_KEY` |
| Exa | `EXA_API_KEY` |
| Tavily | `TAVILY_API_KEY` |
| Firecrawl | `FIRECRAWL_API_KEY` |
| Jina | `JINA_API_KEY` |
| Voyage | `VOYAGE_API_KEY` |
| Together | `TOGETHER_API_KEY` |
| OpenRouter | `OPENROUTER_API_KEY` |
| Perplexity | `PERPLEXITY_API_KEY` |
| FAL | `FAL_API_KEY` |
| Linear | `LINEAR_API_KEY` |
| Slack | `SLACK_TOKEN` or `SLACK_BOT_TOKEN` |
| AWS | `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` (two aliases) |

When in doubt, check the provider's official SDK docs for "environment variable" and match exactly.

## Multi-account env var override

If you want to use different env var names for different accounts (so your code can refer to both at once):

```
openai_personal	yourname/openai/personal/api_key	OPENAI_API_KEY_PERSONAL
openai_work	yourname/openai/work/api_key	OPENAI_API_KEY_WORK
```

Then either run both at once and your code picks based on context:

```bash
aiplus secret-broker run --aliases openai_personal,openai_work -- python switching_agent.py
```

or alias them through a shell wrapper to match SDK conventions per-call:

```bash
aiplus secret-broker run --alias openai_work -- bash -c \
  'export OPENAI_API_KEY="$OPENAI_API_KEY_WORK"; exec python agent.py'
```

The first form is cleaner if your code knows which to use; the second is more robust if the consumer SDK has hardcoded env var expectations.

## Rotation

When a credential is rotated at the backend (new value, same backend path), the alias and the row do NOT change. Every consumer continues to use the same alias; the next `resolve` returns the new value automatically. Rotation is invisible to the alias config.

## Deprecation

To deprecate an alias, just delete the row from `secret-aliases.tsv`. Any consumer trying to `run --alias <removed>` will fail loud. Do not add `_OLD`/`_DEPRECATED` suffixes — those let dead aliases linger.

## Multi-project namespace

The TSV is per-AiPlus-profile, not per-project. To isolate alias sets per project:

- **Option A — one profile.** Use distinguishing alias prefixes: `proj1_openai`, `proj2_openai`. Simpler.
- **Option B — multiple profiles.** Use `aiplus profile install <name>` to switch profiles. Each profile has its own `secret-aliases.tsv`. Same alias name in different profiles can map to different backend paths.

Option A is fine for < ~5 projects. Option B scales better but requires conscious profile switching.

## See also

- `example-aliases.tsv` — 24 realistic example rows
- `example-aliases.md` — annotated walkthrough of the TSV format
- `DESIGN.md` §6 — the configuration model in design context
