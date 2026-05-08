# Archived Node Reference

Archived reference implementation: legacy Node `aiplus-cli` v0.1.3, retained
outside this public source package for behavior audits.

The Rust CLI preserves user-facing command markers and local project workflows:

- `INSTALL_STATUS=PASS`
- `INSTALL_DRY_RUN=PASS`
- `UPDATE_STATUS=PASS`
- `ADD_STATUS=PASS`
- `ADD_DRY_RUN=PASS`
- `STATUS=PASS`
- `DOCTOR_STATUS=PASS|NEEDS_FIX`
- `UNINSTALL_DRY_RUN=PASS`
- `UNINSTALL_STATUS=PASS`
- `MODULE_NOT_AVAILABLE`
- `GLOBAL_CONFIG_UNTOUCHED`
- `AIPLUS_REFRESH_PROMPT=刷新`

## Current Parity Notes

Exact byte-for-byte output parity is not required. The Rust CLI keeps the same
summary-first UX and stable status markers.

Node remains a historical reference for behavior audits and emergency reference
fixes. It is not the active mainline. Compact commands are Rust-native and do
not invoke Node at runtime.

## Known Difference

Rust rejects dangling symlinks in target write paths. A safety review found that
the Node reference can miss some dangling symlink cases. Rust keeps the stricter
behavior.
