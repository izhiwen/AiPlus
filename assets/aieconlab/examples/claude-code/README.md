# Claude Code example — AiEconLab

A synthetic walkthrough of an end-to-end research task routed through the
AiEconLab under the Claude Code runtime.

## Scenario

Owner says: "Pre-review the AER R&R rebuttal letter before I send it."

## Expected routing

1. **Owner → PI**: PI scores task as HEAVY (R&R, external-facing, irreversible-once-sent).
2. **PI → consultant**: `aiplus-auto-team-consultant` fires.
3. **PI → Referee**: full pre-review pass with QJE/AER template.
4. **Referee → PI**: structured report with majors, minors, cosmetic.
5. **PI → Writer (expert summon)**: address major-1 (defensive tone in response to R2).
6. **PI → Theorist**: confirm rebuttal language on identification claims.
7. **PI → Replicator**: clean rerun on every table affected by the rebuttal.
8. **PI → Referee**: second pre-review pass on the revised letter.
9. **PI → Owner**: STOP-gate escalation with pre-submission checklist for explicit Owner go/no-go.

## Files this example would touch

- `paper/response_to_referees.tex` (Writer, on `agent/writer`)
- `paper/sections/structural_mechanism.tex` (Theorist + Writer)
- `output/tables/table_3_iv_main.tex` (Replicator confirms unchanged)
- `.aiplus/agent-memory/_team/open_flags.md` (PI tracks unresolved Referee comments)

## What this example demonstrates

- HEAVY tier fires consultant.
- Writer is summoned (expert), not assumed to be in the core team.
- Replicator pass on tables the rebuttal cites, even if numbers did not
  change — because the rebuttal *cites* them.
- Submission is a STOP-gate that always escalates to the Owner.

## Status

The claude-code runtime adapter is shipped (v0.3+). This README documents
the intended routing; see
[`adapters/claude-code/README.md`](../../adapters/claude-code/README.md)
for current adapter details.
