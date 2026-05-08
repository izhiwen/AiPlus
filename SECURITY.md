# Security And Privacy

AiPlus is project-local by default. It writes project install state under
`.aiplus/` and compact state under `.codex/compact/`.

## Compact Savings Estimate

AiPlus may fetch public release metadata and public model pricing information to
estimate compact savings. It caches pricing data locally, normally under
`~/.cache/aiplus/pricing-cache.json`, with a default 7-day TTL. If the cache is
missing or stale, AiPlus may refresh public pricing automatically. Network
failure does not block compact, checkpoint, resume, or token savings reporting.

AiPlus does not upload prompts, transcripts, project files, checkpoints, savings
ledgers, secrets, billing data, provider account data, or usage history. It does
not connect to provider billing APIs and does not require manual model price
input.

## Updates

`aiplus self update` may fetch public AiPlus release metadata, the approved
release asset, and `checksums.txt`. It installs only the user-level `aiplus`
binary path selected by the command, stages the new binary first, backs up the
old binary, verifies checksum before replacement, and runs a version smoke
check. It does not edit shell profiles, system paths, or global Codex, Claude
Code, or OpenCode configuration.

`aiplus update` updates only the current project's `.aiplus/` modules and
guidance. `aiplus update all` combines self update and project update when safe.
Project updates preserve `.codex/compact/` and the savings ledger.

## Private Profiles And Secret Broker

`aiplus profile install work-with-zhiwen --user --yes` writes user-level
preference files under `~/.config/aiplus/profiles/work-with-zhiwen/`. These
files are for collaboration preferences only and must not contain secret values,
Bitwarden machine tokens, prompt transcripts, compact checkpoints, project file
contents, or provider responses.

`aiplus secret-broker` is the only supported secret access path. It maps approved
aliases to Bitwarden Secrets Manager names and child-process environment
variables:

- `openai` -> `zhiwen/openai/api_key` -> `OPENAI_API_KEY`
- `anthropic` -> `zhiwen/anthropic/api_key` -> `ANTHROPIC_API_KEY`
- `gemini` -> `zhiwen/gemini/api_key` -> `GEMINI_API_KEY`
- `github` -> `zhiwen/github/token` -> `GITHUB_TOKEN`
- `cloudflare` -> `zhiwen/cloudflare/token` -> `CLOUDFLARE_API_TOKEN`
- `kimi` -> `zhiwen/kimi/api_key` -> `KIMI_API_KEY`
- `deepseek` -> `zhiwen/deepseek/api_key` -> `DEEPSEEK_API_KEY`
- `minimax` -> `zhiwen/minimax/api_key` -> `MINIMAX_API_KEY`
- `qwen` -> `zhiwen/qwen/api_key` -> `QWEN_API_KEY`
- `glm` -> `zhiwen/glm/api_key` -> `GLM_API_KEY`
- `openrouter` -> `zhiwen/openrouter/api_key` -> `OPENROUTER_API_KEY`
- `xai` -> `zhiwen/xai/api_key` -> `XAI_API_KEY`
- `groq` -> `zhiwen/groq/api_key` -> `GROQ_API_KEY`
- `mistral` -> `zhiwen/mistral/api_key` -> `MISTRAL_API_KEY`
- `perplexity` -> `zhiwen/perplexity/api_key` -> `PERPLEXITY_API_KEY`
- `together` -> `zhiwen/together/api_key` -> `TOGETHER_API_KEY`
- `cohere` -> `zhiwen/cohere/api_key` -> `COHERE_API_KEY`
- `huggingface` -> `zhiwen/huggingface/token` -> `HUGGINGFACE_TOKEN`
- `voyage` -> `zhiwen/voyage/api_key` -> `VOYAGE_API_KEY`
- `jina` -> `zhiwen/jina/api_key` -> `JINA_API_KEY`
- `replicate` -> `zhiwen/replicate/api_token` -> `REPLICATE_API_TOKEN`
- `fal` -> `zhiwen/fal/api_key` -> `FAL_API_KEY`
- `stability` -> `zhiwen/stability/api_key` -> `STABILITY_API_KEY`
- `elevenlabs` -> `zhiwen/elevenlabs/api_key` -> `ELEVENLABS_API_KEY`
- `tavily` -> `zhiwen/tavily/api_key` -> `TAVILY_API_KEY`
- `exa` -> `zhiwen/exa/api_key` -> `EXA_API_KEY`
- `serper` -> `zhiwen/serper/api_key` -> `SERPER_API_KEY`
- `firecrawl` -> `zhiwen/firecrawl/api_key` -> `FIRECRAWL_API_KEY`
- `brave` -> `zhiwen/brave/api_key` -> `BRAVE_API_KEY`
- `siliconflow` -> `zhiwen/siliconflow/api_key` -> `SILICONFLOW_API_KEY`
- `volcengine_ark` -> `zhiwen/volcengine_ark/api_key` -> `VOLCENGINE_ARK_API_KEY`

By default, `resolve` does not print secret values. `run -- <command...>` injects
approved values only into the child process environment. AiPlus may read
`BWS_ACCESS_TOKEN` for the current process or a macOS Keychain item created by
`aiplus secret-broker token set`; it must not store that token in plaintext repo
files, project install files, compact files, logs, docs, or release artifacts.
The invoked child command is outside AiPlus' control and may print, log,
transmit, or store environment variables. Use `run --` only with trusted commands
for explicit action needs.

Real Bitwarden smoke checks require the Bitwarden Secrets Manager `bws` CLI plus
a read-only machine account token for project `zhiwen-agent-secrets` and machine
account `zhiwen-local-aiplus-agent`. If `bws` is missing, mock-provider tests can
verify alias policy and no-print behavior, but real Bitwarden read access remains
unverified.

Secret-broker audit/status output is metadata-only: alias requested, allow/deny
status, provider status, and timestamp-like operational context. It must never
include raw tokens, auth headers, Bitwarden response bodies, decrypted material,
or secret values.

Savings reports are estimates based on local aggregate metrics and cached public
pricing. They are not billing data, invoices, guaranteed savings, precise cost
measurements, or quality proof.

## Sensitive Local Files

Do not place secrets, API keys, private keys, raw transcripts, provider request
or response bodies, account identifiers, personal data, screenshots with secrets,
or exact private paths in compact files.

Validation is structural and heuristic. It is not a complete secret scanner,
privacy review, legal review, compliance certification, or release approval.
