# TEST-4 Install Order Matrix

This workflow catches order-dependent regressions where AiEconLab files are
written for one runtime but not refreshed when another runtime adapter is
installed later.

## CI policy

`.github/workflows/install-order-matrix.yml` follows the Owner-selected
budget policy from issue #26:

- Pull requests run only when the PR has the `test-infra` label.
- The nightly scheduled run executes the full matrix.
- Manual dispatch executes the full matrix.

The full matrix is 9 install-order cases across 3 GitHub-hosted OS images:
`ubuntu-latest`, `macos-latest`, and `windows-latest`.

## Selected cases

1. `h1_codex_ael_then_claude`: install `codex`, add `aieconlab`, then
   install `claude-code`. This is the H1 ordering case.
2. `codex_ael_then_opencode`: install `codex`, add `aieconlab`, then
   install `opencode`.
3. `claude_ael_then_codex`: install `claude-code`, add `aieconlab`, then
   install `codex`.
4. `claude_ael_then_opencode`: install `claude-code`, add `aieconlab`,
   then install `opencode`.
5. `opencode_ael_then_codex`: install `opencode`, add `aieconlab`, then
   install `codex`.
6. `opencode_ael_then_claude`: install `opencode`, add `aieconlab`, then
   install `claude-code`.
7. `all_runtimes_then_ael`: install all three runtimes, then add
   `aieconlab` last.
8. `claude_ael_then_codex_then_opencode`: install `claude-code`, add
   `aieconlab`, then install `codex` and `opencode` in series.
9. `codex_ael_then_claude_then_opencode`: install `codex`, add
   `aieconlab`, then install `claude-code` and `opencode` in series.

## Assertions

Each job runs `tests/install_order_matrix.py` in a clean temporary git
project with isolated home/config directories. The harness:

- checks `.aiplus/manifest.json` contains exactly the runtimes expected for
  that case and includes the `aieconlab` module;
- verifies core AEL files under `.aiplus/modules/aieconlab`,
  `.aiplus/agents`, `.aiplus/agents/experts`, and
  `.aiplus/consultant-team.toml`;
- verifies Codex files when `codex` is installed: `AGENTS.md` and
  `.aiplus/AGENTS.aiplus.md`;
- verifies Claude Code files when `claude-code` is installed: `CLAUDE.md`,
  20 `.claude/agents/aieconlab-*.md` files, and 4
  `.claude/commands/aiel-*.md` files;
- verifies OpenCode files when `opencode` is installed:
  `.opencode/opencode.json`, 20 `.opencode/agents/aieconlab-*.md` files,
  and 4 `.opencode/commands/aiel-*.md` files;
- requires `aiplus doctor` to report `DOCTOR_STATUS=PASS`.
