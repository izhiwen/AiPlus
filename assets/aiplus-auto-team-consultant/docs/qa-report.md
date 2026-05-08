# QA Report

## AiPlus Ecosystem And Subproduct Alignment

- README.md explains AiPlus as the ecosystem / CLI distribution entry and AiPlus Auto Team Consultant as an independent subproduct/module.
- README.md documents three paths: AiPlus release installer when available, existing local `aiplus` command, and advanced module-only adoption.
- README.md foregrounds `aiplus install codex`, `aiplus install claude-code`, `aiplus install opencode`, and `aiplus install all`.
- README.zh-CN.md mirrors the same beginner flow.
- Both READMEs tell already-open agent sessions to type `刷新` or `refresh` and explain that the agent reloads project-local routing instructions.
- Adapter READMEs are reference/source docs, not the primary install path.
- Safety boundary remains project-local: no upload, telemetry, global config edits, package publish, tag, release, or marketplace submission.
- Stale adapter-heavy scan was run. Remaining matches are advanced/reference layout terms or false positives such as "manually".
- Private path / secret scan was run. No real private project names, private paths, tokens, API keys, passwords, or private keys were found.
- Junk artifact scan was run. No `.DS_Store`, logs, archives, screenshots, temp files, `.env`, or backup files were found.
- Rust CLI command examples were checked against the current `aiplus-rust` command surface: `aiplus install codex`, `aiplus install claude-code`, `aiplus install opencode`, `aiplus install all`, and `刷新` / `refresh`.
- GitHub contents check found no live `install.sh` in `izhiwen/aiplus` at review time, so README labels the one-command installer path as available when the AiPlus release installer is live.

## Previous Docs Polish Addendum

- Template chooser coverage: `core/templates/TEMPLATE_INDEX.md` added and linked from README.md and README.zh-CN.md.
- Claude Code adapter README coverage: 3-minute quick start, command map, agent map, Owner gates, synthetic pressure-test note, and no-global-config boundary documented.
- OpenCode adapter README coverage: 3-minute quick start, project-local setup pattern, command map, agent map, prompt map, Owner gates, synthetic pressure-test note, and no-global-config boundary documented.
- Automation scope: unchanged. No scripts, CLI, package manifests, telemetry, external integration, or autonomous execution scope added by docs polish.

## Local Checks

- Required-file check: `REQUIRED_FILES_PASS`
- JSON parse check: `JSON_PARSE_PASS`
- Manifest reference check: `MANIFEST_REFERENCES_PASS`
- Private-data scan: no matches for real private project names, private paths, GitHub tokens, API keys, passwords, or private keys.
- Forbidden-claim scan: matches are boundary/disclaimer language only.
- Automation scan: no `package.json`, `.mjs`, `.js`, or `.sh` files found.

## Required File Coverage

Expected package areas:

- root docs and release artifacts: present
- `core/docs`: present
- `core/templates`: present
- `adapters/codex`: present
- `adapters/claude-code`: present
- `adapters/opencode`: present
- runtime examples: present

## Publication Gate

GitHub publication is allowed only after local PASS. No tag, GitHub Release, registry publish, marketplace submission, global install, telemetry, MCP server, App connector, or unrelated module modification is approved.

## Result

LOCAL_QA_STATUS=PASS
