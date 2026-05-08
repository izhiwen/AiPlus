#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const pluginRoot = path.resolve(__dirname, "..");
const templateDir = path.join(pluginRoot, "templates");
const compactDir = path.join(process.cwd(), ".codex", "compact");
const checkpointsDir = path.join(compactDir, "checkpoints");
const supportedProtocolVersions = ["0.1.0"];
const supportedTemplateVersions = ["0.1.0"];
const supportedSchemaVersions = ["0.1.0"];

const requiredFiles = [
  "current-handoff.md",
  "decision-log.md",
  "agent-state-ledger.md",
  "evidence-ledger.md",
  "compact-policy.json"
];

const requiredSections = [
  "Protocol Version",
  "Last Updated",
  "Current Goal",
  "Current Phase",
  "Completed Work",
  "Open Blockers",
  "Owner Gates",
  "Next 3 Actions",
  "Do Not Do",
  "Recovery Order"
];

const ownerGateValues = ["APPROVED", "DENIED", "UNKNOWN_PENDING"];
const decisionStatuses = ["DECIDED", "PROVISIONAL", "REVERSED", "NEEDS_VERIFICATION"];
const agentStatuses = ["pending", "running", "blocked", "done", "abandoned"];
const evidenceConfidence = ["official", "vendor_blog", "github", "measured", "inferred", "unknown"];
const taskStatuses = [
  "PASS",
  "FAIL",
  "IN_PROGRESS",
  "NEEDS_VERIFICATION",
  "BLOCKED_OWNER_GATE",
  "BLOCKED_MISSING_FILES",
  "BLOCKED_EXTERNAL_ACCESS",
  "BLOCKED_UNCLEAR_GOAL"
];

class CliError extends Error {
  constructor(exitCode, message) {
    super(message);
    this.exitCode = exitCode;
  }
}

const sensitivePatterns = [
  { label: "api key", re: /\b(api[_-]?key|secret[_-]?key|access[_-]?token)\b\s*[:=]\s*["']?[A-Za-z0-9_\-]{16,}/i },
  { label: "authorization header", re: /\bAuthorization\s*:\s*(Bearer|Basic)\s+[A-Za-z0-9._~+/=-]+/i },
  { label: "jwt", re: /\beyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\b/ },
  { label: "private key", re: /-----BEGIN [A-Z ]*PRIVATE KEY-----/ },
  { label: "cookie", re: /\bCookie\s*:\s*[^;\n]+=[^;\n]+/i },
  { label: "private path", re: /\/Users\/[A-Za-z0-9._-]+|\/home\/[A-Za-z0-9._-]+|Dropbox\/|iCloud/ },
  { label: "email pii", re: /\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b/i },
  { label: "phone pii", re: /\b(?:\+?1[-.\s]?)?\(?\d{3}\)?[-.\s]\d{3}[-.\s]\d{4}\b/ },
  { label: "raw audio/transcript payload", re: /\b(BEGIN TRANSCRIPT|Speaker [0-9]+:|WEBVTT|RIFF....WAVE|provider request body|provider response body)\b/i },
  { label: "har/webrtc dump", re: /\.(har|webrtcdump)\b/i }
];

function usage() {
  console.error("Usage: node core/scripts/compactctl.mjs <init|validate|checkpoint|resume> [--force]");
}

function readText(file) {
  return fs.readFileSync(path.join(compactDir, file), "utf8");
}

function sectionBody(text, heading) {
  const re = new RegExp(`(?:^|\\n)## ${escapeRegExp(heading)}[ \\t]*\\r?\\n([\\s\\S]*?)(?=\\r?\\n## |$)`);
  const match = text.match(re);
  return match ? match[1].trim() : "";
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function nonPlaceholderLines(body) {
  return body
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith("Allowed ") && !line.includes("<ISO8601_TIMESTAMP>"));
}

function parseMarkdownTable(text, headers) {
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.startsWith("|") && !/^(\|\s*-+)/.test(line))
    .slice(1)
    .map((line) => line.split("|").slice(1, -1).map((cell) => cell.trim()))
    .filter((cells) => cells.length >= headers.length)
    .map((cells) => Object.fromEntries(headers.map((header, index) => [header, cells[index] ?? ""])));
}

function scanSensitive(files) {
  const warnings = [];
  for (const file of files) {
    const full = path.join(compactDir, file);
    if (!fs.existsSync(full)) continue;
    const text = fs.readFileSync(full, "utf8");
    for (const pattern of sensitivePatterns) {
      if (pattern.re.test(text)) warnings.push(`${file}: sensitive pattern detected (${pattern.label})`);
    }
  }
  return warnings;
}

function isPathInside(child, parent) {
  const relative = path.relative(parent, child);
  return relative === "" || (relative && !relative.startsWith("..") && !path.isAbsolute(relative));
}

function assertRuntimeWritePath(target) {
  const resolvedTarget = path.resolve(target);
  const resolvedCompactDir = path.resolve(compactDir);
  if (!isPathInside(resolvedTarget, resolvedCompactDir)) {
    throw new CliError(1, `ERROR refusing runtime write outside .codex/compact/: ${target}`);
  }

  const relative = path.relative(path.resolve(process.cwd()), resolvedTarget);
  const parts = relative.split(path.sep).filter(Boolean);
  let current = path.resolve(process.cwd());
  for (const part of parts) {
    current = path.join(current, part);
    if (!fs.existsSync(current)) continue;
    if (fs.lstatSync(current).isSymbolicLink()) {
      throw new CliError(1, `ERROR refusing runtime write through symlink: ${current}`);
    }
  }
}

function mkdirRuntime(dir) {
  assertRuntimeWritePath(dir);
  fs.mkdirSync(dir, { recursive: true });
}

function writeRuntimeFile(file, content) {
  assertRuntimeWritePath(file);
  fs.writeFileSync(file, content);
}

function collectVersionReviewItems(policy) {
  const reviewItems = [];

  if (policy) {
    checkSupportedVersion(policy.protocolVersion, supportedProtocolVersions, "compact-policy.json protocolVersion", reviewItems);
    checkSupportedVersion(policy.templateVersion, supportedTemplateVersions, "compact-policy.json templateVersion", reviewItems);
    checkSupportedVersion(policy.schemaVersion, supportedSchemaVersions, "compact-policy.json schemaVersion", reviewItems);
  }

  for (const file of requiredFiles.filter((file) => file.endsWith(".md"))) {
    if (!fs.existsSync(path.join(compactDir, file))) continue;
    const text = readText(file);
    checkSupportedVersion(sectionBody(text, "Protocol Version"), supportedProtocolVersions, `${file} Protocol Version`, reviewItems);
    const templateVersion = sectionBody(text, "Template Version");
    if (templateVersion) checkSupportedVersion(templateVersion, supportedTemplateVersions, `${file} Template Version`, reviewItems);
    const schemaVersion = sectionBody(text, "Schema Version");
    if (schemaVersion) checkSupportedVersion(schemaVersion, supportedSchemaVersions, `${file} Schema Version`, reviewItems);
  }

  return reviewItems;
}

function checkSupportedVersion(actual, supported, label, reviewItems) {
  const version = String(actual ?? "").trim();
  if (!version || !supported.includes(version)) {
    reviewItems.push(`${label} unsupported or unknown: ${version || "<missing>"}`);
  }
}

function validateState() {
  const errors = [];
  const warnings = [];
  const reviewItems = [];
  const pendingGates = [];
  const deniedGates = [];

  if (!fs.existsSync(compactDir)) {
    errors.push(".codex/compact/ is missing");
    return { ok: false, errors, warnings, reviewItems, pendingGates, deniedGates, nextSafeAction: "" };
  }

  for (const file of requiredFiles) {
    if (!fs.existsSync(path.join(compactDir, file))) errors.push(`${file} is missing`);
  }
  if (errors.length) return { ok: false, errors, warnings, reviewItems, pendingGates, deniedGates, nextSafeAction: "" };

  let policy;
  try {
    policy = JSON.parse(readText("compact-policy.json"));
  } catch (error) {
    errors.push(`compact-policy.json is invalid JSON: ${error.message}`);
  }
  reviewItems.push(...collectVersionReviewItems(policy));

  for (const file of requiredFiles.filter((file) => file.endsWith(".md"))) {
    const text = readText(file);
    for (const section of requiredSections) {
      if (!new RegExp(`^## ${escapeRegExp(section)}[ \\t]*$`, "m").test(text)) {
        errors.push(`${file} missing section: ${section}`);
      }
    }
  }

  const handoff = readText("current-handoff.md");
  const goal = sectionBody(handoff, "Current Goal");
  const phase = sectionBody(handoff, "Current Phase").split(/\s+/)[0];
  const nextActions = nonPlaceholderLines(sectionBody(handoff, "Next 3 Actions"));
  if (!goal) errors.push("current-handoff.md Current Goal is empty");
  if (!taskStatuses.includes(phase)) errors.push(`current-handoff.md Current Phase is not allowed: ${phase || "<empty>"}`);
  if (!nextActions.length) errors.push("current-handoff.md Next 3 Actions is empty");

  for (const file of requiredFiles.filter((file) => file.endsWith(".md"))) {
    const body = sectionBody(readText(file), "Owner Gates");
    const gateMatches = [...body.matchAll(/\b(APPROVED|DENIED|UNKNOWN_PENDING|[A-Z]+_[A-Z_]+)\b/g)].map((m) => m[1]);
    if (!gateMatches.length) errors.push(`${file} Owner Gates has no gate status`);
    for (const gate of gateMatches) {
      if (!ownerGateValues.includes(gate)) errors.push(`${file} Owner gate has invalid status: ${gate}`);
      if (gate === "UNKNOWN_PENDING") pendingGates.push(`${file}: ${body.split(/\r?\n/).find((line) => line.includes(gate))?.trim() ?? gate}`);
      if (gate === "DENIED") deniedGates.push(`${file}: ${body.split(/\r?\n/).find((line) => line.includes(gate))?.trim() ?? gate}`);
    }
  }

  const decisions = parseMarkdownTable(readText("decision-log.md"), ["id", "status", "decision", "rationale", "evidence"]);
  for (const row of decisions) {
    if (!decisionStatuses.includes(row.status)) errors.push(`decision-log.md invalid decision status: ${row.status}`);
  }

  const agents = parseMarkdownTable(readText("agent-state-ledger.md"), ["agent", "role", "status", "ownedScope", "lastEvidence", "nextAction"]);
  for (const row of agents) {
    if (!agentStatuses.includes(row.status)) errors.push(`agent-state-ledger.md invalid agent status: ${row.status}`);
  }

  const evidence = parseMarkdownTable(readText("evidence-ledger.md"), ["id", "confidence", "source", "finding", "artifact"]);
  for (const row of evidence) {
    if (!evidenceConfidence.includes(row.confidence)) errors.push(`evidence-ledger.md invalid evidence confidence: ${row.confidence}`);
  }

  if (policy) {
    if (policy.manualCompactOnly !== true) errors.push("compact-policy.json manualCompactOnly must be true");
    checkArray(policy.allowedOwnerGateStatuses, ownerGateValues, "allowedOwnerGateStatuses", errors);
    checkArray(policy.allowedDecisionStatuses, decisionStatuses, "allowedDecisionStatuses", errors);
    checkArray(policy.allowedAgentStatuses, agentStatuses, "allowedAgentStatuses", errors);
    checkArray(policy.allowedEvidenceConfidence, evidenceConfidence, "allowedEvidenceConfidence", errors);
    checkArray(policy.allowedTaskResultStatuses, taskStatuses, "allowedTaskResultStatuses", errors);
  }

  warnings.push(...scanSensitive(requiredFiles));
  return {
    ok: errors.length === 0 && warnings.length === 0 && reviewItems.length === 0,
    errors,
    warnings,
    reviewItems,
    pendingGates,
    deniedGates,
    nextSafeAction: nextActions[0]?.replace(/^\d+\.\s*/, "") ?? ""
  };
}

function checkArray(actual, allowed, name, errors) {
  if (!Array.isArray(actual)) {
    errors.push(`compact-policy.json ${name} must be an array`);
    return;
  }
  for (const value of actual) {
    if (!allowed.includes(value)) errors.push(`compact-policy.json ${name} invalid value: ${value}`);
  }
}

function commandInit(args) {
  assertOnlyArgs(args, ["--force"]);
  const force = args.includes("--force");
  mkdirRuntime(compactDir);
  mkdirRuntime(checkpointsDir);
  for (const file of requiredFiles) {
    const source = path.join(templateDir, file);
    const target = path.join(compactDir, file);
    if (fs.existsSync(target) && !force) {
      console.log(`SKIP ${target}`);
      continue;
    }
    let content = fs.readFileSync(source, "utf8").replaceAll("<ISO8601_TIMESTAMP>", new Date().toISOString());
    writeRuntimeFile(target, content);
    console.log(`WRITE ${target}`);
  }
  console.log("INIT_PASS");
}

function commandValidate(args) {
  assertOnlyArgs(args, []);
  const result = validateState();
  if (result.ok) {
    console.log("VALIDATION_PASS");
    console.log("Validation is structural only. Passing does not mean safe to compact.");
    process.exit(0);
  }
  for (const warning of result.warnings) console.error(`WARNING ${warning}`);
  for (const reviewItem of result.reviewItems) console.error(`UNKNOWN_NEEDS_REVIEW ${reviewItem}`);
  for (const error of result.errors) console.error(`ERROR ${error}`);
  if (result.reviewItems.length && !result.errors.length && !result.warnings.length) {
    console.error("UNKNOWN_NEEDS_REVIEW");
    process.exit(2);
  }
  console.error("VALIDATION_FAIL");
  process.exit(1);
}

function gitStatusSummary() {
  try {
    execFileSync("git", ["rev-parse", "--is-inside-work-tree"], { cwd: process.cwd(), stdio: "ignore" });
    const status = execFileSync("git", ["status", "--short"], { cwd: process.cwd(), encoding: "utf8" }).trim();
    return {
      isGitRepo: true,
      dirty: Boolean(status),
      summary: status ? status.split(/\r?\n/).slice(0, 20).map((line) => line.replace(process.cwd(), "<REPO_ROOT>")) : []
    };
  } catch {
    return { isGitRepo: false, dirty: false, summary: [] };
  }
}

function commandCheckpoint(args) {
  assertOnlyArgs(args, []);
  mkdirRuntime(checkpointsDir);
  const result = validateState();
  let status = "SAFE_TO_COMPACT";
  let exitCode = 0;
  if (result.errors.length || result.warnings.length || result.deniedGates.length) {
    status = "BLOCKED_DO_NOT_COMPACT";
    exitCode = 1;
  } else if (result.reviewItems.length || result.pendingGates.length) {
    status = "UNKNOWN_NEEDS_REVIEW";
    exitCode = 2;
  }
  const timestamp = new Date().toISOString();
  const checkpoint = {
    timestamp,
    cwd: "<REPO_ROOT>",
    validationResult: result.ok ? "PASS" : "FAIL",
    status,
    gitStatus: gitStatusSummary(),
    pendingGates: result.pendingGates,
    reviewItems: result.reviewItems,
    warnings: result.warnings,
    errors: result.errors,
    nextSafeAction: result.nextSafeAction || null,
    manualCompactOnly: true
  };
  const filename = timestamp.replace(/[:.]/g, "-");
  writeRuntimeFile(path.join(checkpointsDir, `${filename}.json`), JSON.stringify(checkpoint, null, 2) + "\n");
  for (const warning of result.warnings) console.error(`WARNING ${warning}`);
  for (const reviewItem of result.reviewItems) console.error(`UNKNOWN_NEEDS_REVIEW ${reviewItem}`);
  for (const error of result.errors) console.error(`ERROR ${error}`);
  console.log(status);
  console.log(`checkpoint=${path.join(".codex", "compact", "checkpoints", `${filename}.json`)}`);
  process.exit(exitCode);
}

function commandResume(args) {
  assertOnlyArgs(args, []);
  const result = validateState();
  if (!result.ok) {
    console.log("RESUME_BLOCKED");
    for (const warning of result.warnings) console.error(`WARNING ${warning}`);
    for (const reviewItem of result.reviewItems) console.error(`UNKNOWN_NEEDS_REVIEW ${reviewItem}`);
    for (const error of result.errors) console.error(`ERROR ${error}`);
    process.exit(result.reviewItems.length && !result.errors.length && !result.warnings.length ? 2 : 1);
  }
  const handoff = readText("current-handoff.md");
  console.log("RESUME_READY");
  console.log(`current_goal=${singleLine(sectionBody(handoff, "Current Goal"))}`);
  console.log(`current_phase=${singleLine(sectionBody(handoff, "Current Phase"))}`);
  console.log(`open_blockers=${singleLine(sectionBody(handoff, "Open Blockers"))}`);
  console.log(`owner_gates=${singleLine(sectionBody(handoff, "Owner Gates"))}`);
  console.log(`next_safe_action=${singleLine(result.nextSafeAction)}`);
  process.exit(0);
}

function singleLine(value) {
  return (value || "").replace(/\s+/g, " ").trim();
}

function assertOnlyArgs(args, allowed) {
  for (const arg of args) {
    if (!allowed.includes(arg)) throw new CliError(2, `ERROR unsupported argument: ${arg}`);
  }
}

try {
  const [command, ...args] = process.argv.slice(2);
  if (command === "init") commandInit(args);
  else if (command === "validate") commandValidate(args);
  else if (command === "checkpoint") commandCheckpoint(args);
  else if (command === "resume") commandResume(args);
  else {
    usage();
    process.exit(2);
  }
} catch (error) {
  if (error instanceof CliError) {
    console.error(error.message);
    process.exit(error.exitCode);
  }
  console.error(`INTERNAL_ERROR ${error.stack || error.message}`);
  process.exit(3);
}
