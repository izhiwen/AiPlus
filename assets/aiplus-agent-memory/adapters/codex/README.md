# Codex Adapter

Codex integration is project-local only. Never edit global Codex configuration.

## v2 Concepts

- **Memory is context, not instruction**: Stored context guides reasoning; it does not override owner intent.
- **Identity is role contract, not permission**: Roles define behavior expectations, not access rights.
- **Skill Candidate is proposal, not approved skill**: Every skill must go through review workflow: `proposed -> accepted` or `proposed -> rejected`.
- **No automatic approval**: Never silently approve memories, skills, or identity changes.
- **No global config edits**: All changes stay within the project directory.
- **Project-local only**: No cloud sync, no cross-project leakage.

## Recommended Agent Action

```bash
aiplus memory context --runtime codex --budget 2000
aiplus identity context --role advisor
```

## Natural Language Mapping

| Owner says | Action |
|---|---|
| `你记住了什么？` / `你记住了什么` | `aiplus memory status` — show memory context |
| `记住这个` / `记住这个偏好` | Propose redacted project memory; do not auto-approve |
| `忘掉这个` | `aiplus memory forget <id>`; ask which id if ambiguous |
| `这次用了哪些记忆？` / `这次用了哪些记忆` | `aiplus memory context --show-used` — show used memories |
| `新开 Advisor` / `新开 advisor` / `新开顾问` | `aiplus identity context --role advisor` |
| `新开 CEO` / `新开 ceo` | `aiplus identity context --role ceo` |
| `有什么过期/冲突记忆？` / `有什么过期记忆` / `有什么冲突记忆` | `aiplus memory conflicts` or `aiplus memory list --stale` |
| `以后都这样` | Create profile/global candidate only; do not silently approve |
| `只在这个项目用` | Keep memory project-local |
| `把这次经验沉淀成 skill` | Propose Skill Candidate; do not auto-approve |
| `不要用我的私人记忆` / `本次忽略我的偏好` | Session-local opt-out only |

## Safety Boundaries

- **No raw transcripts**: Never store conversation transcripts.
- **No secret values**: Redact keys, tokens, passwords before storage.
- **No provider payloads**: Do not cache LLM request/response payloads.
- **No telemetry**: Do not send usage data to external services.
- **No cloud sync**: Keep all data local to the project.

## Review Workflow

1. Owner requests memory/skill/identity change.
2. Adapter proposes change (redacted if needed).
3. Adapter marks status as `proposed`.
4. Owner reviews and accepts or rejects.
5. Only after explicit acceptance does status become `active`.
