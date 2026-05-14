# Examples — AiPlus-Agent-Key

Three realistic walkthroughs of using the broker in practice.

## Example 1 — Run an agent script with two providers

You are debugging a Python agent that needs both OpenAI and Anthropic keys:

```bash
aiplus secret-broker run \
  --aliases openai,anthropic \
  -- python my_agent.py
```

What happens:
1. The broker reads `~/.config/aiplus/secret-broker/profiles/<your-profile>/secret-aliases.tsv` and finds the alias→backend-path→env-var-name mapping for each requested alias.
2. The broker fetches the Bitwarden access token from the OS keyring (account `aiplus-secret-broker`).
3. The broker shells out to the `bws` CLI once per alias (2 calls) and gets back the current values.
4. The broker spawns `python my_agent.py` with `OPENAI_API_KEY=<value>` and `ANTHROPIC_API_KEY=<value>` in the child's environment (env-var names taken from column 3 of the TSV, not from the alias).
5. Your Python code does `os.environ["OPENAI_API_KEY"]` and `os.environ["ANTHROPIC_API_KEY"]` and makes its API calls.
6. When `python my_agent.py` exits, the broker exits with the same exit code. The resolved values are gone from any process that did not capture them.

Total wall time added: ~150-400ms depending on backend latency.

Actual output prefix:
```
SECRET_BROKER_RUN
requested_aliases=[openai,anthropic]
injected_env=[OPENAI_API_KEY,ANTHROPIC_API_KEY]
skipped_aliases=[]
secret_values_printed=no
<your agent's stdout here>
SECRET_BROKER_RUN_STATUS=PASS
```

## Example 2 — Diagnose a "wrong account" suspicion

You suspect an agent has been firing against the wrong OpenAI account. Check which alias maps to which backend path without printing the secret value:

```bash
# Show resolution metadata for one alias (NO value)
aiplus secret-broker resolve openai
# Output:
#   SECRET_RESOLVE
#   alias=openai
#   provider=bws
#   token_source=keychain
#   secret_key=zhiwen/openai/api_key
#   secret_id_found=yes
#   env_var=OPENAI_API_KEY
#   provider_status=PASS
#   secret_value_printed=no
#   SECRET_RESOLVE_STATUS=PASS

# List all configured aliases with their backend paths and injected env-var names
aiplus secret-broker list
# Output:
#   SECRET_BROKER_LIST
#   anthropic -> zhiwen/anthropic/api_key -> ANTHROPIC_API_KEY
#   github -> zhiwen/github/token -> GITHUB_TOKEN
#   openai -> zhiwen/openai/api_key -> OPENAI_API_KEY
#   ...

# Run the broker's diagnostic
aiplus secret-broker doctor
# Output:
#   SECRET_BROKER_DOCTOR
#   provider=bws
#   bws_cli=yes
#   token_source=keychain
#   keychain_supported=yes
#   secret_values_printed=no
#   SECRET_BROKER_DOCTOR_STATUS=PASS
```

If you genuinely need to inspect a secret value (e.g. to paste it into a third-party admin UI), enable the `--print` policy in your AiPlus profile's `privacy-and-secrets.md` preferences, then:

```bash
aiplus secret-broker resolve openai --print
# Output (with policy enabled): the actual value, visible in shell history
```

If `--print` is **not** enabled, you will get `ERROR --print is disabled by default`. This is intentional.

## Example 3 — CI pipeline injection (aspirational for v0.1)

> **v0.1 caveat:** This example assumes AiPlus has a Linux release artifact. As of v0.1 AiPlus ships **macOS arm64 binaries only**. On Linux GitHub Actions runners the `Install AiPlus` step below will fail until upstream publishes Linux releases (or until you build from source as part of the workflow). The example is kept as the intended shape of CI usage once Linux binaries land.

In a GitHub Actions workflow, you want to inject API keys without storing them in `.github/workflows/` or in workflow logs:

```yaml
# .github/workflows/agent-run.yml
jobs:
  run-agent:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install AiPlus + bws CLI
        run: |
          # ... AiPlus install steps ...
          # ... bws CLI install steps ...

      - name: Run agent with broker-resolved secrets
        env:
          # The broker reads BWS_ACCESS_TOKEN from the environment directly
          # when the OS keyring is unavailable. Simpler than `token set` on
          # CI runners (no Keychain / Secret Service required).
          BWS_ACCESS_TOKEN: ${{ secrets.BWS_ACCESS_TOKEN }}
        run: |
          aiplus secret-broker run \
            --aliases openai,anthropic \
            -- python ci_agent.py
```

The Bitwarden access token is the only thing in GitHub Secrets. All provider keys live in Bitwarden Secrets Manager and are rotated there. No provider key ever appears in `.github/workflows/`, in GitHub Secrets, or in workflow logs.

Local-dev alternative: use `aiplus secret-broker token set` to store the token in your OS keyring instead of an env var; on macOS it triggers a one-time Keychain authorization dialog. CI prefers the env-var path because there is no interactive keyring on most CI runners.

## What this example demonstrates

- The broker is the only piece of AiPlus that touches secret values.
- The CLI surface (`run`, `resolve`, `list`, `doctor`, `status`, `token set/delete`) is identical across local dev and CI.
- The Bitwarden access token is the *only* bootstrap secret; everything else flows through it.
- `token set` only accepts a token via **stdin** (never `--from-env` or a positional arg), so the token never appears in `ps`, in shell history, or in CI logs even briefly.
- Default outputs never print secret values; `--print` is disabled by default and requires explicit profile opt-in to enable.
