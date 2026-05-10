# Release Automation

Local-only release readiness automation for AiPlus v0.5.x.

## Philosophy

- **Dry-run only**: No push, tag, release, upload, publish, deploy.
- **Local temp staging**: All artifacts built to temp directories.
- **Summaries, not secrets**: Never print secret values.
- **WARN vs BLOCK**: WARN for advisory items; BLOCK for real blockers.
- **No network**: Scripts run without external network calls.
- **Exit nonzero on blockers**: Scripts return exit code 1 for BLOCK, 2 for WARN.

## Scripts

### release-doctor.sh

Run full release readiness check:

```bash
cd /Users/steve/Dropbox/Project/AiPlus/aiplus-public
./scripts/release-doctor.sh
```

Checks:
1. Environment & version consistency (Cargo.toml, CLI --version, CHANGELOG)
2. Rust QA (`cargo fmt`, `cargo clippy`, `cargo test`, `cargo metadata`, `git diff --check`)
3. CLI smoke tests (`--help`, `doctor`, `memory doctor`, `profile doctor`, `status`, `compact validate`)
4. Safety & boundary (no push/tag/release in scripts, LICENSE exists, `publish = false`)

Report saved to: `docs/stabilization/release-doctor-report.md`

### secret-boundary-scan.sh

Scan for secrets, private content, telemetry, global config edits:

```bash
cd /Users/steve/Dropbox/Project/AiPlus/aiplus-public
./scripts/secret-boundary-scan.sh
```

Checks:
1. Secret values (API keys, tokens, private keys, bearer tokens)
2. Raw transcripts, checkpoints, `.env` files
3. Private profile content in public assets
4. Telemetry or data upload code
5. Global config modification code
6. Node fallback in Rust code
7. Production project paths in source
8. Bitwarden token references

### subproduct-drift.sh

Compare bundled assets against sibling subproducts:

```bash
cd /Users/steve/Dropbox/Project/AiPlus/aiplus-public
./scripts/subproduct-drift.sh
```

Checks:
1. File count comparison (asset vs source)
2. Critical files present (README.md, MODULES.md, SECURITY.md)
3. No private content leaked into assets
4. Version consistency (if VERSION file exists)

### artifact-check.sh

Dry-run artifact build and verification:

```bash
cd /Users/steve/Dropbox/Project/AiPlus/aiplus-public
./scripts/artifact-check.sh
```

Checks:
1. Build release binary with `cargo build --release`
2. Create temp archive with exclusions
3. Verify archive contents (LICENSE, binary present)
4. Generate SHA-256 checksum
5. Verify no excluded items (`.env`, secrets, checkpoints, logs)
6. Check existing release-artifacts directory

## Exclusions

All scripts exclude:
- `.git/`
- `target/`
- `.codex/`
- `.aiplus/`
- `*.log`
- `*.tmp`
- `.DS_Store`
- `Cargo.lock` (for scans)
- Private profile content (`aiplus-work-with-zhiwen`)

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | PASS |
| 1 | BLOCKED (real blockers) |
| 2 | NEEDS_FIX (warnings only) |

## Integration

Run all checks in sequence:

```bash
cd /Users/steve/Dropbox/Project/AiPlus/aiplus-public
./scripts/release-doctor.sh && \
./scripts/secret-boundary-scan.sh && \
./scripts/subproduct-drift.sh && \
./scripts/artifact-check.sh
```

## Owner Gates

Scripts enforce these gates:
- No push/tag/release/upload code in scripts
- No secret values printed
- No global config modification
- No telemetry
- No private profile content in public assets

Real dangerous actions (push, tag, release, upload) still require explicit Owner approval.
Scripts only automate checks, not approvals.
