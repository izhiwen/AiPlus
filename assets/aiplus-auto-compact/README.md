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
刷新
```

Or:

```text
refresh
```

Meaning: treat this as AiPlus refresh first, report Auto Compact and compact
state, reread `AGENTS.md`, reread `.aiplus/AGENTS.aiplus.md`, read
`.codex/compact/current-handoff.md` if present, enable AiPlus guidance, and
continue the current task.

### Path C: Advanced module-only adoption

If you only want AiPlus Auto Compact, you can read this repository directly,
inspect `core/templates/`, `core/docs/`, and the runtime adapters, then copy or
adapt the compact workflow into your project. This is useful for policy review,
custom integrations, or migration from `codex-compact-protocol`.

The module-only path is not the beginner install path. Keep the legacy Node
helper in [Advanced: Legacy Node Reference](#advanced-legacy-node-reference) for
audits and compatibility checks only.

## What Happens Around Compact

Before compact, the agent should run when available:

```bash
aiplus compact validate
aiplus compact checkpoint
```

If the checkpoint is ready, the agent should recommend manual compact with
language like:

```text
建议现在 compact。AiPlus checkpoint 已准备好。compact 后如果宿主继续把控制权交给我，我会自动恢复；如果工具等待你发消息，随便说“继续”“刷新”“continue”“resume”或类似意思即可。
```

After the host compact completes:

- If the host gives control back automatically, the agent should run
  `aiplus compact resume` and continue without requiring user input.
- If the host requires a user message, any natural continuation should work:

```text
继续
刷新
refresh
continue
resume
go on
接着
```

This is best-effort automatic resume. AiPlus Auto Compact can prepare the
checkpoint and tell the agent how to resume, but it cannot wake a host runtime
that is waiting for the user.

## Daily Commands

```bash
aiplus status
aiplus doctor
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
- `core/scripts/compactctl.mjs`: legacy standalone Node helper retained for compatibility tests and migration reference.

## What It Can Automate

AiPlus Auto Compact can:

- Create local compact state files in a target project.
- Preserve existing compact files unless an explicit force flow is used.
- Validate required files, sections, enum values, policy JSON, version fields, Owner gates, next actions, and obvious sensitive patterns.
- Write local checkpoint JSON under `.codex/compact/checkpoints/`.
- Print resume-oriented state after compaction.
- Support best-effort automatic resume when the host runtime returns control to
  the agent after compact.
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

The current ordinary-user path is the AiPlus CLI (`aiplus`). The older
standalone Node helper remains in this repository for compatibility and
migration review:

```bash
node <REPO_ROOT>/aiplus-auto-compact/core/scripts/compactctl.mjs init
node <REPO_ROOT>/aiplus-auto-compact/core/scripts/compactctl.mjs validate
node <REPO_ROOT>/aiplus-auto-compact/core/scripts/compactctl.mjs checkpoint
node <REPO_ROOT>/aiplus-auto-compact/core/scripts/compactctl.mjs resume
```

Use this only when you are auditing the legacy workflow or testing this module
without the AiPlus CLI.

## Validation

Run package acceptance tests from this repository:

```bash
cd <REPO_ROOT>/aiplus-auto-compact
npm test
node --check core/scripts/compactctl.mjs
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
node --check core/scripts/compactctl.mjs
```

Document behavior changes in `CHANGELOG.md`, update `MODULES.md` when ownership or file layout changes, and update `RELEASE_CHECKLIST.md` before publication review.
