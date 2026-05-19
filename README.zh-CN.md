# AiPlus
[![CI](https://github.com/izhiwen/AiPlus/actions/workflows/ci.yml/badge.svg)](https://github.com/izhiwen/AiPlus/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

[English README](README.md)

我用 AI coding agent 全职写代码已经有大半年 —— 平时主要 Claude Code，偶尔 Codex 拿第二意见，长任务上 OpenCode。大约四个月之后，我发现自己在同一周里把同一个架构决策对同一个 agent 解释了第四遍 —— 顺带把同一把 API key 也对同一个 agent 重新粘贴了第四遍。每天都在烧时间的是这七件事：跨 session 失忆、`/compact` 上反复烧 token、几个 agent 互相抢着当头、估时锚在"人类工程师小时数"上、做 plan 时把安全和上手体验默默推到发版周、一个 agent 在同一个 context window 里同时戴所有帽子，还有每次 session 都要重新给 agent 配 key。AiPlus 就是我为治这七件事写的七个小 Rust 模块（Agent Team 同时治两件）。坦白讲这件事的元层：**我用 AI agent 构建了管理 AI agent 的工具链** —— 这句话听起来有多套娃就有多套娃，但这是这个 repo 存在的真实理由。今天能跑的就在这儿；还没做的事在 `docs/roadmap/`。

![AiPlus 30 秒演示](docs/demo.gif)

## 我们受够的几个痛点

如果你每天都在带 AI coding agent，下面这些可能很熟：

1. **Agent 跨 session 就忘。** 周一教过 naming 规则，周三又问。到周五，同一个架构决策已经讲过四遍了。
2. **长任务在 `/compact` 上反复烧 token。** 撞 token 上限的真正成本是：要么忘了 `/compact`、agent 几个小时来每轮都在重读越来越长的历史；要么 `/compact` 时机不对、下一个 session 头 20% 全花在重新解释已经决定过的事情。无准备的 compact 是长 coding session 里最大的 token 黑洞之一，每月账单都看得见。
3. **多个 agent 互相踩脚。** 没人定谁是 CEO、谁评审、谁实现。三个 agent 都想当头。
4. **估时锚定在"人类工程师小时数"上。** Agent 报"五小时"做 refactor，结果 20 分钟干完。下周类似任务又报五小时，又 20 分钟。没人记账。
5. **Agent 做 plan 时常常忽略最重要的事** —— 用户上手是否容易、安全和隐私、实际执行的 pitfall、AI 集成考量。这些事要么发版周才发现，要么用户投诉之后才发现。
6. **一个 Agent 戴所有帽子。** CEO、reviewer、builder、advisor 全塞进同一个上下文窗口。角色**漂移**，上下文在不同帽子间**污染**，每个帽子都戴得很**浅**。真正的工程团队之所以分工，是因为工作本身就是如此结构化。
7. **每次 agent session 都要重新给 agent 配 key。** 新项目、新对话、新 wrapper 脚本 —— 又一次要 copy-paste `OPENAI_API_KEY=...`、在新 shell 里 `export` env、改 `.env`，或者直接把 key 贴进 prompt"就这一次"。每次都从头来过，永远不能摊销。更糟的是 key 会留在 transcript、`.env`、shell history、截图、CI 日志里 —— 一次误 commit、一次共享屏幕，就泄出去了。
8. **每开新项目，agent 都重新认识你。** 痛点 #1 是同一个项目内**跨 session** 忘事；这是**跨项目**的那一层。你花六个月把 agent 调教成懂你工作流的样子——naming 风格、review 语气、角色身份、工具偏好——下一个项目开张，agent 还是从零开始，没有"基线人格"跟你过来。`how I work` 这层东西没有比项目更高一级的家。

AiPlus 是七个小模块治这**七件项目内**的事（Agent Team 同时治 #3 多 agent 互相踩脚 和 #6 单 agent 角色漂移）。第八件 —— 跨项目偏好失忆 —— 由下面会讲的 [**AiPlus-Work-with-Me**](https://github.com/izhiwen/AiPlus-Work-with-Me) Companion 模板治。另加一个 opt-in 模块 AiEconLab，给应用经济学研究用，详见下。

## 你拿到什么

**Agent Memory** —— Agent 不再失忆。项目约定、命名规则、架构决定，作为本地 JSONL 存在 `.aiplus/memory/`。写入前会过 12 条 redaction 规则剥敏感串，所以你可以放心记偏好，不用担心泄漏。

**Compact Reminder** —— **长对话省 token**。长 Claude Code / Codex / OpenCode session 会两头漏 token：忘了 `/compact` 时上下文溢出、agent 每轮都得重读越来越大的历史；`/compact` 时机不对又会丢任务状态、下一个 session 全花在重新解释上。本模块在 token 阈值 + 任务切点双信号下提醒你恰当时机 compact，自动准备结构化交接，并用 checksum 校验过的 capsule 自动续上 —— **让 token 花在新工作上，而不是重建上下文**。

**Agent Key** —— **不再每个 session 重配 key**。**免费、零配置默认**：每个 key 直接存在你机器的 OS keyring 里（macOS Keychain / Linux Secret Service / Windows Credential Manager），从不落盘。每台机器一次性：

```bash
aiplus secret-broker set --alias openai --auto-prompt   # 原生 OS 密码框
# 或：echo -n "$YOUR_OPENAI_KEY" | aiplus secret-broker set --alias openai
# 每个 provider 重复一次（anthropic、github、…）
```

之后任何项目的任何 Claude Code / Codex / OpenCode session 都自动拿到 key：

```bash
aiplus secret-broker run --aliases openai,anthropic -- python my_agent.py
# child 进程 env 有 OPENAI_API_KEY + ANTHROPIC_API_KEY；退出即清
```

**跨项目共享分两层**：

1. **机器级（始终生效）**：每个 alias 在这台机器上只需 `aiplus secret-broker set` 一次；之后从任何目录跑 `aiplus secret-broker need <alias>` 都从 OS keyring 静默拿到值。Agent 永远不会再问你要 key；即使是从未跑过 `aiplus install` 的全新目录，`need` 一样能用。
2. **cd 自动装载（项目级，opt-in）**：`aiplus install --yes`（装时默认 `[Y/n]` 询问）写入的 shell hook，会在你 `cd` 进入列了该 alias 的项目（项目里有 `.aiplus/keys.toml`）时自动 export `*_API_KEY` 到 env。想在新项目里享受这个 ergonomic flow，跑一次 `aiplus install <runtime>` 即可。

不再 copy-paste，不再改 `.env`，不再把 key 贴进 prompt。（顺带：值默认不打印、绝不进 git。）需要多机同步或团队共享 → opt-in 切到 Bitwarden Secrets Manager 后端（`export AIPLUS_SECRET_PROVIDER=bws`），同样 alias 接口，需要付费订阅。

**Auto Team Consultant** —— Agent 不再忽略关键事项。**一个虚拟团队**（5 位专家成员 + 你项目的用户 persona，**坐同一桌**）会在每次重要 plan 之前被咨询。Coordinator 按复杂度和风险决定咨询规模，让你拿到真实评审团队的价值，但不在每次提交都付成本。

**Agent Team** —— 用常驻团队取代单 Agent 的**角色漂移**。Advisor、CEO、Architect、PM、两名 Engineer、Reviewer 和 QA —— 每个角色都有独立人设、工作区和内存命名空间。Coordinator 把任务路由给正确角色，保存对话记录，清理过时工作区。不再角色污染，不再每顶帽子都戴得很浅。**这个团队自带：**

- **用人话切角色** —— 说"你是 CEO"、"take the reviewer role"、
  "切换到 PI"，agent 就会开始用那个角色回应你，并加载它的内存。
  不用 CLI 命令。Codex、Claude Code、OpenCode（交互模式）都支持。
- **理解意图的安全门** —— 做任何危险操作之前（删文件、发布改动、
  跑受保护的命令），Coordinator 会先理解你到底想做什么，而不只是
  匹配你打的字眼。改个说法、加引号已经骗不过它了。
- **评审和 QA 并行** —— review 步骤和 QA 步骤同时跑，每个角色的
  工作区在任务之间保持就绪，不再每次从头建。典型迭代周期 ~8-10
  分钟，不再 ~15-20，质量门槛不变。

**Agent Velocity** —— Agent 不再瞎报工时。每次估时和实际完成时间记成本地 JSONL。Human-time bias 自动检测。后续估时用基于你自己历史校准过的 AI-native p50 / p90 数字。

**Token Cost** —— `aiplus agent token-cost` 读取 dispatch log，按 1 小时 / 8 小时 / 24 小时统计 token 消耗和 USD 成本，并列出最贵 task。定价来自社区维护的 per-model 表，带离线兜底和本地 override；也可直接跑 standalone `aiplus-token-cost`。

**Companion 模板：[AiPlus-Work-with-Me](https://github.com/izhiwen/AiPlus-Work-with-Me)** —— 上面七个模块都是 *项目本地*，AiPlus-Work-with-Me 是叠在它们之上的 **用户级 profile 包**：协作风格、项目地图、角色身份、工具偏好——填一次，所有项目都继承。fork 它、填占位符、`aiplus profile install AiPlus-Work-with-Me --user --yes` 一次装完。它 **不会**被 `aiplus install` 自动装上——是显式 fork-and-personalize 的 opt-in，解决跨**项目**（不只跨 session）的偏好记忆。

所有数据都留在你项目里的 `.aiplus/`。**不上传，不云同步，不动你的全局 agent 配置。**

## 谁会用这个

AiPlus 同时服务两类受众，底座（substrate）共享：

- **软件工程师** —— 用 Claude Code / Codex / OpenCode 写代码的。`aiplus install` 默认装 SWE 团队（Advisor / CEO / Architect / PM / 2× Engineer / Reviewer / QA + 11 SWE expert）。
- **应用经济学研究者** —— 写论文、做 replication package、跑 LLM-as-measurement。`aiplus add aieconlab` 装上 [**AiEconLab (AEL)**](https://github.com/izhiwen/AiEconLab)：8 个研究角色（Advisor / PI / Theorist / PM / RA-Stata / RA-Python / Referee / Replicator）+ 12 个 expert（含 LLM-as-Measurement Specialist）。**替换** SWE consultant 团队为应用经济学专属版本。

两类受众共用七个 substrate 模块：`aiplus-agent-memory` / `aiplus-compact-reminder` / `aiplus-auto-team-consultant` / `aiplus-agent-team` / `aiplus-agent-key` / `aiplus-agent-velocity` / `aiplus-token-cost`。

## 安装

### 方法 A —— 预编译二进制（推荐，v0.6.0+）

每个 release 包 **两个平台** 的预编译二进制：**Apple Silicon Mac**
（`aarch64-apple-darwin`）和 **Intel Windows**（`x86_64-pc-windows-msvc`）。
Intel Mac、Linux、Windows ARM **不再支持** —— 需要的话请从源码构建。

```bash
# Apple Silicon Mac (M1 / M2 / M3 / M4)
curl -L https://github.com/izhiwen/AiPlus/releases/latest/download/aiplus-aarch64-apple-darwin.tar.gz | tar xz
sudo mv aiplus aiplus-token-cost /usr/local/bin/

# Intel Windows (PowerShell)
# 下载 aiplus-x86_64-pc-windows-msvc.zip，解压 aiplus.exe + aiplus-token-cost.exe，加进 PATH
```

校验和：`https://github.com/izhiwen/AiPlus/releases/latest/download/checksums.txt`

### 方法 B —— 安装脚本（curl-pipe-bash）

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/AiPlus/main/install.sh \
  | bash
```

如果你想先看一眼脚本再跑：

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/AiPlus/main/install.sh -o install.sh
less install.sh
bash install.sh
```

### 把 AiPlus 装进你的项目

```bash
cd MyProject
aiplus install claude-code    # 或：codex, opencode, all
```

验证：

```bash
aiplus status
aiplus doctor
```

MCP 注册命令使用同一套 runtime 名字：

```bash
aiplus mcp-register --runtime claude-code  # 也可用 codex / opencode；claude 仍作为兼容 alias
```

## 支持的 runtime

| Runtime     | 安装命令                       | adapter 落到哪里                          |
|-------------|--------------------------------|-------------------------------------------|
| Claude Code | `aiplus install claude-code`   | `.claude/` 命令                           |
| Codex       | `aiplus install codex`         | `AGENTS.md` 里的托管块                    |
| OpenCode    | `aiplus install opencode`      | `.opencode/` prompts                      |
| 三个全装    | `aiplus install all`           | 全部 adapter                              |

每个 adapter 都是项目本地的。**不动你的全局配置**。

## 日常命令

```bash
aiplus status                        # 所有模块状态
aiplus doctor                        # 跨模块健康检查

# Memory
aiplus memory status
aiplus memory context --runtime claude-code --budget 2000

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
├── .aiplus/compact/             # Compact handoffs 和 capsule
├── .claude/                     # Claude Code adapter (装了的话)
├── .opencode/                   # OpenCode adapter (装了的话)
└── AGENTS.md                    # Codex 托管块 (装了的话)
```

## 七个独立子模块（bundled）

每个模块也作为独立 GitHub repo 发布，方便你单独看或单独采用；
同时 `aiplus install` 会自动把它们装到 `.aiplus/modules/aiplus-<name>/`：

- [AiPlus-Agent-Memory](https://github.com/izhiwen/AiPlus-Agent-Memory) —— 本地 JSONL memory + role identity + skill candidate。
- [AiPlus-Compact-Reminder](https://github.com/izhiwen/AiPlus-Compact-Reminder) —— **长 session 省 token**：恰当时机提示 `/compact` + 结构化交接 + 自动续接，避免溢出和重建上下文。
- [AiPlus-Auto-Team-Consultant](https://github.com/izhiwen/AiPlus-Auto-Team-Consultant) —— 虚拟 expert 团队，每个任务自动 consult。
- [AiPlus-Agent-Team](https://github.com/izhiwen/AiPlus-Agent-Team) —— 常驻 8 core + 11 expert 角色，带 persistent identity。
- [AiPlus-Agent-Key](https://github.com/izhiwen/AiPlus-Agent-Key) —— **不再每个 session 重配 key**。默认免费零配置，用 OS keyring 后端：`aiplus secret-broker set --alias openai` 一次，从此任何项目的任何 agent session 都自动拿到 key。需要多机同步/团队共享时 opt-in Bitwarden Secrets Manager 后端。
- [AiPlus-Agent-Velocity](https://github.com/izhiwen/AiPlus-Agent-Velocity) —— AI-native 工时估计（`aiplus velocity`，跟踪估时 vs 实际、学习 bias、给校准的 p50/p90）。
- [AiPlus-Token-Cost](https://github.com/izhiwen/AiPlus-Token-Cost) —— 从 `.aiplus/agents/dispatch-log.jsonl` 统计 token 和 USD 成本；可直接运行 standalone `aiplus-token-cost`，也可用 bundled `aiplus agent token-cost`。

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

需要一份现成的 "fork → 填占位符 → 一次安装" 模板，解决跨项目 / 跨 session 失忆（让 agent 记住你的协作风格、项目地图、角色身份、工具偏好，不用每个 session 重讲一遍），见 [**AiPlus-Work-with-Me**](https://github.com/izhiwen/AiPlus-Work-with-Me)。它 **不会**被 `aiplus install` 自动装上 —— 你 fork 它、填好占位符（USER.md / sync/projects.toml / secret-aliases.tsv），然后跑一次 `aiplus profile install AiPlus-Work-with-Me --user --yes`。

## 状态

最新发布：见 [Releases](https://github.com/izhiwen/AiPlus/releases/latest)（当前 `v0.5.10`，含 macOS / Linux / Windows 预编译二进制）。`main` 分支活跃开发；下次 cut 前要做的事见 [`docs/roadmap/`](docs/roadmap/)。

## License

[Apache-2.0](LICENSE)
