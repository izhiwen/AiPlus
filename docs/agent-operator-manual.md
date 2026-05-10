# Agent Operator Manual

For agent operators running AiPlus on behalf of a project or team.

## Responsibilities

As an agent operator, you:

1. Install and maintain AiPlus CLI and project configuration
2. Manage compact timing and checkpoint hygiene
3. Monitor memory health and redaction compliance
4. Maintain profile bundles for team preferences
5. Ensure secret-broker aliases are correctly configured

## Daily Operations Checklist

### Start of session

```bash
aiplus doctor
aiplus refresh
```

### Mid-session checks

```bash
aiplus compact remind
aiplus memory status
```

### End of session or before compact

```bash
aiplus compact prepare
aiplus compact checkpoint
```

### After compact

```bash
aiplus compact resume
aiplus compact savings
```

## Maintenance

### Weekly

```bash
aiplus update all
aiplus profile doctor
aiplus memory doctor
aiplus compact validate
```

### On dependency changes

```bash
aiplus pricing update
aiplus self update --dry-run
```

### Profile management

```bash
# Install updated profile
aiplus profile install my-profile --user --source /path/to/source --dry-run
aiplus profile install my-profile --user --source /path/to/source --yes

# Verify
aiplus profile context my-profile
aiplus user context --profile my-profile
```

## Compact Operations

### When to compact

Run `aiplus compact remind` and check:

- `REMINDER_DECISION=remind_now` — good time
- `REMINDER_DECISION=wait` — handoff not current, update handoff first
- `REMINDER_DECISION=blocked` — safety gate active, resolve first

### What to save before compact

`aiplus compact prepare` creates the context capsule. `aiplus compact checkpoint` saves it. Always prepare before checkpoint.

### What to load after compact

`aiplus compact resume` loads the checkpoint and capsule. The agent should report resumption status.

## Memory Hygiene

### Reviewing memories

```bash
aiplus memory list
aiplus memory search "keyword"
aiplus memory context --runtime codex --budget 2000
```

### Cleaning up

```bash
aiplus memory stale
aiplus memory forget <id>
```

Stale records have `confidence=stale` or expired `stale_after` timestamps.

### Redaction compliance

All writes go through `reject_sensitive_memory_text`. If a write is blocked, the content contains a detected sensitive pattern. Do not attempt to bypass — remove the sensitive content.

## Safety Checklist

Before any session:

- [ ] `aiplus doctor` passes
- [ ] `secret_values=none` in all command output
- [ ] No raw transcript or provider payload stored
- [ ] No private profile content in public assets
- [ ] `global_agent_config_edits=none` confirmed

## Emergency Procedures

### Corrupted memory file

```bash
# Back up the file
cp .aiplus/memory/project-memory.jsonl .aiplus/memory/project-memory.jsonl.bak

# Re-initialize
aiplus memory init --project

# Re-add essential memories manually
aiplus memory add --scope project --kind project_fact --text "..."
```

### Corrupted compact state

```bash
# Re-initialize compact files
aiplus compact init

# Re-prepare
aiplus compact prepare
```

### Failed profile install

```bash
# Check backups
ls ~/.config/aiplus/profile-backups/

# Restore from backup if needed
cp -r ~/.config/aiplus/profile-backups/<name>-<timestamp>/ ~/.config/aiplus/profiles/<name>/
```
