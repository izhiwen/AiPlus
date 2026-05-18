#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

python3 - <<'PY'
import json
import pathlib
import re

ROOT = pathlib.Path(".")
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
RUNTIME_ADAPTERS = ["codex", "claude-code", "opencode"]


def fail(message: str) -> None:
    raise SystemExit(message)


def parse_value(raw: str):
    raw = raw.strip()
    if raw.startswith('"') and raw.endswith('"'):
        return raw[1:-1]
    if raw.startswith("[") and raw.endswith("]"):
        items = []
        for item in raw[1:-1].split(","):
            item = item.strip()
            if not item:
                continue
            if not (item.startswith('"') and item.endswith('"')):
                fail(f"unsupported TOML array item: {raw}")
            items.append(item[1:-1])
        return items
    return raw


def load_agent_toml(path: pathlib.Path) -> dict:
    data: dict[str, dict] = {}
    section: str | None = None
    for line in path.read_text().splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        match = re.fullmatch(r"\[([A-Za-z0-9_-]+)\]", stripped)
        if match:
            section = match.group(1)
            data.setdefault(section, {})
            continue
        if "=" not in stripped or section is None:
            continue
        key, value = stripped.split("=", 1)
        data[section][key.strip()] = parse_value(value)
    return data


def load_subagents(path: pathlib.Path) -> list[dict]:
    subagents: list[dict] = []
    current: dict | None = None
    for line in path.read_text().splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped == "[[subagent]]":
            current = {}
            subagents.append(current)
            continue
        if "=" not in stripped or current is None:
            continue
        key, value = stripped.split("=", 1)
        current[key.strip()] = parse_value(value)
    return subagents


manifest_path = ROOT / "aiplus-module.json"
manifest = json.loads(manifest_path.read_text())

if manifest.get("name") != "aieconlab":
    fail("aiplus-module.json name must be aieconlab")

adapters = manifest.get("runtimeAdapters")
if adapters != RUNTIME_ADAPTERS:
    fail(f"runtimeAdapters mismatch: expected {RUNTIME_ADAPTERS}, got {adapters}")

missing_required = [
    path for path in manifest.get("requiredFiles", [])
    if not (ROOT / path).is_file()
]
if missing_required:
    fail(f"manifest requiredFiles missing: {missing_required}")

expert_tomls = sorted((ROOT / "core/templates/experts").glob("*.toml"))
expert_roles = [path.stem for path in expert_tomls]
if len(expert_roles) != 14:
    fail(f"expected 14 expert specs, found {len(expert_roles)}")

expected_roles = CORE_ROLES + expert_roles

for role in CORE_ROLES:
    role_toml = ROOT / "core/templates" / f"{role}.toml"
    persona = ROOT / "core/templates/personas" / f"{role}.md"
    if not role_toml.is_file():
        fail(f"missing core role spec: {role_toml}")
    if not persona.is_file():
        fail(f"missing core role persona: {persona}")
    spec = load_agent_toml(role_toml)
    if spec.get("agent", {}).get("role") != role:
        fail(f"{role_toml}: [agent].role mismatch")
    if spec.get("persona", {}).get("system_prompt_file") != f"personas/{role}.md":
        fail(f"{role_toml}: persona.system_prompt_file mismatch")

for role, path in zip(expert_roles, expert_tomls):
    persona = ROOT / "core/templates/personas" / f"{role}.md"
    if not persona.is_file():
        fail(f"missing expert persona: {persona}")
    spec = load_agent_toml(path)
    if spec.get("agent", {}).get("role") != role:
        fail(f"{path}: [agent].role mismatch")
    if spec.get("agent", {}).get("tier") != "expert":
        fail(f"{path}: expert tier mismatch")
    if spec.get("persona", {}).get("system_prompt_file") != f"personas/{role}.md":
        fail(f"{path}: persona.system_prompt_file mismatch")

for adapter in RUNTIME_ADAPTERS:
    readme = ROOT / "adapters" / adapter / "README.md"
    if not readme.is_file():
        fail(f"missing adapter README: {readme}")

expected_subagent_names = {f"aieconlab-{role}" for role in expected_roles}
expected_personas = {
    f"aieconlab-{role}": f"core/templates/personas/{role}.md"
    for role in expected_roles
}

for adapter in ["claude-code", "opencode"]:
    path = ROOT / "adapters" / adapter / "subagents.toml"
    subagents = load_subagents(path)
    names = {entry.get("name") for entry in subagents}
    if names != expected_subagent_names:
        missing = sorted(expected_subagent_names - names)
        extra = sorted(names - expected_subagent_names)
        fail(f"{path}: subagent mirror mismatch missing={missing} extra={extra}")
    for entry in subagents:
        name = entry["name"]
        persona_file = entry.get("persona_file")
        if persona_file != expected_personas[name]:
            fail(f"{path}: {name} persona_file mismatch: {persona_file}")
        if not entry.get("description", "").strip():
            fail(f"{path}: {name} missing description")

codex_mirror_path = ROOT / "adapters/codex/subagents.toml"
codex_entries = load_subagents(codex_mirror_path)
codex_new_names = {entry.get("name") for entry in codex_entries}
expected_new_names = {"aieconlab-dof-auditor", "aieconlab-rr-strategist"}
if codex_new_names != expected_new_names:
    fail(
        f"{codex_mirror_path}: v0.2.1 beta mirror mismatch "
        f"missing={sorted(expected_new_names - codex_new_names)} "
        f"extra={sorted(codex_new_names - expected_new_names)}"
    )
for entry in codex_entries:
    name = entry["name"]
    if entry.get("persona_file") != expected_personas[name]:
        fail(f"{codex_mirror_path}: {name} persona_file mismatch")
    if not entry.get("description", "").strip():
        fail(f"{codex_mirror_path}: {name} missing description")

alias_owner: dict[str, str] = {}
for role, path in zip(expert_roles, expert_tomls):
    spec = load_agent_toml(path)
    invocation = spec.get("invocation", {})
    aliases = invocation.get("english_aliases", []) + invocation.get("chinese_aliases", [])
    if not aliases:
        fail(f"{path}: missing invocation aliases")
    for alias in aliases:
        key = alias.strip().lower()
        if not key:
            fail(f"{path}: empty invocation alias")
        previous = alias_owner.setdefault(key, role)
        if previous != role:
            fail(
                f"expert trigger collision: alias {alias!r} belongs to "
                f"both {previous} and {role}"
            )

print("TIER2_R1_MANIFEST_CONSISTENCY=PASS")
print("TIER2_R2_TRIGGER_COLLISION=PASS")
PY
