# Security And Privacy

AiEconLab (AEL) is project-local by default. All agent state, personas,
memories, and worktrees live inside `.aiplus/` of the project where AEL
is installed. AEL never uploads data, never syncs to a cloud, and never
edits global agent configuration.

## Data boundaries

AEL is the AiPlus sub-module for applied-economics research workflows.
Research data raises specific concerns that AEL takes seriously:

- **Restricted / IRB-protected data** MUST live outside any path AEL
  manages. AEL provides the **Ethics / IRB Reviewer expert** and the
  **IRB / Disclosure Gate** consultant seat specifically to gate tasks
  that touch restricted data. RA-Python is required to refuse
  operations on `data/restricted/` without an Owner-logged per-task
  authorization memo. See AEL DESIGN.md §15 (Privacy & Safety
  Boundaries) for the full policy.
- **No PII**, no respondent identifiers, no IRB-restricted paths
  belong in agent configs, personas, memories, or worktrees. The
  `aiplus-agent-memory` substrate applies redaction patterns before
  any record is written; AEL inherits this protection.
- **AEA Data Editor replication packages**: AEL's
  **LLM-as-Measurement Specialist** and **Reproducibility Engineer**
  experts coordinate to produce two split packages — public (redacted,
  with synthetic substitutes where needed) and reviewer-only
  (encrypted under journal DUA). The split rule is decided at plan
  time by the IRB / Disclosure Gate consultant seat.

## STOP-gates (always escalate to Owner)

AEL never auto-approves these actions, regardless of which agent is
driving:

1. Journal submission, public preprint, or external archive post
2. Posting to NBER / SSRN / institutional working-paper series
3. Sending an R&R response letter to the editor
4. Sharing data or replication package with parties outside the
   IRB-listed personnel
5. Adding, removing, or reordering authors
6. Touching restricted data without per-task Owner authorization
7. Estimator change that affects the headline result
8. Sample-frame change that affects the headline result
9. Dropping a previously-reported robustness check
10. Posting to social media about the paper
11. Changing the submission target (e.g. QJE → AER)

See AEL DESIGN.md §16 for the full STOP-gate list.

## Reporting a security or privacy issue

AEL is brand-decoupled from AiPlus but functionally depends on the
AiPlus agent substrate. Security/privacy issues in the AiPlus substrate
(agent-memory redaction, compact-reminder handoff format, the
auto-team-consultant engine) should be reported via the AiPlus
[SECURITY.md](https://github.com/izhiwen/AiPlus/blob/main/SECURITY.md).

Issues specific to AEL's research-tuned consultant team, the LLM-as-
Measurement Specialist's validity protocol, or the IRB/Disclosure Gate
behavior should be opened as a GitHub issue on this repository or
emailed to the maintainer:

- `wangecon@outlook.com` (primary)
- `zhw94@pitt.edu` (academic)

For sensitive disclosures (e.g. you found a way for AEL to leak
restricted data outside an authorized path), please use email rather
than a public issue.

## What AEL does not do

- Upload agent state, persona, memory, transcript, or any project file
  to any service
- Run as a background daemon or persistent process
- Store secrets, API keys, IRB-protected paths, restricted-archive
  paths, or any credential in any agent's persona, memory, or
  workspace
- Modify global agent configuration (`~/.codex/`, `~/.claude/`, etc.)
- Modify another project's `.aiplus/`
- Automatically approve any Owner-gated action (see STOP-gates above)
- Introduce network calls beyond what the host runtime (Codex / Claude
  Code / OpenCode) already makes
