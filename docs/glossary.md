# Glossary

| Term | Definition |
|---|---|
| **AiPlus** | The product. Helps AI coding agents keep memory, handoffs, and workflows. |
| **aiplus** | The CLI command, binary, crate, and repository name. |
| **Agent** | An AI coding assistant: Codex, Claude Code, or OpenCode. |
| **Runtime** | The agent platform. One of `codex`, `claude-code`, or `opencode`. |
| **Memory** | Project-local facts, decisions, and preferences stored under `.aiplus/memory/`. |
| **Memory Record** | A single JSON entry in the memory JSONL file with type, scope, confidence, status. |
| **Context Capsule** | A JSON file (`.codex/compact/context-capsule.json`) capturing session state for resume after compact. |
| **Compact** | The host agent's context window compression. AiPlus prepares and resumes around it but does not trigger it. |
| **Handoff** | The current-handoff.md file in `.codex/compact/` describing session state for continuity. |
| **Checkpoint** | A saved snapshot of compact state including handoff, decisions, and capsule. |
| **Remind** | AiPlus checking whether now is a good time to compact. |
| **Watch** | Automated repeated remind checks at intervals. |
| **Role Identity** | A TOML file defining what a role does (activation, output contract, owner gates). |
| **Skill Candidate** | A proposed repeatable workflow pattern. Not an approved skill until owner review. |
| **Profile** | A user-level private configuration installed under `~/.config/aiplus/profiles/`. |
| **Supplemental Bundle** | Optional profile files: `USER.md`, `MEMORY.md`, `preferences/`, `identities/`, `sync/`. |
| **Redaction** | Scanning text for sensitive patterns (API keys, JWT, private keys) and blocking writes. |
| **Owner Gate** | An action requiring explicit owner approval (publish, deploy, external accounts). |
| **Secret Broker** | AiPlus subsystem for resolving approved secret aliases without printing values. |
| **BWS** | Bitwarden Secrets CLI. Used by secret-broker for secret resolution. |
| **JSONL** | JSON Lines format. One JSON object per line. Used for memory and ledger storage. |
| **FTS5** | SQLite full-text search. Used for session search indexing. |
| **Managed Block** | The AiPlus-controlled section in `AGENTS.md` marked with begin/end comments. |
