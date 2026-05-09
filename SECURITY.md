# Security And Privacy

AiPlus is project-local by default. It writes project install state under
`.aiplus/` and compact state under `.codex/compact/`.

## Compact Savings Estimate

AiPlus may fetch public release metadata and public model pricing information to
estimate compact savings. It caches pricing data locally, normally under
`~/.cache/aiplus/pricing-cache.json`, with a default 7-day TTL. If the cache is
missing or stale, AiPlus may refresh public pricing automatically. Network
failure does not block compact, checkpoint, resume, or token savings reporting.

AiPlus does not upload prompts, transcripts, project files, checkpoints, savings
ledgers, secrets, billing data, provider account data, or usage history. It does
not connect to provider billing APIs and does not require manual model price
input.

## Updates

`aiplus self update` may fetch public AiPlus release metadata, the approved
release asset, and `checksums.txt`. It installs only the user-level `aiplus`
binary path selected by the command, stages the new binary first, backs up the
old binary, verifies checksum before replacement, and runs a version smoke
check. It does not edit shell profiles, system paths, or global Codex, Claude
Code, or OpenCode configuration.

`aiplus update` updates only the current project's `.aiplus/` modules and
guidance. `aiplus update all` combines self update and project update when safe.
Project updates preserve `.codex/compact/` and the savings ledger.

## Private Profiles And Secret Broker

`aiplus profile install <private-profile-name> --user --source <path> --yes`
writes user-level preference files under
`~/.config/aiplus/profiles/<private-profile-name>/`. These files are for
collaboration preferences only and must not contain secret values, Bitwarden
machine tokens, prompt transcripts, compact checkpoints, project file contents,
or provider responses.

Use `aiplus profile cleanup --user --yes` or
`aiplus profile migrate <legacy-profile> <canonical-profile> --user --yes` to
back up and remove legacy active profile registrations after the canonical
profile has been installed. Cleanup removes only AiPlus user-level profile
registration files and matching local alias metadata; it must not delete
Bitwarden secrets or secret values.

`aiplus secret-broker` is the only supported secret access path. Private profile
packages may install a local alias table under AiPlus user config. Public AiPlus
does not bundle private Bitwarden namespaces or account identifiers.

By default, `resolve` does not print secret values. `run -- <command...>` injects
approved values only into the child process environment. AiPlus may read
`BWS_ACCESS_TOKEN` for the current process or a macOS Keychain item created by
`aiplus secret-broker token set`; it must not store that token in plaintext repo
files, project install files, compact files, logs, docs, or release artifacts.
The invoked child command is outside AiPlus' control and may print, log,
transmit, or store environment variables. Use `run --` only with trusted commands
for explicit action needs.

Real Bitwarden smoke checks require the Bitwarden Secrets Manager `bws` CLI plus
a read-only machine account token configured by the private profile owner. If
`bws` is missing, mock-provider tests can verify alias policy and no-print
behavior, but real Bitwarden read access remains unverified.
If `bws` is installed but no token is configured, `aiplus secret-broker doctor`
prints `token_source=not_configured` and the next step:
`aiplus secret-broker token set`. Paste the token into the Terminal prompt only;
never paste it into chat or repo files.

Secret-broker audit/status output is metadata-only: alias requested, allow/deny
status, provider status, and timestamp-like operational context. It must never
include raw tokens, auth headers, Bitwarden response bodies, decrypted material,
or secret values.

Savings reports are estimates based on local aggregate metrics and cached public
pricing. They are not billing data, invoices, guaranteed savings, precise cost
measurements, or quality proof.

## Sensitive Local Files

Do not place secrets, API keys, private keys, raw transcripts, provider request
or response bodies, account identifiers, personal data, screenshots with secrets,
or exact private paths in compact files.

Validation is structural and heuristic. It is not a complete secret scanner,
privacy review, legal review, compliance certification, or release approval.
