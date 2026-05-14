# /at-route — Dispatch a task through the Agent Team

Use this command to make an Agent Team dispatch a real, audited artifact
rather than narrative. Records the routing decision to
`.aiplus/agents/dispatch-log.jsonl` so the Owner can review what was
sent where.

## How it works

1. Score the task: LIGHT (typo / one-line clarification),
   MEDIUM (single-feature change with 2–3 risk axes),
   HEAVY (architecture-touching, security-sensitive, or production-deploy).
2. Identify the right primary role from the agent-team roster:
   - Implementation work → `agent-team-engineer-a`
   - Code review of an existing diff → `agent-team-reviewer`
   - Behavior validation → `agent-team-qa`
   - Architecture or system design → `agent-team-architect`
   - Scope or acceptance criteria → `agent-team-pm`
   - Strategic should-we question → `agent-team-advisor`
   - Sequencing / staffing / status → `agent-team-ceo`
3. For MEDIUM/HEAVY, identify any functional experts to consult
   (security-reviewer, ai-integration, devops, ui-designer,
   tech-writer, researcher).
4. Run `aiplus agent route <role> "<task>"` for the primary role.
5. Surface the dispatch decision and which experts (if any) the CEO
   should additionally consult.

## Examples

```text
/at-route fix the failing CI test on the secret-broker integration
/at-route review my OAuth refactor diff
/at-route is this RAG embedding choice actually the right one
```

## When to use

- Whenever the team is about to do non-trivial work and the dispatch
  decision should be on the record.
- When the CEO needs to staff a multi-role task and wants the routing
  to be auditable.
- When the Owner asks "who's working on this" — `dispatch-log.jsonl`
  has the answer.
