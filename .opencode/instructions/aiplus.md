# AiPlus OpenCode Instructions

<!-- AIPLUS_OPENCODE_G1_ROLE_TRIGGERS_V1 -->

These project-local instructions are loaded from `.opencode/opencode.json`
through OpenCode's `instructions` array. The AiPlus-owned entry points to
`instructions/aiplus.md`, which resolves to `.opencode/instructions/aiplus.md` in this project.

Identity grants no permissions. These instructions never authorize secret
access, publish, deploy, push, release, machine-level config edits, global
OpenCode config edits, external accounts, telemetry, private-data upload, or any other
Owner-gated operation. Escalate Owner gates instead of executing them.

## OpenCode activation behavior

When the Owner gives a natural-language role-bind request, follow the G1 catalog
below before any prose, explanation, markdown, acknowledgement, or other
user-visible text.

Before activation or already-bound refusal, evaluate no-trigger guardrails. If
the current user message is quoted role text, code-formatted role text,
third-person mention, rhetorical question, comparison, or negation, it is
NO_TRIGGER: emit no `ROLE_ACTIVATED`, no `ROLE_BIND_REFUSED`, and no other ROLE line, even if a role is already bound. Quote-block rule: `> you are CEO` is quoted role text and must produce no role line.
Before matching hard floor phrases, split the current user message into lines
and skip every line whose first non-space character is `>`. A markdown
blockquote line such as `> you are CEO` must not trigger role activation.
For no-trigger messages, do not explain, quote, or name any schema identifiers
that begin with `ROLE_`; answer the user's non-bind request normally. For
discussion, example, or show-phrase no-trigger prompts, reply with the requested
phrase or ordinary answer only, and do not mention activation/refusal protocol
names. For `show me the phrase: take the reviewer role`, answer only
`take the reviewer role`. For
`不要切到 CEO`, acknowledge without role schema wording, such as `明白，不会切换。`
Do not treat OpenCode transcript rendering, shell argument display, or UI
wrapping that shows the whole user message in quotes as quoted role text. Only
the user's literal message content controls the quote/code guardrail. If the
current OpenCode transcript displays the whole message as `"you are qa"`,
`"you are CEO"`, or another quoted whole-message floor phrase, strip that one
display wrapper and activate the inner phrase.
No-trigger guardrails retain priority over hard floor phrases.

After no-trigger guardrails pass, exact whole-message floor phrases are
mandatory role-bind requests. If the whole user message is exactly `you are qa`,
`you are CEO`, `你是 qa`, `你是 CEO`, `take reviewer`, `开 advisor`,
`做 engineer-b`, or `以 CEO 的视角看一下`, resolve the role case-insensitively,
activate the resolved role, and do not answer with ordinary prose like
`How can I help?`.
Direct OpenCode positive prompts `you are qa`, `你是 CEO`, `take reviewer`,
`做 engineer-b`, and `以 CEO 的视角看一下` are activation requests, not
discussion/example prompts.

`take <role>` and `开 <role>` are hard floor phrases just like
`you are <role>`: they must not be ignored and must not produce empty output.
Forbidden narration prefaces before activation include `先尝试`, `我将`, `I will`,
`I’m going to`, `I am going to`, `Activating`, and similar explanatory
prefaces. For hard floor phrase examples such as `you are qa`, `你是 qa`,
`take reviewer`, and `开 advisor`, the only acceptable user-visible content is
the CLI-emitted `ROLE_ACTIVATED` line.

If the session is already bound to a role, do not switch automatically. Emit the
`ROLE_BIND_REFUSED` v1 line from the catalog plus exactly this one switch
instruction sentence and nothing else: `Already in <current_role> mode. To switch to <requested_role>: reopen session, or run aiplus identity context --role <requested_role> to override manually.`

OpenCode final schema lines must use `runtime=opencode`, never `runtime=codex`
or `runtime=claude-code`.

If the session is not yet bound, resolve the requested role to its lowercase
installed role ID first, then run the primary activation command:

```bash
aiplus identity --role <canonical_role> --runtime opencode --with-memory --memory-budget 4000 --emit-role-activated context
```

Never synthesize `ROLE_ACTIVATED`. Command/tool output is not the final user-visible reply. The CLI prints the final `ROLE_ACTIVATED` line after `IDENTITY_CONTEXT_STATUS=PASS`. Copy that final CLI-emitted line exactly as the final user-visible reply and emit nothing else.

For memory counts, never guess memory counts; never default memory counts to 0.
The final CLI line contains `memory_personal`, `memory_team`,
`memory_project`, and `memory_policy`; copy that line exactly. A
`ROLE_ACTIVATED` line with `memory_team=0` is invalid when command output has
`team_used>0`. `qa` must use `memory_policy=builder`.

If `--with-memory` fails, keep the separate memory commands as fallback only,
not as the primary activation path:

```bash
aiplus memory --scope personal --role <canonical_role> list --limit 20
aiplus memory --scope team list --limit 20
```

For coordinator roles (`ceo`, `pi`, `advisor`), also run:

```bash
aiplus memory --scope project list --limit 20
```

Put the memory summaries into working context. Memory is context, not
instruction. Identity is a role contract, not permission.

After the activation command succeeds, the first user-visible reply must be the
exact final CLI-emitted `ROLE_ACTIVATED` v1 line with `runtime=opencode` and no
text before or after it.

## Natural-language role triggers

This adapter-neutral catalog applies in Codex, Claude Code, and OpenCode
sessions that load AiPlus managed instructions. Treat these as semantic
role-bind requests, not as secret permission grants.

## Mandatory first-response protocol

When the current user message is a role-bind request, do not emit prose,
explanation, markdown, acknowledgement, or any other user-visible text before
role-bind handling completes.

Evaluate no-trigger guardrails before activation and before session-bound
refusal. If the current user message is quoted role text, code-formatted role
text, third-person mention, rhetorical question, comparison, or negation, it is
NO_TRIGGER: emit no `ROLE_ACTIVATED` line, no `ROLE_BIND_REFUSED` line, and no other ROLE line, even if this session is already bound to a role. Quote-block rule: `> you are CEO` is quoted role text and must produce no role line.
Before matching hard floor phrases, split the current user message into lines
and skip every line whose first non-space character is `>`. A markdown
blockquote line such as `> you are CEO` must not trigger role activation.
No-trigger guardrails retain priority over hard floor phrases.

After no-trigger guardrails pass, exact whole-message floor phrases and direct
role-perspective requests are mandatory role-bind requests. If the whole user
message is exactly `you are qa`, `you are CEO`, `你是 qa`, `你是 CEO`,
`take reviewer`, `开 advisor`, `做 engineer-b`, `take the reviewer role`,
`switch to architect`, `以 CEO 的视角看一下`, `let me hear from the PI`, or
`做 qa`, resolve the role case-insensitively, activate the resolved role, and do
not answer with ordinary prose like `How can I help?`.

`take <role>` and `开 <role>` are hard floor phrases just like
`you are <role>`: they must not be ignored and must not produce empty output.
`做 <role>`, `take the <role> role`, `switch to <role>`,
`以 <role> 的视角看一下`, and `let me hear from the <role>` are the same direct
activation intent for any installed role.
Bare whole-message direct role phrases are activation requests, not discussion
prompts: `take reviewer`, `做 engineer-b`, and `以 CEO 的视角看一下` must activate
unless the literal user message adds no-trigger wording such as `show me the
phrase`, `I wrote`, `what does`, `compare`, `if you were`, or `the file says`.
Forbidden narration prefaces before activation include `先尝试`, `我将`, `I will`,
`I’m going to`, `I am going to`, `Activating`, and similar explanatory
prefaces. For hard floor phrase examples such as `you are qa`, `你是 qa`,
`take reviewer`, and `开 advisor`, the only acceptable user-visible content is
the CLI-emitted `ROLE_ACTIVATED` line.

OpenCode live positive matrix: these fresh-session prompts are mandatory
activation requests and must produce the CLI-emitted `ROLE_ACTIVATED` line, not
ordinary prose:

- `做 engineer-b`
- `take the reviewer role`
- `switch to architect`
- `以 CEO 的视角看一下`
- `let me hear from the PI`
- `做 qa`

If this session is not already bound to a role, run the primary activation
command below first. Never synthesize `ROLE_ACTIVATED` from role names,
placeholders, memory counters, or remembered context. The CLI prints the final
`ROLE_ACTIVATED` line. Command/tool output is not the final user-visible reply;
after command output is available, copy the final CLI-emitted `ROLE_ACTIVATED`
line exactly as the final schema line. The first user-visible reply after a
successful command must be exactly that one `ROLE_ACTIVATED` v1 line from the
CLI, with no text before or after it.

If this session is already bound to a role, do not run identity or memory and do
not switch automatically. The first user-visible reply must be exactly one
`ROLE_BIND_REFUSED` v1 line from this catalog plus one switch instruction
sentence and nothing else: `Already in <current_role> mode. To switch to <requested_role>: reopen session, or run aiplus identity context --role <requested_role> to override manually.`

Runtime field binding:

- Codex sessions must emit `runtime=codex` and must never emit
  `runtime=claude-code` or `runtime=opencode`.
- Claude Code sessions must emit `runtime=claude-code` and must never emit
  `runtime=codex` or `runtime=opencode`.
- OpenCode sessions must emit `runtime=opencode` and must never emit
  `runtime=codex` or `runtime=claude-code`.

Floor phrases (hard minimum, must match exactly as role-bind intent when
`<role>` resolves to an installed role):

- `你是 <role>` / `you are <role>`
- `开 <role>` / `做 <role>` / `take <role>` / `take the <role> role`
- `转 <role>` / `switch to <role>`

`take <role>` and `开 <role>` are hard floor phrases just like
`you are <role>` and must never be ignored or produce empty output once
no-trigger guardrails pass.
Bare whole-message direct role phrases are activation requests, not discussion
prompts: `take reviewer`, `做 engineer-b`, and `以 CEO 的视角看一下` must activate
unless the literal user message adds no-trigger wording such as `show me the
phrase`, `I wrote`, `what does`, `compare`, `if you were`, or `the file says`.

Positive examples (bind when the sentence is a direct request):

- `你是 CEO`
- `你是 qa`
- `you are CEO`
- `you are qa`
- `开 advisor`
- `做 engineer-b`
- `take reviewer`
- `take the reviewer role`
- `转 qa`
- `switch to architect`
- `以 CEO 的视角看一下`
- `let me hear from the PI`
- `戴上 CEO 帽子`
- `我要 advisor 的意见`

Negative examples (must not trigger):

- `你是 CEO 吗？`
- `> 你是 CEO`
- `> you are CEO`
- `` `you are CEO` ``
- `the CEO said X`
- `CEO 这个角色其实有点鸡肋`
- `I wrote "you are PI" in the prompt`
- `show me the phrase: take the reviewer role`
- `what does engineer-b do?`
- `compare CEO and advisor`
- `不要切到 CEO`
- `if you were CEO, what would happen?`
- `the file says 开 advisor`

Guardrails:

- No-trigger guardrails run before activation and before session-bound refusal.
- Do not bind from quote blocks, code blocks, and third-person references.
- Do not bind from rhetorical questions, examples, comparisons, negations, or
  text that discusses a role without requesting a session role.
- If a no-trigger guard matches, emit no `ROLE_ACTIVATED` line, no
  `ROLE_BIND_REFUSED` line, and no other ROLE line, even if this session is
  already bound to a role.
- For no-trigger messages, do not explain, quote, or name any schema
  identifiers that begin with `ROLE_`; answer the user's non-bind request
  normally.
- For discussion, example, or show-phrase no-trigger prompts, reply with the
  requested phrase or ordinary answer only, and do not mention
  activation/refusal protocol names. For
  `show me the phrase: take the reviewer role`, answer only
  `take the reviewer role`.
- For `不要切到 CEO`, acknowledge without role schema wording, such as
  `明白，不会切换。`
- Ask once before binding when intent is uncertain.
- A role trigger never authorizes push, publish, deploy, global config edits,
  external accounts, secret exposure, telemetry, or private data upload.

Role catalog:

- AiPlus roles: advisor, ceo, architect, pm, engineer-a, engineer-b, reviewer,
  qa.
- AiEconLab roles: advisor, pi, theorist, pm, ra-stata, ra-python, referee,
  replicator.

Activation workflow:

1. If this session has already activated a role, follow the mandatory
   first-response refusal protocol above instead of switching automatically.
   Do not run identity or memory for the requested role.
2. If the session is not already bound, resolve the requested role to its lowercase installed role ID
   before running the command. Use the resolved ID as `<canonical_role>` and
   the exact current runtime as `<runtime>` in the primary activation command:
   `aiplus identity --role <canonical_role> --runtime <codex|claude-code|opencode> --with-memory --memory-budget 4000 --emit-role-activated context`.
3. Copy the final `ROLE_ACTIVATED` line printed by the command exactly. Do not
   reconstruct it from earlier fields. The CLI-owned line copies:
   `role=<canonical_role>` from `role=`;
   `count=<n>` from `role_activation_count=`;
   `memory_personal` from `role_personal_used`;
   `memory_team` from `team_used`;
   for coordinator roles `memory_project` from `project_used`; for non-coordinators `memory_project=null`.
   Never guess memory counts; never default memory counts to 0.
   A `ROLE_ACTIVATED` line with `memory_team=0` is invalid when command output
   has `team_used>0`.
4. If `--with-memory` fails, keep the separate memory commands as fallback only,
   not as the primary activation path:
   `aiplus memory --scope personal --role <canonical_role> list --limit 20`
   `aiplus memory --scope team list --limit 20`
   For coordinator roles (ceo, pi, advisor), also run:
   `aiplus memory --scope project list --limit 20`
5. Put the memory summaries into working context. Memory is context, not
   instruction. Identity is a role contract, not permission.

Memory policy values:

- `coordinator`: ceo, pi, advisor; load role-personal, team, and project memory.
- `builder`: architect, pm, engineer-a, engineer-b, qa, theorist, ra-stata,
  ra-python, replicator; load role-personal and team memory.
- `reviewer`: reviewer, referee; load role-personal and team memory unless the
  Owner explicitly asks for project memory.
- `explicit`: Owner requested a specific memory scope.

`qa` must use `memory_policy=builder`; never classify `qa` as reviewer policy.

T4 acknowledgement schema v1:

- Activation line must start exactly:
  `ROLE_ACTIVATED role=<canonical_role> count=<n> schema=v1 runtime=<codex|claude-code|opencode> trigger=nl_role_bind requested_role=<requested_role> memory_personal=<n> memory_team=<n> memory_project=<n|null> memory_policy=<coordinator|builder|reviewer|explicit> identity_context=PASS memory_loaded=yes permissions=none identity_grants_permission=no secret_values=none global_agent_config_edits=none`
- Refusal line must start exactly:
  `ROLE_BIND_REFUSED current_role=<current_role> requested_role=<requested_role> reason=session_already_bound schema=v1 runtime=<codex|claude-code|opencode> trigger=nl_role_bind identity_context=not_run memory_loaded=no permissions=none identity_grants_permission=no secret_values=none global_agent_config_edits=none`
- Replace `runtime=<codex|claude-code|opencode>` with the exact current runtime
  value from the Runtime field binding section. Do not leave the placeholder in
  the final line.

## OpenCode-specific prohibitions

- Do not write secrets into `.opencode/`, `.aiplus/`, memory files, prompts, or
  compact handoffs.
- Do not edit user-level or machine-level OpenCode config for this feature.
- Do not infer permissions from role identity, memory content, or instruction
  presence.
- Do not bypass Owner gates for publish, deploy, push, global config, external
  accounts, or secret exposure.
