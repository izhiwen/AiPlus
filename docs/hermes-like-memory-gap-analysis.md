# Hermes-Like Memory Gap Analysis

## Purpose

Compare AiPlus Agent Memory against Hermes (Claude Code's built-in memory system) to identify gaps and inform v2.1+ roadmap. Hermes is the current state-of-the-art for agent memory in coding assistants.

## What Hermes Does

Hermes (Claude Code memory) provides:

1. **Automatic Memory Extraction**
   - Observes conversation in real-time
   - Extracts facts, preferences, decisions, and constraints without explicit commands
   - Uses LLM to summarize and categorize

2. **Implicit Context Injection**
   - Injects relevant memories into every conversation turn automatically
   - No explicit `memory context` command needed
   - Memories are part of the system prompt

3. **Natural Language Search**
   - Owner can ask "What did we decide about auth?"
   - Semantic search over memory corpus
   - Returns relevant records, not just keyword matches

4. **Preference Learning**
   - Learns coding style, conventions, and preferences over time
   - Adapts suggestions based on learned patterns
   - No manual `memory add` required

5. **Memory Lifecycle Management**
   - Automatically marks old memories as stale
   - Suggests updates when contradictions detected
   - Owner can review and approve changes

---

## AiPlus Current Capabilities

| Capability | Status | Notes |
|------------|--------|-------|
| Manual memory add | Yes | `aiplus memory add --text "..."` |
| Memory context output | Yes | `aiplus memory context` prints filtered records |
| Keyword search | Yes | `aiplus memory search --query "auth"` |
| Redaction safety | Yes | Mandatory `reject_sensitive_memory_text()` on write |
| Conflict detection | Yes | `detect_conflicts()` checks divergence and circular refs |
| Stale detection | Yes | `detect_stale()` based on confidence, stale_after, expires_at |
| Profile sync | Partial | Unidirectional profile → project, preferences only |
| Skill candidates | Yes | Pattern-based consolidation proposals |
| Session tracking | Yes | SQLite session index with FTS search |

---

## Gap Analysis

### Gap 1: Automatic Extraction (HIGH)

**Hermes:** Watches conversation, extracts memories automatically.

**AiPlus:** Requires explicit `memory add` or `memory propose`. No automatic extraction from conversation.

**Impact:** Owner must remember to manually capture important decisions and facts. Cognitive load is high.

**v2.1 Mitigation:**
- Add `memory auto-capture` command that analyzes session history and proposes records
- Use session index to identify "significant" sessions (many decisions, files changed, blockers)
- Propose records for Owner review, not automatic insertion

**NOT NOW (out of scope):**
- Real-time conversation monitoring (requires transcript access)
- LLM-based extraction (requires LLM integration, cost, complexity)

---

### Gap 2: Implicit Context Injection (HIGH)

**Hermes:** Memories are automatically injected into every conversation.

**AiPlus:** Requires explicit `aiplus memory context` to output memory. Agent must read and incorporate manually.

**Impact:** Agent may miss relevant context unless Owner explicitly runs the command.

**v2.1 Mitigation:**
- Document best practice: run `aiplus memory context` at session start
- Add `memory context` output to AGENTS.md managed block for automatic inclusion
- Create adapter-specific instructions (Codex/Claude/OpenCode) to remind agent to check memory

**NOT NOW:**
- Automatic injection into system prompt (requires agent integration)
- Runtime memory hook (requires daemon/launchd)

---

### Gap 3: Natural Language / Semantic Search (MEDIUM)

**Hermes:** Owner asks natural language questions; semantic search returns relevant results.

**AiPlus:** Keyword search only (`memory search --query`). No semantic understanding.

**Impact:** Search is brittle. "auth" won't find "authentication" or "login".

**v2.1 Mitigation:**
- Improve keyword search with stemming and synonyms
- Add `memory search --type` and `--scope` filters
- Consider lightweight embedding search (local, no cloud) — evaluate complexity

**NOT NOW:**
- Cloud vector DB
- Full semantic search requiring LLM embeddings

---

### Gap 4: Preference Learning (MEDIUM)

**Hermes:** Learns preferences implicitly from conversation patterns.

**AiPlus:** Preferences must be explicitly added via `memory add` or profile sync.

**Impact:** AiPlus does not adapt to Owner's style over time.

**v2.1 Mitigation:**
- Enhance profile sync to extract preferences from project memory
- Add `memory learn-preferences` command that analyzes active records for patterns
- Store learned preferences in profile memory

**NOT NOW:**
- Real-time preference adaptation
- Behavioral tracking

---

### Gap 5: Automatic Lifecycle Management (LOW)

**Hermes:** Automatically marks memories stale and suggests updates.

**AiPlus:** Stale detection exists but requires manual invocation (`memory stale`). No automatic suggestions.

**Impact:** Memory corpus may accumulate stale records unless Owner actively manages it.

**v2.1 Mitigation:**
- Add stale detection to `memory doctor`
- Add `--suggest-updates` flag to `memory conflicts`
- Consider periodic watch mode for memory health (similar to compact watch)

**NOT NOW:**
- Automatic deletion or modification of records

---

## AiPlus Advantages Over Hermes

| Advantage | Explanation |
|-----------|-------------|
| **Owner Control** | Every memory is explicitly added, reviewed, and approved. No surprises. |
| **Redaction Safety** | Mandatory secret detection prevents accidental persistence of sensitive values. |
| **Project-Local** | No cloud dependency. Works offline. Data stays in repo. |
| **Structured** | Memory records have schema (type, scope, confidence, status, evidence). Not just free text. |
| **Audit Trail** | Every add/forget/accept/reject is logged in audit.jsonl. |
| **Cross-Agent** | Works with Codex, Claude Code, OpenCode. Not tied to one agent. |
| **Owner Gates** | Compact workflow includes explicit approval gates. Hermes has no equivalent. |

---

## v2.1 Roadmap: Closing Gaps Safely

| Priority | Gap | Approach | Risk | Effort |
|----------|-----|----------|------|--------|
| P1 | Automatic extraction | Session-based auto-capture with Owner review | Low | Medium |
| P1 | Implicit injection | AGENTS.md integration + adapter docs | Low | Low |
| P2 | Semantic search | Enhanced keyword + local embeddings eval | Low-Med | Medium |
| P2 | Preference learning | Profile sync enhancement + pattern analysis | Low | Medium |
| P3 | Lifecycle management | Doctor integration + stale suggestions | Low | Low |

## Explicitly Rejected for v2.1

- **Real-time conversation monitoring:** Requires transcript access, privacy risk, complexity
- **Cloud vector DB:** Violates local-first principle
- **LLM-based memory extraction:** Cost, latency, non-determinism
- **Automatic memory modification:** Trust risk — Owner must approve all changes
- **Behavioral tracking / telemetry:** Privacy violation

---

## Conclusion

AiPlus Agent Memory trades **automation** for **control and safety**. It will never match Hermes' seamless implicit behavior, and that is by design. The v2.1 goal is to reduce the manual overhead (auto-capture, better search, preference learning) while maintaining the Owner-controlled, secret-safe, local-first approach.

**Key principle:** Every memory addition is an Owner decision. AiPlus can suggest, but never insert without approval.
