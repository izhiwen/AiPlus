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

v0.4.6 的 one-command installer 先验证 macOS Apple Silicon。其它平台在 release
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
aiplus update all
aiplus self update --dry-run
aiplus compact savings
aiplus pricing status
aiplus profile status
aiplus secret-broker status
aiplus uninstall --dry-run
```

## 私有用户 Profile 与 Secret Broker

AiPlus 也可以安装 user-level private profile，并通过受控 broker 解析运行时
secret，而不会把 private content 放进 public repo。

```bash
aiplus profile install <private-profile-name> --user --source /path/to/private-profile --dry-run
aiplus profile install <private-profile-name> --user --source /path/to/private-profile --yes
aiplus profile status
aiplus profile cleanup --user --dry-run
aiplus profile cleanup --user --yes
aiplus profile migrate <legacy-profile> <canonical-profile> --user --yes
aiplus profile disable <private-profile-name> --user --yes
aiplus profile uninstall <private-profile-name> --user --yes
aiplus secret-broker status
```

private profile 位于 `~/.config/aiplus/profiles/<private-profile-name>/`。它只
存工作偏好和协作规则，不应包含 API key、Bitwarden token、password、prompt
transcript、project file 或 compact checkpoint。

`aiplus profile status` 会把 active canonical profiles 放在 `profiles=[...]`。
legacy compatibility profiles 可能单独出现在 `legacy_profiles=[...]`；canonical
profile 安装后，运行 `aiplus profile cleanup --user --yes` 会先备份再移除 legacy
active registration。

secret 访问统一走 `aiplus secret-broker`。默认
`aiplus secret-broker resolve <alias>` 只验证访问，不打印 secret value。
`aiplus secret-broker list` 会列出 private profile package 安装的 aliases。Public
AiPlus 不内置 private alias namespace。
对于 Bitwarden，AiPlus 会在内存中把 alias key/name 映射到 Bitwarden secret ID，再
通过 `bws` 读取 value；默认只打印 `secret_id_found=yes` 等 metadata，不打印 secret
ID 或 secret value。

真实 Bitwarden smoke check 需要安装 `bws` CLI，并通过 `BWS_ACCESS_TOKEN` 或 macOS
Keychain 提供 read-only machine account token。需要把 key 传给工具时，使用：

```bash
aiplus secret-broker run -- <command...>
```

child command 会在环境变量里收到 approved secrets。AiPlus 不会打印或持久化这些值，
但 child command 自己仍可能 print、log、transmit 或 store 它们。只对你信任且符合当
前 action need 的命令使用 `run --`。

AiPlus 可以读取当前进程里的 `BWS_ACCESS_TOKEN`，也可以读取由
`aiplus secret-broker token set` 创建的 macOS Keychain entry。它不会把 Bitwarden
machine token 存到 repo files、`.aiplus/`、`.codex/compact/`、shell profiles、
logs、docs、compact savings ledger 或 release artifacts。

private profile 可以提供自然语言 mapping。secret status 请求应映射到 metadata-only
checks，绝不暴露 secret value。

## 更新 AiPlus

在 agent 对话里可以说：

```text
升级 AiPlus
```

agent 应先报告 scope：

```text
我会更新 aiplus 命令和当前项目里的 AiPlus 模块；不会修改全局 agent 配置，也不会上传项目数据。
```

然后运行：

```bash
aiplus update all
```

更具体的命令：

```bash
aiplus self update --dry-run  # 检查 user-level CLI update
aiplus self update --yes      # 更新 user-level aiplus command
aiplus update                 # 只更新当前项目的 .aiplus/ modules
aiplus update all             # 更新 CLI，再更新当前项目，然后建议 doctor
```

已安装 guidance 也支持 `update AiPlus`、`把 AiPlus 全部更新到最新版`、
`只更新这个项目的 AiPlus` 和 `更新 aiplus 命令`。

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
aiplus compact savings
```

如果找不到 `aiplus`，请安装 AiPlus 或修复 PATH，不要 fallback 到 Node：

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
```

然后重新打开 terminal，或确认 `~/.local/bin` 已在 PATH 中。

## Compact Savings Estimate

AiPlus 会用本地 aggregate compact metadata 估算 compact 节省。它不要求配置价格、
配置模型、连接 provider account、读取 billing API，也不要求用户手动输入模型价格。

在 agent 对话里说：

```text
看一下 compact 收益
```

或者运行：

```bash
aiplus compact savings
```

默认报告会同时显示本次 compact 和累计：

```text
Auto Compact 节省估算

本次 compact：
- 节约 tokens：约 18k
- token 减少比例：约 41%
- 估算节约成本：约 $0.05
- 恢复信心：HIGH

累计：
- 节约 tokens：约 184k
- 平均减少比例：约 38%
- 估算节约成本：约 $0.46
- pricing 覆盖：8/10 次 compact

仅为估算，不是账单数据。
```

累计减少比例使用 weighted average：
`totalEstimatedTokensSaved / totalEstimatedBaselineTokens * 100`，不是每次
compact percentage 的简单平均。

AiPlus 会把 aggregate savings events 写到
`.codex/compact/savings-ledger.jsonl`。ledger 不应保存 prompts、transcripts、
project file contents、raw checkpoint text、billing data 或 usage history。如果
检测到模型但没有对应价格，AiPlus 仍会报告 token savings 和 reduction percentage；
USD savings 会显示 unavailable 或 partial。

Savings event semantics：

- `prepare`：projected readiness estimate；不计入 completed all-time savings。
- `checkpoint`：candidate estimate；单独 checkpoint 不计入 completed all-time savings。
- `resume`：completed compact cycle；同一个 `checkpointId` 只计一次。

重复运行同一个 checkpoint 的 `resume` 不会重复增加 all-time totals。

Pricing cache policy：

```bash
aiplus pricing status
aiplus pricing update
```

AiPlus 会优先使用 fresh cached pricing。如果 cache 缺失或过期，AiPlus 可能自动刷新
public pricing；network failure 不会阻塞 compact、checkpoint、resume 或 token
savings reporting。`aiplus pricing update` 会显式刷新 public pricing data，并把
cache 写到 user cache directory，通常是 `~/.cache/aiplus/pricing-cache.json`。默认
cache TTL 是 7 天。

## Installer Safety

`install.sh` 会下载 GitHub Release asset，校验 `checksums.txt`，默认只把
`aiplus` command 安装到 `~/.local/bin/aiplus`。它不使用 `sudo`，不静默修改 shell
profiles，不自动安装 project modules，不上传数据，不添加 telemetry，也不修改 global
Codex、Claude Code 或 OpenCode config。AiPlus v0.4.6 先发布已验证的 macOS Apple
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

legacy Node CLI 是 archived/reference-only，不包含在本 public source
package 中。它保留在 private/local AiPlus workspace，用于 behavior audit 和
emergency reference fixes。新的 CLI work 应进入 Rust。

compact commands 已是 Rust-native。Rust runtime assets 不再 install 或 check
`compactctl.mjs`。

## Safety Boundary

AiPlus CLI 不实现 package publish、system/global install、global config edit、
telemetry、auto-update callback、provider account access 或 user data upload。
`aiplus pricing update` 可能获取 public release/pricing metadata 并缓存到本地。它
不上传 prompts、project files、checkpoints、savings ledgers、secrets、billing data
或 usage history。

validation 是 structural 和 heuristic，不是 safety、privacy、compliance、
correctness 或 release certification。
