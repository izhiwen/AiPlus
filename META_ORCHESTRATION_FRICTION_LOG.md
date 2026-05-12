# AiPlus Agent Team v0.1 Implementation — Meta-Orchestration Friction Log

## Entry 1: Subagent Empty Return on Critical Phase
- **ts**: 2026-05-12T20:00:00Z
- **phase**: 20a
- **friction_type**: "agent self-attest"
- **description**: Phase 20a subagent (self-audit run) returned empty result packet. CEO had to manually execute the audit run, discovering that the pre-audit gate was correctly blocking due to manifest hash mismatch. Subagent did not report failure mode.
- **auditor_check_that_would_have_caught**: `v0.1-parity-tests-pass` — if parity tests included audit-run smoke test
- **suggested_v0.2_improvement**: Require subagents to always return structured result packets; empty response = automatic NEEDS_FIX

## Entry 2: Persona Files Never Materialized
- **ts**: 2026-05-12T20:15:00Z
- **phase**: 18
- **friction_type**: "overclaim"
- **description**: Phase 18 subagent claimed "Created 20 persona files" but files were never written to disk. CEO discovered this 30 minutes later during Phase 22 cross-runtime test when status showed 0 agents.
- **auditor_check_that_would_have_caught**: `v0.1-parity-tests-pass` — file existence check
- **suggested_v0.2_improvement**: Add file-system verification step after any subagent claims file creation; cross-ref with `git status`

## Entry 3: Chinese Alias Test Used Wrong Binary
- **ts**: 2026-05-12T20:20:00Z
- **phase**: 24
- **friction_type": "self-execution slip"
- **description**: Phase 24 regression test used `~/.local/bin/aiplus` (old binary) instead of newly built `target/release/aiplus`, causing false failure on `aiplus 团队`.
- **auditor_check_that_would_have_caught**: `EXISTING_SUBCOMMAND_REGRESSION_EVIDENCE` should verify binary path matches build artifact
- **suggested_v0.2_improvement**: Sandbox tests must use explicit binary path variable, never rely on PATH

## Entry 4: Role List Mismatch Between Code and Assets
- **ts**: 2026-05-12T20:25:00Z
- **phase**: 22
- **friction_type**: "schema fixture confusion"
- **description**: `core.rs` had 8 core roles + 6 functional + 5 stub = 19 total, but schema expects 20 (9 core + 6 functional + 5 stub). Embedded assets only had 8 core roles (missing auditor).
- **auditor_check_that_would_have_caught**: `v0.1-parity-tests-pass` with role count assertion
- **suggested_v0.2_improvement**: Single source of truth for role lists (TOML manifest) instead of hardcoded arrays

## Entry 5: GPG Subkey vs Primary Key Fingerprint Mismatch
- **ts**: 2026-05-12T20:30:00Z
- **phase**: 20a
- **friction_type**: "self-execution slip"
- **description**: CEO spent 30 minutes debugging why `git commit -S` fingerprint didn't match recorded fingerprint. Root cause: git uses subkey for signing, but setup recorded primary key fingerprint.
- **auditor_check_that_would_have_caught**: `setup-gpg` self-test with `git commit -S` + `git log --format='%GF'` verification
- **suggested_v0.2_improvement**: `setup-gpg` must verify signing key fingerprint by actually signing a test commit

## Summary
All 5 friction points were caught during the same implementation run, not by the Auditor (which was still being built). In v0.2, these checks should be part of the continuous audit pipeline.
