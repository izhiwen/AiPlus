# Example Aliases — companion to `example-aliases.tsv`

The real broker config file is **TSV** (tab-separated values), not TOML or JSON.
That keeps the config trivially `grep`able and immune to parser-version drift.
But TSV does not allow inline comments. This file provides the annotation that
the `.tsv` file cannot carry inline.

## File format

```
<alias>	<backend-path>	<injected-env-var-name>
```

Three columns, separated by a **single tab character** (not spaces).

| Column | Meaning |
|---|---|
| `alias` | Short lowercase name. What you type into `aiplus secret-broker run --alias <alias>` and `aiplus secret-broker resolve <alias>`. |
| `backend-path` | Path in the Bitwarden Secrets Manager workspace. The broker reads the secret value from this path at resolve time. |
| `injected-env-var-name` | The environment variable name that the child process sees. Typically the SDK convention (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, `GITHUB_TOKEN`, …). |

## Real file location (NOT this repo)

The real config lives at:

```
~/.config/aiplus/secret-broker/profiles/<your-aiplus-profile>/secret-aliases.tsv
```

Replace `<your-aiplus-profile>` with the active profile name (run
`aiplus profile status` if unsure).

**Never** commit a populated `secret-aliases.tsv` into a project repo —
the backend paths reveal your secret-store structure even though no
values are stored.

## Example walkthrough

The row:

```
openai	zhiwen/openai/api_key	OPENAI_API_KEY
```

means:
- When you type `aiplus secret-broker run --alias openai -- python agent.py`,
- the broker looks up the Bitwarden Secrets Manager secret at path
  `zhiwen/openai/api_key`,
- resolves it to its current value,
- spawns `python agent.py` with `OPENAI_API_KEY=<value>` in the child env,
- exits when `python agent.py` exits, with the value gone from any
  process that did not capture it.

## Why these three columns

- **Alias** is short because you type it often.
- **Backend path** is decoupled from the alias so you can re-organize
  Bitwarden later without breaking callers.
- **Env-var name** is decoupled from the alias so you can match the
  SDK convention without renaming aliases. Most SDKs read
  `OPENAI_API_KEY`, `ANTHROPIC_API_KEY`, etc., regardless of what your
  alias is called.

## Real-world example aliases

See `example-aliases.tsv` in this directory for 24 realistic example
aliases covering LLM providers, OAuth tokens, search/retrieval APIs,
and media generation services. The example paths use the placeholder
scope `example/` so the file cannot be confused with a real config.
