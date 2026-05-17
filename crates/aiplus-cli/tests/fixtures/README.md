# G2 dispatch gate fixtures

`g2_dispatch_gate_samples.jsonl` contains mined examples for the G2 semantic
dispatch owner-gate tests. Each line is one JSON object with:

- `id`: stable fixture id.
- `category`: `false_positive` for text that should pass without an owner gate,
  or `true_positive` for text that should trigger an owner gate.
- `expected_decision`: `PASS` means no owner gate; `FAIL` means owner gate.
- `expected_gate`: boolean equivalent of `expected_decision`.
- `task`: task text or a minimal task excerpt to classify.
- `reason`: why this case should pass or fail under verb-object semantics.
- `source_type`: `local_log` for direct local examples, or `derived` for
  minimal examples derived from observed local patterns when exact positives
  were sparse.
- `source_pointer`: local source pointer where available.
