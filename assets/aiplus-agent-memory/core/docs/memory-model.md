# Memory Model

Memory records are JSON objects stored locally. They describe project facts,
preferences, decisions, constraints, handoff notes, or evidence pointers.

Rules:

- no raw transcript
- no secret value
- no provider response body
- no auth header
- no unredacted private path
- no approval transcript
- no project file content

Records may be active, tentative, superseded, rejected, or expired.
