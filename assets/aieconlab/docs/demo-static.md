# Static demo (fallback for the GIF)

Captured live from a clean tmp project against AEL HEAD on
2026-05-13. Use this as a fallback README hero when the animated
GIF isn't ready, or as a copy-paste reference for a screen recording.

The output below is **verbatim** from the actual CLI — no editing
beyond reordering the `agent list` output into canonical core-roles-
first order (the live output is unsorted across hashmap buckets).

---

````
$ mkdir my-paper && cd my-paper
$ git init -q -b main

$ aiplus install codex
AiPlus installed for Codex in this project.
Next: send "AiPlus 刷新", "刷新 AiPlus", "aiplus refresh", or
"aiplus status" to any already-open agent session.
INSTALL_STATUS=PASS

$ aiplus add aieconlab
AiPlus module added: aieconlab

$ aiplus agent list | head -10
All roles:
  - advisor (Advisor) [inactive]
  - pi (PI) [inactive]
  - theorist (Theorist) [inactive]
  - pm (Project Manager) [inactive]
  - ra-stata (RA-Stata) [inactive]
  - ra-python (RA-Python) [inactive]
  - referee (Referee) [inactive]
  - replicator (Replicator) [inactive]
  - llm-measurement (LLM-as-Measurement Specialist) [inactive]
  ...

$ aiplus agent route pi "kickoff the Treaty Ports paper"
Routing task to pi: kickoff the Treaty Ports paper
  Dispatch recorded: .aiplus/agents/dispatch-log.jsonl
  Creating worktree for pi...
Created worktree for pi at /tmp/my-paper.pi (Branch: agent/pi)
````

From zero to a real, persistable git worktree on `agent/pi` branch in
~30 seconds. The PI persona is now ready to be embodied by any host
runtime (Codex, Claude Code, OpenCode) via `aiplus agent talk pi`.

The two key affordances the demo shows:

1. **Module composition** — AiPlus's default install is SWE-tuned;
   `aiplus add aieconlab` makes it a research toolkit. No re-install,
   no rebuild, opt-in.
2. **Real artifacts** — the worktree on `agent/pi` is a regular git
   branch, not a prompt wrapper. The PI's work is real, persistable,
   conflict-trackable, and revertable through normal git.
