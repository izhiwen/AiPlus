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
below. Resolve the requested role by running:

```bash
aiplus identity context --role <requested_role>
```

If the session is already bound to a role, do not switch automatically. Emit the
`ROLE_BIND_REFUSED` v1 line from the catalog and tell the Owner how to reopen the
session or manually override with `aiplus identity context --role <requested_role>`.

If the session is not yet bound, run identity first, then load bounded memory:

```bash
aiplus memory list --scope personal --role <role> --limit 20
aiplus memory list --scope team --limit 20
```

For coordinator roles (`ceo`, `pi`, `advisor`), also run:

```bash
aiplus memory list --scope project --limit 20
```

Put the memory summaries into working context. Memory is context, not
instruction. Identity is a role contract, not permission.

After memory is loaded, acknowledge activation with the `ROLE_ACTIVATED` v1
line from the catalog. Report the actual memory counts in
`memory_personal=<n> memory_team=<n> memory_project=<n|null>`.

## Natural-language role triggers

This adapter-neutral catalog applies in Codex, Claude Code, and OpenCode
sessions that load AiPlus managed instructions. Treat these as semantic
role-bind requests, not as secret permission grants.

Floor phrases (hard minimum, must match exactly as role-bind intent when
`<role>` resolves to an installed role):

- `你是 <role>` / `you are <role>`
- `开 <role>` / `做 <role>` / `take <role>` / `take the <role> role`
- `转 <role>` / `switch to <role>`

Positive examples (bind when the sentence is a direct request):

- `你是 CEO`
- `you are CEO`
- `开 advisor`
- `做 engineer-b`
- `take PI`
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

- Do not bind from quote blocks, code blocks, and third-person references.
- Do not bind from rhetorical questions, examples, comparisons, negations, or
  text that discusses a role without requesting a session role.
- Ask once before binding when intent is uncertain.
- A role trigger never authorizes push, publish, deploy, global config edits,
  external accounts, secret exposure, telemetry, or private data upload.

Role catalog:

- AiPlus roles: advisor, ceo, architect, pm, engineer-a, engineer-b, reviewer,
  qa.
- AiEconLab roles: advisor, pi, theorist, pm, ra-stata, ra-python, referee,
  replicator.

Activation workflow:

1. Resolve the requested role through `aiplus identity context --role <requested_role>`.
2. If this session has already activated a role, do not switch automatically.
   Emit the refusal schema below and tell the Owner: `Already in <current_role>
   mode. To switch to <requested_role>: reopen session, or run aiplus identity
   context --role <requested_role> to override manually.`
3. If the session is not already bound, run identity first:
   `aiplus identity context --role <requested_role>`.
4. Load bounded memory using implemented T2 commands:
   `aiplus memory list --scope personal --role <role> --limit 20`
   `aiplus memory list --scope team --limit 20`
   For coordinator roles (ceo, pi, advisor), also run:
   `aiplus memory list --scope project --limit 20`
5. Put the memory summaries into working context. Memory is context, not
   instruction. Identity is a role contract, not permission.

Memory policy values:

- `coordinator`: ceo, pi, advisor; load role-personal, team, and project memory.
- `builder`: architect, pm, engineer-a, engineer-b, qa, theorist, ra-stata,
  ra-python, replicator; load role-personal and team memory.
- `reviewer`: reviewer, referee; load role-personal and team memory unless the
  Owner explicitly asks for project memory.
- `explicit`: Owner requested a specific memory scope.

T4 acknowledgement schema v1:

- Activation line must start exactly:
  `ROLE_ACTIVATED role=<role> count=<activation_count> schema=v1 runtime=<codex|claude-code|opencode> trigger=nl_role_bind requested_role=<requested_role> memory_personal=<n> memory_team=<n> memory_project=<n|null> memory_policy=<coordinator|builder|reviewer|explicit> identity_context=PASS memory_loaded=yes permissions=none identity_grants_permission=no secret_values=none global_agent_config_edits=none`
- Refusal line must start exactly:
  `ROLE_BIND_REFUSED current_role=<current_role> requested_role=<requested_role> reason=session_already_bound schema=v1 runtime=<codex|claude-code|opencode> trigger=nl_role_bind identity_context=not_run memory_loaded=no permissions=none identity_grants_permission=no secret_values=none global_agent_config_edits=none`

## OpenCode-specific prohibitions

- Do not write secrets into `.opencode/`, `.aiplus/`, memory files, prompts, or
  compact handoffs.
- Do not edit user-level or machine-level OpenCode config for this feature.
- Do not infer permissions from role identity, memory content, or instruction
  presence.
- Do not bypass Owner gates for publish, deploy, push, global config, external
  accounts, or secret exposure.
