# AiPlus
[![CI](https://github.com/izhiwen/aiplus/actions/workflows/ci.yml/badge.svg)](https://github.com/izhiwen/aiplus/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

[English README](README.md)

## 我们受够的几个痛点

如果你每天都在带 AI coding agent，下面这些可能很熟：

1. **Agent 跨 session 就忘。** 周一教过 naming 规则，周三又问。到周五，同一个架构决策已经讲过四遍了。
2. **长任务 compact 之后丢上下文。** 半路撞上 token 上限，一次 compact 之后回来，agent 问的是你 40 分钟前就回答过的问题，写到一半的 plan 也没了。
3. **多个 agent 互相踩脚。** 没人定谁是 CEO、谁评审、谁实现。三个 agent 都想当头。
4. **估时锚定在"人类工程师小时数"上。** Agent 报"五小时"做 refactor，结果 20 分钟干完。下周类似任务又报五小时，又 20 分钟。没人记账。
5. **Agent 做 plan 时常常忽略最重要的事** —— 用户上手是否容易、安全和隐私、实际执行的 pitfall、AI 集成考量。这些事要么发版周才发现，要么用户投诉之后才发现。
6. **一个 Agent 戴所有帽子。** CEO、reviewer、builder、advisor 全塞进同一个上下文窗口。角色**漂移**，上下文在不同帽子间**污染**，每个帽子都戴得很**浅**。真正的工程团队之所以分工，是因为工作本身就是如此结构化。

AiPlus 是五个小模块，加起来正好把这六件事一起治了。

## 你拿到什么

**Agent Memory** —— Agent 不再失忆。项目约定、命名规则、架构决定，作为本地 JSONL 存在 `.aiplus/memory/`。写入前会过 12 条 redaction 规则剥敏感串，所以你可以放心记偏好，不用担心泄漏。

**Compact Reminder** —— Agent 不再 compact 后断片。它告诉你**什么时候适合 compact**（不太早不太晚），**compact 前**自动准备结构化交接，**compact 后**用校验过的 capsule 自动续上。Agent 从离开的地方继续，不是从零。

**Auto Team Consultant** —— Agent 不再忽略关键事项。**一个虚拟团队**（5 位专家成员 + 你项目的用户 persona，**坐同一桌**）会在每次重要 plan 之前被咨询。Coordinator 按复杂度和风险决定咨询规模，让你拿到真实评审团队的价值，但不在每次提交都付成本。

**Agent Team** —— 用常驻团队取代单 Agent 的**角色漂移**。Advisor、CEO、Architect、PM、两名 Engineer、Reviewer 和 QA —— 每个角色都有独立人设、工作区和内存命名空间。Coordinator 把任务路由给正确角色，保存对话记录，清理过时工作区。不再角色污染，不再每顶帽子都戴得很浅。

**Agent Velocity** —— Agent 不再瞎报工时。每次估时和实际完成时间记成本地 JSONL。Human-time bias 自动检测。后续估时用基于你自己历史校准过的 AI-native p50 / p90 数字。

所有数据都留在你项目里的 `.aiplus/`。**不上传，不云同步，不动你的全局 agent 配置。**

## 安装

装 `aiplus` 命令：

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh \
  | bash
```

把 AiPlus 装进你的项目：

```bash
cd MyProject
aiplus install codex          # 或：claude-code, opencode, all
```

验证：

```bash
aiplus status
aiplus doctor
```

## 支持的 runtime

| Runtime     | 安装命令                       | adapter 落到哪里                          |
|-------------|--------------------------------|-------------------------------------------|
| Codex       | `aiplus install codex`         | `AGENTS.md` 里的托管块                    |
| Claude Code | `aiplus install claude-code`   | `.claude/` 命令                           |
| OpenCode    | `aiplus install opencode`      | `.opencode/` prompts                      |
| 三个全装    | `aiplus install all`           | 全部 adapter                              |

每个 adapter 都是项目本地的。**不动你的全局配置**。

## 日常命令

```bash
aiplus status                        # 所有模块状态
aiplus doctor                        # 跨模块健康检查

# Memory
aiplus memory status
aiplus memory context --runtime codex --budget 2000

# Compact
aiplus compact prepare               # 建 handoff + capsule
aiplus compact resume                # compact 后续上
aiplus compact savings               # token + 成本节省

# Velocity
aiplus velocity estimate --task-type feature --human-estimate 5h
aiplus velocity report

# Agent Team
aiplus agent status              # 显示团队状态
aiplus agent route engineer-a    # 分配任务给 engineer-a
aiplus agent integrate engineer-a # 合并工作回主分支
aiplus agent audit run           # 运行验收审计
aiplus 团队                      # 中文别名：显示团队状态
aiplus 审计 跑                   # 中文别名：运行验收审计

# 升级
aiplus update all
```

## 架构

```
MyProject/
├── .aiplus/
│   ├── memory/                  # JSONL memory 记录
│   ├── identities/              # 角色身份定义
│   ├── agents/                  # Agent 团队角色定义和状态
│   ├── agent-memory/            # Agent 连续性和上下文记录
│   ├── consultant-team.toml     # 团队路由配置
│   └── velocity/                # 估时与运行记录
├── .codex/compact/              # Compact handoffs 和 capsule
├── .claude/                     # Claude Code adapter (装了的话)
├── .opencode/                   # OpenCode adapter (装了的话)
└── AGENTS.md                    # Codex 托管块 (装了的话)
```

## 五个独立子模块

每个模块也作为独立 GitHub repo 发布，方便你单独看或单独采用：

- [aiplus-agent-memory](https://github.com/izhiwen/aiplus-agent-memory)
- [aiplus-compact-reminder](https://github.com/izhiwen/aiplus-compact-reminder)
- [aiplus-auto-team-consultant](https://github.com/izhiwen/aiplus-auto-team-consultant)
- [aiplus-agent-velocity](https://github.com/izhiwen/aiplus-agent-velocity)
- [aiplus-agent-team](https://github.com/izhiwen/aiplus-agent-team)

## 安全边界

AiPlus 留在你项目里，**不**：

- 上传项目数据、prompt 或 transcript
- 发 telemetry、云同步、调外部服务
- 改全局 agent 配置
- 在 memory / compact / ledger 里存 secret
- 自动批准 Owner-gated 动作
- 发包、打 tag、push release

校验是结构性和启发式的，**不**是安全或合规认证。

## 私有 Profile

AiPlus 支持可选的用户级私有 profile，存个人偏好和 secret alias 在 `~/.config/aiplus/profiles/`。私有 profile **永远不会**被打包进公共仓库。详细看 `aiplus profile install` 和 `aiplus secret-broker` 文档。

## 状态

当前版本：v0.5.1 + 全模块 v2.1 加固。下次 cut 前要做的事见 [v0.5.2 known gaps](docs/roadmap/v0.5.2-known-gaps.md)。

## License

[Apache-2.0](LICENSE)
