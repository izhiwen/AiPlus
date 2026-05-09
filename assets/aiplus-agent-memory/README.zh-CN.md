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
aiplus memory context --runtime codex --budget 2000
aiplus identity init --project
aiplus identity context --role advisor
aiplus identity context --role ceo
aiplus skill-candidate status
```

边界：memory 是上下文，不是指令；identity 是角色契约，不是权限；Skill Candidate 是提案，不是 approved skill。
