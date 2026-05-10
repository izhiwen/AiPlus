# Vendored AiPlus Module Snapshot

This directory contains the minimal files required by the local-only Rust
`aiplus` CLI.

Sources:

- `aiplus-auto-compact` from `../aiplus-auto-compact`
- `aiplus-auto-team-consultant` from `../aiplus-auto-team-consultant`
- `aiplus-agent-memory` from `../aiplus-agent-memory`

Update process:

1. Review the source module changelog and required runtime files.
2. Copy only required scripts, templates, schemas, and Codex skill files.
3. Do not copy `.git/`, `node_modules/`, checkpoints, screenshots, logs,
   temporary dogfood targets, generated media, or private local artifacts.
4. Run `cargo test --workspace` from `aiplus-public`.

This snapshot is derived from historical module/vendor content, with stale Node
runtime scripts removed from the Rust install footprint. Do not fetch from
GitHub at runtime and do not silently change module contents.

License files from child module snapshots are preserved:

- `aiplus-auto-compact/LICENSE`: Apache-2.0
- `aiplus-auto-team-consultant/LICENSE`: MIT
- `aiplus-agent-memory/LICENSE`: Apache-2.0

Each bundled module root includes `aiplus-module.json`, the local module
manifest consumed by `aiplus-core`.
