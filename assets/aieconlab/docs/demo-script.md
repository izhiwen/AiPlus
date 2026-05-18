# 30-Second Demo Recording Script

Drop-in script for recording the AiPlus + AiEconLab demo GIF that lives
at the top of both READMEs. Designed for 30 seconds total — anything
longer than 45 seconds loses viewers.

## Tools

Recommended recording stack on macOS:
- **Terminal**: iTerm2 with a clean profile (no powerline noise, no
  oh-my-zsh git status spam, ASCII-only prompt)
- **Recording**: `asciinema rec` for terminal-pure cast, or
  [terminalizer](https://github.com/faressoft/terminalizer) for
  styled GIF output
- **Conversion**: `agg` (asciinema → GIF) for SVG-style polished output,
  or `terminalizer render` for theme'd GIF

Alternatively: `Cleanshot X` or `Kap.app` for native screen recording,
plus FFmpeg to compress to GIF.

## Pre-recording checklist

- Terminal width: 100 cols. Anything wider gets cropped in markdown
  rendering on GitHub.
- Font size: 18pt minimum (people watch GIFs on phones too)
- Prompt: short and ASCII-only. Recommend:
  ```bash
  PS1='\W $ '
  ```
- Theme: high contrast. Solarized Dark or Tokyo Night work well.
- No notifications, no system audio. Quit Slack / Discord / mail clients.
- Test on a *fresh* tmp directory so no irrelevant files appear:
  ```bash
  mkdir -p /tmp/demo && cd /tmp/demo && rm -rf .git .aiplus
  ```

## The script (~30 seconds)

Type at a steady, slightly-faster-than-natural pace. Hit Enter without
hesitation. Pause 1 second after each command's output completes before
the next command. **Do not edit out pauses in post** — let the
real timing show. That's what makes it credible.

### Beat 1 (0-5 sec) — Setup

```bash
$ mkdir my-paper && cd my-paper
$ git init -q -b main
```

(no output, just the prompt advancing)

### Beat 2 (5-12 sec) — Install AiPlus

```bash
$ aiplus install codex
AiPlus installed for Codex in this project.
Next: ...
INSTALL_STATUS=PASS
```

(Pause 1 second on `INSTALL_STATUS=PASS` — that's the "ok this is real" moment)

### Beat 3 (12-18 sec) — Add the econ team

```bash
$ aiplus add aieconlab
AiPlus module added: aieconlab
```

### Beat 4 (18-25 sec) — Show the roster

```bash
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
```

(viewer's eye lands on the role names — that's the payoff)

### Beat 5 (25-30 sec) — Route a task

```bash
$ aiplus agent route pi "kickoff the Treaty Ports paper"
Routing task to pi: kickoff the Treaty Ports paper
Created worktree for pi at /tmp/my-paper.pi (Branch: agent/pi)
```

(end with the worktree creation — the moment a viewer realizes "oh,
this isn't just a prompt wrapper, it actually creates real artifacts")

## Stop here. 30 seconds.

Total: ~30 seconds. Don't show `aiplus agent talk pi`, don't show the
full doctor output, don't show CI. The point is the first 30 seconds
of a user's first experience, not the full feature list.

## Variations for context

**For the AiPlus README**: keep the AEL-specific beats (4-5) but
substitute the SWE team:

```bash
$ aiplus install codex      # default install includes agent-team (SWE)
$ aiplus agent list | head -8
  - advisor / ceo / architect / pm / engineer-a / engineer-b / reviewer / qa
$ aiplus agent route ceo "design the new billing endpoint"
Created worktree for ceo at /tmp/my-project.ceo
```

**For Twitter / Bluesky**: 15-second hyper-condensed version:

```bash
$ aiplus install codex && aiplus add aieconlab
$ aiplus agent route pi "kickoff Treaty Ports paper"
Created worktree for pi at /tmp/my-paper.pi (Branch: agent/pi)
```

## After recording

- Compress GIF to <2MB. GitHub README embeds slowly above 2MB.
- Filename: `demo.gif`
- Same path: `demo.gif` in AEL repo
- README Demo section: `![AiEconLab demo](demo.gif)`

## Caption ideas

- "From zero to a real worktree in 30 seconds"
- "AiPlus + AiEconLab: install, add, route a task. That's it."
- "What `aiplus install codex && aiplus add aieconlab` actually does"

## Common mistakes to avoid

- ❌ Typing too fast — looks fake. Real users type at human speed.
- ❌ Editing out the pauses between commands. Let the install/doctor
  output actually appear.
- ❌ Using a flashy color theme that distracts from the text.
- ❌ Including `cd`, `ls`, or `clear` — wastes precious seconds.
- ❌ Recording in a directory with prior `.aiplus/` state — viewer can't
  tell what's old vs new.
- ❌ Including your real name / hostname in the prompt. Strip with
  `PROMPT_COMMAND= PS1='\W $ '`.

## Asciinema cast → GIF pipeline (recommended)

```bash
# Record
asciinema rec demo.cast

# Trim if needed
asciinema cat demo.cast > demo.txt   # inspect timing
# manually edit demo.cast JSONL to trim

# Convert to GIF
agg --speed 1.2 --font-size 18 demo.cast demo.gif

# Verify size
ls -lh demo.gif   # target <2MB
```

Speed 1.2x is the sweet spot — feels natural, fits more content.
