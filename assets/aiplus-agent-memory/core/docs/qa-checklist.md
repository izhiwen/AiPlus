# QA Checklist

- `aiplus memory init --project`
- `aiplus memory status`
- `aiplus memory doctor`
- `aiplus memory context --runtime codex --budget 2000`
- `aiplus identity init --project`
- `aiplus identity context --role advisor`
- `aiplus identity context --role ceo`
- `aiplus skill-candidate status`
- fake HOME tests
- at least three fake project isolation tests
- no-secret scan
- public/private boundary scan
- no global runtime config edits
