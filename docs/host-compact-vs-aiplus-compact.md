# Host Compact vs AiPlus Compact

## Purpose

Clarify the relationship between host-level compact (Codex/Claude Code built-in) and AiPlus Compact Reminder. Help Owners understand when to use each and how they complement each other.

## Host Compact (Codex / Claude Code / Cursor)

### What It Does

- **Checkpoint:** Saves current project state to a checkpoint file
- **Summary:** Generates a natural language summary of work done
- **Resume:** Reads checkpoint + summary to resume work
- **Trigger:** Manual (`/compact`) or automatic (after N messages)

### Strengths

- Integrated into the agent's own context window
- Works across any project without setup
- Simple: one command, done

### Limitations

- No structured Owner gates
- No decision log
- No risk tracking
- No savings estimation
- No context capsule format
- Resume relies on agent re-reading full summary
- No project-local continuity across sessions

---

## AiPlus Compact Reminder

### What It Does

- **Handoff Markdown:** Structured `current-handoff.md` with sections (Session Role, Current Goal, Open Blockers, Owner Gates)
- **Readiness Scoring:** Evaluates whether compact is safe based on validation rules
- **Owner Gates:** Explicit approval checklist (publish, deploy, global config, external accounts, secret exposure)
- **Context Capsule:** Machine-readable `context-capsule.json` with hot/warm/cold tiers, decisions, risks, next actions
- **Savings Ledger:** Tracks estimated tokens and cost saved per compact
- **Watch/Remind:** Periodic or on-demand readiness checks with recommendations

### Strengths

- Owner-controlled and explicit
- Structured, not just a summary
- Gates prevent accidental high-risk actions
- Capsule enables better resume (decisions, risks, next actions)
- Project-local: works offline, no cloud dependency
- Token/cost savings awareness

### Limitations

- Does not actually perform the host compact (does not trigger `/compact`)
- Requires manual two-step workflow
- Resume currently reads handoff, not capsule (v2.1 fix planned)
- No automatic trigger (must run `aiplus compact watch` or `remind`)

---

## Comparison Matrix

| Capability | Host Compact | AiPlus Compact | Notes |
|------------|-------------|----------------|-------|
| Create checkpoint | Yes | No | Host owns checkpoint creation |
| Generate summary | Yes | No | Host generates natural language summary |
| Structured handoff | No | Yes | AiPlus adds markdown sections |
| Owner gates | No | Yes | Explicit approval checklist |
| Decision log | No | Yes | AiPlus tracks decisions in capsule |
| Risk tracking | No | Yes | AiPlus tracks active risks |
| Context capsule (machine-readable) | No | Yes | JSON format for resume |
| Readiness scoring | No | Yes | Validates before compact |
| Savings estimation | No | Yes | Token and cost tracking |
| Automatic trigger | Yes (after N messages) | Partial (watch mode) | Host auto-triggers; AiPlus requires manual watch |
| Resume from checkpoint | Yes | Partial | Host resumes from its own checkpoint; AiPlus resume reads handoff |
| Cross-session continuity | Limited | Yes | AiPlus project-local files persist |
| Offline support | N/A | Yes | AiPlus works without internet |

---

## Recommended Workflow

### Standard Session End

```
1. Run `aiplus compact remind` to check readiness
   → If readiness is READY_TO_COMPACT, proceed
   → If BLOCKED, fix blockers first

2. Perform host compact (`/compact` in Codex/Claude)
   → This creates the actual checkpoint

3. Run `aiplus compact prepare`
   → Creates/updates handoff, capsule, and savings ledger
   → Validates Owner gates

4. Review `.codex/compact/current-handoff.md`
   → Verify session summary, decisions, blockers

5. Run `aiplus compact watch --once` to confirm
   → Should show "reminderDecision": "proceed"
```

### Session Resume

```
1. Run `aiplus compact resume`
   → v0.5.1: reads from handoff.md
   → v2.1 (planned): reads from context-capsule.json

2. Resume host session (Codex/Claude reads its checkpoint)

3. Run `aiplus memory context` to inject project memory
```

---

## What AiPlus Adds That Host Cannot

1. **Owner Gates:** Explicit checklist prevents accidental publishing, deploying, or secret exposure
2. **Decision Preservation:** Decisions are extracted and stored in capsule, not buried in a summary paragraph
3. **Risk Awareness:** Active risks are tracked and surfaced on resume
4. **Cost Consciousness:** Savings ledger makes token/cost tradeoffs visible
5. **Project Continuity:** Files live in the repo, not in the agent's ephemeral context

## What Host Adds That AiPlus Cannot

1. **Actual Checkpoint:** Host creates the binary/structured checkpoint that the agent reads
2. **Automatic Trigger:** Host compact fires after N messages without user action
3. **Agent-Native Resume:** Host resume is seamless — agent just continues
4. **Cross-Project:** Host compact works on any project immediately

---

## Conclusion

AiPlus Compact Reminder is **not a replacement** for host compact. It is a **complement** that adds Owner control, structure, and continuity on top of the host's checkpoint mechanism.

The ideal workflow uses both: host for the actual checkpoint, AiPlus for the structured handoff, gates, and capsule.

---

## v2.1 Plans

- `compact resume` reads from `context-capsule.json` (not handoff)
- `compact trigger-host` command to suggest/invoke host compact
- Link savings estimation to actual host compact metadata
- Document this workflow in README with examples
