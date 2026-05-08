# Compatibility Notes

AiPlus Auto Team Consultant is an independent AiPlus subproduct and a project-local module installed by the Rust-first `aiplus` CLI.

AiPlus is the ecosystem and distribution entry. AiPlus Auto Team Consultant is one module in that family. Users can install it through AiPlus, or inspect this repo directly when they only want the team-consultant workflow.

## AiPlus Ecosystem Installer

Install AiPlus, then install this module into the project. Replace `MyProject` with your project folder:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
cd MyProject
aiplus install codex
```

Then type `AiPlus 刷新`, `刷新 AiPlus`, `aiplus refresh`, or `aiplus status` in
the already-open agent session. Generic `刷新` / `refresh` should still try
AiPlus first after installation, but explicit AiPlus triggers are safer when a
project has its own refresh meaning.

If the project already has an older AiPlus install, `aiplus install codex` safely upgrades AiPlus managed files, backs up replaced managed files under `.aiplus/backups/`, and preserves existing `.codex/compact/` state.

## Codex

Use:

```bash
aiplus install codex
```

Then type `AiPlus 刷新` or `aiplus refresh` in the already-open Codex session.

## Claude Code

Use:

```bash
aiplus install claude-code
```

Then type `AiPlus 刷新` or `aiplus refresh` in the already-open Claude Code session.

## OpenCode

Use:

```bash
aiplus install opencode
```

Then type `AiPlus 刷新` or `aiplus refresh` in the already-open OpenCode session.

## Limits

The adapter source files in this repo help structure session behavior. They do not execute multi-agent workflows automatically, modify global agent config, upload data, add telemetry, or approve external actions.

Module-only adoption is supported for advanced users who want to inspect or copy templates, skills, prompts, and examples directly. It is not the ordinary install path.
