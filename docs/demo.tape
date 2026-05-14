# VHS tape for AiPlus 30-second tour. Generate with:
#   cd aiplus-public && vhs docs/demo.tape
# Output: docs/demo.gif (referenced from the main README).
#
# Why VHS instead of asciinema-to-gif: VHS gives us a single
# declarative script that re-renders identically across machines,
# which matters because the demo will land on the public README and
# we want the same look on Steve's M2 as on the CI box that
# regenerates it later.

Output docs/demo.gif

# Conservative size + theme so the gif renders sharp on github.com
# without being a 5MB asset.
Set FontSize 14
Set Width 1000
Set Height 600
Set Theme "Builtin Solarized Dark"
Set Padding 16
Set PlaybackSpeed 1.0

Hide
Type "cd /tmp && rm -rf demo && mkdir demo && cd demo"
Enter
Type "git init -q && git config user.email demo@example && git config user.name demo"
Enter
Type "echo '# Demo project' > README.md && git add README.md && git commit -q --no-verify -m 'initial / 初始'"
Enter
Type "clear"
Enter
Show

# ---- 1. install ----------------------------------------------------------
Type "# Step 1 — install AiPlus into a fresh project"
Sleep 800ms
Enter
Type "aiplus install codex"
Sleep 400ms
Enter
Sleep 2200ms

# ---- 2. add the consultant team + AEL ----------------------------------
Type "# Step 2 — add the consultant team + AiEconLab"
Sleep 600ms
Enter
Type "aiplus add auto-team-consultant && aiplus add aieconlab"
Sleep 400ms
Enter
Sleep 2400ms

# ---- 3. route an in-scope task ------------------------------------------
Type "# Step 3 — route a task. Consult engine fires, writes JSONL findings"
Sleep 700ms
Enter
Type "aiplus agent route pi 'rewrite the LLM tool use context pipeline'"
Sleep 400ms
Enter
Sleep 2400ms

# ---- 4. route a task that crosses a STOP-gate ---------------------------
Type "# Step 4 — same engine refuses a task that crosses an owner gate"
Sleep 700ms
Enter
Type "aiplus agent route pi 'release the LLM pipeline' || echo '  ^^ dispatch refused (exit non-zero)'"
Sleep 400ms
Enter
Sleep 2600ms

# ---- 5. show the JSONL audit trail --------------------------------------
Type "# Step 5 — gate ledger + consult artifact land under .aiplus/agent-memory/_team/"
Sleep 800ms
Enter
Type "ls .aiplus/agent-memory/_team/"
Sleep 400ms
Enter
Sleep 1800ms

Type "# done. doctor, velocity, dispatch log all stay project-local."
Sleep 1200ms
Enter
