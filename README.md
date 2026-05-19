# AiPlus
[![CI](https://github.com/izhiwen/AiPlus/actions/workflows/ci.yml/badge.svg)](https://github.com/izhiwen/AiPlus/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

[中文 README](README.zh-CN.md)

I've been pair-programming with AI coding agents full-time for the better
part of a year — Codex one day, Claude Code the next, OpenCode for the
long-running stuff. About four months in, I caught myself explaining the
same architectural decision to the same agent for the fourth time that
week — and noticed I was also re-pasting the same API key into the same
agent for the fourth time that week. I was losing hours every day to the
same seven coordination failures: cross-session amnesia, tokens burned
fighting `/compact`, agents racing to lead the same task, estimates
anchored to human-engineer hours, plans that skipped security until
release week, one agent wearing every hat in one context window, and
re-handing the agent my API keys every session. AiPlus is the seven small
Rust modules I built to treat all seven (Agent Team treats two of them).
The honest meta-frame: **I used AI agents to build the toolchain that
manages AI agents.** What's here works for my workflow today; what isn't
yet is in `docs/roadmap/`.

![AiPlus 30-second tour](docs/demo.gif)

## The pains we are tired of

If you spend your days driving AI coding agents, these probably feel familiar:

1. **The agent forgets everything between sessions.** Monday you teach it your
   naming conventions. Wednesday it asks again. By Friday you have explained
   the same architectural decision four times.
2. **Long tasks burn tokens fighting `/compact`.** You hit the token wall
   mid-feature. Either you waited too long to `/compact` and the agent has
   been re-reading bloated history on every turn for hours, or you
   `/compact`-ed at the wrong moment and the next session burns its first
   20% re-explaining what was already decided. Compact-without-preparation
   is one of the highest token costs in long coding sessions, and it shows
   up on every monthly bill.
3. **Multiple agents step on each other's feet.** No one defined who is the
   CEO, who reviews, who builds. Three agents try to lead the same task.
4. **Estimates anchor on human-engineer hours.** The agent says "five hours"
   for a refactor. Twenty minutes later it is done. Next week, same task, same
   five-hour quote. No one keeps the score.
5. **When agents plan, they overlook the things that matter most** — onboarding
   ease, security and privacy, real execution pitfalls, AI integration
   considerations. You only find out at release week, or worse, from a user
   complaint.
6. **One agent wears every hat.** CEO, reviewer, builder, advisor — all crammed
   into the same context window. Roles **drift**, context **pollutes** across
   hats, and the agent does each hat **shallowly**. Real engineering teams
   divide labor because the work *is* that structured.
7. **Every agent session, you re-hand the agent your API keys.** New project,
   new chat, new wrapper script — and every time you are back to
   copy-pasting `OPENAI_API_KEY=...`, exporting env vars in a fresh shell,
   editing a `.env`, or pasting the key directly into a prompt "just for
   this task". The setup never amortizes. Worse, the key ends up in
   transcripts, `.env` files, shell history, screenshots, and CI logs —
   one accidental commit or screen-share and it's exposed.
8. **Every new project starts the agent from zero.** Pain #1 was about
   forgetting within one project; this is the across-project layer. Six
   months tuning the agent to your workflow in Project A — naming style,
   review tone, role identities, tooling preferences — none of that
   carries to Project B. Each project starts with the agent re-meeting
   you. The "how I work" baseline has no home above a single project.

AiPlus is seven core Rust modules that together fix the seven in-project
failure modes (Agent Team treats both #3 multi-agent collision and #6
single-agent role-drift). The eighth — cross-project preference amnesia
— is treated by the [**AiPlus-Work-with-Me**](https://github.com/izhiwen/AiPlus-Work-with-Me)
Companion template described below. Plus one optional, opt-in module —
AiEconLab — for applied-economics research, see below.

## What you get

**Agent Memory** — The agent stops forgetting. Project conventions, naming
rules, architectural decisions are stored as local JSONL under
`.aiplus/memory/`. Twelve redaction patterns strip secrets before any record
is written, so you can capture preferences without leaking them.

**Compact Reminder** — Save tokens on long sessions. Long Codex / Claude
Code / OpenCode sessions bleed tokens two ways: agents that don't `/compact`
in time overflow context and re-read bloated history on every turn, and
agents that `/compact` at the wrong moment lose state and spend the next
session re-explaining what they already knew. This module reminds you
when the timing is right (token threshold + task-handoff-point detection),
auto-prepares a structured handoff before, and recovers from a
checksum-verified capsule after — so tokens go into new work, not into
re-establishing context.

**Agent Key** — Stop telling the agent your keys every time. **Free,
zero-config default**: each key lives in your OS keyring (macOS
Keychain / Linux Secret Service / Windows Credential Manager), never
on disk. One-time per machine:

```bash
aiplus secret-broker set --alias openai --auto-prompt   # native OS dialog
# or: echo -n "$YOUR_OPENAI_KEY" | aiplus secret-broker set --alias openai
# Repeat once per provider (anthropic, github, …)
```

From then on every Codex / Claude Code / OpenCode session in any
project picks up the key automatically:

```bash
aiplus secret-broker run --aliases openai,anthropic -- python my_agent.py
# OPENAI_API_KEY + ANTHROPIC_API_KEY available in env; cleared on exit
```

**Cross-project share works in two layers**:

1. **Machine-wide** (always on): one `aiplus secret-broker set` per
   alias on this machine; every future `aiplus secret-broker need
   <alias>` from any directory resolves silently from the OS
   keyring. The agent never re-asks, and `need` works even in a
   fresh directory that has never run `aiplus install`.
2. **cd-auto-load** (per project, opt-in): the shell hook installed
   by `aiplus install --yes` (default `[Y/n]` prompt) auto-exports
   `*_API_KEY` env vars when you `cd` into a project that lists the
   alias in its `.aiplus/keys.toml`. To get this ergonomic flow in
   a new project, run `aiplus install <runtime>` there once.

No copy-paste, no `.env` shuffling, no key in prompts. (Bonus: values
never default-print, never enter git history.) For multi-machine sync
or team sharing, opt in to the Bitwarden Secrets Manager backend
(`export AIPLUS_SECRET_PROVIDER=bws`) — same alias surface, paid
subscription required.

**Auto Team Consultant** — The agent stops overlooking the important stuff.
A virtual team (5 expert members + your project's user personas, sitting at
the same table) is consulted before every non-trivial plan. A coordinator
scales the consult by complexity and risk so you get the value of a real
review team without paying the cost on every commit.

**Agent Velocity** — The agent stops mis-billing its own work. Every estimate
and actual completion time is logged as local JSONL. Human-time bias is
detected automatically. Future estimates use AI-native p50 and p90 numbers
calibrated from your own history.

**Agent Team** — Replace single-agent **drift** with a permanent team.
Advisor, CEO, Architect, PM, two Engineers, Reviewer, and QA — each with its
own persona, workspace, and memory namespace. A coordinator routes work to the
right role, keeps transcripts, and prunes stale worktrees so your project stays
clean. No more role pollution, no more shallow-each-hat. **The team comes with:**

- **Plain-language role switching** — say "you are CEO", "take the
  reviewer role", or "switch to PI" mid-session and the agent will
  respond as that role with its memory loaded. No CLI command needed.
  Works in Codex, Claude Code, and OpenCode interactive mode.
- **Intent-aware safety gate** — before doing anything risky
  (deleting files, publishing changes, running protected commands),
  the coordinator understands what you're actually asking for, not
  just the words you typed. Rephrasing or putting things in quotes
  can't bypass the check.
- **Parallel review and QA** — the review step and the QA step run
  side by side, and each role's workspace stays ready between tasks
  instead of being rebuilt every turn. Typical iterations land in
  ~8-10 min instead of ~15-20, same quality bar.
- **Adaptive staffing** — the coordinator reads each task, scores
  its complexity (1-5) and risk (0.0-1.0), and scales the team
  accordingly: trivial questions get answered directly, mid-size
  changes pull in one engineer, high-risk or large work fires a
  full HEAVY team. High-risk tasks also pull a reviewer (and QA at
  very high risk) regardless of tier. Run `aiplus agent route
  --score-only "<task>"` to pre-flight any task without spending
  tokens.
- **LLM-judged expert auto-summon** — when a task touches a domain
  with a designated expert (security / docs / LLM integration),
  the coordinator asks a small classification model "does this
  task match this expert's intent?" and joins the matching expert
  to the team. Replaces brittle keyword matching with semantic
  understanding. Configure new experts by adding an `intent_hint`
  string to their role TOML.
- **HEAVY tasks dispatch in parallel** — when the coordinator
  staffs 6 roles for a HEAVY task, they run concurrently across
  runtimes instead of one-at-a-time. ~5.7× faster end-to-end
  dispatch overhead than serial.
- **Tamper-evident audit log** — every coordinator decision is
  written into a sha256-chained log. `aiplus agent audit
  verify-log` walks the chain and surfaces any line that was
  edited or removed after the fact.
- **Hardware-backed commit signing** — on macOS, `aiplus identity
  setup-signing` configures git to sign your commits with a
  Secure Enclave-backed SSH key. Passwordless, biometric-gated,
  no YubiKey purchase needed.
- **Token cost rollups** — `aiplus agent token-cost` reads the
  dispatch log and shows tokens consumed plus USD cost for the
  past 1-hour / 8-hour / 24-hour windows, with a top-5 most
  expensive tasks. Pricing comes from a community-maintained
  per-model table, with offline fallback and a local override for
  enterprise rates.

**Companion: [AiPlus-Work-with-Me](https://github.com/izhiwen/AiPlus-Work-with-Me)** —
where the seven modules above are *project-local*, the AiPlus-Work-with-Me template
is the **user-level profile bundle** that layers on top: your collaboration
style, project map, role identities, tooling preferences — captured once,
then inherited across every project. Fork it, fill in the placeholders,
then `aiplus profile install AiPlus-Work-with-Me --user --yes`. It is **not**
auto-installed by `aiplus install` — it is the explicit fork-and-personalize
opt-in for cross-project (not just cross-session) preference memory.

Everything stays inside `.aiplus/` in your project. Nothing uploads. Nothing
syncs to a cloud. Nothing edits your global agent config.

## Who is this for

AiPlus serves two audiences with the same underlying agent substrate:

- **Software engineers** building with Codex / Claude Code / OpenCode.
  The default `aiplus install` bootstrap installs the SWE-tuned team
  (Advisor, CEO, Architect, PM, 2× Engineer, Reviewer, QA) plus 11
  software-engineering experts.
- **Applied-economics researchers** building papers, replication
  packages, and LLM-as-measurement workflows.
  Run `aiplus add aieconlab` to install [**AiEconLab (AEL)**](https://github.com/izhiwen/AiEconLab) —
  8 research roles (Advisor, PI, Theorist, PM, RA-Stata, RA-Python,
  Referee, Replicator) + 14 experts including LLM-as-Measurement
  Specialist. Replaces the SWE consultant team with one designed from
  first principles for plan-time econ review.

Both audiences share seven bundled substrate modules:
`aiplus-agent-memory`, `aiplus-compact-reminder`,
`aiplus-auto-team-consultant`, `aiplus-agent-team`, `aiplus-agent-key`,
`aiplus-agent-velocity`, and `aiplus-token-cost`.

## Install

### Option A — pre-built binary (recommended, v0.6.0+)

Pre-built binaries ship for two platforms: **Apple Silicon Mac**
(`aarch64-apple-darwin`) and **Intel Windows**
(`x86_64-pc-windows-msvc`). Intel Mac, Linux, and Windows ARM are
not supported — build from source if you need them.

```bash
# Apple Silicon Mac (M1 / M2 / M3 / M4)
curl -L https://github.com/izhiwen/AiPlus/releases/latest/download/aiplus-aarch64-apple-darwin.tar.gz | tar xz
sudo mv aiplus aiplus-token-cost /usr/local/bin/

# Intel Windows (PowerShell)
# Download aiplus-x86_64-pc-windows-msvc.zip from the latest release;
# extract aiplus.exe + aiplus-token-cost.exe, add to PATH.
```

Checksums published at `https://github.com/izhiwen/AiPlus/releases/latest/download/checksums.txt`.

### Option B — install script (curl-pipe-bash)

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/AiPlus/main/install.sh \
  | bash
```

If you'd rather inspect the script before running it:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/AiPlus/main/install.sh -o install.sh
less install.sh
bash install.sh
```

### Install AiPlus into your project

```bash
cd MyProject
aiplus install codex          # or: claude-code, opencode, all
```

Verify:

```bash
aiplus status
aiplus doctor
```

## Runtime support

| Runtime     | Install command              | Where adapters land                      |
|-------------|------------------------------|------------------------------------------|
| Codex       | `aiplus install codex`       | Managed block in `AGENTS.md`             |
| Claude Code | `aiplus install claude-code` | `.claude/` commands                      |
| OpenCode    | `aiplus install opencode`    | `.opencode/` prompts                     |
| All three   | `aiplus install all`         | All adapters                             |

Each adapter is project-local. We do not touch your global config.

## Daily commands

```bash
aiplus status                        # status across all modules
aiplus doctor                        # health checks across all modules

# Memory
aiplus memory status
aiplus memory context --runtime codex --budget 2000

# Compact
aiplus compact prepare               # build handoff + capsule
aiplus compact resume                # restore after compact
aiplus compact savings               # token + cost savings

# Velocity
aiplus velocity estimate --task-type feature --human-estimate 5h
aiplus velocity report

# Agent Team
aiplus agent status              # Show team status
aiplus agent route engineer-a    # Assign task to a specific role
aiplus agent route "<task>"      # Auto-staffed dispatch (coordinator picks roles)
aiplus agent route --score-only "<task>"  # Pre-flight: see who would be staffed, no dispatch
aiplus agent integrate engineer-a # Merge work back
aiplus agent audit run           # Run acceptance audit
aiplus agent audit verify-log    # Verify dispatch-log hash chain (tamper detection)
aiplus agent token-cost          # Tokens + USD cost rollups (1h / 8h / 24h)
aiplus agent token-cost --by-role # Per-role breakdown
aiplus agent talk <role>
aiplus agent invite <role>
aiplus agent dismiss <role>
aiplus agent transcript
aiplus agent prune-worktrees

# Identity (commit signing)
aiplus identity setup-signing --dry-run   # Preview Secure Enclave commit signing setup (macOS)
aiplus identity setup-signing             # Apply it

# Doctor
aiplus doctor                    # Full health check
aiplus doctor --quiet            # Only WARN and FAIL (suppress INFO chatter)

# Updates
aiplus update all
```

## Architecture

```
MyProject/
├── .aiplus/
│   ├── memory/                  # JSONL memory records
│   ├── identities/              # Role identity definitions
│   ├── agents/                  # Agent team role definitions and state
│   ├── agent-memory/            # Agent continuity and context records
│   ├── consultant-team.toml     # Team routing config
│   └── velocity/                # Estimate and run records
├── .aiplus/compact/             # Compact handoffs and capsules
├── .claude/                     # Claude Code adapters (if installed)
├── .opencode/                   # OpenCode adapters (if installed)
└── AGENTS.md                    # Codex managed block (if installed)
```

## The seven bundled standalone modules

Each module ships as its own GitHub repo for inspection / piecewise
adoption AND is auto-installed by `aiplus install` so its schemas,
docs, and adapter content land under `.aiplus/modules/aiplus-<name>/`
in every AiPlus project:

- [AiPlus-Agent-Memory](https://github.com/izhiwen/AiPlus-Agent-Memory) — local JSONL memory + role identity + skill candidates.
- [AiPlus-Compact-Reminder](https://github.com/izhiwen/AiPlus-Compact-Reminder) — **save tokens on long sessions**: detect the right moment to `/compact`, package the handoff before, and resume from a checksum-verified capsule after — tokens go into new work, not re-establishing context.
- [AiPlus-Auto-Team-Consultant](https://github.com/izhiwen/AiPlus-Auto-Team-Consultant) — virtual expert team consulted automatically on each task.
- [AiPlus-Agent-Team](https://github.com/izhiwen/AiPlus-Agent-Team) — standing 8 core + 11 expert roles with persistent identities.
- [AiPlus-Agent-Key](https://github.com/izhiwen/AiPlus-Agent-Key) — **stop telling the agent your keys every time**. Default: free, zero-config OS keyring backend. `aiplus secret-broker set --alias openai` once per machine, then every agent session in any project picks up the key automatically. Opt-in Bitwarden Secrets Manager backend for multi-machine sync / team sharing.
- [AiPlus-Agent-Velocity](https://github.com/izhiwen/AiPlus-Agent-Velocity) — AI-native time estimation (`aiplus velocity` — track estimated vs actual, learn bias, surface calibrated p50/p90).
- [AiPlus-Token-Cost](https://github.com/izhiwen/AiPlus-Token-Cost) — token and USD cost rollups from `.aiplus/agents/dispatch-log.jsonl`; use the standalone `aiplus-token-cost` binary or the bundled `aiplus agent token-cost` command.

## Optional opt-in module

- [**AiEconLab (AEL)**](https://github.com/izhiwen/AiEconLab) — applied-
  economics research team (8 core roles + 14 experts including LLM-as-
  Measurement Specialist; default Python + Stata + LaTeX toolchain).
  Installed via `aiplus add aieconlab`; not bundled by default into
  `aiplus install`. See AEL's README for the install flow and the
  consultant-team replacement it ships.

## Install verb glossary

Two verbs, two distinct purposes:

- **`aiplus install <runtime>`** — wire a runtime adapter (Claude Code /
  Codex / OpenCode) into your project's `.claude/` / `.codex/` /
  `.opencode/` directories. Run once per project per runtime.
- **`aiplus add <module>`** — add a bundled module (agent-memory,
  compact-reminder, auto-team-consultant, agent-team, agent-key,
  agent-velocity, token-cost, aieconlab). The 7 substrate modules are
  auto-installed by `aiplus install`; aieconlab is opt-in via explicit
  `aiplus add`. v0.5.4+ also supports `aiplus add --from-git
  <URL>[@REF]` for third-party modules.

## Safety boundaries

AiPlus stays inside your project. It does not:

- upload project data, prompts, or transcripts
- emit telemetry, sync to cloud, or call external services
- edit global agent configuration (except `aiplus identity
  setup-signing`, which is the one Owner-explicitly-invoked
  subcommand that writes git signing config; it refuses to
  clobber existing config, and `--dry-run` previews what it
  would change)
- store secrets in memory, compact files, or ledgers
- approve Owner-gated actions on its own
- publish packages, create tags, or push releases

Defenses worth knowing about:

- **Dispatch log is hash-chained** — `aiplus agent audit
  verify-log` detects any post-hoc edit or removal in
  `.aiplus/agents/dispatch-log.jsonl`.
- **Mac Secure Enclave commit signing** — opt-in via `aiplus
  identity setup-signing`; the signing key never leaves the
  hardware enclave, so a compromised disk can't be used to
  forge commits.

Validation is structural and heuristic. It is not a safety or compliance
certification.

## Development hooks

Contributors can install the optional local pre-commit hook:

```bash
./scripts/install-hooks.sh
```

The hook refuses commits when `crates/aiplus-cli/Cargo.toml`'s package version
and the `install.sh` fallback version drift apart.

## Private profiles

AiPlus supports optional user-level private profiles for personal preferences
and secret aliases under `~/.config/aiplus/profiles/`. Private profiles are
never bundled into public repositories. See `aiplus profile install` and
`aiplus secret-broker` documentation for details.

For a ready-made fork-and-personalize template that solves cross-project /
cross-session amnesia — your agent remembering your collaboration style,
project map, role identities, and tooling preferences without you re-stating
them every session — see [**AiPlus-Work-with-Me**](https://github.com/izhiwen/AiPlus-Work-with-Me).
It is **not** auto-installed by `aiplus install`; you fork it, fill in the
placeholders (USER.md / sync/projects.toml / secret-aliases.tsv), then run
`aiplus profile install AiPlus-Work-with-Me --user --yes` once.

## Status

Latest release: see [Releases](https://github.com/izhiwen/AiPlus/releases/latest)
(currently `v0.6.0`, pre-built binaries for macOS / Linux / Windows).
Active development on `main`; pre-release notes for the next cut live
under [`docs/roadmap/`](docs/roadmap/).

## License

[Apache-2.0](LICENSE)
