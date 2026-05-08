# AiPlus Auto Compact

[中文 README](README.zh-CN.md)

AiPlus Auto Compact is an independent AiPlus subproduct for compact,
checkpoint, and resume handoffs. It can be used as a bundled module installed
by the AiPlus CLI (`aiplus`), or inspected directly by users who only want the
compact workflow.

Use it when a long Codex, Claude Code, or OpenCode session is close to context
compaction and you want a cleaner local handoff. It helps the agent validate
compact readiness, create a checkpoint before compact, and resume from that
checkpoint after compact.

It does not click a compact button, call `/compact`, upload data, change global
runtime settings, or make compact safe automatically.

## Beginner Flow

### Path A: AiPlus ecosystem install

Install AiPlus, then install the project-local compact module:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
cd MyProject
aiplus install codex
```

If the project already has an older AiPlus install, `aiplus install codex`
safely upgrades AiPlus managed files, backs up replaced managed files under
`.aiplus/backups/`, and preserves existing `.codex/compact/` state.

### Path B: Existing `aiplus` command

```bash
cd MyProject
aiplus install codex
```

For other runtimes, use:

```bash
aiplus install claude-code
aiplus install opencode
aiplus install all
```

The install is project-local. It may write `.aiplus/`, `.codex/compact/`, project `.claude/` files, project `.opencode/` files, and the AiPlus managed block in project `AGENTS.md`. It does not modify global Codex, Claude Code, OpenCode, shell, or package-manager configuration.

Then type this in the already-open Codex, Claude Code, or OpenCode session:

```text
aiplus refresh
```

Other explicit AiPlus refresh triggers:

```text
AiPlus status
resume AiPlus
AiPlus 刷新
刷新 AiPlus
aiplus status
继续 AiPlus
```

Generic `刷新` / `refresh` should still try AiPlus first after installation. If
your project also uses `刷新` for its own state refresh, use `AiPlus 刷新` or
`aiplus refresh` to avoid ambiguity. Meaning: report Auto Compact and compact
state before unrelated project refresh, reread `AGENTS.md`, reread
`.aiplus/AGENTS.aiplus.md`, read `.codex/compact/current-handoff.md` if present,
enable AiPlus guidance, and continue the current task.

### Path C: Advanced module-only adoption

If you only want AiPlus Auto Compact, you can read this repository directly,
inspect `core/templates/`, `core/docs/`, and the runtime adapters, then copy or
adapt the compact workflow into your project. This is useful for policy review,
custom integrations, or migration from `codex-compact-protocol`.

The module-only path is not the beginner install path. Keep the legacy Node
helper in [Advanced: Legacy Node Reference](#advanced-legacy-node-reference) for
audits and compatibility checks only.

## What Happens Around Compact

You do not need to remember compact commands.

In your agent session, say:

```text
prepare compact
```

or:

```text
save progress
```

The agent should use `aiplus compact prepare` as an internal backend tool. If
`prepare` is not available, it may fall back to the closest supported sequence:
`aiplus compact validate` followed by `aiplus compact checkpoint`. If the
checkpoint is ready, the agent should recommend manual compact with language
like:

```text
Ready to compact.

After compact:
- If I continue automatically, you do not need to do anything.
- If I do not reply, send: continue

I will resume from here.
```

After the host compact completes:

- If the agent continues automatically, you do not need to do anything.
- If the agent does not reply, say:

```text
continue
```

The agent should run `aiplus compact resume` and then continue from the
recovered state. Natural continuation phrases such as `continue`, `resume`,
`refresh`, `go on`, `继续`, `刷新`, and `接着` should work.

This is best-effort automatic resume. AiPlus Auto Compact can prepare the
checkpoint and tell the agent how to resume, but it cannot wake a host runtime
that is waiting for the user.

## Compact Savings Estimate

AiPlus Auto Compact supports the AiPlus v0.3 savings estimate flow. Ordinary
users can ask:

```text
show compact savings
```

or:

```text
how many tokens did compact save?
```

Agents should map those requests to:

```bash
aiplus compact savings
```

Savings are local estimates. AiPlus stores aggregate events in
`.codex/compact/savings-ledger.jsonl`, reports latest compact and all-time
totals, and calculates all-time reduction with a weighted formula:
`totalEstimatedTokensSaved / totalEstimatedBaselineTokens * 100`.

Pricing is cache-first. `aiplus compact savings`, `prepare`, `checkpoint`, and
`resume` use fresh cached public pricing when available. If the cache is missing
or stale, AiPlus may refresh public pricing automatically; network failure never
blocks compact. `aiplus pricing update` explicitly refreshes public pricing data.
If a detected model has no pricing, token savings and reduction still report,
while USD savings are unavailable or partial.

Savings reports are estimates only, not billing data. AiPlus does not upload
prompts, project files, checkpoints, savings ledgers, secrets, billing data, or
usage history.

## Updating AiPlus

Users can say this in an agent session:

```text
update AiPlus
```

Default mapping:

```bash
aiplus update all
```

More specific mappings:

- `only update this project's AiPlus` -> `aiplus update`
- `update the aiplus command` -> `aiplus self update`
- `check AiPlus updates` -> `aiplus self update --dry-run` plus `aiplus status`

Before running updates, the agent should state: "I will update the aiplus CLI
and this project's AiPlus modules. I will not edit global agent config or upload
project data."

Existing projects with older `.codex/compact/current-handoff.md` files are
upgraded conservatively by `aiplus install ...` and `aiplus update`: AiPlus backs
up the old handoff, preserves user content, and adds missing role-aware sections
such as `Session Role`, `Workflow Level`, and `Output Contract`.

## Private Profile And Secrets Boundary

AiPlus may be used with a private user-level profile
and a Bitwarden-backed `secret-broker`. AiPlus Auto Compact must treat those as
private runtime layers, not bundled module content.

Compact handoffs, checkpoints, and savings ledgers must never store secret
values, Bitwarden machine tokens, auth headers, provider response bodies, or raw
profile-private material. If a compact flow needs to know whether secrets are
available, agents should run metadata-only checks such as:

```bash
aiplus profile status
aiplus secret-broker status
```

Secret-consuming tools should be run through `aiplus secret-broker run --
<command...>` only for an explicit action need. Do not print resolved secret
values in compact guidance or handoff files.
The child command receives the secret in its environment and can still print,
log, transmit, or store it. Use `run --` only with trusted commands for the
specific action.

Private profiles may install approved secret aliases. Run
`aiplus secret-broker list` for the current local mapping. Real Bitwarden smoke
checks require the Bitwarden Secrets Manager `bws` CLI plus a read-only machine
account token; without `bws`, agents should use mock/local status checks only
and must not fall back to printing values.

If validation is blocked by a real safety problem or denied Owner gate,
`aiplus compact checkpoint` prints `BLOCKED_DO_NOT_COMPACT` and does not create a
normal checkpoint file by default.

## Daily Commands

```bash
aiplus status
aiplus doctor
aiplus compact prepare
aiplus compact score
aiplus compact validate
aiplus compact checkpoint
aiplus compact resume
aiplus uninstall --dry-run
```

Only recommend manual compact after `checkpoint` returns `SAFE_TO_COMPACT` and every Owner gate is explicitly `APPROVED`. `UNKNOWN_PENDING` means `UNKNOWN_NEEDS_REVIEW`; `DENIED` blocks compact recommendation.

## Runtime Choices

| Runtime | Install command | Auto compact support | Recommended use |
| --- | --- | --- | --- |
| Codex | `aiplus install codex` | Reminder/checkpoint only | Run `aiplus compact checkpoint`, review output, then manually use Codex `/compact` or UI compact. |
| Claude Code | `aiplus install claude-code` | Optional reviewed hooks and commands | Use project-local Claude Code adapter files; review hooks before enabling them. |
| OpenCode | `aiplus install opencode` | Project-local command workflow | Use project `.opencode/` commands, agents, and prompts; keep global config unchanged. |

Compatibility aliases are supported by the AiPlus CLI:

```bash
aiplus install claude
aiplus install cc
aiplus install oc
aiplus install --runtime codex
aiplus install --all-runtimes
```

## What This Repository Contains

This repository is the compact workflow module source, not the full AiPlus CLI
distribution.

- `core/templates/`: compact handoff templates.
- `core/schemas/`: JSON schemas for policy and ledger structure.
- `core/docs/`: protocol, Owner gate, and checkpoint/resume reference.
- `adapters/codex/`: Codex plugin and skill assets.
- `adapters/claude-code/`: Claude Code plugin-shaped commands, skill, and optional hook example.
- `adapters/opencode/`: OpenCode project-local config, commands, agents, and prompts.
- `examples/`: synthetic examples for all three runtimes.
- `core/scripts/compactctl.mjs`: archived legacy Node helper retained only for compatibility tests and migration reference; not an active compact path.

## What It Can Automate

AiPlus Auto Compact can:

- Create local compact state files in a target project.
- Preserve existing compact files unless an explicit force flow is used.
- Validate required files, sections, enum values, policy JSON, version fields, Owner gates, next actions, and obvious sensitive patterns.
- Write local checkpoint JSON under `.codex/compact/checkpoints/`.
- Print resume-oriented state after compaction.
- Support best-effort automatic resume after compact.
- Keep runtime setup project-local by default.

## What It Cannot Automate

AiPlus Auto Compact cannot:

- Click UI controls, call `/compact`, or force any runtime to compact.
- Wake a host runtime that is waiting for a user message after compact.
- Verify that compaction is operationally appropriate for every project.
- Detect every secret, private path, or personal-data pattern.
- Replace human review of Owner gates.
- Publish packages, create tags, upload artifacts, or configure cloud services.
- Make Codex, Claude Code, and OpenCode share one internal context implementation.

## Safety And Privacy Boundaries

Keep compact state local unless it has been intentionally reviewed and redacted. Do not place secrets, tokens, API keys, private keys, raw transcripts, provider request or response bodies, HAR/WebRTC dumps, private account identifiers, private paths, or personal data in compact files.

Use placeholders in public docs and examples:

- `<REPO_ROOT>`
- `<TARGET_PROJECT>`
- `<OWNER>`
- `<GITHUB_OWNER>`
- `<EXAMPLE_PROJECT>`
- `<REDACTED_SECRET>`
- `<REDACTED_TOKEN>`
- `<REDACTED_PII>`

Validation is structural and heuristic. A validation pass means the files satisfy the local contract; it does not prove the project is ready to compact, safe to publish, compliant, or private.

## Advanced: Legacy Node Reference

The supported compact execution path is Rust-native AiPlus CLI only:

```bash
aiplus compact validate
aiplus compact checkpoint
aiplus compact resume
```

The older standalone Node helper remains in this repository only as archived
history and a compatibility test fixture. It is not an ordinary-user path and
must not be used as a fallback.

If `aiplus` is not found, install AiPlus or fix PATH instead:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
```

Then reopen the terminal or ensure `~/.local/bin` is on PATH. Do not fallback to
Node.

## Validation

Run package acceptance tests from this repository:

```bash
cd <REPO_ROOT>/aiplus-auto-compact
npm test
```

Run compact checks from a target project where `aiplus` is available:

```bash
cd MyProject
aiplus compact validate
aiplus compact checkpoint
aiplus compact resume
```

Exit codes used by the legacy helper and mirrored by the AiPlus compact flow:

- `0`: pass or safe.
- `1`: blocked or invalid.
- `2`: unknown or inconclusive.
- `3`: internal error.

## Examples

Synthetic examples are provided for:

- [Codex project](examples/codex-project/README.md)
- [Claude Code project](examples/claude-code-project/README.md)
- [OpenCode project](examples/opencode-project/README.md)

These examples use placeholders only and are not copied from private projects.

## Migration

`codex-compact-protocol` is the legacy Codex-first public record. AiPlus Auto
Compact is the current cross-agent compact workflow module, and the AiPlus CLI
(`aiplus`) is the current user-facing path.

Existing `.codex/compact/` state remains compatible. For details, read [Migration From Codex Compact Protocol](docs/migration-from-codex-compact-protocol.md).

## Current Release Status

This repository is public on GitHub as `aiplus-auto-compact`. It is not
published to npm, Cargo, Homebrew, any package registry, or any marketplace. The
AiPlus CLI (`aiplus`) is the intended distribution surface for ordinary users.
Future tags, GitHub Releases, package publications, binary uploads, marketplace
submissions, or installer publication require separate Owner approval.

## Contributing

Keep contributions local-first and runtime-neutral unless the change is explicitly adapter-specific. Public docs and examples must use placeholders instead of private paths or real project data.

Before proposing a release change:

```bash
cd <REPO_ROOT>/aiplus-auto-compact
npm test
```

Document behavior changes in `CHANGELOG.md`, update `MODULES.md` when ownership or file layout changes, and update `RELEASE_CHECKLIST.md` before publication review.
