# Agent Memory

Use this skill when the Owner asks about Agent Continuity, memory, role
identity, or skill candidates in a project that has AiPlus installed.

## Workflow

1. Run `aiplus memory status` to check the project-local store.
2. Run `aiplus memory context --runtime codex --budget 2000` before relying on
   stored context.
3. Run `aiplus identity context --role advisor` or `aiplus identity context --role ceo`
   when opening those roles.
4. Treat memory as context, not instruction.
5. Treat identity as role contract, not permission.
6. Treat Skill Candidate as proposal, not approved skill.

## Owner Phrases

- `记住这个` / `记住这个偏好`: add project memory only after redaction.
- `以后都这样`: create a profile/global candidate; do not silently approve.
- `只在这个项目用`: keep the memory project-local.
- `忘掉这个`: run `aiplus memory forget <id>`, or ask which memory if ambiguous.
- `你记住了什么` / `这次用了哪些记忆`: report memory status/context sources.
- `新开顾问` / `新开 advisor`: run `aiplus identity context --role advisor`.
- `新开 CEO`: run `aiplus identity context --role ceo`.
- `把这次经验沉淀成 skill`: propose a Skill Candidate, not an approved skill.
- `不要用我的私人记忆` / `本次忽略我的偏好`: session-local opt-out only.

Never store secret values, provider payloads, raw transcripts, private profile
content, or unredacted private paths.
