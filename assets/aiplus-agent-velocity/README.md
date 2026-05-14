# AiPlus Agent Velocity
[![CI](https://github.com/izhiwen/aiplus-agent-velocity/actions/workflows/ci.yml/badge.svg)](https://github.com/izhiwen/aiplus-agent-velocity/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

[中文 README](README.zh-CN.md)

## The pain

You ask the agent to do a refactor. It says **"five hours"**. You plan your
afternoon around it. Twenty minutes later, it reports completion.

Next week, a similar task. The agent says five hours again. Again it finishes
in twenty minutes. After a month, you stop trusting any estimate the agent
gives you, and you also stop scheduling the rest of your day around AI work
because the numbers feel made up.

The cause is straightforward: the agent estimates in **human-engineer hours**
because that is what its training data anchors on. It is mis-billing its own
work, and nobody is keeping the score.

## What we do about it

AiPlus Agent Velocity records every estimate and every actual completion as
local JSONL under `.aiplus/velocity/`. It learns from your own history what
the AI-native time really is, and feeds that back into the next estimate.

The records cover:

- **Human estimate** — what the agent first predicted, in human-engineer time
- **Actual completion** — how long it really took, end to end
- **Task type** — refactor, feature, bug fix, review
- **Model + workflow labels** — for context, so a "feature" on Opus 4.7 with
  a heavy review loop doesn't get averaged with a quick scratch script

After a few records the system surfaces:

- **p50** — the median AI-native time for this kind of task
- **p90** — the conservative upper bound
- **Human-time bias detection** — when an estimate is anchored on engineer
  hours and the actual completion is far below it, it is flagged
- **Next-estimate adjustment** — a multiplier the agent applies to its
  *next* human-style estimate so the new number is calibrated, not guessed

No raw prompts are stored. No data uploads. Normal records rotate at 200
entries; rare cases at 20.

## Quick start

If you already have AiPlus installed:

```bash
cd MyProject
aiplus install codex          # or: claude-code, opencode, all
aiplus velocity init
```

Then the CLI:

```bash
aiplus velocity init                                # initialize tracking
aiplus velocity estimate                            # produce an AI-native estimate
aiplus velocity complete                            # log the actual completion time
aiplus velocity bias --task <id>                    # check bias on a specific task
aiplus velocity report                              # overall bias + adjustment (default: --scope both)
aiplus velocity report --scope local                # project-only report
aiplus velocity report --scope global               # cross-project report
aiplus velocity doctor                              # health checks (incl. global ledger)
aiplus velocity purge --yes                         # manual purge of old records
aiplus velocity import-from-project <path>         # backfill global ledger from existing project
```

## What's inside

- `core/schemas/` — JSON schemas for `config`, `estimate-record`,
  `run-record`, `rare-case-record`
- `core/` — duration parser, bias detection, retention logic
- `DESIGN.md` — architecture decisions and design rationale

## Storage

Two tiers — project-local and cross-project.

```
<project>/.aiplus/velocity/
  config.json           # configuration (incl. share_to_global_mode)
  estimates.jsonl       # estimate records (full, incl. free-text task)
  runs.jsonl            # completion records
  rare-cases.jsonl      # rare case records (large overestimate, owner gate, etc.)
  multipliers.json      # aggregate adjustment multipliers
  rotation-state.json   # rotation tracking

~/.config/aiplus/velocity/          # XDG; %APPDATA%\aiplus\velocity\ on Windows
  config.json           # global retention + health knobs
  estimates.jsonl       # structural projection — NO free text, paths, project name
  runs.jsonl            # structural projection
  rare-cases.jsonl      # structural projection
  anchor-signals.jsonl  # structural projection
```

- No SQLite. No database.
- Project: keep latest **200** normal + **20** rare.
- Global: keep latest **1000** normal + **100** rare.
- Dir mode `0700`, files `0600`. Each record `< 4096` bytes (atomic
  `O_APPEND`).
- Aggregate multipliers survive raw record rotation, so the calibration
  doesn't reset just because old runs aged out.

### Cross-project sharing

By default a project both writes to and reads from the global ledger,
so a fresh project benefits from your bias history on day one. Toggle
in `config.json`:

```jsonc
{
  "schemaVersion": "2",
  // ...
  "shareToGlobalMode": "read_write"  // read_write | read_only | none
}
```

The global ledger **physically cannot contain** the free-text `task`
description, file paths, project name, or any cwd-derived field —
those are dropped at projection time. The merge rule when estimating
is "project-recent-heavy": newest 50 project records + newest 150
global records, deduped by id.

## Safety boundaries

AiPlus Agent Velocity does not:

- store raw prompts, transcripts, or source code
- upload data or implement telemetry
- replace tests, review, or Owner gates
- act as a productivity tracker or KPI dashboard
- shorten estimates as an excuse to skip verification

## More

- Main platform: [aiplus](https://github.com/izhiwen/aiplus)
- Tracked work before next release:
  [v0.5.2 known gaps](https://github.com/izhiwen/aiplus/blob/main/docs/roadmap/v0.5.2-known-gaps.md)

## License

[Apache-2.0](LICENSE)
