# AiEconLab

[English README](README.md)

AiEconLab（AEL）为经济学论文项目提供一套 AI 研究团队结构。它不是让一个聊天窗口同时扮演
PI、RA、理论作者、审稿人和复现者，而是把这些职责拆成清晰的角色，每个角色有独立的工作边界和人格设定。

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/AiEconLab/main/install.sh | bash
cd MyPaperProject
ael install
ael talk advisor "What is your role?"
```

第一步安装 `ael` 命令；在论文或复现项目中运行 `ael install` 后，就可以用
`ael talk <role>` 与具体研究角色对话。

## 演示

![AiEconLab demo](demo.gif)

## 角色

- **Advisor**：研究问题、识别策略、投稿风险和长期项目取舍的第二意见。
- **PI**：拆任务、派发角色、整合结果、维持项目主线。
- **Theorist**：识别策略、机制、工具变量、模型逻辑。
- **RA-Stata**：Stata 回归、表格、稳健性检验、可复现 `.do` 流程。
- **RA-Python**：数据清洗、抓取、匹配、GIS、Python 管线。
- **Referee**：投稿前的内部审稿人，专门挑出论文最脆弱的地方。
- **Replicator**：从干净环境重跑项目，找复现包和依赖问题。
- **PM**：期限、范围、阻塞项和里程碑管理。

AEL 还包含文献、写作、计量、LLM-as-measurement、复现工程、史料、IRB/敏感数据、
可视化、计算、实验设计、自由度审计（DoF）、R&R 策略、Job Talk 和合作者协调等专家角色。

## 团队在你的 runtime 里怎么干活

- **用人话切角色。** session 中途说"你是 PI"、"take the referee
  role"、"切到 RA-Stata"，agent 就会用那个角色回应你，并加载它
  的研究内存。不用 CLI 命令。Codex、Claude Code、OpenCode
  （交互模式）都支持。

- **PI 派任务时的意图感知护栏。** PI 把任务交给 RA 之前，
  如果涉及危险操作（删文件、改 live 数据、发布改动），
  Coordinator 会先理解你到底想做什么，而不只是匹配字眼。
  改个说法、加引号都骗不过去了 —— replication 脚本碰共享
  档案或论文稿时特别有用。

- **评审和 QA 并行，PI → RA → Referee cycle 更快。** review
  和 QA 步骤同时跑，每个角色的工作区在任务之间保持就绪。
  典型 robustness 表迭代周期 ~8-10 分钟，不再 ~15-20，质量
  门槛不变。AEL 从底层 AiPlus 自动继承这一速度。

## 安装

安装 CLI：

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/AiEconLab/main/install.sh | bash
```

如果安装器提示目标目录不在 `PATH`，加入：

```bash
export PATH="$HOME/.local/bin:$PATH"
```

在项目中安装 AEL：

```bash
cd MyPaperProject
ael install
```

也可以显式指定运行环境：

```bash
ael install codex
ael install claude-code
ael install opencode
```

检查安装：

```bash
ael status
ael doctor
```

## 日常使用

与 Advisor 对话：

```bash
ael talk advisor "这个识别策略够不够支撑 top-field 投稿？"
```

让 PI 拆任务并派发：

```bash
ael route pi "规划下一张稳健性表，并派给合适的 RA"
```

任务已经明确时，也可以直接找具体角色：

```bash
ael talk ra-stata "给主 IV 表写一个 Stata 执行计划。"
ael talk referee "用最苛刻的审稿人视角读一下这个摘要。"
```

## 为什么要分角色

长项目里，一个 AI 聊天窗口很容易混淆职责：刚调过 Stata 的助手开始写带代码味的引言；
刚帮你论证识别策略的助手又很难像真正审稿人一样挑刺。AEL 把职责拆开：

- RA 的记忆专注于数据、变量和代码决策。
- Theorist 和 Referee 的批评不被执行细节稀释。
- PI 负责整合，避免并行工作互相覆盖。
- Replicator 以干净环境重跑，而不是继承构建者的假设。

## LLM-as-Measurement

AEL 内置 LLM-as-measurement 专家，适用于用大模型给档案文本、开放式回答、
历史文献或其他非结构化材料打分的论文。该角色关注多模型一致性、人工标注验证、
评分稳定性、prompt 版本管理，以及测量误差对估计结果的影响。

示例项目：
[Multi-LLM-Validation-Demo](https://github.com/izhiwen/Multi-LLM-Validation-Demo)。

![两两 LLM 相关性热力图（294 篇档案 × 5 个前沿 LLM，平均 ρ ≈ 0.92）](https://raw.githubusercontent.com/izhiwen/Multi-LLM-Validation-Demo/main/figures/multi_llm_correlation_heatmap.png)

## 安全边界

AEL 保持在你的本地项目内。它不会：

- 上传项目文件、记忆或对话记录
- 作为后台守护进程运行
- 在角色设定中保存受限数据路径或密钥
- 修改无关项目
- 自动批准投稿、公开 working paper、发送 referee response、共享数据或改变作者顺序

## 高级说明

AEL 构建在 AiPlus agent substrate 之上；受支持的用户入口是 `ael` CLI 和本仓库。

## 许可证

Apache-2.0。见 [LICENSE](LICENSE)。
