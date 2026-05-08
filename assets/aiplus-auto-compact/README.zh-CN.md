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
刷新
```

或者：

```text
refresh
```

含义：优先当作 AiPlus refresh，先报告 Auto Compact 和 compact state，再重新读取
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

compact 前，agent 应在可用时运行：

```bash
aiplus compact validate
aiplus compact checkpoint
```

如果 checkpoint 已准备好，agent 应推荐用户手动 compact，话术类似：

```text
建议现在 compact。AiPlus checkpoint 已准备好。compact 后如果宿主继续把控制权交给我，我会自动恢复；如果工具等待你发消息，随便说“继续”“刷新”“continue”“resume”或类似意思即可。
```

host compact 完成后：

- 如果 host 自动把控制权交回给 agent，agent 应运行 `aiplus compact resume` 并继续，不需要用户再发消息。
- 如果 host 需要用户发消息，任何自然的继续意图都应该可用：

```text
继续
刷新
refresh
continue
resume
go on
接着
```

这是 best-effort automatic resume。AiPlus Auto Compact 可以准备 checkpoint，并告诉
agent 如何 resume，但不能唤醒一个正在等待用户消息的 host runtime。

## 日常命令

```bash
aiplus status
aiplus doctor
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
- `core/scripts/compactctl.mjs`：legacy standalone Node helper，仅保留用于 compatibility tests 和 migration reference。

## 它能自动化什么

AiPlus Auto Compact 可以：

- 在目标项目中创建本地 compact state files。
- 除非使用明确的 force flow，否则保留已有 compact files。
- 检查 required files、sections、enum values、policy JSON、version fields、Owner gates、next actions 和明显 sensitive patterns。
- 在 `.codex/compact/checkpoints/` 下写本地 checkpoint JSON。
- compact 后打印 resume-oriented state。
- host runtime 在 compact 后把控制权交回给 agent 时，支持 best-effort automatic resume。
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

当前普通用户路径是 AiPlus CLI（`aiplus`）。旧 standalone Node helper 仍保留在
这个 repo 中，用于 compatibility 和 migration review：

```bash
node <REPO_ROOT>/aiplus-auto-compact/core/scripts/compactctl.mjs init
node <REPO_ROOT>/aiplus-auto-compact/core/scripts/compactctl.mjs validate
node <REPO_ROOT>/aiplus-auto-compact/core/scripts/compactctl.mjs checkpoint
node <REPO_ROOT>/aiplus-auto-compact/core/scripts/compactctl.mjs resume
```

只有在 audit legacy workflow 或没有 AiPlus CLI 时测试这个 module，才使用它。

## 验证

在本 repo 中运行 package acceptance tests：

```bash
cd <REPO_ROOT>/aiplus-auto-compact
npm test
node --check core/scripts/compactctl.mjs
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
node --check core/scripts/compactctl.mjs
```

行为变更要记录在 `CHANGELOG.md`；ownership 或 file layout 改变时更新 `MODULES.md`；publication review 前更新 `RELEASE_CHECKLIST.md`。
