# /aiel-fire-consultant — Fire the AEL research consultant team

Use this command before finalizing any non-trivial research plan — paper
scope, identification strategy, major revision, replication-package
design, IRB submission, or anything that will leave the team. The
consultant team is research-tuned: it includes deep econometrics,
ethics/IRB, LLM-as-measurement, reproducibility, and a writer/viz seat,
plus the project's user personas.

## How it works

1. Read `.aiplus/consultant-team.toml` to see who is seated. AEL's
   research-tuned config replaces AiPlus's default SWE consultant team
   when `aiplus add aieconlab` was run.
2. Score the plan LIGHT / MEDIUM / HEAVY. LIGHT plans skip the
   consultant entirely; MEDIUM consults 2–3 experts; HEAVY runs the
   full table.
3. Run the consult via `aiplus agent route reviewer "<plan-summary>"`
   so each seat gets the plan, reads it through their persona's
   knowledge boundaries, and returns dissent or endorsement.
4. Surface **all** divergent positions. Do NOT flatten into a single
   "consensus" line if seats disagree.
5. Record the consult outcome via `aiplus memory add --kind decision`.

## Examples

```text
/aiel-fire-consultant 用 staggered DID 还是 imputation 估计器 → HEAVY
/aiel-fire-consultant 把 LLM 评分作为主结果是否站得住 → llm-measurement + econometrician + referee
/aiel-fire-consultant 数据共享方案 → ethics-irb + reproducibility + coauthor-liaison
```

## Safety

The consultant team produces recommendations, not approvals. Owner-
gated actions (journal submission, posting, data sharing, authorship-
order changes) still require explicit Owner confirmation after the
consult, never on the table's behalf.
