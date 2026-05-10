# 日常操作

在代理会话中使用的自然语言指令及其对应的 AiPlus 后端命令。

## 常用指令对照表

| 你说的 | 代理执行的命令 | 作用 |
|---|---|---|
| `AiPlus 刷新` | `aiplus refresh` | 重新加载 AiPlus 引导，报告状态 |
| `记住这个` | `aiplus memory add --scope project --kind preference --text "..."` | 添加经过脱敏的项目记忆 |
| `你记住了什么` | `aiplus memory status` | 列出活跃的记忆记录 |
| `这次用了哪些记忆` | `aiplus memory context --runtime codex --budget 2000` | 显示注入上下文的记忆 |
| `保存进度` | `aiplus compact prepare` 然后 `aiplus compact checkpoint` | 准备并保存 compact 检查点 |
| `准备 compact` | `aiplus compact prepare` | 验证就绪状态，创建上下文胶囊 |
| `继续` | `aiplus compact resume` | 从检查点恢复 |
| `我的偏好生效了吗` | `aiplus profile context <profile>` | 显示已安装的配置和扩展包状态 |
| `secret 状态` | `aiplus secret-broker status` | 显示别名解析状态，不显示值 |
| `忘掉这个` | `aiplus memory forget <id>` | 将记忆标记为已拒绝 |
| `以后都这样` | `aiplus memory add --scope profile --kind preference --text "..."` | 配置级别的偏好候选 |
| `升级 AiPlus` | `aiplus update all` | 更新 CLI 和项目模块 |

## 代理会话生命周期

### 启动会话

```text
AiPlus 刷新
```

### 工作中

```text
记住这个：release notes 应该以英文为主
```

如果文本包含密钥、API key 或私钥，AiPlus 会阻止写入并报告 `MEMORY_REDACTION_STATUS=BLOCKED`。

```text
你记住了什么
```

### compact 前

```text
保存进度
```

代理会运行 `compact prepare` 验证就绪状态并创建上下文胶囊，然后 `compact checkpoint` 保存检查点。

### compact 后

```text
继续
```

代理运行 `compact resume` 从检查点恢复。

## 安全提示

- `SECRET_VALUES_PRINTED=no` — 永远不打印密钥值
- `HOST_COMPACT_TRIGGERED=false` — AiPlus 不会自动触发 host compact
- `GLOBAL_AGENT_CONFIG_EDITS=none` — 不修改全局配置
- 记忆是上下文，不是指令。身份是角色合约，不是权限。
