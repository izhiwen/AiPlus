# AiEconLab Terminology

Quick reference for how to refer to AiEconLab in different contexts.
Internal documentation, READMEs, code, commit messages, and external
outreach should all follow these rules.

## The three forms

| Form | Used in | Example |
|---|---|---|
| **AiEconLab** | Brand / display name. Headers, prose, marketing, READMEs, talks, social posts. The canonical user-facing way to refer to the project. | "AiEconLab installs a permanent virtual team..." |
| **AEL** | Abbreviation. Acceptable after first mention introduces the full form. Tables, headings, casual reference, internal docs. | "5 AEL expert seats" |
| `aieconlab` | CLI slug / module identifier / file path. Shell commands, TOML keys, `aiplus add <name>`, internal logging, `.aiplus/` directory names. Always lowercase, no separators. | `aiplus add aieconlab` |

## When to use which

**Headers and headlines**: `AiEconLab`
- ✓ `# AiEconLab Design`
- ✓ "AiEconLab — a permanent virtual research team for applied economists"

**First mention in a document**: `AiEconLab (AEL)`
- ✓ "AiEconLab (AEL) is the sibling of `aiplus-agent-team` for research."
- After this anchor, `AEL` alone is fine in the same document.

**Body text, second mention onwards**: `AEL` or `AiEconLab`, pick one and be consistent within the document
- ✓ "AEL's consultant team replaces..."
- ✓ "AiEconLab's consultant team replaces..."
- ✗ Mixing both in adjacent sentences without reason

**Shell commands and CLI literals**: `aieconlab`
- ✓ `aiplus add aieconlab`
- ✓ `aiplus agent route llm-measurement`
- ✗ `aiplus add AiEconLab` (CLI is case-insensitive, but lowercase is canonical)
- ✗ `aiplus add aiplus-econ-agent-team` (this is a legacy alias; do not use)

**File paths and TOML keys**: `aieconlab`
- ✓ `.aiplus/aieconlab/acceptance/v0.1.0/schema.yaml`
- ✓ `core/templates/consultant-team.aieconlab.toml`
- ✓ `assets/aieconlab/` (in aiplus-public)

**GitHub repo URL**: `AiEconLab`
- ✓ `https://github.com/izhiwen/AiEconLab`
- The lowercase URL `https://github.com/izhiwen/aieconlab` redirects to
  the canonical case, but linking to the canonical form directly avoids
  the redirect hop.

**Git commit messages / git branch names**: `aieconlab` for the slug,
or `AiEconLab` / `AEL` in prose. Both are acceptable in messages.
- ✓ `feat(aieconlab): add LLM-Measurement Specialist`
- ✓ `Reframe AEL consultant team / 重新框定 AEL consultant 团队`

## What NOT to use

- ❌ `econ-agent-team` — legacy slug from before the rename
- ❌ `aiplus-econ-agent-team` — legacy URL form, redirects but obsolete
- ❌ `AI Econ Lab` (with spaces) — not the canonical brand
- ❌ `EconLab` — drops the "Ai" prefix, ambiguous
- ❌ `aieconlab` in prose where `AiEconLab` would be appropriate
  (it's the slug, not the brand)

## Domain-specific notes

**AEL personas**: roles are lowercase-hyphenated slugs internally
(`ra-stata`, `ra-python`, `llm-measurement`), but referred to by their
display name in prose:

| Slug | Display name |
|---|---|
| `advisor` | Advisor |
| `pi` | PI |
| `theorist` | Theorist |
| `pm` | Project Manager |
| `ra-stata` | RA-Stata |
| `ra-python` | RA-Python |
| `referee` | Referee |
| `replicator` | Replicator |
| `llm-measurement` | LLM-as-Measurement Specialist |

**Consultant seats**: the AEL consultant team has 5 expert seats. They
are referred to by their `name =` field in the TOML, e.g., "Design
Credibility", "Contribution Framing", "Day-1 Reproducibility", "IRB /
Disclosure Gate", "LLM-as-Measurement Specialist".

The `id =` field on each seat is internal (used by `aiplus doctor` checks
and the consultant engine's routing). One of them is `id = "ai_integration"`
for schema compatibility — but its name remains "LLM-as-Measurement
Specialist". Do not confuse the schema-compat id with the display name.

## Examples to copy

**Tweet / Bluesky post**:
> AiEconLab (AEL) is a permanent virtual research team for applied
> economists. 8 core roles + 12 experts including an LLM-as-Measurement
> Specialist. Built on AiPlus.

**Academic acknowledgement**:
> "This work used AiEconLab (AEL), the open-source applied-economics
> research-agent toolkit [github.com/izhiwen/AiEconLab]."

**Install instruction**:
> "Install with `aiplus add aieconlab` (requires AiPlus ≥ 0.5.5)."

**Code comment**:
> `// AEL's consultant team replaces the SWE default config at install time.`

**Headline**:
> "Introducing AiEconLab: AI agents that actually understand what a
> referee is going to ask."
