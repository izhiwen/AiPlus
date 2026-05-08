# AiPlus

AiPlus 帮助 AI coding agent 在项目内保留 continuity、handoff 和 review
workflow，支持 Codex、Claude Code 和 OpenCode。

`AiPlus` 是产品/项目名。`aiplus` 是 CLI command、binary、crate 和 repo 名。

## 快速开始

在你想使用 AiPlus 的项目目录里运行：

```bash
AIPLUS_HOME="$HOME/aiplus"; test -d "$AIPLUS_HOME" || git clone https://github.com/izhiwen/aiplus.git "$AIPLUS_HOME"; (cd "$AIPLUS_HOME" && cargo build --release); "$AIPLUS_HOME/target/release/aiplus" install codex
```

然后在同一个项目里已经打开的 Codex、Claude Code 或 OpenCode session 输入：

```text
刷新
```

英文也可以：

```text
refresh
```

如果 `aiplus` command 已经在你的 `PATH` 里，项目安装只需要：

```bash
aiplus install codex
```

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

## 当前安装状态

上面的 quick start 现在采用 source build，因为本 repo 还没有发布 GitHub Release
binary 或 installer script。

旧文档中的 `<AIPLUS_SOURCE>` 意思是“你 clone AiPlus repo 的目录”。不要把尖括号
placeholder 原样输入 terminal。

## Future Installer Plan

未来理想的 beginner flow 是：

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
cd MyProject
aiplus install codex
```

这个 flow 目前还没有启用。它需要 Owner 批准 GitHub Release binaries、checksums、
installer script，以及任何 global/PATH install behavior。未来 installer 不应静默
修改 shell profiles，不应自动安装 project modules，不应上传数据，不应添加 telemetry，
也不应修改 global Codex、Claude Code 或 OpenCode config。

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

AiPlus 不实现 publish、push、tag、release creation、global install、global config
edit、telemetry、auto-update 或 runtime network fetch。

validation 是 structural 和 heuristic，不是 safety、privacy、compliance、
correctness 或 release certification。
