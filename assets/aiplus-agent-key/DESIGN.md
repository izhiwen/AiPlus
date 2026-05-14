# AiPlus-Agent-Key Design

Status: draft v0.1.0
Acceptance schema (binding): `.aiplus/agent-key/acceptance/v0.1.0/schema.yaml`
Scope: local-first, zero-persistence, alias-based secret resolution layer for AI coding agents.

---

## 1. One-line positioning

`aiplus-agent-key` is the runtime-only secret-resolution layer for AiPlus. It maps short lowercase aliases (e.g. `openai`, `anthropic`) to a secret-manager backend (Bitwarden Secrets Manager via the `bws` CLI in v0.1), resolves them on demand, injects the resolved values into child-process environment variables under their SDK-conventional names (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`), and exits without persisting anything.

The CLI surface is `aiplus secret-broker`. It ships inside the AiPlus binary for v0.1; the source code is physically extracted into a standalone crate in v0.2.

---

## 2. Problem

A real Owner running AI coding agents day to day hits five recurring secret-handling failures:

1. **Secrets leak into git.** A `.env` file gets committed once and stays in the history forever, even after revert. Or a key surfaces in a code review patch. Or it lives in shell history that gets backed up.

2. **Secrets leak into agent context.** You paste a key into a prompt "just this once" to debug a stuck task. From then on, the key lives in the agent's compact-handoff transcript, in memory snapshots, in the conversation cache. A future Owner of the same project, or a future support session you share with the model provider, sees it.

3. **Wrong-account confusion.** Owners typically have multiple accounts at the same provider — personal vs work OpenAI, sandbox vs production Anthropic, employer-paid vs personal Claude. Without an alias layer, the wrong key fires against the wrong workspace and the mistake surfaces in billing or compliance.

4. **Rotation is painful.** Keys expire or get revoked. Without an alias layer, the Owner has to find every occurrence — shell rc, project `.env` files, CI configs, docker-compose files, IDE settings — and update each. With an alias layer, the Owner updates the backend once, and every consumer sees the new value on the next resolution.

5. **Default-print risk.** Most secret-management CLIs default to *printing* the resolved value. A screenshot, a screen-share, a tmux scrollback dump — any of those exposes the secret to whoever sees the terminal next.

`aiplus-agent-key` addresses all five.

---

## 3. What this plugin is NOT

To prevent scope drift, four explicit non-goals:

1. **Not a secrets vault.** It does not store secret values. The vault is the backend (Bitwarden Secrets Manager in v0.1). This module is a *resolver* and *injector*.

2. **Not a credential provisioning tool.** It does not create accounts, rotate keys, or call provider APIs to mint credentials. Rotation is done in the backend; the broker just resolves whatever the backend currently returns.

3. **Not a permission system for human users.** Authorization is handled by the backend's own access-control system (Bitwarden machine access tokens, scoped to a workspace). This module trusts the backend's decision.

4. **Not a long-lived daemon.** Every `resolve` and `run` call is stateless. There is no background process, no socket, no cache layer between invocations.

---

## 4. Solution overview — five core decisions

1. **Aliases are an allowlist.** Only aliases explicitly declared in the Owner's profile config can be resolved. Random environment-variable names cannot trigger backend lookups.

2. **Resolution is per-invocation; nothing is cached.** Every `run` call fetches fresh from the backend. This is slightly slower (~100-300ms per resolve depending on backend latency) but eliminates a whole class of "stale secret" and "leaked cache" failure modes.

3. **Injection is via child-process env vars, not files.** The resolved value never touches the filesystem. The child process sees the env var; when it exits, the value is gone from any process that did not capture it.

4. **`--print` is disabled by default and visible when enabled.** The default `resolve` output is resolution *metadata* (alias name, provider, backend path) — not the value. Enabling `--print` requires an explicit opt-in via the AiPlus profile's `privacy-and-secrets.md` preferences. Even when enabled, every `--print` invocation is recorded by audit metadata, and the flag's use is visible in shell history. Print-by-default is the most common secret-leak vector; this design closes it.

5. **The Bitwarden access token lives in the OS keyring, not in any config file.** The token is the bootstrap secret — the one secret that lets all other secrets be resolved. It is treated specially: stored in macOS Keychain / Linux Secret Service / Windows Credential Manager, never in a project file, never in `~/.config/aiplus/`.

---

## 5. Architecture

```
┌──────────────────────────────────────────────────────────────┐
│  User Owner / CI runner                                       │
│  $ aiplus secret-broker run --alias openai -- ./agent.py     │
└───────────────┬──────────────────────────────────────────────┘
                │  (1) parse CLI, read alias→backend-path→env-var-name
                │      mapping from ~/.config/aiplus/secret-broker/
                │      profiles/<p>/secret-aliases.tsv
                ▼
┌──────────────────────────────────────────────────────────────┐
│  aiplus secret-broker (this module)                          │
└───────────────┬──────────────────────────────────────────────┘
                │  (2) read Bitwarden access token from OS keyring
                │      (account: aiplus-secret-broker)
                ▼
┌──────────────────────────────────────────────────────────────┐
│  Bitwarden Secrets Manager API (backend)                     │
└───────────────┬──────────────────────────────────────────────┘
                │  (3) return secret values for the requested
                │      alias-mapped paths
                ▼
┌──────────────────────────────────────────────────────────────┐
│  aiplus secret-broker (this module)                          │
└───────────────┬──────────────────────────────────────────────┘
                │  (4) spawn child process with env injected:
                │      OPENAI_API_KEY=<value>  (visible to child only)
                ▼
┌──────────────────────────────────────────────────────────────┐
│  Child process (the user's agent / script)                   │
│  Reads os.environ["OPENAI_API_KEY"], makes API call.         │
│  On exit, the value is gone from any process that didn't     │
│  explicitly persist it.                                       │
└──────────────────────────────────────────────────────────────┘
```

The broker is intentionally a thin layer. It does not implement caching, retry, rate-limiting, or persistence. It is a key-to-value pipe with safety boundaries.

Resolution shells out to the **`bws` CLI** (Bitwarden Secrets) for backend lookups. The `bws` binary must be installed and on `$PATH`; the broker does not link against any Bitwarden client library directly.

---

## 6. Alias configuration model

Aliases live in `~/.config/aiplus/secret-broker/profiles/<profile-name>/secret-aliases.tsv`. The format is **TSV** (tab-separated values), three columns per row:

```
openai	yourname/openai/api_key	OPENAI_API_KEY
anthropic	yourname/anthropic/api_key	ANTHROPIC_API_KEY
github	yourname/github/token	GITHUB_TOKEN
```

| Column | Meaning |
|---|---|
| **alias** | Short lowercase name. What you type at `--alias`. |
| **backend-path** | Path in the Bitwarden Secrets Manager workspace. Where the value lives. |
| **env-var-name** | Env var name the child process will see. Typically the SDK convention. |

The three columns decouple three concerns: *what you type* (alias), *where the value lives* (backend path), *what your code reads* (env var). Each can change independently without breaking the others.

Other config — Bitwarden project name, OS keyring account name, audit-log retention — is set in the AiPlus profile's `privacy-and-secrets.md` preferences and is not in the TSV.

---

## 7. CLI surface

All commands are `aiplus secret-broker <subcommand> [args]`:

| Subcommand | Description | Output |
|---|---|---|
| `status` | Is the broker installed and configured? Which backend? Is auth current? | `SECRET_BROKER_STATUS=PASS\|NEEDS_FIX` + structured key/value (no secret values) |
| `doctor` | Validate config, test backend reachability, check OS keyring access, check `bws` CLI presence. | `SECRET_BROKER_DOCTOR_STATUS=PASS\|NEEDS_FIX` + reasons |
| `list` | List all configured aliases. | `alias -> backend-path -> env-var-name` rows. No values. |
| `resolve <alias>` | Resolve one alias (positional argument). | resolution metadata (alias, provider, backend path). NO value by default. `--print` requires explicit opt-in. |
| `run --alias <alias> -- <cmd ...>` | Resolve one alias, spawn child process with env var injected. | `SECRET_BROKER_RUN_STATUS=PASS` + child stdout/stderr; exits with child's exit code |
| `run --aliases A,B,C -- <cmd ...>` | Same but with multiple aliases. | Same |
| `token set` | Write a new access token into the OS keyring. Reads token from **stdin** only. | confirmation; never echoes the token |
| `token delete` | Remove the stored access token from the OS keyring. | confirmation |

### Backwards-compatibility note

For v0.1, the CLI subcommand name remains `secret-broker` (not `key`) to avoid breaking existing AiPlus users. The README and DESIGN.md frame the feature as "agent key management" because that is the audience-facing concept, but the binary keeps its current name.

---

## 8. Threat model

| Threat | Mitigation |
|---|---|
| Secret committed to git | `.gitignore` blocks `*.env`, `*.token`, `*.key`, `*.credentials`, `*.bw-export`; no value-bearing config in any AiPlus-managed path |
| Secret in shell history | Default `resolve` does not print values; `run` does not require typing the value at all |
| Secret in agent context / transcript | `run` injects via env vars, not via prompt text; agent never sees the value as a literal |
| Secret in `tmux`/screen scrollback | Default output omits the value; `--print` is opt-in and explicit |
| Stale cached value | No cache; every call resolves fresh from backend |
| Bitwarden access token leaked from disk | Token never on disk; lives in OS keyring only |
| Wrong key fired (account confusion) | Aliases force the Owner to name *which* account before resolution; no ambient default |
| Stale alias mapping after rotation | Rotation is at the backend; mapping does not change; first call after rotation gets new value |
| Child process logging the env var to disk | OUT OF SCOPE — the child's own behavior is its responsibility |
| Backend access token compromised | OUT OF SCOPE — recovery is Bitwarden-side: revoke the access token, generate a new one, store under same OS keyring account |
| User screen-recording the terminal during `--print` | Documented warning; explicit opt-in flag |

---

## 9. Privacy & safety boundaries

`aiplus-agent-key` does NOT:

- Upload any data anywhere except the configured backend.
- Persist resolved secret values to disk, log, or in-memory cache between invocations.
- Print secret values by default.
- Resolve any alias not declared in the Owner's profile config.
- Read or write `~/.codex`, `~/.claude`, or any other runtime's global config.
- Modify any project file outside `.aiplus/` and `.aiplus/modules/aiplus-agent-key/`.

It DOES:

- Talk to the Bitwarden Secrets Manager API on every resolve.
- Read the access token from the OS keyring once per invocation.
- Spawn child processes with selected env vars set.
- Write structured (no-secret-values) status to stdout in commands like `doctor` and `list`.

---

## 10. STOP gates

Actions that always escalate to the Owner; never auto-approved:

1. **`token set`** — writing a new Bitwarden access token into the OS keyring. The Owner must pipe the token in via stdin; the broker does not fetch or generate it.
2. **`token delete`** — clearing the Bitwarden access token. Effectively logs the broker out.
3. **Adding a new backend type** (1Password / Vault / AWS) — v0.2 feature, requires Owner-supplied configuration.
4. **Any operation that would write secret values to disk** — refused at all times; if a user passes a flag that would do this, the broker errors out.

---

## 11. Integration with AiPlus and other modules

`aiplus-agent-key` is a runtime utility, not a multi-agent role player. It does not participate in `aiplus agent ...` orchestration commands. It composes with other AiPlus modules as follows:

- **AiPlus main CLI** — ships the `secret-broker` subcommand binary in v0.1.
- **AiPlus-Agent-Memory** — agent memory should NEVER record secret values. The broker enforces this by never letting memory layer read its output.
- **AiPlus-Compact-Reminder** — when an agent is compacted, the broker's resolved values are *not* in the compact handoff. Only alias names appear if the agent's own code mentions them.
- **AiPlus-Agent-Team / AiEconLab** — agents in these teams call the broker via `aiplus secret-broker run` from inside their worktree commands. The broker's safety boundaries apply transparently.

---

## 12. MVP roadmap

**v0.1 (this release):**
- Public-facing module published as a sibling repo
- Documents the existing `aiplus secret-broker` subcommand
- Bitwarden Secrets Manager backend
- All 7 subcommands working (`status`, `doctor`, `list`, `resolve`, `run`, `token`)
- Acceptance schema + structural invariants test

**v0.2 (planned):**
- Source-code extraction into standalone Rust crate (`aiplus-secret-broker-core`), still consumed by AiPlus CLI
- 1Password backend
- AWS Secrets Manager backend
- HashiCorp Vault backend
- env-file fallback backend (for offline development; refuses to load if file is in any tracked git path)
- Auditable resolve log (records alias names + timestamps; never values)
- Token-refresh automation for OAuth tokens (PAT regeneration is still manual)

**v0.3 (speculative):**
- Pluggable backend protocol so third parties can write backends
- Cross-machine alias config sync (read-only, alias names + backend paths only, no values)

---

## 13. Known limitations

- **One profile at a time.** v0.1 assumes one active AiPlus profile. Multi-profile alias sets need explicit profile switching.
- **Bitwarden-only in v0.1.** Other backends require waiting for v0.2.
- **No usage limit / rate-limit.** A misbehaving script can call `run` in a tight loop and exhaust Bitwarden API quota. v0.2 may add a soft local rate limit.
- **No automatic alias discovery.** The Owner must manually edit `secret-aliases.tsv`. v0.2 may add `aiplus secret-broker scaffold` to interactively build the config.
- **No multi-platform OS keyring abstraction beyond macOS Keychain.** v0.2 adds explicit support for Linux Secret Service and Windows Credential Manager.

---

## 14. Decisions

| Decision | Choice | Why |
|---|---|---|
| Alias case convention | SCREAMING_SNAKE_CASE | Matches env-var convention; reduces friction; reads as a name not a value |
| Backend prefix in mapping | `bw:`, `op:`, `vault:`, etc. | Lets one alias config target multiple backends; avoids implicit defaults |
| Access token storage | OS keyring | Mature, OS-level encryption; standard tooling expectation |
| Default output of `resolve` | env-var line WITHOUT value | Print-by-default is the most common secret-leak vector |
| Child-process injection | env vars, not args, not stdin | Env vars are not visible in `ps`/`/proc/<pid>/cmdline`; args and stdin are |
| Resolution caching | none | Stale secret > slow resolve; the latency cost is in the tens of ms range |
| Public module today, source extraction later | yes | v0.1 ships the public face fast; v0.2 does the Rust refactor when the API is stable |
| Module name `aiplus-agent-key`, subcommand `secret-broker` | yes | Don't break existing users; the README explains the relationship |

---

## 15. Acceptance criteria

The acceptance schema at `.aiplus/agent-key/acceptance/v0.1.0/schema.yaml` is binding. Every release must pass:

1. `aiplus-module.json` loads, declares `name = "agent-key"`, declares `runtimeAdapters = ["codex", "claude-code", "opencode"]`.
2. `README.md` and `README.zh-CN.md` are present and substantive (≥ 5KB each).
3. `DESIGN.md` is present and substantive (≥ 8KB).
4. `core/example-aliases.tsv` is present and is a parseable TOML file.
5. `core/example-aliases.tsv` contains NO real secret values, NO real Bitwarden workspace IDs, NO real provider account names beyond example placeholders.
6. `core/alias-conventions.md` is present.
7. `tests/acceptance.test.sh` is present and executable; exits 0 on a fresh clone.
8. The `.gitignore` blocks `*.env`, `*.token`, `*.key`, `*.credentials`, `*.bw-export`.
9. CI workflow at `.github/workflows/ci.yml` runs `acceptance.test.sh` on push and PR.
10. No file in the repo contains a string that looks like a real OpenAI / Anthropic / Bitwarden secret (heuristic check: 32+ alphanumeric chars matching common API-key patterns).

Any behavioral change must update both the schema and its sibling `.test.sh`.

---

End of v0.1.0 design document.
