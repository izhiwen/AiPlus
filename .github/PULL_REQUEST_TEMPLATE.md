<!-- Thanks for opening a PR. Please fill in the sections below. -->

## What changed

<!-- 1-3 sentences. Reference the issue if any. Closes: #N -->

## Why

<!-- The reason. What pain does this fix? -->

## Which AiPlus layer

- [ ] aiplus CLI (top-level commands)
- [ ] agent-memory
- [ ] compact-reminder
- [ ] auto-team-consultant
- [ ] agent-team
- [ ] aieconlab assets snapshot (canonical source lives in [AiEconLab repo](https://github.com/izhiwen/AiEconLab))
- [ ] runtime adapter (codex / claude-code / opencode)
- [ ] documentation
- [ ] release pipeline / CI

## Checks

- [ ] `cargo build --release --bin aiplus` succeeds
- [ ] `cargo fmt --all -- --check` clean
- [ ] `cargo clippy --workspace -- -D warnings` clean
- [ ] `cargo test --workspace` passes
- [ ] If this changes user-facing CLI surface, runtime adapter
      coverage is consistent across codex/claude-code/opencode
- [ ] No secret values, API keys, or credentials introduced
- [ ] No global agent configuration writes introduced
- [ ] If this changes release pipeline, manually triggered the
      Release workflow on a test tag to confirm
