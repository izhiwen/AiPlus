# AiPlus

AiPlus 帮助 AI coding agent 在项目内保留 continuity、handoff 和 review
workflow，支持 Codex、Claude Code 和 OpenCode。

`AiPlus` 是产品/项目名。`aiplus` 是 CLI command、binary、crate 和 repo 名。

## 快速开始

安装 `aiplus` command：

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
```

然后把 AiPlus 安装到当前项目：

```bash
cd MyProject
aiplus install codex
```

如果当前项目已经有旧版 AiPlus install，同一条命令会安全升级 AiPlus managed
files，把被替换的 managed files 备份到 `.aiplus/backups/`，并保留
`.codex/compact/` state。

然后在同一个项目里已经打开的 Codex、Claude Code 或 OpenCode session 输入：

```text
AiPlus 刷新
```

想 compact 或保存进度时，也是在 agent session 里说：

```text
帮我准备 compact
```

或者：

```text
保存进度
```

compact 后如果 agent 没有自动回复，说：

```text
继续
```

English triggers also work: `AiPlus refresh`, `prepare compact`, `save progress`,
and `continue`.

泛用的 `刷新` / `refresh` 在安装后仍应优先尝试 AiPlus refresh。如果项目自己也把
`刷新` 当作项目状态刷新，请使用 `AiPlus 刷新` 或 `aiplus refresh` 避免歧义。当你
明确要求 AiPlus 时，agent 应先报告 Auto Compact、Auto Team Consultant 和
compact-state 状态，再处理无关的项目 refresh。

Claude Code：

```bash
aiplus install claude-code
```

OpenCode：

```bash
aiplus install opencode
```

v0.2.1 的 one-command installer 先验证 macOS Apple Silicon。其它平台在 release
asset 发布并验证前，请使用 [Developer Build](#developer-build)。

## Runtime Choices

可以为单个 runtime 或全部 supported runtimes 安装 AiPlus：

```bash
aiplus install codex
aiplus install claude-code
aiplus install opencode
aiplus install all
```

Runtime adapters 都是 project-local。Codex 使用 project `AGENTS.md` managed
block，Claude Code 使用 project `.claude/` files，OpenCode 使用 project
`.opencode/` files。

## 常用检查

```bash
aiplus status
aiplus refresh
aiplus doctor
aiplus update
aiplus uninstall --dry-run
```

## 会安装什么

AiPlus 只写 project-local files：

- `.aiplus/`
- `.codex/compact/`
- project `.claude/` adapter files
- project `.opencode/` adapter files
- project `AGENTS.md` 中的 AiPlus managed block

Bundled modules：

- **AiPlus Auto Compact** (`auto-compact`)：compact、checkpoint、validate 和
  resume workflow assets。
- **AiPlus Auto Team Consultant** (`auto-team-consultant`)：Advisor、CEO、
  Reviewer 和 Builder routing assets。

## Compact And Resume

你不需要记住 compact 命令。

在 agent 对话里说：

```text
帮我准备 compact
```

或者：

```text
保存进度
```

agent 会自动用 AiPlus backend tools 检查 readiness 并准备 checkpoint。如果可以
compact，它会用普通语言回复：

```text
现在可以 compact 了。

compact 后如果我没自动继续，你发一句“继续”就行。我会从刚才的位置接着做。
```

compact 后，如果它没有自动继续，你说：

```text
继续
```

AiPlus 会 best-effort resume：

- 如果 agent 自动继续，你不需要做任何事。
- 如果 agent 没回复，发一句 `继续`。

AiPlus 不能强制 host compact，不能点击 UI compact，不能代替你调用 `/compact`，也
不能在 host 要求用户输入时主动唤醒 agent。

高级用户和 maintainer 可以直接运行 backend commands：

```bash
aiplus compact prepare
aiplus compact score
aiplus compact checkpoint --level standard
aiplus compact resume
```

如果找不到 `aiplus`，请安装 AiPlus 或修复 PATH，不要 fallback 到 Node：

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
```

然后重新打开 terminal，或确认 `~/.local/bin` 已在 PATH 中。

## Installer Safety

`install.sh` 会下载 GitHub Release asset，校验 `checksums.txt`，默认只把
`aiplus` command 安装到 `~/.local/bin/aiplus`。它不使用 `sudo`，不静默修改 shell
profiles，不自动安装 project modules，不上传数据，不添加 telemetry，也不修改 global
Codex、Claude Code 或 OpenCode config。AiPlus v0.2.1 先发布已验证的 macOS Apple
Silicon asset；其它平台 asset 仍是 planned。

见 [distribution-plan.md](docs/distribution-plan.md) 和
[installer-plan.md](docs/installer-plan.md)。

## Developer Build

```bash
git clone https://github.com/izhiwen/aiplus.git
cd aiplus
cargo build --release
```

在目标项目中运行：

```bash
~/aiplus/target/release/aiplus install codex
```

旧文档中的 `<AIPLUS_SOURCE>` 意思是“你 clone AiPlus repo 的目录”。不要把尖括号
placeholder 原样输入 terminal。

## Public-Ready Docs

- [MODULES.md](MODULES.md)
- [architecture.md](docs/architecture.md)
- [public-repo-plan.md](docs/public-repo-plan.md)
- [distribution-plan.md](docs/distribution-plan.md)
- [installer-plan.md](docs/installer-plan.md)
- [binary-artifact-matrix.md](docs/binary-artifact-matrix.md)
- [migration-from-node-cli.md](docs/migration-from-node-cli.md)
- [qa-release-readiness.md](docs/qa-release-readiness.md)
- [safety.md](docs/safety.md)
- [RELEASE_CHECKLIST.md](RELEASE_CHECKLIST.md)

## Node Reference Status

legacy Node CLI 是 archived/reference-only v0.2.1，不包含在本 public source
package 中。它保留在 private/local AiPlus workspace，用于 behavior audit 和
emergency reference fixes。新的 CLI work 应进入 Rust。

compact commands 已是 Rust-native。Rust runtime assets 不再 install 或 check
`compactctl.mjs`。

## Safety Boundary

AiPlus CLI 不实现 publish、push、tag、release creation、system/global install、
global config edit、telemetry、auto-update 或 runtime network fetch。v0.2.1
installer 只写 user-level `~/.local/bin/aiplus` command。

validation 是 structural 和 heuristic，不是 safety、privacy、compliance、
correctness 或 release certification。
