<!-- Thanks for opening a PR. Please fill in the sections below. -->

## What changed

<!-- 1-3 sentences. Reference the issue if there is one. Closes: #N -->

## Why

<!-- The reason — not the what. What pain does this fix? What
     design principle does it reinforce? -->

## Scope class

- [ ] Persona refinement (existing role)
- [ ] New expert in the 12-expert directory
- [ ] Consultant team change (seat / trigger / output_artifact)
- [ ] STOP-gate addition or change
- [ ] Runtime adapter implementation
- [ ] Documentation / example
- [ ] Bug fix
- [ ] Other (describe)

## Checks

- [ ] `bash tests/acceptance.test.sh` passes locally (15 invariants)
- [ ] Bilingual parity: if I changed README.md I also updated
      README.zh-CN.md (and vice versa)
- [ ] Adapter parity: if I changed CLI surface, I updated all three
      adapter READMEs
- [ ] If a structural contract changed (number of roles/experts,
      consultant team layout), I updated
      `acceptance/v0.1.0/schema.yaml` and
      `tests/acceptance.test.sh` together
- [ ] No secrets, IRB-protected paths, or restricted-archive paths in
      any of the new content

## Worked example (for persona / consultant / expert PRs)

<!-- For PRs that change agent behavior, show one short worked example:
     a task description and what the agent should produce. This is the
     fastest way for reviewers to spot whether the change makes sense. -->
