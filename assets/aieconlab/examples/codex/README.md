# Codex example — AiEconLab

A synthetic walkthrough of an end-to-end research task routed through the
AiEconLab under the codex runtime.

## Scenario

Owner says: "I want to add a robustness check to the Treaty Ports paper
using prefecture-pair fixed effects before next Monday's seminar."

## Expected routing

1. **Owner → PI**: PI scores task as MEDIUM (new spec, identification-adjacent, deadline-bearing).
2. **PI → consultant**: `aiplus-auto-team-consultant` fires; consultant surfaces the cluster-level question.
3. **PI → Theorist**: spec-extension request for the prefecture-pair definition.
4. **Theorist → PI**: 1-paragraph spec extension signed off.
5. **PI → RA-Stata**: dispatch with the signed-off spec.
6. **RA-Stata → PI**: implementation report with assertions and `.tex` table.
7. **PI → Replicator**: clean-room rerun on the new table.
8. **Replicator → PI**: MATCH or MISMATCH report.
9. **PI → Referee**: pre-review of the new paragraph for the paper.
10. **Referee → PI**: flags (if any) for Writer to address.
11. **PI → Owner**: status report with timeline against Monday's seminar.

## Files this example would touch

- `code/robustness/prefecture_pair_fe.do` (RA-Stata)
- `output/tables/table_A3_pair_fe.tex` (RA-Stata)
- `paper/sections/robustness.tex` (Writer, behind Referee gate)
- `.aiplus/agent-memory/_team/` (PI logs dispatch + open flags)

## What this example demonstrates

- The PI does not skip Theorist for an identification-adjacent task.
- The Replicator pass happens before Referee pre-review (numbers before prose).
- Writer is gated behind Referee pre-review for the new paragraph.
- The Owner receives one status report, not five role-specific updates.

## Status

The codex runtime adapter is shipped (v0.3+). This README documents the
intended routing; see [`adapters/codex/README.md`](../../adapters/codex/README.md)
for current adapter details.
