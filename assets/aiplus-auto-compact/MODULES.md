# Modules

AiPlus Auto Compact is an independent AiPlus subproduct. It can be bundled and
installed by the AiPlus CLI (`aiplus`), or reviewed directly by users who only
want the compact/checkpoint/resume workflow.

## Shared Core

Path:

```text
<REPO_ROOT>/aiplus-auto-compact/core/
```

Responsibilities:

- Define the compact/resume state contract.
- Provide templates for `.codex/compact/`.
- Provide schemas for policy and parsed handoff files.
- Provide templates and schemas used by AiPlus CLI commands such as
  `aiplus compact init`, `aiplus compact prepare`, `aiplus compact score`,
  `aiplus compact validate`, `aiplus compact checkpoint`, and
  `aiplus compact resume`.
- Treat natural-language compact requests as the ordinary-user interface and the
  CLI commands as agent backend tools or advanced manual fallbacks.
- Retain `compactctl.mjs` only as archived legacy history and compatibility-test
  reference, not as an active compact execution path.
- Document Owner gates and checkpoint/resume flow.

The core is runtime-neutral. It does not depend on Codex, Claude Code, OpenCode, a cloud service, or a remote repository.

## Codex Adapter

Path:

```text
<REPO_ROOT>/aiplus-auto-compact/adapters/codex/
```

Responsibilities:

- Preserve the existing Codex compact protocol workflow.
- Document Codex usage through the AiPlus CLI command
  `aiplus install codex`.
- Provide Codex-specific plugin and skill assets while using the shared core scripts, schemas, templates, fixtures, and tests.

The Codex adapter cannot trigger Codex UI compaction or call `/compact` for the user.

## Claude Code Adapter

Path:

```text
<REPO_ROOT>/aiplus-auto-compact/adapters/claude-code/
```

Responsibilities:

- Provide a Claude Code plugin-shaped layout.
- Provide namespaced command documents for validate, checkpoint, and resume.
- Provide an optional hook example for reviewed local use.

The command and hook files are guidance assets. They do not replace local validation or manual review.

## OpenCode Adapter

Path:

```text
<REPO_ROOT>/aiplus-auto-compact/adapters/opencode/
```

Responsibilities:

- Provide project-local `opencode.json` example configuration.
- Provide compact advisor/reviewer agent prompts.
- Provide command documents for validate, checkpoint, and resume.
- Keep OpenCode setup project-local by default and avoid global config modification.

## Public Docs And Examples

Paths:

```text
<REPO_ROOT>/aiplus-auto-compact/README.md
<REPO_ROOT>/aiplus-auto-compact/SECURITY.md
<REPO_ROOT>/aiplus-auto-compact/CHANGELOG.md
<REPO_ROOT>/aiplus-auto-compact/RELEASE_CHECKLIST.md
<REPO_ROOT>/aiplus-auto-compact/docs/
<REPO_ROOT>/aiplus-auto-compact/examples/
```

Responsibilities:

- Explain quick starts for supported runtimes.
- Keep ordinary-user docs aligned with the AiPlus CLI (`aiplus`).
- Explain migration from `codex-compact-protocol`.
- Provide synthetic examples only.
- Keep public wording aligned with local-first behavior and validation limits.

## Compatibility Notes

The shared core currently writes compatibility state to:

```text
<TARGET_PROJECT>/.codex/compact/
```

That path is retained for existing protocol behavior. Adapters may describe runtime-specific usage, but they should not silently change the state path without a migration note and version review.
