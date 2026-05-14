# Changelog

All notable changes to AiEconLab (AEL) are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

(no unreleased changes yet — last released cut was v0.1.1)

## [0.1.1] — 2026-05-13

A polish release rolling up everything since v0.1.0: the 12th expert,
the research-tuned consultant team replacing SWE default, an install
smoke-test watchdog, a full set of community files, an honest beta
walkthrough, and ready-to-post outreach drafts.

### Added

- **LLM-as-Measurement Specialist** (12th expert) with full persona
  (~10K, 5 worked examples). Owns the validity protocol when LLMs or
  any frontier text model are used as measurement instruments on text
  data: multi-model cross-validation panel design, hand-coded
  subsample protocols, held-out test docs, inter-rater agreement
  metrics, prompt versioning, leakage prevention, AEA Data Editor
  compatibility.
- **AEL research-tuned consultant team**
  (`core/templates/consultant-team.aieconlab.toml`) replacing the
  default SWE consultant. Five expert seats designed from first
  principles for applied-econ research at plan time: Design
  Credibility, Contribution Framing, Day-1 Reproducibility, IRB /
  Disclosure Gate, LLM-as-Measurement. Three user personas
  (Anonymous Top-Tier Referee, Job-Market Audience, External
  Replicator) fire in HEAVY tier or risk ≥ 0.7. Five owner gates
  mirror DESIGN.md §16 STOP-gates. LIGHT tier skips consult by
  design — AEL consult is a strategic review board, not a daily team.
- Phase C external install path: `aiplus add --from-git
  https://github.com/izhiwen/AiEconLab` (requires aiplus ≥ v0.5.4) for
  installing the live HEAD of AEL without waiting for an AiPlus CLI
  release.
- Install smoke test CI workflow
  (`.github/workflows/install-smoke.yml`) that builds the aiplus CLI
  from source, vendors live AEL into the bundle, and runs the full
  install + opt-in + doctor + agent-route flow in clean tmp projects.
  Catches schema-mismatch class regressions.
- 15th acceptance invariant: `consultant_team_present` verifies the
  consultant TOML has the 5 expected member ids, 3 user_evidence
  personas, 5 owner gates, and `light.review_mode = "skip"`.

### Changed

- `consultant-team.aieconlab.toml` schema refactored to match the
  deployed `aiplus-auto-team-consultant` schema:
  - `[[members]]` use `id =` (was `lens_id =`)
  - User personas moved from `[[members]]` to
    `[[user_evidence.personas]]`
  - The LLM-as-Measurement seat takes `id = "ai_integration"` to
    satisfy the doctor's check while keeping its display name
- `aiplus-module.json`:
  - `requires.substrate_modules` now lists only the three actually-
    needed external substrate modules (`agent-memory`,
    `compact-reminder`, `auto-team-consultant`); velocity is built
    into the aiplus CLI core
  - `requiredFiles` extended to include the consultant team config
    and the LLM-Measurement Specialist persona
  - `doctorChecks` gains `consultant-team` and
    `llm-measurement-expert` entries
- AEL is **opt-in**, not auto-installed. AiPlus CLI ≥ v0.5.5 sets
  `auto_install: false` on the aieconlab module spec so that
  `aiplus install codex` does not pollute every AiPlus project with
  research-only files. Users who want AEL run
  `aiplus add aieconlab`.

### Fixed

- Expert directory headline count corrected from "11 specialists" to
  "12 specialists" across README, README.zh-CN, DESIGN.md, three
  adapter READMEs, and parent `MODULES.md`.
- ASCII architecture diagrams in README and DESIGN.md use canonical
  case (`AiPlus-Agent-Memory  AiPlus-Compact-Reminder
  AiPlus-Agent-Velocity`) consistent with the renamed sibling repos.
- LICENSE file now contains the full Apache-2.0 text instead of a
  4-line summary, so GitHub correctly detects the license as
  Apache-2.0 rather than NOASSERTION.

### Security

- Added `SECURITY.md` documenting AEL's data boundaries, STOP-gates,
  IRB / restricted-data policy, and disclosure path. AEL inherits the
  `aiplus-agent-memory` redaction patterns; AEL-specific concerns
  (consultant team, LLM-as-Measurement validity, IRB Gate) report to
  this repo.

## [0.1.0] — 2026-05-13

### Added

Initial release. Sibling of `AiPlus-Agent-Team` for applied-economics
research workflows.

- **8 core roles** with full personas (Identity & Voice, Knowledge
  Boundaries, Escalation, Memory Namespace, Forbidden Actions, 5
  worked examples each):
  - Advisor (strategic conversation, framing, second opinion)
  - PI (execution coordinator, dispatches and reports)
  - Theorist (identification, model structure, conceptual framework)
  - Project Manager (scope, acceptance criteria, deadlines)
  - RA-Stata (main regressions, tables, figures)
  - RA-Python (data cleaning, scraping, archive ingestion, GIS;
    dormant by default)
  - Referee (internal top-tier journal pre-review)
  - Replicator (clean-room reproducibility audit)
- **11 experts** in the expert directory:
  - Shipped (8): Lit Reviewer, Writer / Editor, Econometrician (Deep),
    Reproducibility Engineer, Historical Sources Specialist, Job Talk
    Coach, Visualization Specialist, Ethics / IRB Reviewer
  - v0.2 stubs (3): Survey / Experiment, Computation, Co-Author Liaison
- Three runtime adapter scaffolds (codex, claude-code, opencode) with
  parity, ship as placeholder READMEs for v0.1.
- Synthetic examples per runtime (`examples/codex/`,
  `examples/claude-code/`, `examples/opencode/`).
- 14-invariant acceptance schema and `tests/acceptance.test.sh`.
- GitHub Actions CI workflow validating JSON / YAML / TOML files plus
  running the acceptance test on every push.
- DESIGN.md (~17K) documenting the role mapping rationale, AEL's
  divergence from the AiPlus-Agent-Team SWE template, the routing
  protocol, the memory model, the worktree policy, and the STOP-gate
  inventory.
- Default toolchain Python + Stata + LaTeX, with R and Julia
  supported when declared.

### Architecture

- Brand-decoupled from AiPlus: AEL has its own GitHub repo, release
  cycle, and audience. Functionally depends on the AiPlus substrate
  (`agent-memory`, `compact-reminder`, `agent-velocity`,
  `auto-team-consultant`).
- Three-layer memory: personal (per-agent), team (PI-shared), project
  (existing `.aiplus/memory/`). Project memory wins on conflict.
- Git worktree workspaces: each code-touching role gets an isolated
  working directory so RA-Stata and RA-Python can work in parallel
  without silent overwrites.
- State-level permanence with warm bench: agent identity lives on
  disk; process is ephemeral, spawned only when PI routes a task.

### Owner-gated actions (STOP-gates)

12 STOP-gates that the team never auto-approves, including journal
submission, working-paper posting, referee response sending, data
sharing, and authorship changes. See DESIGN.md §16.

[unreleased]: https://github.com/izhiwen/AiEconLab/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/izhiwen/AiEconLab/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/izhiwen/AiEconLab/releases/tag/v0.1.0
