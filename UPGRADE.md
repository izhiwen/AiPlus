# Upgrade Guide

This guide covers cross-version migration notes for users coming from
older AiPlus releases. For new-feature highlights, see
[CHANGELOG.md](./CHANGELOG.md).

## From any v0.5.x → v0.5.16 (current)

**Action: none required.** Project state (`.aiplus/manifest.json`)
written by any v0.5.x is readable by v0.5.16 — the schema version
matcher in `is_supported_manifest_schema` covers `0.5.0` through
`0.5.16` inclusive. Just install the new binary and your existing
projects keep working.

### What changes you'll see after upgrading

| Surface | Before v0.5.16 | After v0.5.16 |
|---|---|---|
| MCP tools count | 3 (route/status/set-team) | 11 (+ 8 lifecycle/inspection tools) |
| `aiplus doctor --check-keyring` | doesn't exist | new diagnostic command |
| `aiplus agent dispatch-history` | doesn't exist | new query command |
| Dispatch log schema | 0.1.1 (success only) | 0.2.0 (success / fail / canceled) |
| `aiplus mcp-register --scope` | doesn't exist | new flag |
| install.sh runtime detection | dump-and-go | offers `mcp-register` |
| Linux binary size | 10.2 MB (v0.5.11) | 7.4 MB |
| Linux libdbus runtime dep | required pre-installed | vendored (statically linked) |
| Uncaught-error prefix | `INTERNAL_ERROR …` | `AIPLUS_UNEXPECTED_ERROR …` |

### Non-breaking changes that affect scripting

If you parse aiplus output in scripts:

- `INTERNAL_ERROR` prefix no longer appears in source. If you grep for
  it as a regression sentinel, switch to `AIPLUS_UNEXPECTED_ERROR`.
- `dispatch-log.jsonl` entries now include `outcome` / `dispatchId` /
  `errorReason` / `errorDetail` fields. Old entries deserialize with
  `outcome="success"` by default — no schema migration needed.

### One-time recommended action

After upgrading, run once per project:

```bash
aiplus doctor --check-keyring   # verify OS keyring path works
aiplus mcp-register             # wire up MCP for your installed runtimes
```

Neither is strictly required — aiplus runs fine without them — but
they unlock the full PI-as-MCP-host experience.

## From v0.4.x → v0.5.x

**Action: re-install per project.** The agent-team schema bumped
between v0.4 and v0.5 in ways that require an install rewrite. Run:

```bash
cd MyProject
aiplus install <your-runtime>   # rewrites .aiplus/ to v0.5 schema
aiplus doctor                   # verify
```

Your existing dispatch logs and memory under `.aiplus/agents/` and
`.aiplus/memory/` are preserved — only the team config and persona
mirrors get rewritten.

### Breaking changes since v0.4

- `aiplus install` now writes `.aiplus/manifest.json` with
  `schemaVersion: "0.5.x"`. v0.4 binaries won't accept this; if you
  must roll back, delete `.aiplus/manifest.json` first.
- The `aiplus-agent-team` and `aiplus-aieconlab` modules became opt-in
  v0.5.4+: `aiplus add aieconlab` is now required to install the
  econ-research team (was bundled in v0.4).
- The deprecated `aiplus agent acceptance` subcommand moved under
  `aiplus agent audit`. The old form prints a redirect hint.

## From pre-v0.4 (v0.3.x and earlier)

We recommend a fresh `aiplus install` rather than an in-place upgrade.
Pre-v0.4 project state is not guaranteed to round-trip through any
v0.5.x doctor check.

```bash
# Back up what matters:
mv .aiplus .aiplus.v0.3-backup

# Re-install fresh on v0.5.16:
aiplus install <your-runtime>
```
