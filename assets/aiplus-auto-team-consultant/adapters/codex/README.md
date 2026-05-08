# Codex Adapter

This directory is the Codex adapter source used by the Rust-first `aiplus` CLI.

Most users should install it from their project with:

```bash
aiplus install codex
```

Then type `刷新` or `refresh` in the already-open Codex session.

## What This Adapter Provides

The adapter preserves the AiPlus Auto Team Consultant Codex Skill behavior from the source module: session-local routing, expert lens selection, Consultant Packet output, CEO Handoff, Gate Packet, pressure-test labeling, and Owner Gate judgment.

The adapter uses shared templates from `../../core/templates/`.

When choosing an output format, start with `../../core/templates/TEMPLATE_INDEX.md`.

## Boundaries

- It does not execute agents automatically by itself.
- It does not publish, push, deploy, or contact external accounts without Owner approval.
- It does not provide safety, compliance, legal, privacy, product-quality, or release-readiness guarantees.
- Pressure-Test output is simulated only and must be labeled `SIMULATED_PRESSURE_TEST_ONLY`.
