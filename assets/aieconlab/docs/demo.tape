# VHS tape for AiEconLab 30-second tour. Generate with:
#   cd aieconlab && vhs docs/demo.tape
# Output: demo.gif (referenced from the main README).
#
# Mirrors aiplus-public/docs/demo.tape pattern: VHS gives a single
# declarative script that re-renders identically on any machine, so the
# README demo looks the same on author's M2 as on a CI runner that
# regenerates it later.

Output demo.gif

Set FontSize 14
Set Width 1000
Set Height 600
Set Theme "Builtin Solarized Dark"
Set Padding 16
Set PlaybackSpeed 1.0

Hide
Type "cd /tmp && rm -rf aieconlab-demo && mkdir aieconlab-demo && cd aieconlab-demo"
Enter
Type "git init -q && git config user.email demo@example && git config user.name demo"
Enter
Type "echo '# My paper project' > README.md && git add README.md && git commit -q --no-verify -m 'initial / 初始'"
Enter
Type "export AIPLUS_SKIP_VERSION_CHECK=1 AIPLUS_SKIP_SHELL_INIT=1"
Enter
Type "clear"
Enter
Show

# ---- 1. install AiPlus + AEL ---------------------------------------------
Type "# Step 1 — install AiPlus + AiEconLab into a fresh paper project"
Sleep 800ms
Enter
Type "aiplus install claude-code --yes && aiplus add aieconlab"
Sleep 400ms
Enter
Sleep 2800ms

# ---- 2. show the research team -------------------------------------------
Type "# Step 2 — your AI now has a research team: 8 core roles + 12 experts"
Sleep 700ms
Enter
Type "aiplus agent status | head -30"
Sleep 400ms
Enter
Sleep 2600ms

# ---- 3. route a real research task --------------------------------------
Type "# Step 3 — route a Stata task. PI scores it, picks RA-Stata"
Sleep 700ms
Enter
Type "aiplus agent route pi 'estimate the main IV spec with cluster-robust SEs'"
Sleep 400ms
Enter
Sleep 2600ms

# ---- 4. route a HEAVY research move that fires consultant ---------------
Type "# Step 4 — research-flavored task: PI consults plan-time experts first"
Sleep 700ms
Enter
Type "aiplus agent route pi 'draft referee response on the IV exogeneity criticism'"
Sleep 400ms
Enter
Sleep 2800ms

# ---- 5. show the per-role audit trail -----------------------------------
Type "# Step 5 — each role has its own workspace + transcript"
Sleep 700ms
Enter
Type "ls .aiplus/agents/ && aiplus agent transcript | tail -5"
Sleep 400ms
Enter
Sleep 2200ms

Type "# done. Stata RA's memory, IV critique, referee response — all role-isolated."
Sleep 1200ms
Enter
