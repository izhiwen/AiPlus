# AiPlus user interview scripts (30 minutes each)

Three scripts for three audiences. Same overarching shape:

1. **Warm-up (3 min)** — who they are, what they build, what AI tools
   they use today
2. **Pain-point inventory (10 min)** — open-ended, no pitching, let
   them name pains in their own words
3. **AiPlus demo (10 min)** — let them try `aiplus install codex` +
   one real task in their own repo. Don't lead, watch where they
   stall
4. **Reaction + objection (5 min)** — what worked / what didn't /
   what would they pay for / what's the dealbreaker
5. **Logistics (2 min)** — would they try v0.2, can you contact them
   again, any peers worth talking to

Recording: ask permission up front. Take notes on a paper / iPad even
if recording — paper note signal trumps "interesting" highlights
because you can only physically write things that actually surprised
you.

---

## Script 1 — Peer PhD student in economics (research user)

**Best opener** (warm + casual, no AiPlus mention):
> "I'm trying to figure out how other applied-econ people manage the
> tooling around a paper. Mind if I record 30 minutes? I want to hear
> what's annoying you about your current workflow before I describe
> what I built — otherwise I'll just confirm my own bias."

### Warm-up (3 min)

- What paper are you working on right now?
- Are you co-authored or solo? RA-supported?
- What does a typical Wednesday afternoon of paper-work look like?
- Do you use AI tools at all? (Codex, ChatGPT, Claude, Copilot, …)
  *If yes:* which one, for what?

### Pain-point inventory (10 min — DO NOT PITCH)

Read these questions slowly. Let them answer fully. If they ask
"is this what your tool solves?" say "tell me first, I'll show you
after."

- When you came back to your paper this Monday, what did you have to
  re-learn that you'd known on Friday?
- Last time you swapped between writing the intro and re-running a
  robustness check — what was the friction?
- Last time you broke a result by accident (changed an estimator,
  dropped a robustness column, etc.), how did you find out? How long
  did it take?
- When your co-author asks for a status update, where do you look to
  remember what you'd promised them?
- When you use an AI tool, what does it ask you for that you wish it
  already knew?
- When you do NOT use the AI tool for a task, why not?
- If your laptop disappeared tonight, what would be hardest to
  reproduce next week?

### AiPlus + AEL demo (10 min)

Hand over your laptop OR have them install on theirs:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/AiPlus/main/install.sh | bash
cd ~/some-paper-repo   # their actual repo
aiplus install claude-code
aiplus add aieconlab
```

Then watch them try **one real thing they'd actually do today**.
Examples to suggest if they freeze:

- "Ask the PI agent to plan the next 2 weeks"
- "Have the Replicator check whether table 3 still reproduces"
- "Have the Writer draft the next paragraph of the intro"

Note: where they pause, where they ask "what does this command do",
where they ignore the consult output, where the persona response
matches/mismatches their expectation.

### Reaction + objection (5 min)

- What was the first thing that felt weird?
- What surprised you (good or bad)?
- If we removed exactly one feature, which one and why?
- If we added exactly one feature, which one and why?
- Would you keep this installed for a week? What would have to be
  true for you to invite a co-author into it?
- What's the deal-breaker — what makes you uninstall in week 2?

### Logistics (2 min)

- Mind if I check back in 2 weeks?
- Anyone else worth talking to?

---

## Script 2 — Working software engineer (Codex / Claude Code / Cursor user)

**Best opener** (technical, peer-to-peer):
> "I built a Rust toolchain on top of AI coding agents and I'm
> trying to get a brutal 30-minute review from someone who actually
> uses agents full-time. Bring your worst skepticism."

### Warm-up (3 min)

- What stack are you in this week? (Languages, runtime)
- Which agents do you use? (Codex, Claude Code, Cursor, Aider, OpenCode)
- Solo or team? If team — do other people use agents too?
- Self-employed / startup / FAANG / academic?

### Pain-point inventory (10 min)

- Show me what happened the last time your agent compacted in the
  middle of a real task. (Their actual screen / scrollback if
  possible — gold.)
- How do you keep your agent honest about time? Do you track
  estimates vs actuals at all?
- When you run multiple agents in the same repo, how do they not
  step on each other?
- What's the last thing you re-taught an agent that it had been told
  before?
- If you had to explain your project conventions to a new agent in
  one paragraph, what's the paragraph?
- What's a thing you've explicitly NOT given the agent access to,
  and why?

### AiPlus demo (10 min)

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/AiPlus/main/install.sh | bash
cd ~/some-real-project
aiplus install claude-code  # or their preferred runtime
```

Have them try:

1. `aiplus memory add` something they'd actually want the agent to
   remember
2. `aiplus agent route engineer-a "<a real task they'd dispatch>"`
3. Trigger a fake STOP-gate: `aiplus agent route engineer-a "release the foo pipeline"` → watch them react to the dispatch refusal
4. `aiplus secret-broker list` → see if they go "wait, that's all
   31 of my keys?"

Note where they react with "oh that's clever" vs "that's overkill"
vs "I already do this with X."

### Reaction + objection (5 min)

- Where did this feel like over-engineering?
- Where did it feel under-built?
- The hardest objection: would you keep `aiplus install` in a
  greenfield project? Why or why not?
- What would make this a no-brainer install?
- Stack comparisons: does Copilot Workspace / Cursor / Continue
  already do parts of this?

### Logistics (2 min)

- Open to a follow-up after v0.2?
- Who on your team should I talk to next?

---

## Script 3 — Curious non-technical or adjacent-technical user

This person uses ChatGPT for writing, maybe a bit of Cursor, doesn't
ship code. Most useful for: "is the install friction tolerable for a
non-engineer."

**Best opener**:
> "I built a tool for managing AI assistants. I have no idea if it's
> usable by anyone other than me. Could I have 30 minutes to watch
> you try it and tell me where it breaks?"

### Warm-up (3 min)

- What do you use ChatGPT / Claude / etc. for in a typical week?
- Do you ever use Cursor / Claude Code / a coding-flavored tool?
- Have you ever written a bash command? (No judgment — calibration.)

### Pain-point inventory (10 min — even shorter)

- Last time the AI assistant lost context — what happened?
- Last time you wished it remembered something across sessions?
- Last time you didn't trust what it produced — what was the tell?

### Demo (10 min — YOU DRIVE, they observe + react)

Don't make them type. Open a terminal, install AiPlus, run two
commands, ask them to read the output aloud. Watch where they get
lost or confused.

- Did the install script feel safe or scary?
- Did the team-of-agents framing make sense or confuse?
- When you saw the dispatch refused (STOP-gate), did that feel
  paternalistic or protective?

### Reaction (5 min)

- If a friend who's a researcher / engineer told you "use this," would
  you?
- What did you NOT understand?
- What would have to be different for you to install it on your own
  machine?

### Logistics (2 min)

- Mind if I follow up if v0.2 is more user-friendly?

---

## Cross-script: what to LOOK for

After all 3 interviews, sit with notes and look for:

1. **One pain that recurred in ≥2 of 3** → strong v0.2 signal
2. **One feature that ≥2 of 3 ignored** → maybe doesn't earn its keep
3. **One install-friction moment in ≥2 of 3** → friction-removal v0.2
4. **One sentence one of them used that you didn't have words for** →
   that's your README copy
5. **One person who said "I'd pay for this"** → that's the user
   profile to optimize for in v0.2

## What NOT to do

- Don't argue. If they call something dumb, write down "they called
  it dumb." Don't explain why it's not dumb. The point isn't to
  convince them; it's to learn.
- Don't demo features. Let them ask. Most demos are bait that
  prevents you from hearing real reactions.
- Don't take silence for agreement. Ask "what was that pause about?"
- Don't show them this script. They should not know what you're
  measuring.

## Logistics math

- 3 interviews × 30 min = 90 min
- Transcription + notes: 30 min per interview = 90 min
- Synthesis: 60 min
- **Total: ~4 hours**, gets you better-than-AB-test v0.2 priorities
