# AiPlus Token Cost

`aiplus-token-cost` 为 AiPlus 派遣日志提供 token 消耗和 USD 成本统计。

它读取 `.aiplus/agents/dispatch-log.jsonl`，汇总过去 1 小时、8 小时、
24 小时窗口，并列出每个窗口里成本最高的任务。定价优先使用本地
override，其次使用本地缓存的 LiteLLM 定价表，最后使用二进制内嵌兜底。

## 用法

在包含 `.aiplus/` 的项目根目录运行：

```bash
aiplus-token-cost
aiplus-token-cost --by-role
aiplus-token-cost --window 24h
aiplus-token-cost --top-n 10
aiplus agent token-cost
```

独立二进制由 `izhiwen/AiPlus-Token-Cost` 维护。AiPlus release archive 会把
该二进制和 `aiplus` 一起打包，因此用户既可以直接运行 standalone binary，
也可以通过 bundled CLI 的 `aiplus agent token-cost` 调用。

## 数据边界

- 读取：`.aiplus/agents/dispatch-log.jsonl` 的 `usage_tokens` 字段。
- 写入：`.aiplus/agents/token-cost-snapshots.jsonl`。
- 网络：可选 LiteLLM pricing JSON 拉取，本地缓存 24 小时。项目里的
  `.aiplus/pricing.toml` override 会跳过在线拉取路径。

没有 telemetry。不读取 prompt、task 内容、API response 或 secret。
