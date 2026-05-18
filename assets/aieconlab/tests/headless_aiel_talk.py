#!/usr/bin/env python3
"""Headless TEST-3 harness for AiEconLab /aiel-talk behavior.

This intentionally validates runtime output, not installed file presence.
Claude Code and OpenCode have native headless command paths. Codex currently
does not: AiPlus `agent talk --runtime codex` launches interactive `codex`
instead of `codex exec`, so the harness records that limitation as an explicit
skip with evidence.
"""

from __future__ import annotations

import argparse
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass


DEFAULT_ROLES = ("advisor", "pi", "ra-stata", "referee")
ROLE_MARKERS = {
    "advisor": ("advisor",),
    "pi": ("pi", "principal investigator", "principal-investigator"),
    "ra-stata": ("ra-stata", "stata"),
    "referee": ("referee",),
}
PERSONA_MARKER_GROUPS = {
    "advisor": (
        ("research", "question", "framing", "agenda"),
        ("identification", "strategy", "design", "credibility"),
        ("paper", "publication", "tradeoff", "risk"),
        ("responsibility", "duty", "role", "task"),
    ),
    "pi": (
        ("research", "project", "milestone", "scope"),
        ("dispatch", "coordinate", "integrate", "execution"),
        ("artifact", "replicator", "rerun", "clean-room"),
        ("responsibility", "duty", "role", "task"),
    ),
    "ra-stata": (
        ("regression", "specification", "estimate", "model"),
        ("analysis", "table", "empirical", "coefficient"),
        ("data", "dataset", "results", "robustness"),
        ("responsibility", "duty", "role", "task"),
    ),
    "referee": (
        ("review", "critique", "submission", "pre-review"),
        ("methodology", "argument", "structure", "rigor"),
        ("manuscript", "paper", "evidence", "coherence"),
        ("responsibility", "duty", "role", "task"),
    ),
}


def persona_marker_ok(role: str, lower: str) -> bool:
    """Accept any role-specific persona marker from the relaxed groups."""
    for group in PERSONA_MARKER_GROUPS[role]:
        if any(marker in lower for marker in group):
            return True
    return False


@dataclass(frozen=True)
class CommandResult:
    args: list[str]
    returncode: int
    stdout: str
    stderr: str

    @property
    def combined(self) -> str:
        return "\n".join(part for part in (self.stdout, self.stderr) if part)


def run(
    args: list[str],
    *,
    cwd: pathlib.Path,
    env: dict[str, str],
    timeout: int = 120,
    check: bool = True,
) -> CommandResult:
    print(f"+ {' '.join(args)}", flush=True)
    proc = subprocess.run(
        args,
        cwd=cwd,
        env=env,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
    )
    result = CommandResult(args, proc.returncode, proc.stdout, proc.stderr)
    if check and proc.returncode != 0:
        print(result.combined[-4000:], file=sys.stderr)
        raise SystemExit(f"command failed with exit {proc.returncode}: {' '.join(args)}")
    return result


def github_notice(kind: str, title: str, message: str) -> None:
    escaped = message.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")
    print(f"::{kind} title={title}::{escaped}")


def require_binary(name: str) -> None:
    if shutil.which(name) is None:
        raise SystemExit(f"required binary not found on PATH: {name}")


def require_env(name: str, runtime: str) -> None:
    if not os.environ.get(name):
        raise SystemExit(f"{runtime} requires GitHub secret/env {name}; it is not set")


def isolated_env(runtime: str, aiplus_bin: pathlib.Path) -> dict[str, str]:
    env = os.environ.copy()
    env["PATH"] = f"{aiplus_bin.parent}:{env.get('PATH', '')}"
    # Keep provider API keys from the workflow environment, but isolate runtime
    # config/cache writes from the runner home.
    home = pathlib.Path(tempfile.mkdtemp(prefix=f"ael-test3-{runtime}-home-"))
    env["HOME"] = str(home)
    env["USERPROFILE"] = str(home)
    env["CODEX_HOME"] = str(home / "codex")
    env["XDG_CONFIG_HOME"] = str(home / "xdg")
    env["CLAUDE_CONFIG_DIR"] = str(home / "claude")
    env["OPENCODE_CONFIG_DIR"] = str(home / "opencode")
    for path in (
        pathlib.Path(env["CODEX_HOME"]),
        pathlib.Path(env["XDG_CONFIG_HOME"]),
        pathlib.Path(env["CLAUDE_CONFIG_DIR"]),
        pathlib.Path(env["OPENCODE_CONFIG_DIR"]),
    ):
        path.mkdir(parents=True, exist_ok=True)
    return env


def create_project(runtime: str, aiplus_bin: pathlib.Path, env: dict[str, str]) -> pathlib.Path:
    project = pathlib.Path(tempfile.mkdtemp(prefix=f"ael-test3-{runtime}-project-"))
    run(["git", "init", "-q", "-b", "main"], cwd=project, env=env)
    run(["git", "config", "user.email", "ci@example.com"], cwd=project, env=env)
    run(["git", "config", "user.name", "CI"], cwd=project, env=env)
    run([str(aiplus_bin), "install", runtime], cwd=project, env=env, timeout=180)
    run([str(aiplus_bin), "add", "aieconlab"], cwd=project, env=env, timeout=180)
    run([str(aiplus_bin), "agent", "set-team", "aieconlab"], cwd=project, env=env, timeout=180)
    return project


def assert_persona_output(runtime: str, role: str, result: CommandResult) -> None:
    output = result.combined.strip()
    lower = output.lower()
    if result.returncode != 0:
        print(output[-4000:], file=sys.stderr)
        raise SystemExit(f"{runtime}/{role} exited {result.returncode}")
    forbidden = ("unknown command", "command not found", "not found: /aiel-talk", "unrecognized command")
    if any(token in lower for token in forbidden):
        print(output[-4000:], file=sys.stderr)
        raise SystemExit(f"{runtime}/{role} did not execute /aiel-talk cleanly")
    missing = ["aieconlab"] if "aieconlab" not in lower else []
    responsibility_ok = persona_marker_ok(role, lower)
    role_ok = any(marker in lower for marker in ROLE_MARKERS[role])
    if not responsibility_ok:
        missing.append(f"persona marker for {role}")
    if not role_ok:
        missing.append(f"one of {ROLE_MARKERS[role]}")
    if missing:
        print(output[-4000:], file=sys.stderr)
        raise SystemExit(f"{runtime}/{role} missing persona markers: {', '.join(missing)}")
    print(f"TEST3_RESULT runtime={runtime} role={role} status=PASS")


def role_prompt(role: str) -> str:
    return (
        f"What is your role? Reply in 2-4 sentences. Identify your role as {role}, "
        "say that you are part of AiEconLab, and describe one concrete research "
        "responsibility from your persona."
    )


def run_claude(project: pathlib.Path, roles: tuple[str, ...], env: dict[str, str]) -> None:
    require_env("ANTHROPIC_API_KEY", "claude-code")
    require_binary("claude")
    model = env.get("AEL_CLAUDE_MODEL", "sonnet")
    for role in roles:
        result = run(
            [
                "claude",
                "--print",
                "--dangerously-skip-permissions",
                "--model",
                model,
                "--max-budget-usd",
                env.get("AEL_CLAUDE_MAX_BUDGET_USD", "0.25"),
                f"/aiel-talk {role} {role_prompt(role)}",
            ],
            cwd=project,
            env=env,
            timeout=int(env.get("AEL_TEST3_TIMEOUT_SECONDS", "240")),
            check=False,
        )
        assert_persona_output("claude-code", role, result)


def run_opencode(project: pathlib.Path, roles: tuple[str, ...], env: dict[str, str]) -> None:
    require_env("OPENAI_API_KEY", "opencode")
    require_binary("opencode")
    model = env.get("AEL_OPENCODE_MODEL", "openai/gpt-4o-mini")
    for role in roles:
        result = run(
            [
                "opencode",
                "run",
                "--dir",
                str(project),
                "--model",
                model,
                "--command",
                "aiel-talk",
                "--dangerously-skip-permissions",
                role,
                role_prompt(role),
            ],
            cwd=project,
            env=env,
            timeout=int(env.get("AEL_TEST3_TIMEOUT_SECONDS", "240")),
            check=False,
        )
        assert_persona_output("opencode", role, result)


def skip_codex_with_evidence(
    project: pathlib.Path,
    aiplus_bin: pathlib.Path,
    env: dict[str, str],
    aiplus_source: pathlib.Path | None,
) -> None:
    require_binary("codex")
    help_result = run(
        [str(aiplus_bin), "agent", "talk", "--help"],
        cwd=project,
        env=env,
        timeout=30,
        check=False,
    )
    codex_help = run(["codex", "exec", "--help"], cwd=project, env=env, timeout=30, check=False)
    evidence = [
        "`aiplus agent talk --help` exposes no prompt/print/headless flag; current builds expose only runtime selection plus `<ROLE>`.",
        "`codex exec` is the available non-interactive Codex mode, but AiPlus talk currently invokes top-level interactive `codex`.",
        "Tracked upstream as https://github.com/izhiwen/AiPlus/issues/97.",
    ]
    if aiplus_source:
        talk_rs = aiplus_source / "crates" / "aiplus-cli" / "src" / "agent" / "talk.rs"
        if talk_rs.exists():
            text = talk_rs.read_text(encoding="utf-8")
            if '"codex"' in text and "args: vec![]" in text and "cmd.args(&runtime.args).arg(prompt)" in text:
                evidence.append(
                    "Source evidence: `talk.rs` sets Codex args to `vec![]` and appends the persona prompt to interactive `codex`."
                )
    print(help_result.combined)
    print(codex_help.combined[:2000])
    message = " ".join(evidence)
    github_notice("warning", "TEST-3 Codex headless skip", message)
    print(f"TEST3_RESULT runtime=codex role=ALL status=SKIP reason={message}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--runtime", choices=("claude-code", "codex", "opencode"), required=True)
    parser.add_argument("--aiplus-bin", type=pathlib.Path, required=True)
    parser.add_argument("--aiplus-source", type=pathlib.Path)
    parser.add_argument("--roles", nargs="+", default=list(DEFAULT_ROLES))
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    aiplus_bin = args.aiplus_bin.resolve()
    if not aiplus_bin.exists():
        raise SystemExit(f"aiplus binary not found: {aiplus_bin}")
    roles = tuple(args.roles)
    unknown_roles = sorted(set(roles) - set(DEFAULT_ROLES))
    if unknown_roles:
        raise SystemExit(f"unsupported TEST-3 role(s): {', '.join(unknown_roles)}")

    env = isolated_env(args.runtime, aiplus_bin)
    if args.runtime == "claude-code":
        require_env("ANTHROPIC_API_KEY", "claude-code")
    elif args.runtime == "opencode":
        require_env("OPENAI_API_KEY", "opencode")

    project = create_project(args.runtime, aiplus_bin, env)
    print(f"TEST3_PROJECT={project}")

    if args.runtime == "claude-code":
        run_claude(project, roles, env)
    elif args.runtime == "opencode":
        run_opencode(project, roles, env)
    else:
        skip_codex_with_evidence(project, aiplus_bin, env, args.aiplus_source)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
