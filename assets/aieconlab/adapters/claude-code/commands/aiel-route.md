# /aiel-route — Route a task to the right AEL role

Use this command to explicitly route work through the AEL coordinator
when you want PI-style dispatch rather than letting auto-routing pick a
subagent for you.

## How it works

1. Read `.aiplus/consultant-team.toml` to see which roles and experts are
   currently seated at the project's research table.
2. Score the task LIGHT / MEDIUM / HEAVY using the adaptive coordinator
   rules (see `.aiplus/agents/pi.toml` and the PI persona).
3. Dispatch via `aiplus agent route <role> "<task>"`:
   - LIGHT — one core role (often ra-stata / ra-python / writer) and
     skip the consultant team.
   - MEDIUM — one or two experts matching the risk axes.
   - HEAVY — full consultant table including user personas.
4. Capture the dispatch decision via `aiplus memory add --kind decision`
   so future sessions can see what was routed where.

## Examples

```text
/aiel-route 帮我清一下 NLSY97 这份数据 → ra-python (LIGHT)
/aiel-route 主回归用谁的 DID 估计量比较稳 → theorist + econometrician (MEDIUM)
/aiel-route 投稿前挑刺一下 → referee + econometrician + viz-specialist (HEAVY)
```

## Safety

Owner-gated actions (journal submission, working-paper posting, sending
referee responses, data sharing, authorship-order changes) never auto-
approve from this command. They surface as recommendations awaiting
Owner confirmation.
