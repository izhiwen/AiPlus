# Daily Workflows

Natural-language phrases you can use in your agent session and their AiPlus backend commands.

## Quick Reference Table

| What you say | What the agent runs | What it does |
|---|---|---|
| `AiPlus refresh` / `AiPlus 刷新` | `aiplus refresh` | Reloads AiPlus guidance, reports status |
| `记住这个` / `remember this` | `aiplus memory add --scope project --kind preference --text "..."` | Adds a redacted project memory |
| `你记住了什么` / `what do you remember` | `aiplus memory status` | Lists active memory records |
| `这次用了哪些记忆` / `which memories were used` | `aiplus memory context --runtime codex --budget 2000` | Shows memory injected into context |
| `保存进度` / `save progress` | `aiplus compact prepare` then `aiplus compact checkpoint` | Prepares and saves a compact checkpoint |
| `准备 compact` / `prepare compact` | `aiplus compact prepare` | Validates readiness, creates context capsule |
| `继续` / `continue` | `aiplus compact resume` | Resumes from checkpoint after compact |
| `我的偏好生效了吗` / `are my preferences active` | `aiplus profile context <profile>` | Shows installed profile and bundle status |
| `secret 状态` / `secret status` | `aiplus secret-broker status` | Shows alias resolution status, never values |
| `忘掉这个` / `forget this` | `aiplus memory forget <id>` | Marks a memory as rejected |
| `以后都这样` / `always do this` | `aiplus memory add --scope profile --kind preference --text "..."` | Profile-level preference candidate |
| `新开 CEO` | `aiplus identity context --role ceo` | Loads CEO role identity |
| `新开 advisor` | `aiplus identity context --role advisor` | Loads Advisor role identity |
| `把这次经验沉淀成 skill` | `aiplus skill-candidate propose --title "..."` | Proposes a new skill candidate |
| `compact savings` / `show compact savings` | `aiplus compact savings` | Shows token/USD savings estimates |
| `update AiPlus` / `升级 AiPlus` | `aiplus update all` | Updates CLI and project modules |

## Agent Session Lifecycle

### Starting a session

```text
AiPlus refresh
```

The agent should report: installed modules, memory record count, compact state, profile presence.

### During work

```text
记住这个：release notes should be in English first
```

The agent adds a redacted project memory. If the text contains secrets, API keys, or private keys, AiPlus blocks the write and reports `MEMORY_REDACTION_STATUS=BLOCKED`.

```text
你记住了什么
```

The agent shows active memory records with IDs, types, and status.

### Before compact

```text
save progress
```

The agent runs `aiplus compact prepare` to validate readiness and create a context capsule, then `aiplus compact checkpoint` to save the checkpoint. It should tell you:

```text
Ready to compact.

After compact:
- If I continue automatically, you do not need to do anything.
- If I do not reply, send: continue
```

### After compact

```text
continue
```

The agent runs `aiplus compact resume` and picks up from the checkpoint.

### Checking secrets

```text
secret 状态
```

The agent runs `aiplus secret-broker status`. Output shows alias names and resolution status. It never prints secret values.

## Chinese Equivalents

| Chinese | English equivalent |
|---|---|
| `AiPlus 刷新` | `AiPlus refresh` |
| `记住这个` | `remember this` |
| `你记住了什么` | `what do you remember` |
| `这次用了哪些记忆` | `which memories were used` |
| `保存进度` | `save progress` |
| `准备 compact` | `prepare compact` |
| `继续` | `continue` |
| `我的偏好生效了吗` | `are my preferences active` |
| `secret 状态` | `secret status` |
| `忘掉这个` | `forget this` |
| `以后都这样` | `always do this` |
| `升级 AiPlus` | `update AiPlus` |
