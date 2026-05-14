# AiPlus Agent Velocity
[![CI](https://github.com/izhiwen/aiplus-agent-velocity/actions/workflows/ci.yml/badge.svg)](https://github.com/izhiwen/aiplus-agent-velocity/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

[English README](README.md)

## 痛点

你让 agent 做一个 refactor。它说 **"五小时"**。你按这个安排了一下午。20 分钟之后，它说做完了。

下周，类似任务。Agent 又说五小时。又是 20 分钟干完。**一个月之后，你不再相信 agent 给的任何估时**，也不再围绕 AI 工作安排你接下来的时间——因为这些数字感觉是编出来的。

原因很直接：agent 用**人类工程师小时数**估自己的工，因为它训练数据就是按这个 anchor 的。**它在错估自己的工，但没人记账**。

## 我们的解决方案

AiPlus Agent Velocity 把每次估时和每次实际完成时间都记成本地 JSONL，存在 `.aiplus/velocity/`。它**从你自己的历史里学** AI-native 的真实时间是多少，然后把这个反馈进下一次估时。

记录覆盖：

- **Human estimate** —— Agent 一开始按人类工程师时间估的数字
- **Actual completion** —— 实际从头到尾花了多久
- **Task type** —— refactor / feature / bug fix / review
- **Model + workflow 标签** —— 提供上下文，避免把"Opus 4.7 上做的重型 review feature"和"快速脚本"平均到一起

积累几条记录之后，系统给出：

- **p50** —— 这类任务的 AI-native 中位时间
- **p90** —— 保守的上界
- **Human-time bias 检测** —— 估时锚定在人类工程师小时、实际完成远低于此时，会被 flag
- **Next-estimate 调整** —— Agent 用一个 multiplier 自动校准**下一次**人类风格的估时，让新数字是校准过的，不是猜的

**不存原始 prompt。不上传任何数据。** Normal 记录滚动保留 200 条；Rare cases 保留 20 条。

## 入门

如果你已经装了 AiPlus：

```bash
cd MyProject
aiplus install codex          # 或：claude-code, opencode, all
aiplus velocity init
```

然后 CLI：

```bash
aiplus velocity init                                # 初始化追踪
aiplus velocity estimate                            # 给出 AI-native 估时
aiplus velocity complete                            # 记录实际完成时间
aiplus velocity bias --task <id>                    # 检查特定任务的 bias
aiplus velocity report                              # 整体 bias 和 adjustment 报告（默认 --scope both）
aiplus velocity report --scope local                # 仅本项目
aiplus velocity report --scope global               # 仅跨项目
aiplus velocity doctor                              # 健康检查（含全局 ledger）
aiplus velocity purge --yes                         # 手动清理老记录
aiplus velocity import-from-project <path>         # 把已有项目数据回填到全局 ledger
```

## 仓库结构

- `core/schemas/` —— `config` / `estimate-record` / `run-record` / `rare-case-record` 的 JSON schema
- `core/` —— duration parser、bias 检测、retention 逻辑
- `DESIGN.md` —— 架构决策和设计 rationale

## 存储

两层 —— 项目本地 + 跨项目共享。

```
<project>/.aiplus/velocity/
  config.json           # 配置（含 share_to_global_mode）
  estimates.jsonl       # 估时记录（完整，含自由文本 task）
  runs.jsonl            # 完成记录
  rare-cases.jsonl      # rare case (大幅高估、owner gate 命中等)
  multipliers.json      # 聚合 adjustment multiplier
  rotation-state.json   # 滚动状态

~/.config/aiplus/velocity/          # XDG；Windows 上是 %APPDATA%\aiplus\velocity\
  config.json           # 全局 retention + health 配置
  estimates.jsonl       # 结构化投影 —— 不含自由文本、路径、项目名
  runs.jsonl            # 结构化投影
  rare-cases.jsonl      # 结构化投影
  anchor-signals.jsonl  # 结构化投影
```

- **没有 SQLite，没有 database**。
- 项目级：保留最新 **200** normal + **20** rare。
- 全局：保留最新 **1000** normal + **100** rare。
- 目录 `0700`，文件 `0600`。每条记录 `< 4096` 字节（atomic `O_APPEND`）。
- 聚合 multiplier 会在原始记录滚动之后存活下来，所以校准**不会因为老记录被淘汰而重置**。

### 跨项目共享

默认每个项目对全局 ledger 既读又写——这样新项目一上来就能用上你的
bias 历史。在 `config.json` 里切换：

```jsonc
{
  "schemaVersion": "2",
  // ...
  "shareToGlobalMode": "read_write"  // read_write | read_only | none
}
```

全局 ledger **物理上不可能装下**自由文本 `task`、文件路径、项目名或
任何源自 cwd 的字段——这些字段在投影阶段就被丢弃。估时时的合并规则
是 "project-recent-heavy"：取本项目最新 50 条 + 全局最新 150 条，按
id 去重。

## 安全边界

AiPlus Agent Velocity **不会**：

- 存原始 prompt、transcript 或源代码
- 上传数据或实现 telemetry
- 替代测试、评审或 Owner gate
- 充当生产力追踪器或 KPI 仪表盘
- 把估时改短当作跳过验证的借口

## 更多

- 主平台：[aiplus](https://github.com/izhiwen/aiplus)
- 下次发布前要跟进的事：
  [v0.5.2 known gaps](https://github.com/izhiwen/aiplus/blob/main/docs/roadmap/v0.5.2-known-gaps.md)

## License

[Apache-2.0](LICENSE)
