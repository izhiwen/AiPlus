# Security

AiPlus Auto Compact stores compact/resume state in local project files. Treat those files as potentially sensitive project records.

## Supported Versions

This repository is public on GitHub and versioned `0.1.0` in `package.json`. It
is not published to npm, Cargo, Homebrew, any package registry, or any
marketplace. Security fixes should target the current public repository unless a
release branch is created by `<OWNER>`.

## Reporting

Report security concerns to `<OWNER>` through the private project channel approved for this repository. Do not include secrets, tokens, private keys, raw transcripts, provider payloads, or personal data in the report body.

Use placeholders such as `<REPO_ROOT>`, `<TARGET_PROJECT>`, `<OWNER>`, `<GITHUB_OWNER>`, `<EXAMPLE_PROJECT>`, `<REDACTED_SECRET>`, `<REDACTED_TOKEN>`, and `<REDACTED_PII>`.

## Data Boundaries

AiPlus Auto Compact is local-first:

- It writes compact state under `<TARGET_PROJECT>/.codex/compact/`.
- It writes checkpoint JSON under `<TARGET_PROJECT>/.codex/compact/checkpoints/`.
- It does not require cloud sync, telemetry, remote APIs, or hosted storage.
- It does not publish, push, tag, upload artifacts, or configure external services.

Do not store:

- API keys, tokens, cookies, Authorization headers, private keys, or JWTs.
- Raw provider requests or responses.
- Raw transcripts, screenshots containing secrets, HAR files, or WebRTC dumps.
- Private account identifiers, billing identifiers, device identifiers, or personal data.
- Exact private paths or cloud-sync paths.
- Real Owner approvals that should not be public.

## Validation Limits

`aiplus compact validate` performs structural and heuristic checks. The archived
legacy `compactctl.mjs` helper is retained only for compatibility tests and
migration review; it is not an active compact execution path. These checks can
catch missing required files, invalid enum values, invalid policy JSON,
unsupported versions, denied or pending Owner gates, and some obvious sensitive
patterns.

Validation is not a full secret scanner, privacy review, legal review, or project approval process. Review compact state manually before sharing it, committing it, or using it as the basis for a compact recommendation.

## Handling Findings

If sensitive content appears in compact files:

1. Replace it with a placeholder.
2. Rerun validation.
3. Remove or rotate any exposed credential outside this repository using the owning service's process.
4. Document the remediation in redacted form only.

If a checkpoint contains sensitive content, delete the checkpoint file from local history and regenerate it after redaction. Do not commit checkpoint files unless `<OWNER>` has explicitly approved their contents.
