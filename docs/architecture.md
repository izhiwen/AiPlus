# AiPlus Rust Architecture

AiPlus is Rust-first for the local CLI. The current primary Rust workspace is:

```text
aiplus/
  Cargo.toml
  crates/aiplus-cli/
    Cargo.toml
    src/main.rs
    tests/parity.rs
  assets/
  docs/
```

`crates/aiplus-cli` builds the `aiplus` binary. The crate currently contains CLI
parsing, file planning, safe writes, runtime adapter generation, manifest
serialization, status, doctor, update, add, uninstall, and Rust-native compact
logic.

## Asset Strategy

The selected strategy is Option C: a Rust-side `assets/` directory derived from
existing vendor snapshots. Runtime installation copies from this local snapshot.
No network fetch is used at runtime, and the CLI does not depend on GitHub.

## License

The Rust mainline/public-ready package is Apache-2.0. The workspace `LICENSE`
and Cargo metadata use Apache-2.0. Bundled child module snapshots preserve their
existing licenses:

- `aiplus-auto-compact`: Apache-2.0
- `aiplus-auto-team-consultant`: MIT

Licensing is not a safety, privacy, compliance, correctness, or release
certification.

## Manifest

The CLI release is v0.2.1. The installed project manifest schema remains
`0.2.1` for compatibility with existing local installs:

```json
{
  "schemaVersion": "0.2.1",
  "installer": "aiplus",
  "installerVersion": "0.2.1",
  "targetRoot": "...",
  "runtimeAdapters": ["codex"],
  "modules": {
    "auto-compact": {
      "version": "0.1.0",
      "source": "bundled",
      "path": ".aiplus/modules/aiplus-auto-compact"
    }
  },
  "managedFiles": []
}
```

## Runtime Adapters

- Codex: `AGENTS.md` managed block pointing to `.aiplus/AGENTS.aiplus.md`.
- Claude Code: `.claude/commands/aiplus-refresh.md` and
  `.claude/agents/aiplus-advisor.md`.
- OpenCode: `.opencode/opencode.json`, command, agent, and prompt files.

## Compact Status

`compact init` creates the project-local compact state from vendored templates
with Rust safe writes. `compact prepare`, `compact score`, `compact validate`,
`compact checkpoint`, and `compact resume` are Rust-native and do not invoke
Node. In ordinary agent sessions, natural language such as "prepare compact" or
"save progress" is the preferred interface; the CLI commands are backend tools
for agents, advanced manual fallbacks, and maintainer debugging commands.

`COMPACT_RUST_NATIVE_STATUS=PASS` marks compact commands that use Rust-native
logic.

## Compact Savings Estimate

AiPlus records aggregate savings events in
`.codex/compact/savings-ledger.jsonl` during `compact prepare`, permitted
`compact checkpoint`, and `compact resume`. Events store estimated token counts,
weighted reduction inputs, pricing coverage, model hint confidence, and cost
estimate availability. They do not store prompt text, transcript text, project
file contents, raw checkpoint text, billing data, provider account data, or usage
history.

`aiplus compact savings` reads the local ledger and uses fresh cached pricing
when available. If pricing cache is missing or stale, AiPlus may refresh public
pricing automatically. It reports latest compact and all-time totals. All-time
reduction is weighted:
`totalEstimatedTokensSaved / totalEstimatedBaselineTokens * 100`.

`aiplus pricing update` explicitly refreshes public pricing data from the
network. Pricing data is cached in a user cache such as
`~/.cache/aiplus/pricing-cache.json` with a default 7-day TTL. Compact commands
continue when pricing fetch fails, pricing is missing, stale, or unavailable.

## Public Repository Layout

The public repository name is `aiplus` and the Rust workspace is the repository
root:

```text
aiplus/
  README.md
  README.zh-CN.md
  MODULES.md
  Cargo.toml
  Cargo.lock
  crates/
    aiplus-cli/
      Cargo.toml
      src/main.rs
      tests/parity.rs
  assets/
    README.md
    aiplus-auto-compact/
    aiplus-auto-team-consultant/
  docs/
    architecture.md
    safety.md
    public-repo-plan.md
    distribution-plan.md
    binary-artifact-matrix.md
    migration-from-node-cli.md
    qa-release-readiness.md
    node-parity.md
    dogfood-report.md
  tests/
    README.md
  CHANGELOG.md
  RELEASE_CHECKLIST.md
```

The public user-facing product name is `AiPlus`. The repository, binary, shell
command, and crate/package identifiers remain lowercase `aiplus`/`aiplus-cli`
where required by command and package conventions.

## Archived Node Boundary

The Node CLI remains outside the beginner path and is not included in this
public source package. It is an archived/reference implementation for behavior
audits only. Public Rust docs should link to Node parity only from advanced or
migration sections.
