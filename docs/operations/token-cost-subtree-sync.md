# aiplus-token-cost subtree sync

**Status**: NORMATIVE for future updates to `crates/aiplus-token-cost/`.

---

## Source of truth

`crates/aiplus-token-cost/` is a **mirror** of [izhiwen/AiPlus-Token-Cost](https://github.com/izhiwen/AiPlus-Token-Cost). All bug fixes, features, and version bumps land **in the standalone repo first**, then sync to aiplus-public.

Why: shipped as both standalone (`aiplus-token-cost` binary) and bundled (via `aiplus install`); having two divergent sources would create drift.

## How to sync

When `izhiwen/AiPlus-Token-Cost` ships a new version (e.g., v0.1.1, v0.2.0):

### Option 1 — git subtree pull (recommended)

```bash
cd ~/Projects/AiPlus/aiplus-public
git subtree pull --prefix=crates/aiplus-token-cost \
  https://github.com/izhiwen/AiPlus-Token-Cost.git v0.1.1 \
  --squash -m "chore(token-cost): subtree sync to v0.1.1"
cargo build --workspace                # verify it still compiles
cargo test --workspace                 # verify tests still pass
git push origin main
```

Note: this only works cleanly if `crates/aiplus-token-cost/` was originally added via `git subtree add`. If it wasn't (initial v0.1.0 was copied), see Option 2 first for one-time setup.

### Option 2 — manual sync (until subtree history is established)

```bash
cd ~/Projects/AiPlus/aiplus-public
TARGET="crates/aiplus-token-cost"
TAG="v0.1.1"

# Clean target, then copy the tagged version
rm -rf "$TARGET"
git clone --depth 1 --branch "$TAG" https://github.com/izhiwen/AiPlus-Token-Cost.git /tmp/aiplus-token-cost-sync
mkdir -p "$TARGET/src" "$TARGET/tests"
cp /tmp/aiplus-token-cost-sync/Cargo.toml "$TARGET/"
cp /tmp/aiplus-token-cost-sync/src/{lib.rs,pricing.rs,rollup.rs,snapshot.rs,embedded.rs,error.rs} "$TARGET/src/"
cp /tmp/aiplus-token-cost-sync/tests/* "$TARGET/tests/"
cp /tmp/aiplus-token-cost-sync/README.md "$TARGET/"
rm -rf /tmp/aiplus-token-cost-sync

# IMPORTANT: don't copy main.rs — the workspace member is library-only.
# The standalone binary lives only in the standalone repo.

# IMPORTANT: adjust Cargo.toml to use workspace-relative dependencies
# (remove [[bin]] section, change explicit deps to .workspace = true).

cargo build --workspace
cargo test --workspace
git add "$TARGET"
git commit -m "chore(token-cost): manual sync to $TAG"
git push origin main
```

After the first successful manual sync, switch to Option 1 (subtree pull) for subsequent updates.

## Cargo.toml differences

The standalone repo's `Cargo.toml` has:
- `[lib]` + `[[bin]]` (binary entry for standalone CLI)
- Explicit version-pinned dependencies (no workspace)
- `[profile.release]` settings

The aiplus-public mirror's `Cargo.toml` should have:
- `[lib]` only (no `[[bin]]` — the bin is the standalone-repo concern)
- `workspace = true` for `anyhow`, `serde`, `serde_json`, `toml`
- Explicit version for `chrono` (not in workspace deps)
- No `[profile.release]` (inherited from workspace)

When syncing, the `Cargo.toml` always needs manual adjustment. Diff carefully.

## Anti-patterns

- ❌ Editing `crates/aiplus-token-cost/` in aiplus-public directly — your fix won't propagate to standalone users.
- ❌ Skipping the standalone-repo PR review — both repos should converge on the same tagged version.
- ❌ Subtree-pulling a `main` branch instead of a tag — drifts toward unreleased state.

## Verification post-sync

Run after any sync:

```bash
cd ~/Projects/AiPlus/aiplus-public
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
./target/debug/aiplus agent token-cost --help    # subcommand still works
```

## Bumping bundled version reference

If AiPlus-Token-Cost releases v0.2.0 with a binary-format change that aiplus-public's `install.sh` / `install.ps1` should download:

1. Subtree-sync as above.
2. Update `assets/aiplus-token-cost/aiplus-module.json` version field.
3. Update `assets/aiplus-token-cost/CHANGELOG.md` to mirror standalone CHANGELOG.
4. Update aiplus-public root `CHANGELOG.md` `## Unreleased` section.
5. The dual-binary release workflow (`.github/workflows/release.yml`) downloads "latest" by default — it picks up the new standalone version automatically on next aiplus-public release.

---

— Advisor, 2026-05-19, Phase C of G-AT-TOKEN-COST-STANDALONE-1.
