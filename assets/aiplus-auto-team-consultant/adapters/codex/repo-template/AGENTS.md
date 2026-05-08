# Agent Instructions

Use AiPlus Auto Team Consultant only as session-local decision support.

Default to LIGHT. Escalate to MEDIUM or HEAVY only when the task risk justifies it.

Explicit AiPlus refresh triggers are `AiPlus 刷新`, `刷新 AiPlus`,
`aiplus refresh`, `aiplus status`, `AiPlus status`, `继续 AiPlus`, and
`resume AiPlus`. If the project also uses `刷新` or `refresh` for project status,
report AiPlus status and Auto Team Consultant status before unrelated project
refresh when AiPlus is mentioned.

Do not publish, push, globally install, contact external accounts, use private project data outside the active target, expose private data externally, upload it, use private data not explicitly provided or named by the Owner for this task, expose secrets/tokens/private paths, or claim safety/compliance/legal/product-quality guarantees without explicit Owner approval.

Pressure-tests must be labeled `SIMULATED_PRESSURE_TEST_ONLY`.
