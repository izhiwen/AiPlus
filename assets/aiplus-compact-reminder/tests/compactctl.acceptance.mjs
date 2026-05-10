#!/usr/bin/env node
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";

const repoRoot = path.resolve(import.meta.dirname, "..");
const cli = path.join(repoRoot, "core", "scripts", "compactctl.mjs");
const fixtureRoot = path.join(repoRoot, "fixtures", "valid-compact");
const tests = [];

function test(name, fn) {
  tests.push({ name, fn });
}

function runCompactctl(cwd, args) {
  return spawnSync(process.execPath, [cli, ...args], {
    cwd,
    encoding: "utf8",
    maxBuffer: 1024 * 1024
  });
}

function tempTarget(prefix = "compactctl-acceptance-") {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

function copyDir(source, target) {
  fs.cpSync(source, target, { recursive: true });
}

function fixtureTarget() {
  const target = tempTarget();
  copyDir(fixtureRoot, target);
  fs.mkdirSync(path.join(target, ".codex", "compact", "checkpoints"), { recursive: true });
  updatePolicy(target, (policy) => ({
    ...policy,
    templateVersion: "0.1.0",
    schemaVersion: "0.1.0"
  }));
  return target;
}

function compactFile(target, file) {
  return path.join(target, ".codex", "compact", file);
}

function readCompact(target, file) {
  return fs.readFileSync(compactFile(target, file), "utf8");
}

function writeCompact(target, file, text) {
  fs.writeFileSync(compactFile(target, file), text);
}

function updatePolicy(target, update) {
  const policyPath = compactFile(target, "compact-policy.json");
  const policy = JSON.parse(fs.readFileSync(policyPath, "utf8"));
  fs.writeFileSync(policyPath, JSON.stringify(update(policy), null, 2) + "\n");
}

function replaceCompact(target, file, from, to) {
  const text = readCompact(target, file);
  assert.match(text, typeof from === "string" ? new RegExp(escapeRegExp(from)) : from);
  writeCompact(target, file, text.replace(from, to));
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function assertExit(result, code) {
  assert.equal(result.status, code, `expected exit ${code}, got ${result.status}\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
}

function assertStdout(result, pattern) {
  assert.match(result.stdout, pattern, `stdout did not match ${pattern}\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
}

function assertStderr(result, pattern) {
  assert.match(result.stderr, pattern, `stderr did not match ${pattern}\nstdout:\n${result.stdout}\nstderr:\n${result.stderr}`);
}

function checkpointJsonPath(target, result) {
  const match = result.stdout.match(/^checkpoint=(.+)$/m);
  assert.ok(match, `missing checkpoint line\nstdout:\n${result.stdout}`);
  return path.join(target, match[1]);
}

test("init creates required files and reports stdout/stderr contract", () => {
  const target = tempTarget();
  const result = runCompactctl(target, ["init"]);

  assertExit(result, 0);
  assertStdout(result, /WRITE .*current-handoff\.md/);
  assertStdout(result, /INIT_PASS/);
  assert.equal(result.stderr, "");

  for (const file of [
    "current-handoff.md",
    "decision-log.md",
    "agent-state-ledger.md",
    "evidence-ledger.md",
    "compact-policy.json"
  ]) {
    assert.ok(fs.existsSync(compactFile(target, file)), `${file} was not created`);
  }
  assert.ok(fs.statSync(compactFile(target, "checkpoints")).isDirectory(), "checkpoints directory was not created");
});

test("init is idempotent and preserves existing files without --force", () => {
  const target = tempTarget();
  assertExit(runCompactctl(target, ["init"]), 0);

  const handoff = compactFile(target, "current-handoff.md");
  fs.appendFileSync(handoff, "\nLOCAL_MARKER\n");
  const result = runCompactctl(target, ["init"]);

  assertExit(result, 0);
  assertStdout(result, /SKIP .*current-handoff\.md/);
  assertStdout(result, /INIT_PASS/);
  assert.equal(result.stderr, "");
  assert.match(fs.readFileSync(handoff, "utf8"), /LOCAL_MARKER/);
});

test("valid fixture validates cleanly", () => {
  const target = fixtureTarget();
  const result = runCompactctl(target, ["validate"]);

  assertExit(result, 0);
  assertStdout(result, /^VALIDATION_PASS$/m);
  assertStdout(result, /Validation is structural only/);
  assert.equal(result.stderr, "");
});

test("missing compact file fails validation with stderr errors", () => {
  const target = fixtureTarget();
  fs.rmSync(compactFile(target, "evidence-ledger.md"));

  const result = runCompactctl(target, ["validate"]);

  assertExit(result, 1);
  assert.equal(result.stdout, "");
  assertStderr(result, /ERROR evidence-ledger\.md is missing/);
  assertStderr(result, /^VALIDATION_FAIL$/m);
});

test("invalid policy JSON fails validation", () => {
  const target = fixtureTarget();
  writeCompact(target, "compact-policy.json", "{ not json\n");

  const result = runCompactctl(target, ["validate"]);

  assertExit(result, 1);
  assert.equal(result.stdout, "");
  assertStderr(result, /ERROR compact-policy\.json is invalid JSON:/);
  assertStderr(result, /^VALIDATION_FAIL$/m);
});

test("invalid enum fails validation", () => {
  const target = fixtureTarget();
  replaceCompact(target, "decision-log.md", "| DEC-001 | DECIDED |", "| DEC-001 | SHIPPED |");

  const result = runCompactctl(target, ["validate"]);

  assertExit(result, 1);
  assert.equal(result.stdout, "");
  assertStderr(result, /ERROR decision-log\.md invalid decision status: SHIPPED/);
  assertStderr(result, /^VALIDATION_FAIL$/m);
});

test("denied Owner gate blocks checkpoint", () => {
  const target = fixtureTarget();
  replaceCompact(target, "current-handoff.md", "APPROVED: Fixture owner gate", "DENIED: Fixture owner gate");

  const result = runCompactctl(target, ["checkpoint"]);

  assertExit(result, 1);
  assertStdout(result, /^BLOCKED_DO_NOT_COMPACT$/m);
  assert.equal(result.stderr, "");

  const checkpoint = JSON.parse(fs.readFileSync(checkpointJsonPath(target, result), "utf8"));
  assert.equal(checkpoint.status, "BLOCKED_DO_NOT_COMPACT");
  assert.deepEqual(checkpoint.errors, []);
  assert.equal(checkpoint.cwd, "<REPO_ROOT>");
});

test("pending Owner gates carry forward to checkpoint JSON and resume output", () => {
  const target = fixtureTarget();
  for (const file of [
    "current-handoff.md",
    "decision-log.md",
    "agent-state-ledger.md",
    "evidence-ledger.md"
  ]) {
    replaceCompact(target, file, "APPROVED:", "UNKNOWN_PENDING:");
  }

  const checkpointResult = runCompactctl(target, ["checkpoint"]);
  assertExit(checkpointResult, 2);
  assertStdout(checkpointResult, /^UNKNOWN_NEEDS_REVIEW$/m);
  assert.equal(checkpointResult.stderr, "");

  const checkpoint = JSON.parse(fs.readFileSync(checkpointJsonPath(target, checkpointResult), "utf8"));
  assert.equal(checkpoint.status, "UNKNOWN_NEEDS_REVIEW");
  assert.equal(checkpoint.validationResult, "PASS");
  assert.equal(checkpoint.pendingGates.length, 4);
  assert.ok(checkpoint.pendingGates.every((gate) => gate.includes("UNKNOWN_PENDING")));

  const resumeResult = runCompactctl(target, ["resume"]);
  assertExit(resumeResult, 0);
  assertStdout(resumeResult, /^RESUME_READY$/m);
  assertStdout(resumeResult, /owner_gates=.*UNKNOWN_PENDING/);
  assertStdout(resumeResult, /next_safe_action=Run validate\./);
  assert.equal(resumeResult.stderr, "");
});

test("missing policy templateVersion and schemaVersion require review", () => {
  const target = fixtureTarget();
  updatePolicy(target, (policy) => {
    const { templateVersion, schemaVersion, ...withoutVersions } = policy;
    return withoutVersions;
  });

  const validateResult = runCompactctl(target, ["validate"]);
  assertExit(validateResult, 2);
  assert.equal(validateResult.stdout, "");
  assertStderr(validateResult, /UNKNOWN_NEEDS_REVIEW compact-policy\.json templateVersion unsupported or unknown: <missing>/);
  assertStderr(validateResult, /UNKNOWN_NEEDS_REVIEW compact-policy\.json schemaVersion unsupported or unknown: <missing>/);
  assertStderr(validateResult, /^UNKNOWN_NEEDS_REVIEW$/m);

  const checkpointResult = runCompactctl(target, ["checkpoint"]);
  assertExit(checkpointResult, 2);
  assertStdout(checkpointResult, /^UNKNOWN_NEEDS_REVIEW$/m);
  assertStderr(checkpointResult, /UNKNOWN_NEEDS_REVIEW compact-policy\.json templateVersion unsupported or unknown: <missing>/);
  assertStderr(checkpointResult, /UNKNOWN_NEEDS_REVIEW compact-policy\.json schemaVersion unsupported or unknown: <missing>/);

  const checkpoint = JSON.parse(fs.readFileSync(checkpointJsonPath(target, checkpointResult), "utf8"));
  assert.equal(checkpoint.status, "UNKNOWN_NEEDS_REVIEW");
  assert.equal(checkpoint.validationResult, "FAIL");
  assert.deepEqual(checkpoint.errors, []);
  assert.deepEqual(checkpoint.warnings, []);
  assert.deepEqual(checkpoint.reviewItems, [
    "compact-policy.json templateVersion unsupported or unknown: <missing>",
    "compact-policy.json schemaVersion unsupported or unknown: <missing>"
  ]);
});

test("unsupported policy templateVersion and schemaVersion require review", () => {
  const target = fixtureTarget();
  updatePolicy(target, (policy) => ({
    ...policy,
    templateVersion: "9.9.9",
    schemaVersion: "9.9.9"
  }));

  const validateResult = runCompactctl(target, ["validate"]);
  assertExit(validateResult, 2);
  assert.equal(validateResult.stdout, "");
  assertStderr(validateResult, /UNKNOWN_NEEDS_REVIEW compact-policy\.json templateVersion unsupported or unknown: 9\.9\.9/);
  assertStderr(validateResult, /UNKNOWN_NEEDS_REVIEW compact-policy\.json schemaVersion unsupported or unknown: 9\.9\.9/);
  assertStderr(validateResult, /^UNKNOWN_NEEDS_REVIEW$/m);

  const checkpointResult = runCompactctl(target, ["checkpoint"]);
  assertExit(checkpointResult, 2);
  assertStdout(checkpointResult, /^UNKNOWN_NEEDS_REVIEW$/m);
  assertStderr(checkpointResult, /UNKNOWN_NEEDS_REVIEW compact-policy\.json templateVersion unsupported or unknown: 9\.9\.9/);
  assertStderr(checkpointResult, /UNKNOWN_NEEDS_REVIEW compact-policy\.json schemaVersion unsupported or unknown: 9\.9\.9/);

  const checkpoint = JSON.parse(fs.readFileSync(checkpointJsonPath(target, checkpointResult), "utf8"));
  assert.equal(checkpoint.status, "UNKNOWN_NEEDS_REVIEW");
  assert.equal(checkpoint.validationResult, "FAIL");
  assert.deepEqual(checkpoint.errors, []);
  assert.deepEqual(checkpoint.warnings, []);
  assert.deepEqual(checkpoint.reviewItems, [
    "compact-policy.json templateVersion unsupported or unknown: 9.9.9",
    "compact-policy.json schemaVersion unsupported or unknown: 9.9.9"
  ]);
});

test("secret and PII leakage warnings fail validation", () => {
  const target = fixtureTarget();
  const keyName = ["api", "key"].join("_");
  const keyValue = ["sk", "test", "secret", "1234567890"].join("_");
  const emailProbe = ["owner", "@", "example", ".", "com"].join("");
  fs.appendFileSync(compactFile(target, "current-handoff.md"), `\n${keyName} = "${keyValue}"\n${emailProbe}\n`);

  const result = runCompactctl(target, ["validate"]);

  assertExit(result, 1);
  assert.equal(result.stdout, "");
  assertStderr(result, /WARNING current-handoff\.md: sensitive pattern detected \(api key\)/);
  assertStderr(result, /WARNING current-handoff\.md: sensitive pattern detected \(email pii\)/);
  assertStderr(result, /^VALIDATION_FAIL$/m);
});

test("empty next action fails validation", () => {
  const target = fixtureTarget();
  replaceCompact(
    target,
    "current-handoff.md",
    /## Next 3 Actions\n\n[\s\S]*?\n## Do Not Do/,
    "## Next 3 Actions\n\n\n## Do Not Do"
  );

  const result = runCompactctl(target, ["validate"]);

  assertExit(result, 1);
  assert.equal(result.stdout, "");
  assertStderr(result, /ERROR current-handoff\.md Next 3 Actions is empty/);
  assertStderr(result, /^VALIDATION_FAIL$/m);
});

test("resume-blocked case reports blocked state on stdout", () => {
  const target = fixtureTarget();
  fs.rmSync(compactFile(target, "compact-policy.json"));

  const result = runCompactctl(target, ["resume"]);

  assertExit(result, 1);
  assertStdout(result, /^RESUME_BLOCKED$/m);
  assertStderr(result, /ERROR compact-policy\.json is missing/);
});

test("checkpoint writes parseable JSON output with redacted cwd", () => {
  const target = fixtureTarget();
  const result = runCompactctl(target, ["checkpoint"]);

  assertExit(result, 0);
  assertStdout(result, /^SAFE_TO_COMPACT$/m);
  assert.equal(result.stderr, "");

  const jsonPath = checkpointJsonPath(target, result);
  const checkpoint = JSON.parse(fs.readFileSync(jsonPath, "utf8"));
  assert.equal(checkpoint.status, "SAFE_TO_COMPACT");
  assert.equal(checkpoint.validationResult, "PASS");
  assert.equal(checkpoint.cwd, "<REPO_ROOT>");
  assert.equal(checkpoint.nextSafeAction, "Run validate.");
  assert.equal(checkpoint.manualCompactOnly, true);
  assert.deepEqual(checkpoint.warnings, []);
  assert.deepEqual(checkpoint.errors, []);
  assert.ok(!JSON.stringify(checkpoint).includes(target), "checkpoint leaked target temp path");
});

test("unknown command exits with usage contract", () => {
  const target = tempTarget();
  const result = runCompactctl(target, ["nope"]);

  assertExit(result, 2);
  assert.equal(result.stdout, "");
  assertStderr(result, /^Usage: node core\/scripts\/compactctl\.mjs <init\|validate\|checkpoint\|resume> \[--force\]$/m);
});

let passed = 0;
const failures = [];
for (const { name, fn } of tests) {
  try {
    fn();
    passed += 1;
    console.log(`ok ${passed + failures.length} - ${name}`);
  } catch (error) {
    failures.push({ name, error });
    console.log(`not ok ${passed + failures.length} - ${name}`);
    console.log(error?.stack || error);
  }
}

console.log(`1..${tests.length}`);
console.log(`# ${passed}/${tests.length} compactctl acceptance tests passed`);
if (failures.length) {
  console.log(`# ${failures.length} compactctl acceptance test(s) failed`);
  process.exit(1);
}
