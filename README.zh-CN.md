# AiPlus Rust CLI

本仓库是 AiPlus `aiplus` binary 的 Rust mainline workspace。

推荐 public repo name：`aiplus`。

package/crate name 暂时保留 `aiplus-cli`；binary name 是 `aiplus`。

License：Apache-2.0。该 license 适用于本 workspace 中的 Rust
mainline/public-ready package。bundled child module snapshots 保留其既有
license。license 不是 safety、privacy、compliance、correctness 或 release
certification。

## 初学者流程

构建当前 local source candidate：

```bash
cd aiplus
cargo build --release
```

在目标项目中安装：

```bash
cd MyProject
<AIPLUS_SOURCE>/target/release/aiplus install codex
```

如果本地测试 binary 已经在 PATH：

```bash
cd MyProject
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

## Runtime Installs

```bash
aiplus install codex
aiplus install claude-code
aiplus install opencode
aiplus install all
```

Runtime adapters 都是 project-local：Codex 更新 project `AGENTS.md` managed
block，Claude Code 写 project `.claude/` files，OpenCode 写 project
`.opencode/` files。

兼容 alias 仍保留：

```bash
aiplus install claude
aiplus install cc
aiplus install oc
aiplus install --runtime codex
aiplus install --all-runtimes
```

## 维护命令

```bash
aiplus status
aiplus doctor
aiplus update
aiplus update auto-compact
aiplus update auto-team-consultant
aiplus add auto-compact
aiplus add auto-team-consultant
aiplus compact validate
aiplus compact checkpoint
aiplus compact resume
aiplus uninstall --dry-run
```

## Public-Ready Docs

- [architecture.md](docs/architecture.md)
- [public-repo-plan.md](docs/public-repo-plan.md)
- [distribution-plan.md](docs/distribution-plan.md)
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

## 安全边界

CLI 只写 project-local files：`.aiplus/`、`.codex/compact/`、project
`.claude/` adapter files、project `.opencode/` adapter files，以及 project
`AGENTS.md` 中的 AiPlus managed block。

它不实现 publish、push、tag、release creation、global install、global config
edit、telemetry、auto-update 或 runtime network fetch。
