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
aiplus install codex
```

然后在同一个项目里已经打开的 Codex、Claude Code 或 OpenCode session 输入：

```text
刷新
```

英文也可以：

```text
refresh
```

Claude Code：

```bash
aiplus install claude-code
```

OpenCode：

```bash
aiplus install opencode
```

v0.1.0 的 one-command installer 先验证 macOS Apple Silicon。其它平台在 release
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

在适合 compact 的时机前，先让 agent 准备状态：

```bash
aiplus compact validate
aiplus compact checkpoint
```

agent 应该用普通语言建议 compact：

```text
建议现在 compact。AiPlus checkpoint 已准备好。compact 后如果宿主继续把控制权交给我，我会自动恢复；如果工具等待你发消息，随便说“继续”“刷新”“continue”“resume”或类似意思即可。
```

host compact 之后，AiPlus 会 best-effort resume：

- 如果 host 自动把控制权交回 agent，agent 应自动运行 `aiplus compact resume`。
- 如果 host 等待用户消息，随便说 `继续`、`刷新`、`continue`、`resume`、
  `refresh`、`go on` 或 `接着` 都可以。

AiPlus 不能强制 host compact，不能点击 UI compact，不能代替你调用 `/compact`，也
不能在 host 要求用户输入时主动唤醒 agent。

## Installer Safety

`install.sh` 会下载 GitHub Release asset，校验 `checksums.txt`，默认只把
`aiplus` command 安装到 `~/.local/bin/aiplus`。它不使用 `sudo`，不静默修改 shell
profiles，不自动安装 project modules，不上传数据，不添加 telemetry，也不修改 global
Codex、Claude Code 或 OpenCode config。AiPlus v0.1.0 先发布已验证的 macOS Apple
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

legacy Node CLI 是 archived/reference-only v0.1.3，不包含在本 public source
package 中。它保留在 private/local AiPlus workspace，用于 behavior audit 和
emergency reference fixes。新的 CLI work 应进入 Rust。

compact commands 已是 Rust-native。Rust runtime assets 不再 install 或 check
`compactctl.mjs`。

## Safety Boundary

AiPlus CLI 不实现 publish、push、tag、release creation、system/global install、
global config edit、telemetry、auto-update 或 runtime network fetch。v0.1.0
installer 只写 user-level `~/.local/bin/aiplus` command。

validation 是 structural 和 heuristic，不是 safety、privacy、compliance、
correctness 或 release certification。
