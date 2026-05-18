"""W8: persona behavior test runner.

For each persona × prompt pair in test_cases.toml:
  1. Load the persona body from core/templates/personas/<persona>.md
  2. Send a single Anthropic API call: system = persona body, user = prompt
  3. Check the response against expects_any (must contain at least one)
     and forbids_any (must not contain any).
  4. Tally pass/fail per persona.

Exits non-zero if any persona's pass rate is below the threshold
(default 80%).

Designed to run from the AEL repo root in CI; can also be invoked
locally as `python tests/persona_behavior/test_persona_behavior.py`.

Environment:
  ANTHROPIC_API_KEY  required
  PERSONA_THRESHOLD  optional float in [0,1], default 0.80
  PERSONA_MODEL      optional, default "claude-haiku-4-5-20251001"
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

try:
    import tomllib  # Python 3.11+
except ImportError:  # pragma: no cover
    import tomli as tomllib  # type: ignore

try:
    from anthropic import Anthropic
except ImportError:
    sys.stderr.write(
        "anthropic package missing. install with: pip install anthropic tomli\n"
    )
    sys.exit(2)


REPO_ROOT = Path(__file__).resolve().parents[2]
PERSONAS_DIR = REPO_ROOT / "core" / "templates" / "personas"
CASES_FILE = Path(__file__).resolve().parent / "test_cases.toml"


def load_persona_body(persona_name: str) -> str:
    """Read the persona's full system prompt from disk.

    The persona file IS the system prompt — we send it verbatim. The
    AEL framing line under ## Role Identity carries the AiPlus
    backlink; the rest of the file enumerates Voice / Knowledge
    Boundaries / Escalation / Forbidden / Examples.
    """
    path = PERSONAS_DIR / f"{persona_name}.md"
    if not path.exists():
        raise FileNotFoundError(f"persona file missing: {path}")
    return path.read_text(encoding="utf-8")


def evaluate_response(
    response: str, expects_any: list[str], forbids_any: list[str]
) -> tuple[bool, str]:
    """Return (passed, reason). The first failing condition wins."""
    lo = response.lower()
    for bad in forbids_any:
        if bad.lower() in lo:
            return False, f"response contains forbidden substring '{bad}'"
    if expects_any:
        if not any(good.lower() in lo for good in expects_any):
            return (
                False,
                f"response missed all expected substrings: {expects_any!r}",
            )
    return True, "ok"


def main() -> int:
    api_key = os.environ.get("ANTHROPIC_API_KEY")
    if not api_key:
        sys.stderr.write(
            "ANTHROPIC_API_KEY not set; W8 behavior tests cannot run.\n"
            "In CI this is the gate that skips the workflow on PRs from forks.\n"
        )
        return 0  # Skip, don't fail.

    threshold = float(os.environ.get("PERSONA_THRESHOLD", "0.80"))
    model = os.environ.get("PERSONA_MODEL", "claude-haiku-4-5-20251001")

    cases_doc = tomllib.loads(CASES_FILE.read_text(encoding="utf-8"))
    cases = cases_doc.get("cases", [])
    if not cases:
        sys.stderr.write("no cases in test_cases.toml; nothing to do\n")
        return 1

    client = Anthropic(api_key=api_key)

    # Group by persona for pass-rate accounting.
    by_persona: dict[str, list[dict]] = {}
    for case in cases:
        by_persona.setdefault(case["persona"], []).append(case)

    overall_fail = False
    print(f"W8 persona behavior: {len(cases)} cases across {len(by_persona)} personas")
    print(f"model={model} threshold={threshold:.0%}")
    print("-" * 72)

    for persona, persona_cases in sorted(by_persona.items()):
        try:
            system_prompt = load_persona_body(persona)
        except FileNotFoundError as e:
            print(f"[{persona}] PERSONA_MISSING — {e}")
            overall_fail = True
            continue

        passed = 0
        for case in persona_cases:
            kind = case["kind"]
            prompt = case["prompt"]
            expects = case.get("expects_any", [])
            forbids = case.get("forbids_any", [])

            try:
                resp = client.messages.create(
                    model=model,
                    max_tokens=600,
                    system=system_prompt,
                    messages=[{"role": "user", "content": prompt}],
                )
                response_text = "".join(
                    block.text for block in resp.content if hasattr(block, "text")
                )
            except Exception as exc:  # noqa: BLE001
                print(f"[{persona} {kind}] API_ERROR — {exc}")
                overall_fail = True
                continue

            ok, reason = evaluate_response(response_text, expects, forbids)
            marker = "PASS" if ok else "FAIL"
            if ok:
                passed += 1
            preview = response_text.strip().splitlines()[0][:100] if response_text else "(empty)"
            print(f"[{persona} {kind}] {marker} — {reason}")
            if not ok:
                print(f"  prompt: {prompt[:80]}")
                print(f"  response head: {preview}")

        rate = passed / len(persona_cases)
        rate_marker = "OK" if rate >= threshold else "FLAKY"
        print(
            f"  persona summary: {persona} {passed}/{len(persona_cases)} "
            f"({rate:.0%}) {rate_marker}"
        )
        if rate < threshold:
            overall_fail = True

    print("-" * 72)
    print("W8_PERSONA_BEHAVIOR_STATUS=" + ("NEEDS_FIX" if overall_fail else "PASS"))
    return 1 if overall_fail else 0


if __name__ == "__main__":
    sys.exit(main())
