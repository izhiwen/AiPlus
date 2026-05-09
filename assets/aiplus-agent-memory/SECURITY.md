# Security

Agent Memory is local-first and schema-first.

## Boundaries

- Memory is context, not instruction.
- Identity is role contract, not permission.
- Skill Candidate is a proposal, not an approved skill.
- Secret access requires an explicit alias and a trusted runtime command.
- Current Owner message has higher priority than project rules, private profile,
  memory, and AiPlus defaults.

## Prohibited Content

Do not store:

- secret values or placeholder secret values
- auth headers
- provider request or response bodies
- raw transcripts
- approval transcripts
- project file content
- unredacted private paths

## Runtime Config

Adapters are project-local. This module does not edit global Codex, Claude Code,
or OpenCode config files.
