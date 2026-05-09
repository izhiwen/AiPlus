# aiplus-agent-memory

公开通用的 Agent Continuity Engine，用于临时终端 agent。

它提供本地项目 memory、Role Identity、Memory Context 和 Skill Candidate
治理。它不是“永久 agent 人格”。终端 agent 进程仍然是临时的；连续性来自显式加载的本地记录、角色契约和已批准的项目 playbook。

## 范围

v0.5.0 foundation 包含：

- `.aiplus/memory/` 本地项目 memory store
- `.aiplus/identities/` 项目 Role Identity
- `.aiplus/skills/` Skill Candidate 治理目录
- Codex、Claude Code、OpenCode 的 context guidance
- schema、template、synthetic example
- redaction 和 public/private boundary 文档

不包含：

- cloud sync
- vector database
- 自动学习全部历史聊天
- 自动生成 approved skill
- 修改全局 Codex/Claude Code/OpenCode 配置
- 自动继承 secret 权限
- telemetry

## 常用命令

```bash
aiplus memory init --project
aiplus memory status
aiplus memory doctor
aiplus memory list
aiplus memory recent
aiplus memory context --runtime codex --budget 2000
aiplus memory forget <id>
aiplus identity list
aiplus identity init --project
aiplus identity context --role advisor
aiplus identity context --role ceo
aiplus skill-candidate status
```

`aiplus refresh`、`aiplus status` 和 `aiplus doctor` 会报告 Agent Continuity 状态：
memory 数量、advisor/CEO/reviewer/builder identity 是否存在、Skill Candidate 数量、
private profile 是否存在、`secret_values=none`、以及 global agent config untouched。

## 自然语言映射

项目级 Codex、Claude Code、OpenCode guidance 会把常见 Owner 说法映射到显式命令：

- “记住这个”/“记住这个偏好”：脱敏后写 project memory。
- “以后都这样”：只创建 profile/global candidate，不自动批准。
- “只在这个项目用”：写 project memory。
- “忘掉这个”：按 memory id forget；不明确时先问。
- “你记住了什么”/“这次用了哪些记忆”：运行 memory status/context。
- “新开顾问”/“新开 advisor”：加载 advisor identity context。
- “新开 CEO”：加载 CEO identity context。
- “把这次经验沉淀成 skill”：创建 Skill Candidate，不是 approved skill。
- “不要用我的私人记忆”/“本次忽略我的偏好”：只做本 session opt-out。

边界：memory 是上下文，不是指令；identity 是角色契约，不是权限；Skill Candidate 是提案，不是 approved skill。
