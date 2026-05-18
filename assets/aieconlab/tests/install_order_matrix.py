#!/usr/bin/env python3
"""Cross-platform TEST-4 install-order matrix harness.

Runs one representative install-order case in a clean git project and
asserts that every installed runtime has its expected AiEconLab artifacts.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


CORE_ROLES = [
    "advisor",
    "pi",
    "theorist",
    "pm",
    "ra-stata",
    "ra-python",
    "referee",
    "replicator",
]

EXPERTS = [
    "lit-reviewer",
    "writer",
    "econometrician",
    "reproducibility",
    "historical-sources",
    "job-talk-coach",
    "viz-specialist",
    "ethics-irb",
    "llm-measurement",
    "survey-experiment",
    "computation",
    "coauthor-liaison",
    "dof-auditor",
    "rr-strategist",
]

AEL_AGENT_NAMES = CORE_ROLES + EXPERTS
AEL_COMMANDS = [
    "aiel-fire-consultant.md",
    "aiel-route.md",
    "aiel-status.md",
    "aiel-talk.md",
]


CASES = {
    "h1_codex_ael_then_claude": {
        "description": "codex + aieconlab -> install claude-code (H1)",
        "steps": [
            ("install", "codex"),
            ("add", "aieconlab"),
            ("install", "claude-code"),
        ],
        "expected_runtimes": ["codex", "claude-code"],
    },
    "codex_ael_then_opencode": {
        "description": "codex + aieconlab -> install opencode",
        "steps": [
            ("install", "codex"),
            ("add", "aieconlab"),
            ("install", "opencode"),
        ],
        "expected_runtimes": ["codex", "opencode"],
    },
    "claude_ael_then_codex": {
        "description": "claude-code + aieconlab -> install codex",
        "steps": [
            ("install", "claude-code"),
            ("add", "aieconlab"),
            ("install", "codex"),
        ],
        "expected_runtimes": ["claude-code", "codex"],
    },
    "claude_ael_then_opencode": {
        "description": "claude-code + aieconlab -> install opencode",
        "steps": [
            ("install", "claude-code"),
            ("add", "aieconlab"),
            ("install", "opencode"),
        ],
        "expected_runtimes": ["claude-code", "opencode"],
    },
    "opencode_ael_then_codex": {
        "description": "opencode + aieconlab -> install codex",
        "steps": [
            ("install", "opencode"),
            ("add", "aieconlab"),
            ("install", "codex"),
        ],
        "expected_runtimes": ["opencode", "codex"],
    },
    "opencode_ael_then_claude": {
        "description": "opencode + aieconlab -> install claude-code",
        "steps": [
            ("install", "opencode"),
            ("add", "aieconlab"),
            ("install", "claude-code"),
        ],
        "expected_runtimes": ["opencode", "claude-code"],
    },
    "all_runtimes_then_ael": {
        "description": "install all 3 runtimes -> add aieconlab last",
        "steps": [
            ("install", "codex"),
            ("install", "claude-code"),
            ("install", "opencode"),
            ("add", "aieconlab"),
        ],
        "expected_runtimes": ["codex", "claude-code", "opencode"],
    },
    "claude_ael_then_codex_then_opencode": {
        "description": "install claude-code + add aieconlab + 2 others in series",
        "steps": [
            ("install", "claude-code"),
            ("add", "aieconlab"),
            ("install", "codex"),
            ("install", "opencode"),
        ],
        "expected_runtimes": ["codex", "claude-code", "opencode"],
    },
    "codex_ael_then_claude_then_opencode": {
        "description": "install codex + add aieconlab + 2 others in series",
        "steps": [
            ("install", "codex"),
            ("add", "aieconlab"),
            ("install", "claude-code"),
            ("install", "opencode"),
        ],
        "expected_runtimes": ["codex", "claude-code", "opencode"],
    },
}


def fail(message: str) -> None:
    print(f"::error::{message}", file=sys.stderr, flush=True)
    raise SystemExit(1)


def run(cmd: list[str], cwd: Path, env: dict[str, str]) -> None:
    print(f"+ {' '.join(cmd)}", flush=True)
    subprocess.run(cmd, cwd=cwd, env=env, text=True, check=True)


def read_text(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except FileNotFoundError:
        fail(f"missing expected file: {path}")


def require_file(root: Path, relative: str) -> Path:
    path = root / relative
    if not path.is_file():
        fail(f"missing expected file: {relative}")
    return path


def require_contains(root: Path, relative: str, needle: str) -> None:
    text = read_text(require_file(root, relative))
    if needle not in text:
        fail(f"{relative} missing expected text: {needle}")


def load_manifest(root: Path) -> dict:
    manifest_path = require_file(root, ".aiplus/manifest.json")
    try:
        return json.loads(read_text(manifest_path))
    except json.JSONDecodeError as exc:
        fail(f".aiplus/manifest.json is invalid JSON: {exc}")


def require_core_ael_files(root: Path) -> None:
    require_file(root, ".aiplus/modules/aieconlab/aiplus-module.json")
    require_file(root, ".aiplus/consultant-team.toml")

    for role in CORE_ROLES:
        require_file(root, f".aiplus/agents/{role}.toml")

    for expert in EXPERTS:
        require_file(root, f".aiplus/agents/experts/{expert}.toml")

    require_contains(root, ".aiplus/consultant-team.toml", 'id = "ai_integration"')
    require_contains(root, ".aiplus/consultant-team.toml", "[user_evidence]")
    require_contains(root, ".aiplus/consultant-team.toml", 'light.review_mode = "skip"')


def require_codex_files(root: Path) -> None:
    require_contains(root, "AGENTS.md", "<!-- BEGIN AIPLUS MANAGED BLOCK -->")
    require_contains(root, ".aiplus/AGENTS.aiplus.md", "<!-- BEGIN AIECONLAB_TEAM -->")


def frontmatter_fields(path: Path) -> dict[str, str]:
    text = read_text(path)
    if not text.startswith("---\n"):
        fail(f"{path} missing YAML frontmatter")
    end = text.find("\n---", 4)
    if end == -1:
        fail(f"{path} missing YAML frontmatter close")
    fields: dict[str, str] = {}
    for line in text[4:end].splitlines():
        if ":" in line:
            key, value = line.split(":", 1)
            fields[key.strip()] = value.strip()
    return fields


def require_markdown_agent_dir(root: Path, runtime_dir: str) -> None:
    agent_dir = root / runtime_dir / "agents"
    if not agent_dir.is_dir():
        fail(f"missing expected agent directory: {runtime_dir}/agents")

    expected = {f"aieconlab-{name}.md" for name in AEL_AGENT_NAMES}
    actual = {path.name for path in agent_dir.glob("aieconlab-*.md")}
    if actual != expected:
        missing = sorted(expected - actual)
        extra = sorted(actual - expected)
        fail(f"{runtime_dir}/agents mismatch; missing={missing} extra={extra}")

    for name in sorted(expected):
        path = agent_dir / name
        fields = frontmatter_fields(path)
        expected_name = path.stem
        if fields.get("name") != expected_name:
            fail(f"{path} has wrong frontmatter name: {fields.get('name')}")
        if not fields.get("description"):
            fail(f"{path} missing frontmatter description")


def require_markdown_command_dir(root: Path, runtime_dir: str) -> None:
    command_dir = root / runtime_dir / "commands"
    if not command_dir.is_dir():
        fail(f"missing expected command directory: {runtime_dir}/commands")

    actual = {path.name for path in command_dir.glob("aiel-*.md")}
    expected = set(AEL_COMMANDS)
    if actual != expected:
        missing = sorted(expected - actual)
        extra = sorted(actual - expected)
        fail(f"{runtime_dir}/commands mismatch; missing={missing} extra={extra}")

    for name in sorted(expected):
        path = command_dir / name
        text = read_text(path)
        slash_command = f"/{path.stem}"
        if slash_command not in text:
            fail(f"{path} does not reference slash command {slash_command}")


def require_claude_files(root: Path) -> None:
    require_contains(root, "CLAUDE.md", "<!-- BEGIN AIECONLAB MANAGED BLOCK -->")
    require_markdown_agent_dir(root, ".claude")
    require_markdown_command_dir(root, ".claude")


def require_opencode_files(root: Path) -> None:
    config_path = require_file(root, ".opencode/opencode.json")
    try:
        config = json.loads(read_text(config_path))
    except json.JSONDecodeError as exc:
        fail(f".opencode/opencode.json is invalid JSON: {exc}")
    if config.get("$schema") != "https://opencode.ai/config.json":
        fail(".opencode/opencode.json missing expected schema")

    require_markdown_agent_dir(root, ".opencode")
    require_markdown_command_dir(root, ".opencode")


def require_doctor_pass(bin_path: Path, project: Path, env: dict[str, str]) -> None:
    cmd = [str(bin_path), "doctor"]
    print(f"+ {' '.join(cmd)}", flush=True)
    proc = subprocess.run(
        cmd,
        cwd=project,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    print(proc.stdout, flush=True)
    if proc.returncode != 0:
        fail(f"aiplus doctor exited with {proc.returncode}")
    if "DOCTOR_STATUS=PASS" not in proc.stdout.splitlines():
        fail("aiplus doctor did not report DOCTOR_STATUS=PASS")


def isolated_env(project: Path, bin_path: Path) -> dict[str, str]:
    env = os.environ.copy()
    fake_home = project / "fake-home"
    fake_userprofile = project / "fake-userprofile"
    fake_codex_home = project / "fake-codex-home"
    fake_xdg = project / "fake-xdg"
    fake_appdata = project / "fake-appdata" / "Roaming"
    fake_localappdata = project / "fake-appdata" / "Local"

    for path in [
        fake_home,
        fake_userprofile,
        fake_codex_home,
        fake_xdg,
        fake_appdata,
        fake_localappdata,
    ]:
        path.mkdir(parents=True, exist_ok=True)

    env.update(
        {
            "PATH": f"{bin_path.parent}{os.pathsep}{env.get('PATH', '')}",
            "HOME": str(fake_home),
            "USERPROFILE": str(fake_userprofile),
            "CODEX_HOME": str(fake_codex_home),
            "XDG_CONFIG_HOME": str(fake_xdg),
            "APPDATA": str(fake_appdata),
            "LOCALAPPDATA": str(fake_localappdata),
        }
    )
    return env


def run_case(case_id: str, bin_path: Path, keep_project: bool) -> None:
    case = CASES[case_id]
    temp_root = Path(os.environ.get("RUNNER_TEMP", tempfile.gettempdir()))
    project = Path(tempfile.mkdtemp(prefix=f"ael-order-{case_id}-", dir=temp_root))
    env = isolated_env(project, bin_path)

    print(f"TEST4_CASE_ID={case_id}", flush=True)
    print(f"TEST4_CASE_DESCRIPTION={case['description']}", flush=True)
    print(f"TEST4_PROJECT={project}", flush=True)

    try:
        run(["git", "init", "-q", "-b", "main"], project, env)
        run(["git", "config", "user.email", "ci@example.com"], project, env)
        run(["git", "config", "user.name", "CI"], project, env)

        for action, value in case["steps"]:
            if action == "install":
                run([str(bin_path), "install", value], project, env)
            elif action == "add":
                run([str(bin_path), "add", value], project, env)
            else:
                fail(f"unknown case action: {action}")

        manifest = load_manifest(project)
        modules = manifest.get("modules", {})
        if "aieconlab" not in modules:
            fail("manifest missing aieconlab module")

        adapters = set(manifest.get("runtimeAdapters", []))
        expected_runtimes = set(case["expected_runtimes"])
        if adapters != expected_runtimes:
            fail(
                "runtimeAdapters mismatch; "
                f"expected={sorted(expected_runtimes)} actual={sorted(adapters)}"
            )

        require_core_ael_files(project)
        if "codex" in expected_runtimes:
            require_codex_files(project)
        if "claude-code" in expected_runtimes:
            require_claude_files(project)
        if "opencode" in expected_runtimes:
            require_opencode_files(project)

        require_doctor_pass(bin_path, project, env)
        print(f"TEST4_CASE_STATUS=PASS case={case_id}", flush=True)
    finally:
        if keep_project:
            print(f"TEST4_PROJECT_RETAINED={project}", flush=True)
        else:
            shutil.rmtree(project, ignore_errors=True)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--case", required=True, choices=sorted(CASES))
    parser.add_argument("--aiplus-bin", required=True, type=Path)
    parser.add_argument(
        "--keep-project",
        action="store_true",
        help="retain the temp project for local debugging",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    bin_path = args.aiplus_bin.resolve()
    if not bin_path.is_file():
        fail(f"aiplus binary does not exist: {bin_path}")
    run_case(args.case, bin_path, args.keep_project)


if __name__ == "__main__":
    main()
