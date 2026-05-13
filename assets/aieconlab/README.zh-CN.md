# AiEconLab (AEL)

> **面向应用经济学家的永久虚拟研究团队。**
> 8 个核心角色（Advisor / PI / Theorist / PM / RA-Stata / RA-Python / Referee / Replicator）
> 加 12 位专家。默认工具栈 Python + Stata + LaTeX。

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

[English README](README.md)

## 安装前提

AiEconLab 建在 [AiPlus](https://github.com/izhiwen/AiPlus) agent substrate 之上。先装 AiPlus：

```bash
# 安装 AiPlus (>= 0.5.2)
# 见 https://github.com/izhiwen/AiPlus

# 然后装 AiEconLab 依赖的三个底座模块：
aiplus add agent-memory          # 每个 agent 的项目本地 memory
aiplus add compact-reminder      # 节省 token 的 compact + 结构化 resume
aiplus add auto-team-consultant  # 规划前顾问层
# (velocity 校准是 aiplus CLI 的内建 subcommand，无需 add 单独模块)
```

AiEconLab 是独立项目（`github.com/izhiwen/AiEconLab`），有自己的发布节奏和受众，但功能上依赖 AiPlus 提供的 memory、compact、velocity、consult-before-plan 这四层基础设施。

## 痛点

你让 agent 先帮你想 paper 的研究问题，再清洗数据，再做识别策略，再写引言，再写 referee response。第三个任务还没结束，它就**漂移**了：同一段 prompt history 里混着研究 intuition、Stata 语法、文献片段、和驳回意见，结果是哪个都做得不到位。

更糟的是，共享上下文**跨角色污染**。Theorist 的 framing 漏进 RA 的回归 spec；文献 review 的笔记被数据清洗的 debug 输出埋掉；稳健性检查在 context 窗口里被无关 scratch 挤掉。

你想靠让 agent 多戴几顶帽子来弥补。但一个 agent 同时戴 PI、Theorist、Econometrician、RA、Referee、Replicator 帽子，每顶都戴得**浅**。真实研究项目分工是因为工作本身*就*是这么结构化的。

### 不是这些痛

AiEconLab 专门解决**应用经济学研究的角色分工与执行**问题。其它 AiPlus 插件解决相邻但不同的问题：

| 插件 | 解决的痛 | 为什么不是 AiEconLab |
|---|---|---|
| [AiPlus-Agent-Team](https://github.com/izhiwen/AiPlus-Agent-Team) | 软件工程角色漂移 | 架构相同，角色不同 —— 那个发软件工程角色，这个发研究角色 |
| [AiPlus-Agent-Memory](https://github.com/izhiwen/AiPlus-Agent-Memory) | **失忆** —— session 之间忘记上下文 | 给单个 agent 记忆；不分角色 |
| [AiPlus-Auto-Team-Consultant](https://github.com/izhiwen/AiPlus-Auto-Team-Consultant) | **盲点** —— 规划时漏掉关键风险 | 规划*前*建议；不执行也不持久 |
| [AiPlus-Compact-Reminder](https://github.com/izhiwen/AiPlus-Compact-Reminder) | **烧 token** —— 长 session 反复重新加载同样上下文，token 烧得快 | compact + 结构化 resume 给单 agent 省 token；不分角色 |
| [AiPlus-Agent-Velocity](https://github.com/izhiwen/AiPlus-Agent-Velocity) | **估错时** —— 估算锚在人类工时 | 校准单 agent 估算；不构建团队 |

AiEconLab 和 [AiPlus-Agent-Team](https://github.com/izhiwen/AiPlus-Agent-Team) 是兄弟模块 —— 可以共存在同一个项目里（比如同时维护 paper 和 replication package 软件仓库的研究者）。

## 解决方案

**用永久研究团队替代单 agent 漂移。**

AiEconLab 在你的项目中安装一个由 8 个核心角色组成的永久虚拟团队：Advisor（导师顾问）、PI（你 / 主作者）、Theorist（理论建模）、PM（项目管理）、RA-Stata（Stata 助研）、RA-Python（Python 助研）、Referee（内部 referee）、Replicator（复现工程师）。每个角色有自己的 persona、workspace、memory namespace。Owner（你，主作者）只和 Advisor 与 PI 对话；PI 协调其它角色。

具体覆盖：

- **角色隔离** —— 每个 agent 只加载自己的 persona 和个人 memory。RA 看不到 Theorist 的推理，反之亦然。
- **Git worktree 工作区** —— 涉及代码的角色各自有独立工作目录，RA-Stata 和 RA-Python 可以并行而不互相覆盖。冲突通过 git 暴露，而不是悄悄盖掉。
- **三层 memory** —— 个人（每 agent）、团队（PI 共享）、项目（已有 `.aiplus/memory/`）。冲突时项目层胜出，临时的团队共识不会覆盖持久项目共识。
- **专家目录** —— 12 位专家（文献综述员、写作师、计量专家、复现工程师、史料专家、Job Talk Coach 等）默认休眠，PI 在 trigger 命中时召唤。
- **自适应路由** —— PI 对每个任务打分（LIGHT / MEDIUM / HEAVY），只动员需要的角色。小修小补一个 RA 处理；投稿前 final pass 出动全团队。

默认工具栈 **Python + Stata + LaTeX**。R 和 Julia 在项目声明时支持。

无后台进程、无云端同步、无上传。每个 agent 是 state-level 永久 —— 文件在硬盘上，但进程是临时的，PI 路由任务时才被唤起。

## 安装

将此模块加入你的项目：

```bash
cd MyResearchProject
aiplus add aieconlab
aiplus install codex          # 或: claude-code, opencode, all
```

`aiplus add aieconlab` 干三件事：

1. 装上所有 8 个核心角色配置和 persona（Advisor、PI、Theorist、PM、RA-Stata、RA-Python、Referee、Replicator）。
2. 装上所有 12 个 expert 配置（9 个 shipped + 3 个 v0.2 stub）。
3. **替换**默认 SWE consultant 团队（来自 `AiPlus-Auto-Team-Consultant` 的 `.aiplus/consultant-team.toml`）为 `consultant-team.aieconlab.toml` —— 5 个为应用经济学研究 plan-time 量身设计的 expert 席位、3 个 user persona、5 个 owner gate（mirror AEL DESIGN §16 STOP-gates）、LIGHT 任务默认跳过 consult。

如果同一项目里也装了 `aiplus-agent-team`（SWE），AEL 的 consultant 配置会覆盖 SWE 那个 —— 两个 consultant 配置共存放在 v0.2 roadmap。

## 快速开始

```bash
aiplus agent status              # 显示团队名册、激活专家、warm bench
aiplus agent route ra-stata      # 把任务分给 RA-Stata
aiplus agent integrate ra-stata  # 把 RA-Stata 的分支合回 main
aiplus agent audit run           # 跑验收审核
```

通过 PI 派发任务：

```text
aiplus agent route "用 cluster-robust SE 跑主 IV spec"
```

PI 给任务打分、挑出对的人、向你汇报。

其它常用命令：

```bash
aiplus agent doctor            # 检查配置、worktree、memory 布局
aiplus agent list              # 列出所有角色（核心 + 专家）
aiplus agent talk theorist     # 直接和某个角色对话
aiplus agent invite lit-reviewer       # 召唤一位专家进 active team
aiplus agent dismiss lit-reviewer      # 把专家移出 active team
aiplus agent transcript        # 显示最近活动用于审计
aiplus agent prune-worktrees   # 清理过期 worktree
```

## 架构概览

```
                  aieconlab             ← 协调层
                           ↓ uses
               AiPlus-Auto-Team-Consultant           ← 决策支持层
                           ↓ uses
    AiPlus-Agent-Memory  AiPlus-Compact-Reminder  AiPlus-Agent-Velocity
               ←——————— 共享基础设施层 ———————→
```

AiEconLab 是协调层。建在四个已有 AiPlus 插件之上：

- **AiPlus-Agent-Memory** —— 每个 agent 在 `.aiplus/agent-memory/<role>/` 下有命名空间 memory
- **AiPlus-Compact-Reminder** —— 每个长时运行的 agent 跑自己的 token-saving compact 循环；PI 跟踪每个 agent 的 compact 状态
- **AiPlus-Agent-Velocity** —— 每个 agent 有自己的 velocity 记录，单位是研究专属的（回归 spec、表格、图、paper section）
- **AiPlus-Auto-Team-Consultant** —— PI 在 MEDIUM 和 HEAVY 任务前触发顾问；顾问发现汇入团队简报

### 五个核心设计决策

1. **8 角色永久核心团队** —— 模块加入项目时自动安装。
2. **专家目录** —— 12 个 on-demand 专家，trigger 命中才召唤。
3. **State-level 永久 + warm bench** —— agent 身份在硬盘上；进程临时，PI 路由任务时才生成。
4. **Git worktree 工作区** —— 每个涉及代码的角色有独立工作目录，RA-Stata 和 RA-Python 可并行无声覆盖风险。
5. **三层 memory** —— 个人 / 团队 / 项目（已有 `.aiplus/memory/`）。冲突时项目层胜出。

完整设计原理、路由协议、memory 模型、worktree 策略、验收标准请见 [`DESIGN.md`](DESIGN.md)。

## 内容

- `core/templates/` —— 8 个核心角色 TOML 配置，加 team-wide `econ-team.toml` 和 AEL 研究专属 `consultant-team.aieconlab.toml`
- `core/templates/personas/` —— 角色 persona prompts (advisor, pi, theorist, pm, ra-stata, ra-python, referee, replicator) + 9 个 shipped expert persona
- `core/templates/personas/_stubs/` —— 3 个 v0.2 stub expert（survey-experiment、computation、coauthor-liaison）
- `core/templates/experts/` —— 12 个 expert 配置（9 shipped + 3 stub），含与 consultant 团队 seat 5 配对的 **LLM-as-Measurement Specialist**
- `adapters/codex/` —— Codex 插件和 skill 资产
- `adapters/claude-code/` —— Claude Code 项目本地命令与 agents
- `adapters/opencode/` —— OpenCode 项目本地配置、commands、prompts
- `examples/` —— 三个 runtime 的合成示例
- `tests/acceptance.test.sh` —— 15 项结构 invariants（每次 push 都跑）
- `.aiplus/aieconlab/acceptance/v0.1.0/schema.yaml` —— 绑定验收 schema

## 贡献

我们欢迎在插件 scope 内（应用经济学研究角色分工与执行，不是软件工程，不是规划顾问）的贡献。

1. **先开 issue** —— 比 typo 大的改动都先开 issue。`aieconlab` 范围紧。
2. **遵循 TOML + markdown persona 模式** —— 每个 agent 配置在 `.aiplus/agents/<role>.toml`，persona prompt 在 `.aiplus/agents/personas/<role>.md`。
3. **保持 adapter 对齐** —— 改 CLI 表面必须同步更新三个 adapters。
4. **配置改完跑 `aiplus agent doctor`** —— 验证 worktree、memory 布局、TOML schema。
5. **验收标准强制** —— 见 `.aiplus/aieconlab/acceptance/v0.1.0/schema.yaml`。行为变化必须更新 schema 和配套 `.test.sh`。

## 安全边界

AiEconLab 不会：

- 上传 agent state、persona、memory 或 transcript 到任何服务
- 作为后台进程或常驻服务运行
- 在 agent 的 persona、memory、workspace 里存 secret、IRB 保护路径、限制档案位置
- 修改全局 agent 配置（~/.codex、~/.claude 等）
- 修改其它项目的 `.aiplus/`
- 自动批准 Owner gate 的操作（投稿、发送 referee response、共享数据、推 paper 到公开 archive、声明作者顺序）
- 引入新的网络调用（host runtime 已有的除外）

## 更多

- 主平台：[AiPlus](https://github.com/izhiwen/AiPlus)
- 兄弟模块：[AiPlus-Agent-Team](https://github.com/izhiwen/AiPlus-Agent-Team)

## 许可证

[Apache-2.0](LICENSE)
