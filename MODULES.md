# AiPlus Module Index

This public repository contains the Rust mainline `aiplus` CLI and bundled
module snapshots used for project-local installs.

Each bundled snapshot includes `aiplus-module.json`. The Rust `aiplus-core`
crate validates these manifests against the static registry before `doctor`
reports module health.

## Included In This Repository

| Component | Status | License | Purpose |
| --- | --- | --- | --- |
| `aiplus` Rust CLI | Mainline public binary/source package | Apache-2.0 | Project-local install/update/add/status/doctor/uninstall/compact workflows |
| `assets/aiplus-auto-compact` | Bundled snapshot | Apache-2.0 | Proactive compact reminder, checkpoint, handoff, savings preview, validate, score, and resume workflow assets |
| `assets/aiplus-auto-team-consultant` | Bundled snapshot | MIT, preserved | Advisor/CEO/Reviewer/Builder routing assets |
| `assets/aiplus-agent-memory` | Bundled snapshot | Apache-2.0 | Agent Continuity foundation for local Memory Context, Role Identity, and Skill Candidate governance |

## Independent Public Module Records

| Module | Repo | Status |
| --- | --- | --- |
| `aiplus-auto-compact` | `https://github.com/izhiwen/aiplus-auto-compact` | Independent public module record |
| `aiplus-auto-team-consultant` | `https://github.com/izhiwen/aiplus-auto-team-consultant` | Independent public module record |
| `aiplus-agent-memory` | public/general AiPlus subproduct | Introduced in v0.5.0 |
| `codex-compact-protocol` | `https://github.com/izhiwen/codex-compact-protocol` | Legacy Codex-first public record |

## Not Included

The archived Node reference CLI `aiplus-cli` is not included in this public
source package. It remains a private/local behavior-audit reference in the
AiPlus workspace and is not the mainline path.

The legacy local `auto-team-consultant` source package is not included here.

## Publication Boundaries

This source repository publication does not approve or perform:

- Git tags
- GitHub Releases
- binary uploads
- `cargo publish`
- npm publication
- Homebrew release
- shell installer publication
- npm wrapper publication
- global install
- global config edits
- marketplace submission
- telemetry
