# AiPlus

[English README](README.md)

## 痛点

你打开一个新的 agent session，这周第三次解释同一个项目规范。Codex 长任务跑到一半，context 满了，agent 丢了半条上下文。compact 之后它像失忆了一样，问你早已回答过的问题。让三个不同 agent 协作时，它们互相踩脚，因为没人约定好谁是 CEO、谁是 reviewer、谁是 builder。而 agent 说"这件事要 5 小时"，你凭经验知道通常 20 分钟就做完了，但没人把这条记下来。

## 解决方案

AiPlus 把 agent 工作流变成可信的工程实践。它在 `.aiplus/` 下维护项目级本地记忆，让 agent 跨 session 记住规范。Auto Compact 在 context 耗尽前准备结构化交接，compact 后从 checksum 验证的 capsule 自动续上。Auto Team Consultant 安装决策系统，在正确深度把任务路由给正确角色。Agent Velocity 默默记录估时与实际，检测 human-time bias，并调整下一次猜测。全部本地运行，不上传，不联网。

## 快速开始

安装 `aiplus` 命令：

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
```

把 AiPlus 安装到当前项目：

```bash
cd MyProject
aiplus install codex
```

验证状态：

```bash
aiplus status
aiplus doctor
```

## 内部组成

- **Agent Memory** (`agent-memory`) — 项目级 JSONL 记忆、角色身份、skill candidate 治理
- **Auto Compact** (`auto-compact`) — 主动 compact 提醒、checkpoint、handoff、resume 工作流
- **Auto Team Consultant** (`auto-team-consultant`) — L0-L5 路由，含 Advisor、CEO、Reviewer、Builder 视角
- **Agent Velocity** (`agent-velocity`) — AI-native 时间校准，含 bias 检测与 retention

## 路线图

见 [v0.5.2 known gaps](docs/roadmap/v0.5.2-known-gaps.md) 了解当前技术债与延期工作。
