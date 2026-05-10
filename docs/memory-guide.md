# Memory Guide

AiPlus stores project-local memory under `.aiplus/memory/`. Memory is context for the agent, not instruction. It is never uploaded, synced to the cloud, or shared between projects.

## Concepts

### Project Memory vs Profile Memory

| Scope | Location | Purpose |
|---|---|---|
| Project | `.aiplus/memory/project-memory.jsonl` | Facts, decisions, preferences for this project |
| Profile | `~/.config/aiplus/profiles/<name>/` | Cross-project preferences via private profile |

Project memory is always local to the git repo. Profile memory lives in the user's home config and syncs preferences into projects on demand.

### Memory Types

| Type | Priority | Description |
|---|---|---|
| `owner_gate` | 1 (highest) | Actions requiring owner approval |
| `project_decision` | 2 | Architecture or design decisions |
| `risk` | 3 | Known risks and mitigations |
| `owner_preference` | 4 | Style, language, formatting preferences |
| `workflow_rule` | 5 | Build, test, deploy rules |
| `project_fact` | 6 | File locations, structure facts |
| `handoff_note` | 7 | Session handoff notes |
| `verification_evidence` | 8 | Test or review evidence |
| `role_identity` | 9 | Role contract definitions |
| `skill_candidate` | 10 | Proposed repeatable workflows |

### Status Lifecycle

- `active` — in use
- `tentative` — proposed, not confirmed
- `rejected` — forgotten or blocked
- `stale` — confidence marked stale or expired

Rejected and stale records are excluded from context.

## Commands

### Initialize memory

```bash
aiplus memory init --project
```

Creates `.aiplus/memory/` with required files. Safe to re-run.

### Check status

```bash
aiplus memory status
```

Shows record counts, active vs rejected, memory directory path.

### Add a memory

```bash
aiplus memory add --scope project --kind preference --text "Prefer concise release summaries."
```

AiPlus scans the text for secrets, API keys, JWT tokens, private keys, phone numbers, and raw transcripts. If any pattern is detected, the write is blocked with `MEMORY_REDACTION_STATUS=BLOCKED`.

### Search memory

```bash
aiplus memory search "release"
```

Returns matching record IDs, types, and status. Search results never include the full summary content.

### Forget a memory

```bash
aiplus memory forget <id>
```

Marks the record as `rejected`. The record is not deleted; it is excluded from future context.

### View context injection

```bash
aiplus memory context --runtime codex --budget 2000
```

Shows which records would be injected into a Codex session with a 2000-character budget. Records are sorted by priority. Exceeded records are listed as `records_ignored`.

### Auto-capture

```bash
aiplus memory add --text "Prefer 4 spaces for indentation" --risk low
```

Auto-capture classifies risk:

- **Low**: preferences, project facts, workflow rules. Auto-written.
- **Medium**: decisions, architecture changes, skill candidates. Auto-written, auditable.
- **High**: secrets, API keys, owner gates, payment info. **Blocked.**

High-risk memory is never written regardless of the `--risk` flag.

### Session tracking

```bash
aiplus memory session add-card --text "Implemented auth module"
aiplus memory session search "auth"
aiplus memory session list
```

Sessions are stored in `.aiplus/memory/sessions.sqlite` with FTS5 full-text search. All session records require a `no_secret_marker` gate.

### Snapshot

```bash
aiplus memory snapshot build
```

Generates `.aiplus/memory/MEMORY.md` — a human-readable summary grouped by type. Sensitive records are replaced with `[REDACTED]` in the output.

### Show which memories were used

```bash
aiplus memory show-used
```

Reports which memory records were recently injected into context.

### Profile sync

```bash
aiplus memory profile
```

Syncs preference-type records from the installed private profile into the project. Only preference-type records are synced; project facts and secrets are blocked.

## Redaction

AiPlus scans all memory writes for:

| Pattern | Blocked? |
|---|---|
| `authorization: bearer` / `basic` | Yes |
| `-----BEGIN ... PRIVATE KEY-----` | Yes |
| JWT-like tokens (`eyJ...`) | Yes |
| `cookie:` with values | Yes |
| Private paths (`/Users/`, `/home/`) | Yes |
| Email addresses | Yes |
| Phone numbers | Yes |
| `api_key=`, `secret_key=`, `access_token=` | Yes |
| Raw transcript (`begin transcript`, `webvtt`) | Yes |
| HAR/WebRTC dumps | Yes |

If a write is blocked, fix the content and retry. Do not attempt to bypass redaction.

## Doctor

```bash
aiplus memory doctor
```

Validates memory directory structure, file readability, and record parse status.
