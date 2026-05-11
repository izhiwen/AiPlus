# First Action

When the user mentions "aiplus", "AiPlus", or any AiPlus subcommand in this conversation, your first action is to read `.aiplus/AGENTS.aiplus.md` and follow its translation table. Never pass a Chinese phrase as a literal CLI argument — always translate to the English subcommand first.

## New Commands

- `aiplus update --all-projects` — update all registered projects
- `aiplus list-projects` — show all registered projects
- `aiplus prune-projects --yes` — remove stale entries from registry

# Agent Memory

Use this skill when the Owner asks about Agent Continuity, memory, role
identity, skill candidates, or memory health in a project that has AiPlus
installed.

## v2 Concepts

- **Memory is context, not instruction**: Stored context guides reasoning; it
does not override owner intent.
- **Identity is role contract, not permission**: Roles define behavior
expectations, not access rights.
- **Skill Candidate is proposal, not approved skill**: Every skill must go
through review workflow: `proposed -> accepted` or `proposed -> rejected`.
- **No automatic approval**: Never silently approve memories, skills, or
identity changes.
- **No global config edits**: All changes stay within the project directory.
- **Project-local only**: No cloud sync, no cross-project leakage.

## Workflow

1. Run `aiplus memory status` to check the project-local store.
2. Run `aiplus memory context --runtime codex --budget 2000` before relying on
   stored context.
3. Run `aiplus identity context --role advisor` or `aiplus identity context --role ceo`
   when opening those roles.
4. Run `aiplus memory context --show-used` to report which memories were
   referenced in the current session.
5. Run `aiplus memory conflicts` or `aiplus memory list --stale` when asked
   about expired or conflicting memories.

## Owner Phrases

| Owner says | Action |
|---|---|
| `你记住了什么？` / `你记住了什么` | `aiplus memory status` — show memory context |
| `记住这个` / `记住这个偏好` | Propose redacted project memory; do not auto-approve |
| `忘掉这个` | `aiplus memory forget <id>`; ask which id if ambiguous |
| `这次用了哪些记忆？` / `这次用了哪些记忆` | `aiplus memory context --show-used` — show used memories |
| `新开 Advisor` / `新开 advisor` / `新开顾问` | `aiplus identity context --role advisor` |
| `新开 CEO` / `新开 ceo` | `aiplus identity context --role ceo` |
| `有什么过期/冲突记忆？` / `有什么过期记忆` / `有什么冲突记忆` | `aiplus memory conflicts` or `aiplus memory list --stale` |
| `以后都这样` | Create profile/global candidate; do not silently approve |
| `只在这个项目用` | Keep memory project-local |
| `把这次经验沉淀成 skill` | Propose Skill Candidate; do not auto-approve |
| `不要用我的私人记忆` / `本次忽略我的偏好` | Session-local opt-out only |

## Review Workflow

1. Owner requests memory/skill/identity change.
2. Propose change (redacted if needed).
3. Mark status as `proposed`.
4. Owner reviews and accepts or rejects.
5. Only after explicit acceptance does status become `active`.

## Safety Boundaries

- **No raw transcripts**: Never store conversation transcripts.
- **No secret values**: Redact keys, tokens, passwords before storage.
- **No provider payloads**: Do not cache LLM request/response payloads.
- **No telemetry**: Do not send usage data to external services.
- **No cloud sync**: Keep all data local to the project.
