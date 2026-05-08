# AiPlus Auto Team Consultant

AiPlus Auto Team Consultant 是一个独立的 AiPlus subproduct，也是可以安装到项目里的 team-consultant module。它帮助已经打开的 AI agent 判断一个任务需要多深的团队 review：快速检查、一个聚焦专家视角，还是有边界的 team discussion。

它属于 AiPlus ecosystem，也可以从这个 repo 单独理解或采用。AiPlus 是完整生态和 CLI distribution entry；AiPlus Auto Team Consultant 是这个产品家族里的一个独立 module，不会假装自己是完整 AiPlus CLI。

它不是单独运行的 app，不上传数据，不修改全局 Codex / Claude Code / OpenCode 设置，也不会自动执行危险动作。

## 先看这里

当你希望当前 AI agent session 做更好的 routing 判断时使用它：

- 这件事应该走 `LIGHT`、`MEDIUM`，还是 `HEAVY`？
- 直接建议是否足够，还是需要一个 specialist lens？
- CEO Prompt 是否需要先 review？
- 是否触发 Owner gate 或 safety boundary？
- 是否需要模拟 pressure-test 输入？

刷新后，agent 会按本地 instructions 选择最小可用 review 深度，说明跳过哪些 lenses，标注 simulated pressure-test，并在 Owner-gated actions 前暂停。

## Path A：推荐 AiPlus ecosystem 路径

先安装 AiPlus，再把这个 module 安装到你的项目。把 `MyProject` 换成你的项目目录名：

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
cd MyProject
aiplus install codex
```

然后在已经打开的 agent session 里输入：

```text
刷新
```

或：

```text
refresh
```

如果项目里已经有旧版 AiPlus install，`aiplus install codex` 会安全升级 AiPlus managed files，把被替换的 managed files 备份到 `.aiplus/backups/`，并保留已有 `.codex/compact/` state。

## Path B：已经安装了 `aiplus` command

在你的项目目录里运行。把 `MyProject` 换成你的项目目录名：

```bash
cd MyProject
aiplus install codex
```

然后在已经打开的 agent session 里输入：

```text
AiPlus 刷新
```

其它明确 AiPlus refresh 触发语：

```text
刷新 AiPlus
aiplus refresh
aiplus status
AiPlus status
继续 AiPlus
resume AiPlus
```

泛用的 `刷新` / `refresh` 在安装后仍应优先尝试 AiPlus refresh。如果项目自己也把
`刷新` 当作项目状态刷新，请使用 `AiPlus 刷新` 或 `aiplus refresh` 避免歧义。这会
让当前 agent session 先报告 AiPlus status，再处理无关的项目 refresh，重新读取项目
本地的 AiPlus instructions，报告 Auto Team Consultant 状态，并使用 Auto Team
Consultant 的 routing 行为。

## Path C：高级 module-only 采用

如果你只想使用 AiPlus Auto Team Consultant，可以直接把这个 repo 当作参考源。高级用户可以查看或复制其中的 project-local templates、skills、prompts、adapter files 和 synthetic examples 到自己的 workflow。

这不是普通用户的优先安装路径。多数用户应该使用上面的 AiPlus CLI 路径。

## Runtime 选择

```bash
aiplus install codex
aiplus install claude-code
aiplus install opencode
aiplus install all
```

用哪个 agent，就安装对应 runtime。`all` 会安装三个 runtime 的 project-local 支持。

## 它做什么

AiPlus Auto Team Consultant 给当前 agent 一个简单 routing protocol：

- `LIGHT`：简单任务的快速检查
- `MEDIUM`：重要 prompt、docs、plan 或实现选择的聚焦 review
- `HEAVY`：高风险或重大决策的 full council

默认应该用 `LIGHT`，只有风险足够高时才升级。

它帮助当前 agent 判断：

- 什么时候只需要快速单视角检查
- 什么时候需要一个聚焦 specialist view
- 什么时候需要有边界的 team discussion
- CEO Prompt 什么时候需要 review
- 什么时候触发 safety 或 Owner gate
- 什么时候 simulated pressure-test 有用

## 什么时候用

当你希望当前 agent 做这些事时使用：

- 执行前 review prompt
- 判断是否需要 specialist lens
- 准备 CEO-style task handoff
- review 时区分 blocker 和 concern
- Builder 完成修改后请求 review
- 对用户可见 copy/flow 做模拟 pressure-test
- 明确 Owner-gated actions

## 在 agent session 里输入什么

运行 `aiplus install ...` 后，可以试：

```text
Use auto-team-consultant. Role=Advisor. Review this synthetic onboarding prompt for calendar access. Return Consultant Packet only. Do not edit files.
```

Builder handoff 示例：

```text
Use auto-team-consultant. Role=Builder. Summarize changed files, verification run, known risks, and who should review next.
```

compact readiness 不要求普通用户记住 compact 命令。自然语言是主界面：

```text
帮我准备 compact
保存进度
compact 后继续
```

agent 应在 compact 前把 `aiplus compact prepare` 当作 AiPlus backend tool 使用，
compact 后用 `aiplus compact resume` 恢复。这些命令主要是 agent 内部工具、高级
用户 fallback 和 maintainer debug commands。AiPlus Auto Team Consultant 应在
compact handoff 中保留角色上下文：Advisor recommendations、CEO task cards、
Reviewer findings、Builder changed files、Owner gates 和 next safe action。

旧项目升级时，AiPlus 会在 `aiplus install ...` 和 `aiplus update` 中保守迁移 legacy
compact handoff：先备份旧 handoff，保留用户内容，再补缺失的 role-aware fields。如果
compact readiness 被真实安全问题或 denied Owner gate 阻塞，AiPlus 应报告
`BLOCKED_DO_NOT_COMPACT`，而不是创建普通 checkpoint。

compact savings 场景下，用户可以问：

```text
看一下 compact 收益
compact 帮我省了多少？
```

agent 应映射到 `aiplus compact savings`。Savings 只是 estimates，不是 billing data，
也不是 workflow quality proof。Auto Team Consultant 可以把 savings 当作操作上下文，
但不能把 savings 当作 review、CEO plan 或 release gate 正确性的证据。

AiPlus update 场景下，用户可以说：

```text
升级 AiPlus
```

默认映射是 `aiplus update all`。更具体的映射：

- `只更新这个项目的 AiPlus` -> `aiplus update`
- `更新 aiplus 命令` -> `aiplus self update`
- `检查 AiPlus 更新` -> `aiplus self update --dry-run` 加 `aiplus status`

运行 update 前，agent 应说明不会修改全局 agent 配置，也不会上传项目数据。

private profile 和 secret status 场景下，用户可以说：

```text
work-with-zhiwen status
secret 状态
检查 API key
```

agent 应映射到 metadata-only checks，例如 `aiplus profile status`、
`aiplus secret-broker status` 或 `aiplus secret-broker doctor`。Auto Team
Consultant 可以使用 user-level profile，但优先级必须低于当前 Owner message 和项目规
则，而且不能把 private profile material 复制到 public docs、task packets、compact
files 或 result packets。

如果任务明确需要 key，agent 应优先使用 `aiplus secret-broker run -- <command...>`，
让 approved values 只进入 child process environment。绝不能 print、paste、log、
summarize、compact 或 persist secret values。

## 四种角色

- `Advisor`：直接建议、prompt review、策略判断、CEO-ready handoff
- `CEO`：拆任务、分配 scoped work、整合 result packets
- `Reviewer`：输出 findings、blockers、risks、missing tests
- `Builder`：说明 changed files、verification run、known risks 和 review request

## Pressure-Test

Pressure-Test 是对用户视角的模拟输入，只用于 user-facing perception risk。

每个 pressure-test 都必须标注：

```text
SIMULATED_PRESSURE_TEST_ONLY
```

它不是真实用户研究，不是 validation，不是 safety approval，也不是 release approval。

## Project-Local 安全边界

AiPlus Auto Team Consultant 只提供 session-local decision-support。

它不会：

- 自动 spawn agents
- 上传数据
- 添加 telemetry
- 修改全局 agent 设置
- 自动 publish、push、tag、release 或 deploy
- 批准 Owner-gated actions
- 替代 Owner decisions
- 执行真实用户研究
- 保证 safety、compliance、correctness、privacy、legal readiness、product quality 或 public-release readiness

当前 agent 仍然负责 scope control、verification 和 Owner-gated actions。

## Runtime 支持

| Runtime | Install command | What gets added | Automation level |
| --- | --- | --- | --- |
| Codex | `aiplus install codex` | project-local Codex instructions | session-local |
| Claude Code | `aiplus install claude-code` | project-local Claude Code commands/instructions | project-local |
| OpenCode | `aiplus install opencode` | project-local OpenCode commands/prompts | project-local |
| All | `aiplus install all` | all supported runtime files | project-local |

## 高级：Core 和 Adapters

普通用户优先使用 `aiplus install ...`。

本 repo 也保留可复用源文件：

- `core/docs/`：runtime-neutral protocol docs
- `core/templates/`：packet 和 routing templates
- `adapters/codex/`：Codex instruction source
- `adapters/claude-code/`：Claude Code project-local command/agent source
- `adapters/opencode/`：OpenCode project-local config/command/agent/prompt source
- `examples/`：只包含 synthetic examples

不确定用哪个 packet 时，看 `core/templates/TEMPLATE_INDEX.md`。

## 当前状态

这是 AiPlus Auto Team Consultant 的 public source module。推荐用户路径是 Rust-first `aiplus` CLI：

```bash
aiplus install codex
```

不包含 npm package、package registry publish、GitHub Release、git tag、marketplace submission、global install、telemetry、MCP server、App connector 或 autonomous executor。
