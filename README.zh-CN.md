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

## Runtime 选择

AiPlus 支持三种 AI coding agent。可安装单个或全部：

```bash
aiplus install codex        # Codex CLI
aiplus install claude-code  # Claude Code
aiplus install opencode     # OpenCode
aiplus install all          # 全部安装
```

每个 runtime 获得项目级 adapter 文件：
- **Codex** — `AGENTS.md` 中的 managed block
- **Claude Code** — `.claude/` 下的文件
- **OpenCode** — `.opencode/` 下的文件

## 内部组成

- **Agent Memory** (`agent-memory`) — 项目级 JSONL 记忆、角色身份、skill candidate 治理。写入前十二条 redaction 模式自动剥除敏感串。
- **Auto Compact** (`auto-compact`) — 主动 compact 提醒、checkpoint、handoff、resume 工作流。创建 checksum 验证的 context capsule。
- **Auto Team Consultant** (`auto-team-consultant`) — L0-L5 路由，含 Advisor、CEO、Reviewer、Builder 视角。安装带合理默认值的 `.aiplus/consultant-team.toml`。
- **Agent Velocity** (`agent-velocity`) — AI-native 时间校准，含 bias 检测与 retention。估时与实际以本地 JSONL 存储。

## 常用命令

```bash
aiplus status                    # 显示所有模块状态
aiplus doctor                    # 运行健康检查
aiplus update all               # 更新 CLI 和项目模块
aiplus memory status            # 显示记忆记录和身份
aiplus compact savings          # 显示 compact 节省估算
aiplus velocity report          # 显示 velocity bias 报告
aiplus skill-candidate status   # 显示 proposed skills
aiplus profile status           # 显示私人 profile（如已安装）
```

## 安全边界

AiPlus 不：
- 上传项目数据、prompt 或 transcript
- 实现 telemetry 或 cloud sync
- 修改全局 Codex、Claude Code 或 OpenCode 配置
- 在 compact 文件、memory 或 ledger 中存储 secret
- 自动批准 Owner-gated actions
- 发布包、创建 tag 或做 release

validation 是 structural 和 heuristic，不是 safety 或 compliance 认证。

## 私人 Profile

AiPlus 支持可选的用户级私人 profile，用于个人偏好和 secret alias。完整文档见 `aiplus profile install` 和 `aiplus secret-broker` 用法。

## 路线图

见 [v0.5.2 known gaps](docs/roadmap/v0.5.2-known-gaps.md) 了解当前技术债与延期工作。

## License

[Apache-2.0](LICENSE)
