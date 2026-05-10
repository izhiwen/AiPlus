# AiPlus

[English README](README.md)

## 为什么存在

你一定经历过这些。周一早上打开新 session，第三次解释项目规范。到周三 agent 已经忘了命名规则。Codex 长任务跑到一半，context window 满了，agent 丢了半条上下文。compact 之后它像失忆了一样，问你早已回答过的问题。多个 agent 协作时互相踩脚，因为没人定义谁来主导、谁来 review、谁来实现。而 agent 说"这件事要五小时"，你凭经验知道通常二十分钟就做完了，但没人把这条记下来，所以下次估时还是一样离谱。

AiPlus 用四个集成模块解决这些问题，全部在本地运行。

## 它能做什么

**Agent Memory** 把项目规范以 JSONL 形式存在 `.aiplus/memory/` 下。任何记录写入前，十二条 redaction 模式自动剥除 password、JWT、raw transcript 等敏感串。agent 跨 session 记住你的命名规则、编码标准和架构决策。被拒绝或已忘记的记录留在 store 里，但默认不进入上下文。

**Auto Compact** 在 context window 耗尽前准备结构化交接。它把 decision log、agent state 和 evidence 捕获进 checksum 验证的 capsule。compact 后，`aiplus compact resume` 读取 capsule 并自动恢复上下文。agent 从断点继续，不是从零开始。

**Auto Team Consultant** 在项目中安装路由系统。它定义清晰角色：Advisor 负责直接建议，CEO 负责任务拆分，Reviewer 负责审阅发现，Builder 负责实现。任务从 L0 直接建议路由到 L5 完整治理。AI Integration 是默认专家团队成员，不是事后补充。

**Agent Velocity** 把每次估时和实际完成时间记为本地 JSONL，存在 `.aiplus/velocity/` 下。它检测 human-time bias：当估时锚定在工程师小时而不是 agent 分钟时触发提醒。积累几条记录后，它输出 p50 和 p90 的 AI-native 估时，并调整下一次猜测。

全部数据留在项目内的 `.aiplus/` 下。不上传，不出本机。

## 安装

安装 `aiplus` 命令：

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
```

把 AiPlus 安装到当前项目：

```bash
cd MyProject
aiplus install codex        # 或: claude-code, opencode, all
```

验证安装：

```bash
aiplus status
aiplus doctor
```

## Runtime 支持

AiPlus 支持三种 AI coding agent，各自获得项目级 adapter 文件：

| Runtime | 安装命令 | Adapter 文件 |
|---------|---------|-------------|
| Codex | `aiplus install codex` | `AGENTS.md` 中的 managed block |
| Claude Code | `aiplus install claude-code` | `.claude/` 下的命令 |
| OpenCode | `aiplus install opencode` | `.opencode/` 下的 prompts |
| 全部 | `aiplus install all` | 所有 adapter |

可安装单个或全部 runtime。每个 adapter 都是项目级，不触碰全局配置。

## 日常命令

```bash
# 状态与健康
aiplus status                      # 显示所有模块状态
aiplus doctor                      # 跨模块运行健康检查

# 记忆
aiplus memory status              # 显示记忆记录和身份
aiplus memory context --runtime codex --budget 2000

# Compact
aiplus compact prepare            # 构建 handoff 和 context capsule
aiplus compact resume             # compact 后恢复
aiplus compact savings            # 显示 token 和成本节省

# Velocity
aiplus velocity estimate --task-type feature --human-estimate 5h
aiplus velocity report            # 显示 bias 和调整报告

# 团队
aiplus skill-candidate status     # 显示 proposed skills

# 更新
aiplus update all                 # 更新 CLI 和所有项目模块
```

## 架构

```
MyProject/
├── .aiplus/
│   ├── memory/              # JSONL 记忆记录
│   ├── identities/          # 角色身份定义
│   ├── skills/              # Skill candidates
│   ├── consultant-team.toml # 团队路由配置
│   └── velocity/            # 估时和完成记录
├── .codex/compact/          # Compact handoffs 和 capsules
├── .claude/                 # Claude Code adapters（如已安装）
├── .opencode/               # OpenCode adapters（如已安装）
└── AGENTS.md                # Codex managed block（如已安装）
```

## 安全边界

AiPlus 完全在项目目录内运行：

- 不上传项目数据、prompt 或 transcript
- 无 telemetry、cloud sync 或外部服务
- 不修改全局 Codex、Claude Code 或 OpenCode 配置
- 不在 compact 文件、memory 或 ledger 中存储 secrets
- 不自动批准 Owner-gated actions
- 不发布包、创建 tag 或做 release

validation 是 structural 和 heuristic，不是 safety 或 compliance 认证。

## 私人 Profile

AiPlus 支持可选的用户级私人 profile，用于个人偏好和 secret alias。这些位于 `~/.config/aiplus/profiles/`，永远不会被打包进 public repository。完整文档见 `aiplus profile install` 和 `aiplus secret-broker` 用法。

## 项目状态

当前版本：v0.5.1，所有模块已完成 v2.1 加固。

见 [v0.5.2 known gaps](docs/roadmap/v0.5.2-known-gaps.md) 了解技术债和计划工作。

## License

[Apache-2.0](LICENSE)
