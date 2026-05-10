# 快速开始

3 分钟上手。5 条命令。

## 前提条件

- macOS Apple Silicon（已验证）。其他平台请用 [开发者构建](#开发者构建)。
- AI 编程助手：Codex、Claude Code 或 OpenCode。

## 第 1 步：安装 CLI

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
```

安装到 `~/.local/bin/aiplus`。重新打开终端或确认 `~/.local/bin` 在 PATH 中。

验证：

```bash
aiplus --version
```

## 第 2 步：安装到项目

```bash
cd MyProject
aiplus install codex
```

将 `codex` 替换为 `claude-code` 或 `opencode`。安装全部运行时：

```bash
aiplus install all
```

已有安装会安全升级，备份在 `.aiplus/backups/`。

## 第 3 步：刷新代理

在已打开的代理会话中输入：

```text
AiPlus 刷新
```

代理会读取 AiPlus 引导文件并报告状态。

## 第 4 步：健康检查

```bash
aiplus doctor
```

验证安装、清单、记忆、compact 状态和适配器文件。所有检查应显示 `PASS`。

## 第 5 步：compact 前保存进度

```text
保存进度
```

compact 后代理无回复时：

```text
继续
```

## 开发者构建

如果发行版不支持你的平台：

```bash
git clone https://github.com/izhiwen/aiplus.git
cd aiplus
cargo build --release
```

然后在目标项目中：

```bash
~/aiplus/target/release/aiplus install codex
```

## 下一步

- [日常操作](daily-workflows.md) — 自然语言命令对照表
- [记忆指南](../memory-guide.md) — 项目记忆、搜索、遗忘
- [Compact 指南](../compact-guide.md) — remind、prepare、checkpoint、resume
- [故障排除](troubleshooting.md) — doctor、常见问题修复
