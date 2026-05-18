#!/usr/bin/env bash
# Deterministic persona contract harness for AEL golden cases.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

python3 - "$@" <<'PY'
from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any


REPO_ROOT = Path.cwd()
REQUIRED_RUNTIMES = ("codex", "claude-code", "opencode")


def fail(message: str) -> None:
    print(f"FAIL: {message}", file=sys.stderr)


def load_cases(cases_dir: Path) -> list[tuple[Path, dict[str, Any]]]:
    if not cases_dir.is_dir():
        raise SystemExit(f"cases directory not found: {cases_dir}")

    cases: list[tuple[Path, dict[str, Any]]] = []
    for path in sorted(cases_dir.rglob("*.json")):
        with path.open("r", encoding="utf-8") as handle:
            data = json.load(handle)
        if isinstance(data, dict) and "cases" in data:
            for item in data["cases"]:
                cases.append((path, item))
        elif isinstance(data, dict):
            cases.append((path, data))
        else:
            raise SystemExit(f"{path}: top-level JSON must be an object")
    return cases


def require_case(path: Path, case: dict[str, Any]) -> None:
    for key in ("id", "persona", "prompt", "runtime", "fixture_outputs", "assertions"):
        if key not in case:
            raise SystemExit(f"{path}: case missing required key: {key}")

    prompt = case["prompt"]
    if not isinstance(prompt, dict) or not isinstance(prompt.get("input"), str):
        raise SystemExit(f"{path}: prompt.input must be a string")

    runtime = case["runtime"]
    if not isinstance(runtime, dict) or runtime.get("type") != "fixture-matrix":
        raise SystemExit(f"{path}: runtime.type must be 'fixture-matrix'")

    if runtime.get("deterministic") is not True:
        raise SystemExit(f"{path}: runtime.deterministic must be true")

    adapters = runtime.get("adapters")
    if adapters != list(REQUIRED_RUNTIMES):
        raise SystemExit(
            f"{path}: runtime.adapters must be {list(REQUIRED_RUNTIMES)!r}"
        )

    fixture_outputs = case["fixture_outputs"]
    if not isinstance(fixture_outputs, dict):
        raise SystemExit(f"{path}: fixture_outputs must be an object")
    for runtime_name in REQUIRED_RUNTIMES:
        output = fixture_outputs.get(runtime_name)
        if not isinstance(output, str) or not output.strip():
            raise SystemExit(
                f"{path}: fixture_outputs.{runtime_name} must be a non-empty string"
            )

    assertions = case["assertions"]
    if not isinstance(assertions, list) or not assertions:
        raise SystemExit(f"{path}: assertions must be a non-empty list")

    for assertion in assertions:
        if not isinstance(assertion, dict):
            raise SystemExit(f"{path}: each assertion must be an object")
        markers = assertion.get("any")
        if not isinstance(markers, list) or not markers:
            raise SystemExit(f"{path}: assertion.any must be a non-empty list")
        if not all(isinstance(marker, str) and marker for marker in markers):
            raise SystemExit(f"{path}: assertion.any markers must be non-empty strings")


def evaluate_output(case: dict[str, Any], runtime_name: str, output_text: str) -> list[str]:
    output = output_text.lower()
    problems: list[str] = []

    for forbidden in case.get("forbidden_any", []):
        if forbidden.lower() in output:
            problems.append(f"{runtime_name}: forbidden marker present: {forbidden!r}")

    for assertion in case["assertions"]:
        name = assertion.get("name", "unnamed")
        markers = assertion["any"]
        if not any(marker.lower() in output for marker in markers):
            problems.append(f"{runtime_name}/{name}: missing one of {markers!r}")

    return problems


def evaluate(case: dict[str, Any]) -> tuple[bool, list[str], list[str]]:
    problems: list[str] = []
    passed_runtimes: list[str] = []

    for runtime_name in REQUIRED_RUNTIMES:
        runtime_problems = evaluate_output(
            case, runtime_name, case["fixture_outputs"][runtime_name]
        )
        if runtime_problems:
            problems.extend(runtime_problems)
        else:
            passed_runtimes.append(runtime_name)

    return not problems, problems, passed_runtimes


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(
        description="Run deterministic AEL persona golden-contract cases."
    )
    parser.add_argument(
        "--cases-dir",
        default="tests/golden",
        help="Directory containing JSON golden cases (default: tests/golden).",
    )
    parser.add_argument(
        "--positive-only",
        action="store_true",
        help="Run only passing contract cases; skip expect_fail regression cases.",
    )
    parser.add_argument(
        "--negative-only",
        action="store_true",
        help="Run only expect_fail regression cases.",
    )
    args = parser.parse_args(argv)

    if args.positive_only and args.negative_only:
        raise SystemExit("--positive-only and --negative-only are mutually exclusive")

    cases_dir = (REPO_ROOT / args.cases_dir).resolve()
    loaded_cases = load_cases(cases_dir)
    if not loaded_cases:
        raise SystemExit(f"no persona contract cases found under {cases_dir}")

    failures = 0
    positives = 0
    negatives = 0

    for path, case in loaded_cases:
        require_case(path, case)
        expect_fail = bool(case.get("expect_fail", False))
        if args.positive_only and expect_fail:
            continue
        if args.negative_only and not expect_fail:
            continue

        ok, problems, passed_runtimes = evaluate(case)
        label = f"{case['persona']}/{case['id']}"

        if expect_fail:
            negatives += 1
            if ok:
                fail(f"{label}: expected a contract violation, but all markers passed")
                failures += 1
            else:
                print(f"PASS expected-fail: {label} caught {problems[0]}")
            continue

        positives += 1
        if ok:
            print(f"PASS: {label} runtimes={','.join(passed_runtimes)}")
        else:
            fail(f"{label}: " + "; ".join(problems))
            failures += 1

    if args.negative_only and negatives == 0:
        fail("no expect_fail regression cases were selected")
        failures += 1
    if args.positive_only and positives == 0:
        fail("no positive contract cases were selected")
        failures += 1
    if not args.positive_only and not args.negative_only:
        if positives == 0:
            fail("no positive contract cases found")
            failures += 1
        if negatives == 0:
            fail("no expect_fail regression case found")
            failures += 1

    print(
        "PERSONA_CONTRACTS_STATUS="
        + ("PASS" if failures == 0 else "FAIL")
        + f" positives={positives} expected_fail={negatives}"
    )
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
PY
