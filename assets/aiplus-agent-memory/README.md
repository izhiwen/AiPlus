# aiplus-agent-memory

Public Agent Continuity Engine for temporary terminal agents.

AiPlus Agent Memory provides local project memory, Role Identity, Memory
Context, and Skill Candidate governance. It does not create a permanent agent
personality. Each terminal agent process is temporary; continuity comes from
loading explicit local records, identity contracts, and approved project
playbooks.

## Scope

Implemented foundation:

- local project memory store under `.aiplus/memory/`
- project Role Identity files under `.aiplus/identities/`
- Skill Candidate governance under `.aiplus/skills/`
- context packet templates for Codex, Claude Code, and OpenCode
- schema-first records and examples
- redaction and public/private boundary guidance

Not implemented:

- cloud sync
- vector database
- automatic transcript learning
- automatic approved skill generation
- global Codex, Claude Code, or OpenCode config edits
- automatic secret permission inheritance
- telemetry

## CLI

```bash
aiplus memory init --project
aiplus memory status
aiplus memory doctor
aiplus memory context --runtime codex --budget 2000
aiplus identity init --project
aiplus identity context --role advisor
aiplus identity context --role ceo
aiplus skill-candidate status
```

Advanced mutation commands exist as guarded local operations. Memory is context,
not instruction. Identity is role contract, not permission. A Skill Candidate is
a proposal, not an approved skill.

## Private Profiles

Private profiles such as `aiplus-work-with-zhiwen` may consume this engine, but
private profile content must not be bundled into public release assets.
