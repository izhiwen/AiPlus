# AiPlus Auto Compact

AiPlus Auto Compact 是 AiPlus 家族里的独立子产品，用来做 compact /
checkpoint / resume handoff。它可以作为 AiPlus CLI（`aiplus`）安装的 bundled
module 使用，也可以被只想采用 compact workflow 的用户直接阅读和改造。

当 Codex、Claude Code 或 OpenCode 的长任务快要 context compaction 时，它帮助
agent 检查 compact readiness、compact 前创建 checkpoint、compact 后从
checkpoint resume，让 handoff 更干净。

它不会替你点击 compact 按钮，不会调用 `/compact`，不会上传数据，不会修改全局
runtime 设置，也不会保证 compact 一定安全。

## 初学者流程

### Path A: AiPlus ecosystem install

先安装 AiPlus，再把 project-local compact module 安装到你的项目：

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
cd MyProject
aiplus install codex
```

如果项目里已经有旧版 AiPlus install，`aiplus install codex` 会安全升级 AiPlus
managed files，把被替换的 managed files 备份到 `.aiplus/backups/`，并保留已有
`.codex/compact/` state。

### Path B: 已安装 `aiplus` command

如果 `aiplus` 已经在你的 `PATH` 上，在你要保护的项目中运行：

```bash
cd MyProject
aiplus install codex
```

其他 runtime：

```bash
aiplus install claude-code
aiplus install opencode
aiplus install all
```

安装是 project-local。它可能写入 `.aiplus/`、`.codex/compact/`、project `.claude/` files、project `.opencode/` files，以及 project `AGENTS.md` 里的 AiPlus managed block。它不会修改全局 Codex、Claude Code、OpenCode、shell 或 package-manager 配置。

然后在同一个项目里已经打开的 Codex、Claude Code 或 OpenCode session 输入：

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
`刷新` 当作项目状态刷新，请使用 `AiPlus 刷新` 或 `aiplus refresh` 避免歧义。含义：
先报告 Auto Compact 和 compact state，再处理无关的项目 refresh，重新读取
`AGENTS.md`、`.aiplus/AGENTS.aiplus.md`，如果存在则读取
`.codex/compact/current-handoff.md`，启用 AiPlus guidance，并继续当前任务。

### Path C: Advanced / module-only adoption

如果你只想用 AiPlus Auto Compact，可以直接阅读这个 repo，检查
`core/templates/`、`core/docs/` 和各 runtime adapters，然后把 compact workflow
复制或改造成适合自己项目的版本。这适合 policy review、自定义集成，或从
`codex-compact-protocol` 迁移。

module-only path 不是 beginner install path。旧 Node helper 只保留在
[Advanced: Legacy Node Reference](#advanced-legacy-node-reference) 里，用于 audit
和 compatibility checks。

## Compact 前后会发生什么

你不需要记住 compact 命令。

在 agent 对话里说：

```text
帮我准备 compact
```

或者：

```text
保存进度
```

agent 应把 `aiplus compact prepare` 当作 backend tool 自动使用。如果 `prepare`
还不可用，它可以 fallback 到最接近的支持序列：`aiplus compact validate` 后接
`aiplus compact checkpoint`。如果 checkpoint 已准备好，agent 应推荐用户手动
compact，话术类似：

```text
现在可以 compact 了。

compact 后如果我没自动继续，你发一句“继续”就行。我会从刚才的位置接着做。
```

host compact 完成后：

- 如果 agent 自动继续，你不需要做任何事。
- 如果 agent 没回复，说：

```text
继续
```

agent 应运行 `aiplus compact resume`，然后从恢复出来的状态继续。`continue`、
`resume`、`refresh`、`go on`、`继续`、`刷新`、`接着` 等自然继续意图都应该可用。

这是 best-effort automatic resume。AiPlus Auto Compact 可以准备 checkpoint，并告诉
agent 如何 resume，但不能唤醒一个正在等待用户消息的 host runtime。

## Compact Savings Estimate

AiPlus Auto Compact 支持 AiPlus v0.3 savings estimate flow。普通用户可以在 agent
对话里问：

```text
看一下 compact 收益
```

或者：

```text
compact 帮我省了多少？
```

agent 应把这些请求映射到：

```bash
aiplus compact savings
```

Savings 是本地估算。AiPlus 会把 aggregate events 写到
`.codex/compact/savings-ledger.jsonl`，报告本次 compact 和累计 totals，并用 weighted
formula 计算累计 reduction：
`totalEstimatedTokensSaved / totalEstimatedBaselineTokens * 100`。

Pricing 是 cache-first。`aiplus compact savings`、`prepare`、`checkpoint`、`resume`
优先使用 fresh cached public pricing。如果 cache 缺失或过期，AiPlus 可能自动刷新
public pricing；network failure 不会阻塞 compact。`aiplus pricing update` 会显式刷新
public pricing data。如果检测到模型但没有价格，token savings 和 reduction 仍会报告，
USD savings 会显示 unavailable 或 partial。

Savings reports 只是 estimates，不是 billing data。AiPlus 不上传 prompts、project
files、checkpoints、savings ledgers、secrets、billing data 或 usage history。

## 更新 AiPlus

用户可以在 agent 对话里说：

```text
升级 AiPlus
```

默认映射：

```bash
aiplus update all
```

更具体的映射：

- `只更新这个项目的 AiPlus` -> `aiplus update`
- `更新 aiplus 命令` -> `aiplus self update`
- `检查 AiPlus 更新` -> `aiplus self update --dry-run` 加 `aiplus status`

运行 update 前，agent 应说明：我会更新 aiplus 命令和当前项目里的 AiPlus 模块；不会
修改全局 agent 配置，也不会上传项目数据。

已有旧 `.codex/compact/current-handoff.md` 的项目会被 `aiplus install ...` 和
`aiplus update` 保守升级：AiPlus 会先备份旧 handoff，保留用户内容，然后只补缺失的
role-aware sections，例如 `Session Role`、`Workflow Level` 和 `Output Contract`。

如果 validation 因真实安全问题或 denied Owner gate 被阻塞，`aiplus compact
checkpoint` 会打印 `BLOCKED_DO_NOT_COMPACT`，默认不会创建普通 checkpoint file。

## Private Profile 与 Secret Boundary

AiPlus 可以配合 private user-level profile 和
Bitwarden-backed `secret-broker` 使用。AiPlus Auto Compact 必须把它们视为 private
runtime layer，而不是 bundled module content。

compact handoff、checkpoint 和 savings ledger 绝不能存 secret value、Bitwarden
machine token、auth header、provider response body 或 raw profile-private
material。如果 compact flow 需要知道 secret 是否可用，agent 应运行 metadata-only
检查：

```bash
aiplus profile status
aiplus secret-broker status
```

如果 `aiplus profile status` 报告 `legacy_profiles=[...]`，canonical private
profile 安装后运行 `aiplus profile cleanup --user --yes`。Compact flow 应使用
canonical active profile，不应把 private profile material 写入 handoff 或 checkpoint。

只有在明确 action need 下，才用 `aiplus secret-broker run -- <command...>` 给子进程
注入 secret。不要在 compact guidance 或 handoff files 里打印 resolved secret value。
child command 会在 environment 里收到 secret，仍可能自己 print、log、transmit 或
store。只对可信且符合当前 action need 的命令使用 `run --`。

Private profile 可以安装 approved secret aliases。当前本地映射以 `aiplus secret-broker list` 为准。真实 Bitwarden smoke check 需要 Bitwarden Secrets Manager `bws` CLI 和 read-only machine account token；如果没有 `bws`，agent 只能做 mock/local status check，不能 fallback 到打印 secret value。

## 日常命令

```bash
aiplus status
aiplus doctor
aiplus compact prepare
aiplus compact score
aiplus compact validate
aiplus compact checkpoint
aiplus compact resume
aiplus uninstall --dry-run
```

只有当 `checkpoint` 返回 `SAFE_TO_COMPACT`，并且所有 Owner gates 都明确为 `APPROVED`，才可以推荐 manual compact。`UNKNOWN_PENDING` 表示 `UNKNOWN_NEEDS_REVIEW`；`DENIED` 会阻塞 compact recommendation。

## Runtime 选择

| Runtime | Install command | Auto compact support | Recommended use |
| --- | --- | --- | --- |
| Codex | `aiplus install codex` | reminder/checkpoint only | 运行 `aiplus compact checkpoint`，review 输出，然后手动使用 Codex `/compact` 或 UI compact。 |
| Claude Code | `aiplus install claude-code` | optional reviewed hooks and commands | 使用 project-local Claude Code adapter files；启用 hooks 前先 review。 |
| OpenCode | `aiplus install opencode` | project-local command workflow | 使用 project `.opencode/` commands、agents、prompts；默认不改 global config。 |

AiPlus CLI 也支持兼容 alias：

```bash
aiplus install claude
aiplus install cc
aiplus install oc
aiplus install --runtime codex
aiplus install --all-runtimes
```

## 这个 repo 里有什么

这个 repository 是 compact workflow module source，不是完整的 AiPlus CLI
distribution。

- `core/templates/`：compact handoff templates。
- `core/schemas/`：policy 和 ledger structure 的 JSON schemas。
- `core/docs/`：protocol、Owner gate、checkpoint/resume reference。
- `adapters/codex/`：Codex plugin 和 skill assets。
- `adapters/claude-code/`：Claude Code plugin-shaped commands、skill、optional hook example。
- `adapters/opencode/`：OpenCode project-local config、commands、agents、prompts。
- `examples/`：三个 runtime 的 synthetic examples。
- `core/scripts/compactctl.mjs`：archived legacy Node helper，仅保留用于 compatibility tests 和 migration reference；不是 active compact path。

## 它能自动化什么

AiPlus Auto Compact 可以：

- 在目标项目中创建本地 compact state files。
- 除非使用明确的 force flow，否则保留已有 compact files。
- 检查 required files、sections、enum values、policy JSON、version fields、Owner gates、next actions 和明显 sensitive patterns。
- 在 `.codex/compact/checkpoints/` 下写本地 checkpoint JSON。
- compact 后打印 resume-oriented state。
- compact 后支持 best-effort automatic resume。
- 默认保持 runtime setup project-local。

## 它不能自动化什么

AiPlus Auto Compact 不能：

- 点击 UI controls、调用 `/compact`，或强制任何 runtime compact。
- 唤醒一个 compact 后正在等待用户消息的 host runtime。
- 证明每个项目都适合 compact。
- 检出所有 secrets、private paths 或 personal-data patterns。
- 取代 Owner gates 的人工 review。
- publish packages、create tags、upload artifacts 或配置 cloud services。
- 让 Codex、Claude Code、OpenCode 共享同一种内部 context implementation。

## 安全和隐私边界

compact state 默认应留在本地，除非已经明确 review 和 redaction。不要把 secrets、tokens、API keys、private keys、raw transcripts、provider request/response bodies、HAR/WebRTC dumps、private account identifiers、private paths 或 personal data 写入 compact files。

公开 docs 和 examples 使用 placeholder：

- `<REPO_ROOT>`
- `<TARGET_PROJECT>`
- `<OWNER>`
- `<GITHUB_OWNER>`
- `<EXAMPLE_PROJECT>`
- `<REDACTED_SECRET>`
- `<REDACTED_TOKEN>`
- `<REDACTED_PII>`

validation 是 structural 和 heuristic。通过 validation 只代表文件满足本地 contract；不代表项目已经适合 compact、安全发布、compliant 或 private。

## Advanced: Legacy Node Reference

当前支持的 compact execution path 只使用 Rust-native AiPlus CLI：

```bash
aiplus compact validate
aiplus compact checkpoint
aiplus compact resume
```

旧 standalone Node helper 只作为 archived history 和 compatibility test
fixture 保留在这个 repo 中。它不是普通用户路径，也不能作为 fallback。

如果找不到 `aiplus`，请安装 AiPlus 或修复 PATH：

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
```

然后重新打开 terminal，或确认 `~/.local/bin` 已在 PATH 中。不要 fallback 到 Node。

## 验证

在本 repo 中运行 package acceptance tests：

```bash
cd <REPO_ROOT>/aiplus-auto-compact
npm test
```

在已有 `aiplus` 的目标项目中运行 compact checks：

```bash
cd MyProject
aiplus compact validate
aiplus compact checkpoint
aiplus compact resume
```

legacy helper 和 AiPlus compact flow 使用的 exit codes：

- `0`：pass 或 safe。
- `1`：blocked 或 invalid。
- `2`：unknown 或 inconclusive。
- `3`：internal error。

## 示例

synthetic examples：

- [Codex project](examples/codex-project/README.md)
- [Claude Code project](examples/claude-code-project/README.md)
- [OpenCode project](examples/opencode-project/README.md)

这些 examples 只使用 placeholders，不来自 private projects。

## 迁移

`codex-compact-protocol` 是 legacy Codex-first public record。AiPlus Auto
Compact 是当前 cross-agent compact workflow module，AiPlus CLI（`aiplus`）是当前
user-facing path。

已有 `.codex/compact/` state 仍兼容。详情见 [Migration From Codex Compact Protocol](docs/migration-from-codex-compact-protocol.md)。

## 当前发布状态

这个 repository 已在 GitHub public 发布为 `aiplus-auto-compact`。它没有发布到
npm、Cargo、Homebrew、任何 package registry 或 marketplace。AiPlus CLI
（`aiplus`）是普通用户的预期 distribution surface。未来 tags、GitHub
Releases、package publications、binary uploads、marketplace submissions 或
installer publication 都需要单独 Owner approval。

## 贡献

除非变更明确属于某个 adapter，否则保持 local-first 和 runtime-neutral。公开 docs 和 examples 必须使用 placeholders，不要使用 private paths 或真实 project data。

提出 release change 前运行：

```bash
cd <REPO_ROOT>/aiplus-auto-compact
npm test
```

行为变更要记录在 `CHANGELOG.md`；ownership 或 file layout 改变时更新 `MODULES.md`；publication review 前更新 `RELEASE_CHECKLIST.md`。
