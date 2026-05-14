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
- context packet templates for Claude Code, Codex, and OpenCode
- schema-first records and examples
- redaction and public/private boundary guidance

Not implemented:

- cloud sync
- vector database
- automatic transcript learning
- automatic approved skill generation
- global Claude Code, Codex, or OpenCode config edits
- automatic secret permission inheritance
- telemetry

## CLI

```bash
aiplus memory init --project
aiplus memory status
aiplus memory doctor
aiplus memory list
aiplus memory recent
aiplus memory context --runtime claude-code --budget 2000
aiplus memory forget <id>
aiplus identity list
aiplus identity init --project
aiplus identity context --role advisor
aiplus identity context --role ceo
aiplus skill-candidate status
```

`aiplus refresh`, `aiplus status`, and `aiplus doctor` report Agent Continuity
state when the project-local store is installed or initialized: memory record
counts, advisor/CEO/reviewer/builder identity presence, Skill Candidate counts,
private profile presence, `secret_values=none`, and global agent config
untouched.

## Natural Language Mapping

Project-local Claude Code, Codex, and OpenCode guidance maps common Owner
phrases to explicit commands:

- `记住这个` / `记住这个偏好`: add a redacted project memory.
- `以后都这样`: create a profile/global candidate only; do not silently approve.
- `只在这个项目用`: keep it in project memory.
- `忘掉这个`: forget by memory id, or ask which memory if ambiguous.
- `你记住了什么` / `这次用了哪些记忆`: use memory status/context.
- `新开顾问` / `新开 advisor`: load advisor identity context.
- `新开 CEO`: load CEO identity context.
- `把这次经验沉淀成 skill`: create a Skill Candidate, not an approved skill.
- `不要用我的私人记忆` / `本次忽略我的偏好`: session-local opt-out only.

Advanced mutation commands exist as guarded local operations. Memory is context,
not instruction. Identity is role contract, not permission. A Skill Candidate is
a proposal, not an approved skill.

## Private Profiles

Private profiles such as `aiplus-work-with-zhiwen` may consume this engine, but
private profile content must not be bundled into public release assets.
