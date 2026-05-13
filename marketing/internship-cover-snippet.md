# Internship cover-letter snippet

Drop-in paragraph (100–150 words) — paste into the body of a cover letter,
replace the `[COMPANY-SPECIFIC SENTENCE]` placeholder with one sentence
that ties to the specific role.

---

I've been pair-programming full-time with AI coding agents (Codex, Claude
Code, OpenCode) for the better part of a year, and to keep that workflow
from leaking hours I built AiPlus — five small Rust modules that treat
the six recurring failure modes of multi-agent coding (cross-session
amnesia, post-compact context loss, role pollution inside a single agent,
human-hour time estimates that don't map, agents stepping on each other,
plans that miss security and UX) as coordination problems rather than
prompt-engineering problems. One design decision I'm particularly careful
about: the auditor agent reads a different context root and runs from a
separate git worktree from the builder, because an auditor that shares
context is just self-attestation in a costume. The honest meta-frame is
that I used AI agents to build the toolchain that manages AI agents.
[COMPANY-SPECIFIC SENTENCE]

---

**Usage notes:**

- The `[COMPANY-SPECIFIC SENTENCE]` placeholder should be one sentence
  that links AiPlus to the company's product, research direction, or
  open role. Examples:
  - For a model lab: "I'd like to bring that same outer-loop discipline
    to your agent-evaluation work."
  - For a developer-tools company: "I'd like to work on the part of
    the stack where agents have to coordinate, not just complete."
  - For a research role: "I'd like to do that work alongside people who
    are publishing on multi-agent failure modes."
- Keep the paragraph as one block — splitting it shortens the depth
  signal.
- Don't link to AiPlus inline in the cover letter; let the recruiter
  find it on the resume / GitHub link.
