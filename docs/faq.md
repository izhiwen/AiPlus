# FAQ

## General

**Q: What is AiPlus?**

AiPlus helps AI coding agents (Codex, Claude Code, OpenCode) keep project-local memory, handoffs, and review workflows. It is a CLI tool that the agent calls, not a plugin or daemon.

**Q: Does AiPlus send data anywhere?**

No. AiPlus is fully local. It writes to `.aiplus/`, `.codex/compact/`, and `~/.config/aiplus/`. The only network access is `aiplus pricing update` (fetches public pricing) and `aiplus self update` (checks for new releases).

**Q: Does AiPlus edit my global agent config?**

No. All writes are project-local or user-profile-scoped (`~/.config/aiplus/`). AiPlus never edits global Codex, Claude Code, or OpenCode configuration.

## Memory

**Q: Can AiPlus read my secrets?**

AiPlus scans memory text for secret patterns (API keys, JWT, private keys) and blocks writes that match. It does not store, print, or transmit secrets.

**Q: What happens to rejected memories?**

They stay in the JSONL file with `status=rejected` but are excluded from context injection. `aiplus memory forget <id>` marks records as rejected.

**Q: Can I sync memory across projects?**

Profile preferences can sync from a private profile into project memory via `aiplus memory profile`. Project facts stay project-local.

## Compact

**Q: Does AiPlus compact for me?**

No. AiPlus prepares, reminds, and saves state, but the host agent must trigger the actual compact. AiPlus cannot press the compact button.

**Q: What is a context capsule?**

A JSON file (`.codex/compact/context-capsule.json`) created by `compact prepare` that captures the session objective, state, decisions, owner gates, and safety markers. It is used by `compact resume` to restore context.

**Q: What is `compact watch`?**

A mode that runs `compact remind` at intervals, emitting JSON status. Useful for automated monitoring. Handles SIGINT and SIGTERM cleanly.

## Profile

**Q: What is a private profile?**

A user-level profile (e.g., `aiplus-work-with-zhiwen`) installed under `~/.config/aiplus/profiles/`. It stores cross-project preferences and working rules. It must not contain secrets or project-specific files.

**Q: What is the supplemental bundle?**

Optional files installed alongside the profile: `USER.md`, `MEMORY.md`, `preferences/`, `identities/`, `sync/`. These extend the profile with additional context.

**Q: Is `sync/` cloud sync?**

No. `sync/` contains local policy files that define how profile preferences map to projects. There is no network sync, no cloud service, and no external connection.
