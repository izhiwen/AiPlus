# Migrating from `aiplus-auto-compact`

The `aiplus-auto-compact` module has been merged into the core `aiplus` CLI. The standalone module is no longer needed because compact functionality is now built-in and maintained as a first-class subcommand. This simplifies installation, eliminates version skew between the CLI and the compact module, and ensures that all users receive compact improvements automatically with every CLI update.

## What Changed

| Old | New |
|-----|-----|
| `aiplus-auto-compact` installed as a separate module under `.aiplus/modules/aiplus-auto-compact/` | `aiplus compact` built into the core CLI |
| `aiplus auto-compact` (legacy alias) | `aiplus compact` (unchanged, now native) |
| Separate module updates via `aiplus update --module aiplus-auto-compact` | Updates automatically with `aiplus update` |
| Module manifest entry `module = "aiplus-auto-compact"` | Manifest entry `module = "aiplus-compact"` (serde alias preserves old manifests) |

## Recovery for Existing Installations

If you previously installed the standalone `aiplus-auto-compact` module, follow these steps to migrate:

1. **Run the built-in install or update command**

   ```bash
   aiplus install
   # or
   aiplus update
   ```

   This will detect the legacy module and migrate your configuration automatically.

2. **Verify the migration succeeded**

   ```bash
   aiplus doctor
   ```

   Look for the compact section in the output. It should report no errors and confirm that `aiplus compact` is available.

3. **Optional: remove the old module directory**

   After confirming everything works, you may delete the legacy module files:

   ```bash
   rm -rf ~/path/to/project/.aiplus/modules/aiplus-auto-compact/
   ```

## Important Notes

- The CLI subcommand `aiplus compact` is **unchanged**. Your existing workflows, scripts, and muscle memory continue to work exactly as before.
- A serde alias preserves backward compatibility for existing manifests. If your project still contains `module = "aiplus-auto-compact"`, it will deserialize correctly and map to the built-in compact handler.
- No manual manifest editing is required.
