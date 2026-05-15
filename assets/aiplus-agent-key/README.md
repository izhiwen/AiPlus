# AiPlus-Agent-Key

> **Alias-based, zero-persistence secret resolution for AI coding agents.**
> Your agent calls `OPENAI_KEY_WORK`; the broker resolves it to a real value at runtime, injects it into the child process's environment, and forgets it. Never written to disk. Never printed by default. Never in your git history.

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

[中文 README](README.zh-CN.md)

## Prerequisites

AiPlus-Agent-Key is implemented as a subcommand of the [AiPlus](https://github.com/izhiwen/AiPlus) CLI (`aiplus secret-broker`). Install AiPlus first:

```bash
# macOS (arm64): one-shot installer
curl -fsSL https://raw.githubusercontent.com/izhiwen/AiPlus/main/install.sh | bash

# Verify the secret-broker subcommand is available:
aiplus secret-broker --help
```

> **Platform support — v0.1:** AiPlus currently ships binaries only for **macOS arm64** (Apple Silicon). Linux and macOS Intel users must build from source (see AiPlus README → Developer Build) until upstream publishes additional release artifacts. CI runners on Linux therefore cannot use this module today via prebuilt binaries; the env-var token method (Method B in Setup) and the GitHub Actions example in `examples/README.md` are written for the day Linux binaries land.

**Bitwarden Secrets Manager** is the v0.1 backend. You need:

- A [Bitwarden Secrets Manager](https://bitwarden.com/products/secrets-manager/) workspace.
- The `bws` CLI installed and on `$PATH`. On macOS: `brew install bws`. On Linux: follow [Bitwarden's CLI install docs](https://bitwarden.com/help/secrets-manager-cli/).
- A machine access token for the workspace; see Setup below.

Other backends (1Password, AWS Secrets Manager, HashiCorp Vault, env-file fallback) are planned for v0.2.

## The pain

You run AI coding agents (Claude Code / Codex / OpenCode) all day. Each agent needs API keys: an OpenAI key, an Anthropic key, maybe a separate work account, maybe a tools key (Linear, Slack, AWS). Five recurring failure modes:

1. **Leaked into git.** A `.env` file gets accidentally committed. Or a key sits in shell history. Or it appears in an agent's chat transcript before you remember to delete it.

2. **Leaked into agent context.** You paste the key into a prompt "just this once" to fix a stuck task. Now it's in the agent's compact-handoff transcript, in memory snapshots, in the conversation cache.

3. **Mixed accounts.** You have three OpenAI accounts (personal, work, sandbox). The wrong key fires against the wrong workspace and you only realize when the bill shows up.

4. **Rotation pain.** A key expires or gets revoked. Now you need to update it in eight places: shell rc, three project `.env` files, two CI configs, two docker-compose files.

5. **Print-by-default risk.** Most secret-management CLIs default to *printing* the secret. One screenshot, one screen-share, one tmux scrollback dump, and the value escapes.

## What this does

**Map an alias to a backend secret path. Resolve at runtime. Inject into the child process's environment. Never persist.**

```bash
# In your project, agent code or shell script:
aiplus secret-broker run --aliases openai,anthropic -- python my_agent.py
```

Translation:
1. The broker reads its alias-to-backend mapping from `~/.config/aiplus/secret-broker/profiles/<profile>/secret-aliases.tsv`.
2. For each alias, it fetches the current value from the Bitwarden Secrets Manager (`bws`) backend.
3. It runs `python my_agent.py` with `OPENAI_API_KEY=<resolved>` and `ANTHROPIC_API_KEY=<resolved>` set in the child's environment (the env-var name is the SDK convention recorded per alias, not the alias itself).
4. When `python my_agent.py` exits, the resolved values are gone. Not written. Not cached. Not in shell history.

Other commands:

```bash
aiplus secret-broker status                  # is it installed, what backend, is auth current
aiplus secret-broker doctor                  # validate config, test backend reachability
aiplus secret-broker list                    # list configured aliases as `alias -> backend-path -> env-var-name` (NO values)
aiplus secret-broker resolve openai          # show resolution metadata (alias, provider, backend path). NO value by default.
aiplus secret-broker run --alias openai -- <command...>          # run one command with one secret injected
aiplus secret-broker run --aliases openai,anthropic -- <cmd...>  # run with multiple secrets injected
echo "<bws-access-token>" | aiplus secret-broker token set       # store the Bitwarden access token in OS keyring (stdin only)
aiplus secret-broker token delete            # remove the stored access token
```

`--print` exists but is **disabled by default**. To enable, set the explicit policy in your AiPlus profile preferences (`privacy-and-secrets.md`). The flag also leaves a trace in shell history, which is intentional — secret printing is a deliberate, audit-visible action.

### `run` without `--alias`/`--aliases`

If you call `aiplus secret-broker run -- <command>` with no alias selector, the broker injects a **curated default subset** of your configured aliases — the well-known LLM-provider keys and a few common platform tokens (e.g. `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GITHUB_TOKEN`, `CLOUDFLARE_API_TOKEN`, plus the major LLM providers it recognises). Other aliases are listed under `skipped_aliases=[...]` in the output. To inject exactly the set you want, always pass `--alias` or `--aliases` explicitly. The default-injection is a convenience for "open a shell and have my LLM keys ready" sessions; it is not for production scripts.

## Alias naming conventions

The TSV has three columns per row: `<alias>	<backend-path>	<env-var-name>`. Choose values like this:

- **Alias** — short, lowercase, single word: `openai`, `anthropic`, `github`, `cloudflare`. This is what you type at the CLI.
- **Backend path** — scoped path in your Bitwarden Secrets Manager workspace: `<scope>/<provider>/<key-name>` (e.g. `yourname/openai/api_key`).
- **Env var name** — the env var your code reads, typically the SDK convention: `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GITHUB_TOKEN`.

For multi-account setups, use `<provider>_<account>` aliases:

```
openai_personal	yourname/openai/personal/api_key	OPENAI_API_KEY
openai_work	yourname/openai/work/api_key	OPENAI_API_KEY
```

See [`core/alias-conventions.md`](core/alias-conventions.md) for the full guide, [`core/example-aliases.tsv`](core/example-aliases.tsv) for 24 realistic example rows, and [`core/example-aliases.md`](core/example-aliases.md) for the annotated walkthrough.

## Setup

1. **Install AiPlus** (see Prerequisites). Verify `aiplus secret-broker status` runs.
2. **Install the `bws` CLI** per Bitwarden's instructions. The broker shells out to `bws` for resolution; if it is missing, `aiplus secret-broker doctor` prints `bws_cli=no`.
3. **Create a Bitwarden Secrets Manager workspace** and **generate a machine access token** with read-only access to the secrets you want to expose.
4. **Provide the access token via one of two methods:**

   **Method A — OS keyring (recommended for local development):**
   ```bash
   echo "<your-bws-access-token>" | aiplus secret-broker token set
   ```
   The token is read from stdin only (never as an argument; that protects against shell history and `ps` exposure). On macOS, the **first call triggers a Keychain authorization dialog** — click *Always Allow* so future sessions can read the token without prompting. On Linux, this requires Secret Service (e.g. gnome-keyring) to be available.

   **Method B — environment variable (CI, Docker, headless boxes):**
   ```bash
   export BWS_ACCESS_TOKEN="<your-bws-access-token>"
   ```
   When `BWS_ACCESS_TOKEN` is set, the broker uses it directly and bypasses the keyring. `aiplus secret-broker status` will then report `token_source=env`. Suitable for CI runners where there is no interactive Keychain.

5. **Choose a profile name and create the alias directory.** The broker reads aliases from `~/.config/aiplus/secret-broker/profiles/<profile>/secret-aliases.tsv`. Profile name is freeform — `default`, your username, your machine name, or whatever — it just needs to be a directory under the path above:
   ```bash
   PROFILE=default   # any name; AiPlus will auto-discover it
   mkdir -p "$HOME/.config/aiplus/secret-broker/profiles/$PROFILE"
   ```
   (If `aiplus profile status` reports `profiles=[]` you can ignore that — the secret-broker profile directory is independent of AiPlus's installed-profile list.)

6. **Write your alias TSV.** Each row is **tab-separated** (three columns: alias, backend path, env var name):
   ```bash
   cat > "$HOME/.config/aiplus/secret-broker/profiles/$PROFILE/secret-aliases.tsv" <<'EOF'
   openai	yourname/openai/api_key	OPENAI_API_KEY
   anthropic	yourname/anthropic/api_key	ANTHROPIC_API_KEY
   github	yourname/github/token	GITHUB_TOKEN
   EOF
   ```

7. **Verify with `doctor`:**
   ```bash
   aiplus secret-broker doctor
   ```
   Expected: `SECRET_BROKER_DOCTOR_STATUS=PASS`. If it fails, the output contains a `next=...` line that names the exact recovery command (e.g. `next=run aiplus secret-broker token set in Terminal`). Follow the hint and re-run doctor.

8. **Use it:**
   ```bash
   aiplus secret-broker run --alias openai -- python my_agent.py
   ```
   Inside the child process, `os.environ["OPENAI_API_KEY"]` is set. When the child exits, the value is gone.

## Safety boundaries

AiPlus-Agent-Key does NOT:

- **Persist** resolved secret values anywhere. No file. No memory cache between calls. No log.
- **Print** secrets by default. `aiplus secret-broker resolve <alias>` returns resolution *metadata* (alias name, provider, backend path) but NOT the value. `--print` is disabled by default and requires an explicit opt-in via profile preferences; even when enabled, the flag's use is intentionally visible in shell history.
- **Upload** anything. The broker talks only to your configured backend and your child process.
- **Read** any secret you have not explicitly mapped via alias. Aliases are an allowlist.
- **Bypass** the OS keyring. The Bitwarden access token never lives in a config file in this repo or in your project.
- **Modify** global config (`~/.codex`, `~/.claude`, etc.). All AiPlus writes are scoped to `~/.config/aiplus/`.
- **Commit** to your repo. The `.gitignore` shipped with this module blocks `*.env`, `*.token`, `*.key`, `*.credentials`, `*.bw-export`.

What this does NOT protect against:
- A child process that *itself* writes secret values to disk (e.g., your agent logging the env var). Your code must be careful.
- A user passing `--print` and screen-recording the terminal.
- A backend whose own credentials are compromised.

## Architecture overview

```
                  AiPlus-Agent-Key                      ← runtime alias resolution
                            ↓ uses
                   Bitwarden Secrets Manager            ← backend (v0.1)
                   1Password / Vault / AWS              ← backends (v0.2)
                   ↓ injects into env vars
                  <child process>                       ← your agent
```

The broker is a stateless layer between the backend (where secret values actually live) and the child process (where they're temporarily injected). No state is kept in the broker itself; every `run` call resolves fresh.

See [`DESIGN.md`](DESIGN.md) for the full design rationale, threat model, and backend protocol.

## Status

| Component | v0.1 | v0.2 (planned) |
|---|---|---|
| CLI subcommand `aiplus secret-broker` | ✓ ships in AiPlus | enhancements |
| Bitwarden Secrets Manager backend (via `bws` CLI) | ✓ | refinements |
| Metadata-only audit log (alias names + timestamps, no values) | ✓ | retention controls + structured log file |
| 1Password backend | — | ✓ |
| AWS Secrets Manager backend | — | ✓ |
| HashiCorp Vault backend | — | ✓ |
| Env-file fallback backend (offline dev only) | — | ✓ |
| Source-code extraction into standalone Rust crate | — | ✓ |
| Token-refresh automation (OAuth flows) | partial | ✓ |

## What's inside

- `core/example-aliases.tsv` — 24 realistic example alias rows
- `core/example-aliases.md` — annotated walkthrough of the TSV format
- `core/alias-conventions.md` — naming conventions guide
- `adapters/{claude-code,codex,opencode}/` — runtime-adapter scaffolds (v0.1 placeholders)
- `examples/` — synthetic walkthrough
- `DESIGN.md` — design rationale and threat model
- `.aiplus/agent-key/acceptance/v0.1.0/schema.yaml` — acceptance schema
- `tests/acceptance.test.sh` — structural invariants test

## More

- Main platform: [AiPlus](https://github.com/izhiwen/AiPlus)
- Sibling modules: [AiPlus-Agent-Team](https://github.com/izhiwen/AiPlus-Agent-Team), [AiEconLab](https://github.com/izhiwen/AiEconLab)

## License

[Apache-2.0](LICENSE)
