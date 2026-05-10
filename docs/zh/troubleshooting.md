# 故障排除

常见问题及诊断方法。

## 第一步：运行 Doctor

```bash
aiplus doctor
aiplus memory doctor
aiplus profile doctor
aiplus secret-broker status
```

## 常见问题

### `aiplus: command not found`

CLI 不在 PATH 中。

```bash
ls ~/.local/bin/aiplus        # 检查是否安装
# 如缺失，重新安装：
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
# 如已安装但不在 PATH：
export PATH="$HOME/.local/bin:$PATH"
```

### OpenCode 配置 JSON 无效

```bash
aiplus doctor
```

如果显示 `NEEDS_FIX opencode config JSON parse failed`，`.opencode/config.json` 文件有语法错误。修正 JSON 后重新运行 doctor。

AiPlus 会拒绝 OpenCode 配置中包含顶层 `aiplus` 键的文件。

### 安装的二进制比源码旧

```bash
aiplus --version                    # 检查已安装版本
cd aiplus-public
cargo build --release
cp target/release/aiplus ~/.local/bin/aiplus
```

或使用自更新：

```bash
aiplus self update --dry-run
aiplus self update --yes
```

### Compact 没有自动触发

AiPlus 无法触发 host compact。它只能提醒和准备。compact 必须由你或 host 代理触发。

```bash
aiplus compact remind
```

- `REMINDER_DECISION=wait` — handoff 不够新，先更新
- `REMINDER_DECISION=blocked` — 安全门阻止了 compact

### 记忆写入被脱敏拦截

如果 `memory add` 返回 `MEMORY_REDACTION_STATUS=BLOCKED`，文本包含检测到的敏感模式。移除密钥、API key、私钥、JWT token、电话号码或转录内容后重试。

### 配置文件找不到

```bash
aiplus profile status              # 查看已安装的配置
aiplus profile install my-profile --user --source /path/to/source --yes
```

### 出现 legacy 配置

```bash
aiplus profile cleanup --user --dry-run
aiplus profile cleanup --user --yes
```

### Secret broker 返回 "not configured"

```bash
aiplus secret-broker status
aiplus secret-broker list
```

密钥需要包含 `secret-aliases.tsv` 的私有配置和 `BWS_ACCESS_TOKEN` 或 macOS Keychain 条目。

### `compact watch` 留下残留进程

```bash
ps aux | grep 'aiplus.*watch'
kill <pid>
```

### `.aiplus/` 中有断开的符号链接

```bash
aiplus doctor
# 删除断开的链接
rm .aiplus/<broken-link>
aiplus install codex
```

## 更多帮助

- [FAQ](../faq.md)
- [记忆指南](../memory-guide.md)
- [Compact 指南](../compact-guide.md)
- [术语表](../glossary.md)
