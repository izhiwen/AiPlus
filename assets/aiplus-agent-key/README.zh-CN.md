# AiPlus-Agent-Key

> **AI 编程 agent 的别名式、零持久化密钥解析层。**
> Agent 调用 `OPENAI_KEY_WORK`；broker 在运行时解析出真实值、注入子进程环境变量、用完即忘。不写盘、默认不打印、不进 git 历史。

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

[English README](README.md)

## 安装前提

AiPlus-Agent-Key 作为 [AiPlus](https://github.com/izhiwen/AiPlus) CLI 的子命令实现（`aiplus secret-broker`）。先安装 AiPlus：

```bash
# macOS (arm64) 一行安装：
curl -fsSL https://raw.githubusercontent.com/izhiwen/AiPlus/main/install.sh | bash

# 验证 secret-broker 子命令可用：
aiplus secret-broker --help
```

> **平台支持 — v0.1：** AiPlus 目前只发布 **macOS arm64**（Apple Silicon）binary。Linux 和 macOS Intel 用户需要从源码 build（见 AiPlus README → Developer Build），等上游发更多平台的 release。Linux CI runner 目前**无法用 prebuilt binary 接入本模块**；env-var token 方式（Setup Method B）和 `examples/README.md` 里的 GitHub Actions 例子是为 Linux binary 发布后准备的。

**Bitwarden Secrets Manager** 是 v0.1 后端。你需要：

- 一个 [Bitwarden Secrets Manager](https://bitwarden.com/products/secrets-manager/) workspace
- 装好 `bws` CLI 且在 `$PATH` 上。macOS：`brew install bws`。Linux：照 [Bitwarden CLI install 文档](https://bitwarden.com/help/secrets-manager-cli/) 来。
- workspace 的 machine access token（见下方 Setup）

其它后端（1Password / AWS Secrets Manager / HashiCorp Vault / env-file fallback）规划在 v0.2。

## 痛点

你每天跑一堆 AI 编程 agent（Codex / Claude Code / OpenCode），每个 agent 都要 API key：一个 OpenAI key，一个 Anthropic key，可能还有 work 账户 key，还有工具 key（Linear / Slack / AWS）。五种反复发生的失败模式：

1. **泄到 git 里。** `.env` 不小心 commit 了。或者 key 留在 shell history 里。或者在 agent 对话里出现过一次，转录里就留下了。

2. **泄到 agent context 里。** "就这一次"你把 key 贴到 prompt 里救一个卡住的任务。从此它进了 agent 的 compact handoff、memory snapshot、context cache。

3. **混账户。** 你有三个 OpenAI 账户（personal / work / sandbox）。错的 key 打到错的 workspace 上，账单一来才发现。

4. **轮换之痛。** key 过期或被吊销。你要在八处更新：shell rc、三个项目的 .env、两个 CI config、两个 docker-compose。

5. **默认打印的风险。** 大部分密钥管理 CLI *默认就打印*密钥值。一张截图、一次 screen share、一次 tmux scrollback，就漏了。

## 解决方案

**把 alias 映射到后端的 secret 路径。运行时解析。注入到子进程环境变量。永不持久化。**

```bash
# 在你的项目、agent 代码、或 shell 脚本里：
aiplus secret-broker run --aliases openai,anthropic -- python my_agent.py
```

翻译：
1. broker 从 `~/.config/aiplus/secret-broker/profiles/<profile>/secret-aliases.tsv` 读 alias 到后端的映射
2. 对每个 alias，从 Bitwarden Secrets Manager (`bws`) 后端取当前值
3. 启动 `python my_agent.py`，子进程环境里 `OPENAI_API_KEY=<解析值>` 和 `ANTHROPIC_API_KEY=<解析值>` 已注入（注入的 env var 名是 alias 行第三列声明的 SDK 约定名，不是 alias 自己）
4. 子进程退出，解析值就消失了。不写盘、不缓存、不进 shell history

其它命令：

```bash
aiplus secret-broker status                  # 是否已装、用什么后端、auth 是否有效
aiplus secret-broker doctor                  # 验证配置、测试后端可达性
aiplus secret-broker list                    # 列出 `alias -> backend-path -> env-var-name`（不显示值）
aiplus secret-broker resolve openai          # 输出解析 metadata (alias / provider / backend path)，默认不显示值
aiplus secret-broker run --alias openai -- <command...>          # 单个 secret 注入
aiplus secret-broker run --aliases openai,anthropic -- <cmd...>  # 多 secret 注入
echo "<bws-access-token>" | aiplus secret-broker token set       # 把 Bitwarden access token 存进 OS keyring（只接受 stdin）
aiplus secret-broker token delete            # 删除已存的 access token
```

`--print` 存在但**默认禁用**。要启用，需要在 AiPlus profile 的 preferences（`privacy-and-secrets.md`）里显式开启。即使启用，加上 `--print` 也会在 shell history 里留痕迹 —— 这是有意的，secret 打印必须是可审计的明示动作。

### `run` 不带 `--alias`/`--aliases` 的默认行为

`aiplus secret-broker run -- <command>` 如果不指定 alias，broker 会注入一个**默认子集**：主流 LLM provider key（OPENAI_API_KEY、ANTHROPIC_API_KEY 等）加几个常用平台 token（GITHUB_TOKEN、CLOUDFLARE_API_TOKEN）。其它 alias 在输出里以 `skipped_aliases=[...]` 列出。生产脚本里始终显式传 `--alias` 或 `--aliases`；默认注入只是"开个 shell 让常用 LLM key 就绪"场景的方便功能。

## Alias 命名规范

TSV 每行三列：`<alias>	<backend-path>	<env-var-name>`。各列怎么命名：

- **Alias** —— 短、小写、单词：`openai`、`anthropic`、`github`、`cloudflare`。CLI 里你输入的就是这个。
- **Backend path** —— Bitwarden Secrets Manager workspace 里的路径：`<scope>/<provider>/<key-name>`（如 `yourname/openai/api_key`）。
- **Env var name** —— 你代码里读的 env var，一般用 SDK 约定：`OPENAI_API_KEY`、`ANTHROPIC_API_KEY`、`GITHUB_TOKEN`。

多账户场景，用 `<provider>_<account>` alias：

```
openai_personal	yourname/openai/personal/api_key	OPENAI_API_KEY
openai_work	yourname/openai/work/api_key	OPENAI_API_KEY
```

完整规范见 [`core/alias-conventions.md`](core/alias-conventions.md)，24 行真实示例见 [`core/example-aliases.tsv`](core/example-aliases.tsv)，TSV 格式注释版见 [`core/example-aliases.md`](core/example-aliases.md)。

## Setup

1. **装 AiPlus**（见安装前提）。`aiplus secret-broker status` 跑得通即可。
2. **装 `bws` CLI**（按 Bitwarden 文档）。Broker 通过 shell 调 `bws` 解析；如果没装，`aiplus secret-broker doctor` 会显示 `bws_cli=no`。
3. **建 Bitwarden Secrets Manager workspace + 生成 machine access token**（只读访问你要暴露的那组 secrets）。
4. **用两种方式之一提供 access token：**

   **方式 A —— OS keyring（本地开发推荐）：**
   ```bash
   echo "<你的-bws-access-token>" | aiplus secret-broker token set
   ```
   token **只从 stdin 读**（绝不接受命令行参数 —— 防 shell history / `ps` 泄漏）。macOS 上**首次调用会弹 Keychain 授权对话框**，点 *Always Allow*，之后 session 不再问。Linux 需要 Secret Service（如 gnome-keyring）。

   **方式 B —— 环境变量（CI / Docker / headless 机器）：**
   ```bash
   export BWS_ACCESS_TOKEN="<你的-bws-access-token>"
   ```
   设了 `BWS_ACCESS_TOKEN` broker 就直接用，跳过 keyring。`aiplus secret-broker status` 这时会显示 `token_source=env`。适合 CI runner 等没有交互式 Keychain 的场景。

5. **挑一个 profile 名，建 alias 目录。** Broker 从 `~/.config/aiplus/secret-broker/profiles/<profile>/secret-aliases.tsv` 读 alias。profile 名随你 —— `default`、你的用户名、机器名都行，只要是这个路径下的一个目录：
   ```bash
   PROFILE=default   # 任何名字，AiPlus 会自动发现
   mkdir -p "$HOME/.config/aiplus/secret-broker/profiles/$PROFILE"
   ```
   （`aiplus profile status` 如果显示 `profiles=[]` 别在意 —— secret-broker 的 profile 目录和 AiPlus 装的 profile 列表是分开的。）

6. **写 alias TSV。** 每行 **tab 分隔**（三列：alias、backend 路径、env var 名）：
   ```bash
   cat > "$HOME/.config/aiplus/secret-broker/profiles/$PROFILE/secret-aliases.tsv" <<'EOF'
   openai	yourname/openai/api_key	OPENAI_API_KEY
   anthropic	yourname/anthropic/api_key	ANTHROPIC_API_KEY
   github	yourname/github/token	GITHUB_TOKEN
   EOF
   ```

7. **用 `doctor` 验：**
   ```bash
   aiplus secret-broker doctor
   ```
   期望 `SECRET_BROKER_DOCTOR_STATUS=PASS`。如果失败，输出会带一行 `next=...`，告诉你具体跑哪条命令修（如 `next=run aiplus secret-broker token set in Terminal`）。照着改完再 `doctor` 一次。

8. **使用：**
   ```bash
   aiplus secret-broker run --alias openai -- python my_agent.py
   ```
   子进程里 `os.environ["OPENAI_API_KEY"]` 已注入。子进程退出后值就消失。

## 安全边界

AiPlus-Agent-Key **不会**：

- **持久化**解析出来的密钥值。不写文件、不在调用之间缓存、不进日志。
- **默认打印**密钥。`aiplus secret-broker resolve <alias>` 返回的是解析*元数据*（alias 名、provider、backend 路径），**不**返回值。`--print` 默认禁用，需要在 profile preferences 里显式开启；即使开启，加 `--print` 这个动作也会在 shell history 里被记录 —— 故意的。
- **上传**任何东西。Broker 只跟你配置的后端和你的子进程通信。
- **读取**任何你没有显式映射成 alias 的密钥。Alias 是 allowlist。
- **绕过** OS keyring。Bitwarden access token 不会写进这个仓库或你项目里的任何配置文件。
- **修改**全局配置（`~/.codex` / `~/.claude` 等）。AiPlus 所有写都在 `~/.config/aiplus/` 范围内。
- **被 commit** 到你的 repo。这个模块带的 `.gitignore` 屏蔽了 `*.env`、`*.token`、`*.key`、`*.credentials`、`*.bw-export`。

这个模块**不能防住**：
- 子进程*自己*把密钥值写到硬盘（比如你的 agent 把 env var 写进日志）。代码层面你要小心。
- 用户加了 `--print` 然后录屏 / share 屏幕。
- 后端本身被攻陷。

## 架构概览

```
                  AiPlus-Agent-Key                      ← 运行时 alias 解析
                            ↓ uses
                   Bitwarden Secrets Manager            ← 后端 (v0.1)
                   1Password / Vault / AWS              ← 后端 (v0.2)
                   ↓ 注入 env vars
                  <子进程>                              ← 你的 agent
```

Broker 是后端（密钥值真正在的地方）和子进程（密钥值暂时注入的地方）之间的无状态层。Broker 自己不存任何状态；每次 `run` 都重新解析。

完整设计原理、威胁模型、后端协议见 [`DESIGN.md`](DESIGN.md)。

## 状态

| 组件 | v0.1 | v0.2 (规划) |
|---|---|---|
| CLI 子命令 `aiplus secret-broker` | ✓ 在 AiPlus 内 | 增强 |
| Bitwarden Secrets Manager 后端 (通过 `bws` CLI) | ✓ | 细化 |
| 仅记元数据的 audit 日志（alias 名 + 时间戳，不含值） | ✓ | 保留控制 + 结构化日志文件 |
| 1Password 后端 | — | ✓ |
| AWS Secrets Manager 后端 | — | ✓ |
| HashiCorp Vault 后端 | — | ✓ |
| env-file fallback 后端 (仅离线开发) | — | ✓ |
| Rust 源码物理拆分为独立 crate | — | ✓ |
| Token 自动 refresh (OAuth) | 部分 | ✓ |

## 内容

- `core/example-aliases.tsv` —— 24 行真实示例 alias
- `core/example-aliases.md` —— TSV 格式的注释版 walkthrough
- `core/alias-conventions.md` —— 命名规范指南
- `adapters/{codex,claude-code,opencode}/` —— runtime adapter scaffold (v0.1 占位符)
- `examples/` —— 合成 walkthrough
- `DESIGN.md` —— 设计原理 + 威胁模型
- `.aiplus/agent-key/acceptance/v0.1.0/schema.yaml` —— 验收 schema
- `tests/acceptance.test.sh` —— 结构性 invariants 测试

## 更多

- 主平台：[AiPlus](https://github.com/izhiwen/AiPlus)
- 兄弟模块：[AiPlus-Agent-Team](https://github.com/izhiwen/AiPlus-Agent-Team)、[AiEconLab](https://github.com/izhiwen/AiEconLab)

## 许可证

[Apache-2.0](LICENSE)
