# Profile Bundle Plan v1.0

## Install Contract

`aiplus profile install <profile> --source <path> --user --yes`:

1. Install core files (always): `profile.toml`, `AGENTS.profile.md`
2. Install supplemental bundle files if present:
   - `USER.md` → `<profile-dir>/USER.md`
   - `MEMORY.md` → `<profile-dir>/MEMORY.md`
   - `preferences/` → `<profile-dir>/preferences/` (recursively)
   - `identities/` → `<profile-dir>/identities/` (recursively)
   - `sync/` → `<profile-dir>/sync/` (recursively)
3. Install secret aliases if present (existing behavior, to secret-broker dir)
4. Preserve directory structure under install target
5. Backup existing profile dir before overwrite
6. Skip files/directories that do not exist in source (optional)
7. Atomic writes via `write_file_atomic`
8. No symlink following into unsafe paths

## Context Contract

`aiplus profile context <profile>`:
- Load profile metadata from installed `profile.toml`
- Summarize supplemental bundle presence (file counts, not content)
- Output redacted summary (no secret values, no raw transcripts)

`aiplus user context`:
- Load from installed profile `USER.md`
- Truncate if too large (> 8KB), show first 2KB + "... [truncated]"
- Redact phone-like and secret-assignment patterns

`aiplus memory context` (existing, project-local) — keep as-is.

`aiplus identity context --role <role>`:
- Try project-local identity first
- Fall back to installed profile identity if project missing
- Redact sensitive fields

## Doctor Contract

`aiplus profile doctor [profile]`:
- Validate core files exist
- Check supplemental bundle files presence
- Verify identity files are parseable TOML
- Verify memory directory exists if profile declares it
- Verify sync policy files are parseable if present
- Confirm public/private boundary preserved
- Never print secret values

## Redaction/Secret Boundary

- Use `aiplus_core::redaction::reject_sensitive_memory_text` for memory/user context
- Use `aiplus_core::redaction::has_secret_assignment` for doctor scans
- Strip JWT-like tokens
- Strip phone-like numbers
- Never copy `secret-aliases.tsv` into public assets

## Public/Private Boundary

- Public CLI code stays in `aiplus-public`
- Private profile content stays in `aiplus-work-with-zhiwen`
- Install copies files to `~/.config/aiplus/profiles/<name>/`
- No private content embedded in public release binaries
- Docs use synthetic/redacted examples only

## Rollback Behavior

- `backup_profile_dir` already backs up entire profile directory before install
- Uninstall removes profile dir and secret-broker aliases
- No additional rollback mechanism needed for this scope
