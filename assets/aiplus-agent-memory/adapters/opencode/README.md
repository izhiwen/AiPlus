# OpenCode Adapter

OpenCode integration is project-local. Use project commands, prompts, and agents
that call:

```bash
aiplus memory context --runtime opencode --budget 2000
aiplus identity context --role advisor
```

Natural language mapping:

- `记住这个` / `记住这个偏好`: add a redacted project memory.
- `以后都这样`: create a profile/global candidate only; do not silently approve.
- `只在这个项目用`: project memory.
- `忘掉这个`: forget by memory id, or ask which id if ambiguous.
- `你记住了什么` / `这次用了哪些记忆`: memory status/context.
- `新开顾问` / `新开 advisor`: advisor identity context.
- `新开 CEO`: CEO identity context.
- `把这次经验沉淀成 skill`: Skill Candidate only.
- `不要用我的私人记忆` / `本次忽略我的偏好`: session-local opt-out.

Do not edit global OpenCode configuration.
