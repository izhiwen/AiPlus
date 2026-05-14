# /aiel-talk — Open a session as a specific AEL role

Use this command to load a specific AEL role's persona as the active
operating context. Equivalent to spawning the matching subagent
(`aieconlab-<role>`) but called explicitly so the role swap is logged.

## How it works

1. Resolve the role from the user's argument. Accept any alias from
   `.aiplus/agents/<role>.toml`'s `chinese_aliases` / `english_aliases`.
2. Run `aiplus agent talk <role>` to open the role's worktree and load
   the persona file (`.aiplus/agents/personas/<role>.md`) as system
   prompt. Read team memory + project memory + the role's personal
   memory under `.aiplus/agent-memory/<role>/`.
3. Acknowledge the role switch with the role's display name and voice
   before continuing.

## Examples

```text
/aiel-talk pi          # Principal Investigator
/aiel-talk theorist    # Theorist
/aiel-talk ra-stata    # RA-Stata
/aiel-talk referee     # Internal referee
/aiel-talk llm-measurement  # LLM-as-measurement expert
/aiel-talk 主作者       # → resolves to pi via Chinese alias
```

## Hand-off discipline

- After switching roles, do not silently inherit context from the
  previous role. State which memory you read (team / project / personal)
  before producing work.
- When the role's escalation target (e.g. PI, Owner) needs to weigh in,
  do not auto-act on their behalf — pause and surface the question.
- When done, write a short decision/handoff record via `aiplus memory
  add --kind decision` so the next role can pick up cleanly.
