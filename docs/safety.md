# Safety

The Rust CLI is project-local only.

This document describes boundaries and checks. It is not a safety, privacy,
compliance, correctness, or release certification.

## Allowed Writes

- `.aiplus/`
- `.codex/compact/`
- `.claude/` project adapter files
- `.opencode/` project adapter files
- AiPlus managed block in project `AGENTS.md`

## Forbidden Actions

The Rust CLI does not implement:

- npm publish
- cargo publish
- registry publish
- GitHub push, tag, or release
- Homebrew release
- marketplace submission
- system/global install
- global Codex, Claude Code, or OpenCode config edits
- telemetry
- user data upload
- remote auto-update
- shell profile edits

Allowed network boundary:

- AiPlus may fetch public pricing/release metadata by default for Compact
  Savings Estimate and write a local pricing cache.
- `aiplus compact savings`, `prepare`, `checkpoint`, and `resume` use fresh cache
  first; when cache is missing or stale they may refresh public pricing
  automatically. Network failure must not block compact, checkpoint, resume, or
  token savings reporting.
- The default pricing cache TTL is 7 days.
- AiPlus does not upload prompts, project files, checkpoints, savings ledgers,
  secrets, provider account data, billing data, or usage history.

## Publication Gates

Owner approval remains required before:

- creating additional public repos
- changing license away from Apache-2.0 or changing public legal wording
- pushing commits outside the reviewed release scope
- creating or pushing git tags beyond the approved release
- creating GitHub Releases beyond the approved release
- uploading binary artifacts beyond the verified macOS Apple Silicon
  asset and `checksums.txt`
- publishing to package registries
- creating Homebrew formulas or taps
- publishing installer channels beyond the reviewed `install.sh`
- publishing npm compatibility wrappers
- installing binaries into system/global paths
- modifying `$CODEX_HOME`, `~/.codex`, `~/.claude`, OpenCode global config,
  shell profiles, `~/.cargo/bin`, `/usr/local/bin`, or system paths

Owner approved v0.3.0 GitHub Release creation, the verified macOS Apple Silicon
binary upload, `checksums.txt`, `install.sh`, and user-level installation to
`~/.local/bin/aiplus`.

## Compact Savings Boundary

Savings estimates are local, aggregate, and approximate. They are not billing
data, guaranteed savings, exact token accounting, compliance evidence, or quality
proof. Unknown model pricing must not silently use generic pricing as if it were
model-specific.

## Write Safety

The CLI rejects absolute paths, `..` traversal, and symlink components in write
paths, including dangling symlinks. Existing differing files are not overwritten
unless the command explicitly supports the required force/backup/yes flow.

Uninstall requires an AiPlus manifest and refuses unknown `.aiplus/` entries
unless `--force` is supplied. `.codex/compact/` is preserved by uninstall.

## Public Wording Boundary

Allowed wording:

- local-only
- Owner-gated
- structural validation
- heuristic scan
- public-ready candidate
- release-readiness checklist

Do not claim:

- guaranteed safe
- certified
- compliant
- secure by default
- production-ready
- official
- endorsed
- privacy guaranteed
- safety approved

## License Boundary

The Rust mainline/public-ready package is Apache-2.0. Bundled child module
snapshots preserve their existing licenses. Licensing is not a safety, privacy,
compliance, correctness, or release certification.
