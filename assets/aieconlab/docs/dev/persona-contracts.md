# Persona Contract Golden Tests

Issue #58 adds deterministic persona contract tests for AEL roles. The harness
does not call a live model in CI. It reads JSON cases under `tests/golden/`,
checks one fixture output for each supported runtime surface (`codex`,
`claude-code`, and `opencode`), and applies semantic-OR marker assertions to
each runtime output.

Run all cases:

```bash
bash tests/run_persona_contracts.sh
```

Run only positive cases:

```bash
bash tests/run_persona_contracts.sh --positive-only
```

Run only deliberate regression cases:

```bash
bash tests/run_persona_contracts.sh --negative-only
```

## Case Format

Create one JSON file under `tests/golden/<persona>/<short-name>.json`.

Required fields:

- `id`: stable case id used in harness output.
- `persona`: persona slug, for example `lit-reviewer`, `theorist`, or
  `referee`.
- `kind`: short category such as `in_scope`, `boundary`, or `regression`.
- `prompt.input`: the user prompt the fixture represents.
- `runtime.type`: must be `fixture-matrix`.
- `runtime.adapters`: must be `["codex", "claude-code", "opencode"]`.
- `runtime.deterministic`: must be `true`.
- `fixture_outputs`: deterministic text to assert against for each runtime.
- `assertions`: list of semantic-OR marker groups.

Each assertion uses this shape:

```json
{
  "name": "role-marker",
  "any": ["referee", "pre-review", "top-5"]
}
```

The assertion passes for a runtime when any marker in `any` appears in that
runtime's `fixture_outputs` value, case-insensitively. A case passes only when
all three runtimes pass every assertion group and no `forbidden_any` marker
appears.

## Adding a Case

1. Add a focused prompt that exercises one contract obligation.
2. Write deterministic, short fixture outputs for `codex`, `claude-code`, and
   `opencode`.
3. Add at least three assertion groups:
   team marker, role marker, and domain or boundary marker.
4. Prefer semantic alternatives over brittle phrasing. For example, use
   `["referee", "pre-review", "top-5"]` instead of one exact sentence.
5. Add `forbidden_any` for obvious bad behavior, such as claiming an action was
   completed when the role is only allowed to route or comment.
6. Run `bash tests/run_persona_contracts.sh`.

## Regression Fixtures

A deliberate negative case sets `"expect_fail": true`. The normal harness run
expects at least one runtime in that case to fail its marker assertions; if the
case starts passing across all three runtimes, the harness exits non-zero. Keep
at least one negative case so CI proves the checks can catch a missing required
marker.
