# Migration from aiplus-auto-compact

## What Changed

The bundled module previously named `aiplus-auto-compact` has been renamed to `aiplus-compact-reminder`. This change reflects the module's core purpose: proactive compact reminders and structured handoffs, not just the compact action itself.

## Old → New Mapping

| Old | New |
|-----|-----|
| `aiplus-auto-compact` | `aiplus-compact-reminder` |
| Module slug `auto-compact` | Module slug `compact-reminder` |
| `assets/aiplus-auto-compact/` | `assets/aiplus-compact-reminder/` |

## Recovery Steps

If you have an existing install with the old module name:

1. **Update**: Run `aiplus update` or `aiplus install` in your project. The new module will be installed automatically.

2. **Verify**: Run `aiplus doctor` to confirm the new module is recognized and healthy.

3. **Clean up** (optional): Remove the old module directory:
   ```bash
   rm -rf ~/path/to/project/.aiplus/modules/aiplus-auto-compact
   ```

## Backward Compatibility

- The CLI subcommand `aiplus compact` is **unchanged** for muscle memory continuity.
- Existing manifests with the old slug `"auto-compact"` remain deserializable via serde alias.
- No action is required unless you want to clean up old directories.
