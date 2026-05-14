# AiPlus Agent Velocity Design

Status: draft v0.1
Scope: local-first human-time bias correction for AI agent task estimates

## 1. One-Line Positioning

`aiplus-agent-velocity` corrects human-time bias in AI agent task estimates.

In plain language:

AiPlus Agent Velocity helps a CEO agent stop estimating AI work as if a human engineer were doing it. It records how long similar AI agent tasks actually took, detects large overestimates such as "planned 5h, finished in 20m", and uses that evidence to produce shorter, AI-native estimates next time.

## 2. Problem

Agents often overestimate how long they or other agents need to complete a task.

The suspected root cause is human-time anchoring:

- The agent has seen many examples of human engineering estimates.
- It maps task size to human engineer time.
- It then applies that human estimate to a strong LLM agent.
- The result is severe overestimation of AI agent work time.

Example:

```text
Human-style estimate: 5h
Actual strong-agent time: 20m
Overestimate ratio: 15x
```

This causes several product and workflow problems:

- CEO agents create task prompts that are too small for modern models.
- Overnight or 5-hour work windows are underused.
- Agents may think a fast completion is suspicious instead of recognizing an estimate error.
- Future prompts repeat the same overestimation because no calibration record exists.
- Owner planning becomes distorted by human-time assumptions.

## 3. Non-Goals

This project is not:

- a model leaderboard
- an agent KPI system
- a productivity surveillance tool
- a billing or cost-estimation system
- a project-management dashboard
- a replacement for tests, review, or Owner gates
- a long-term data warehouse
- a cloud telemetry system
- a raw transcript capture system

Velocity estimates are local operational calibration signals. They are not quality guarantees or release approval.

## 4. Core Principle

Estimate speed, but protect quality.

A shorter AI-native estimate is not permission to skip:

- tests
- review
- bad-state checks
- documentation sync
- secret/private boundary checks
- Owner gates

Fast completion should trigger verification and calibration, not busywork.

## 5. Product Model

AiPlus Agent Velocity performs five jobs:

1. Record the original estimate.
2. Identify whether the estimate looks human-time anchored.
3. Record the actual AI agent active time after completion.
4. Compute overestimate ratio and observed AI speedup.
5. Use recent records to produce future AI-native estimates.

## 6. Key Concepts

### Human Estimate

The time a human engineer might need, or an agent's initial estimate when it likely comes from human engineering intuition.

### AI-Native Estimate

The estimated time a strong AI agent should need, based on local history and task characteristics.

### Actual Active Time

The time the AI agent actively worked, excluding Owner waiting time and external blockers when possible.

### Wall Clock Time

The total elapsed time from task start to task finish.

### Tool Wait Time

Time spent waiting for commands, tests, builds, downloads, or long-running tools.

### Human-Time Bias

A detected pattern where the original estimate is close to human engineering time, but actual AI completion time is much shorter and consistent with AI history.

### Overestimate Ratio

```text
overestimate_ratio = original_estimate_minutes / actual_active_minutes
```

Example:

```text
300m / 20m = 15x
```

### Observed AI Speedup

```text
observed_ai_speedup = human_baseline_minutes / actual_active_minutes
```

### Time Source Policy

v0.1 should not try to infer exact active work time from raw transcripts, provider logs, or background monitoring.

Allowed time sources:

- explicit `estimate` command input
- explicit `complete --actual ...` command input
- optional `start` / `complete` timestamps if a future command adds `start`
- Result Packet metadata when already provided by an agent

Forbidden time sources:

- raw session transcript timing
- provider request/response logs
- background activity monitoring
- editor telemetry
- shell history scraping

If exact active time is unknown, the CLI should accept a conservative manual value and mark:

```text
actual_time_source=manual
confidence=low|medium
```

## 7. MVP Scope

v0.1 should be deliberately small:

- local JSON/JSONL storage only
- no SQLite
- no cloud sync
- no dashboard
- no telemetry
- no automatic memory promotion
- no raw prompt or transcript storage

The MVP should prove one thing:

AiPlus can detect and quantify cases where agent task time was dramatically overestimated because of human-time anchoring.

## 8. Local Storage

Default path:

```text
.aiplus/velocity/
  config.json
  estimates.jsonl
  runs.jsonl
  anchor-signals.jsonl
  rare-cases.jsonl
  multipliers.json
  aggregates.json
  rotation-state.json
```

### config.json

Stores retention and safety settings.

Recommended defaults:

```json
{
  "schemaVersion": 1,
  "maxRecords": 200,
  "rareCaseMaxRecords": 20,
  "maxBytesPerJsonl": 1048576,
  "retainDays": 90,
  "minBucketSamples": 8,
  "rawContentAllowed": false,
  "memoryIntegration": "disabled"
}
```

Retention precedence:

1. Always keep at most `maxRecords` normal records.
2. Always keep at most `rareCaseMaxRecords` rare-case records.
3. `retainDays` is a soft freshness window for confidence reporting, not a reason to delete all sparse history if the total record count is still small.
4. `multipliers.json` may retain aggregate statistics after raw records rotate out.

### estimates.jsonl

One line per pre-task estimate.

### runs.jsonl

One line per completed task run.

### anchor-signals.jsonl

One line per human-time anchoring detection event.

### rare-cases.jsonl

Small retained set of unusual or important calibration examples.

Rare case examples:

- overestimate ratio >= 5x
- severe underestimate
- Owner gate involved
- release/deploy task
- security/privacy boundary issue
- external dependency blocker
- test failure caused major delay

### multipliers.json

Lightweight aggregate calibration by bucket.

### aggregates.json

Global summary and report-friendly counts.

### rotation-state.json

Tracks retention runs, records pruned, and last rotation time.

## 9. Retention Strategy

No SQLite. No unlimited history.

Default:

```text
keep latest 200 normal records
keep latest 20 rare cases
keep aggregate multipliers
delete older raw records
```

Retention is a product feature, not an implementation detail. The project optimizes for recent calibration, not permanent analytics.

Retention steps after each write:

1. Redact and validate the new record.
2. Append to JSONL.
3. Update `multipliers.json`.
4. Classify whether the event is a rare case.
5. Trim normal records to latest 200.
6. Trim rare cases to latest 20.
7. Update `rotation-state.json`.
8. Run lightweight sensitive-pattern scan.

Normal retention should not interrupt the user.

Only warn when:

- write fails
- JSONL is corrupted
- retention reduces matched sample count below 3
- confidence drops from high/medium to low
- a secret/private/raw transcript pattern is found

## 10. Record Schemas

### Estimate Record

```json
{
  "schemaVersion": 1,
  "id": "est_...",
  "taskId": "task_...",
  "createdAt": "2026-05-10T00:00:00Z",
  "projectId": "aiplus",
  "taskType": "compact-hardening",
  "repoArea": "aiplus-public",
  "agentRole": "ceo",
  "runtime": "opencode",
  "model": "kimi-k2.7",
  "workflowLevel": "HEAVY",
  "estimateBasis": "human_engineer_time",
  "humanBaselineMinutes": 300,
  "humanBaselineSource": "owner_estimate|agent_initial_estimate|manual|unknown",
  "humanEstimateMinutes": 300,
  "aiNativeEstimateP50Minutes": 45,
  "aiNativeEstimateP90Minutes": 90,
  "confidence": "medium",
  "matchedRecords": 12,
  "humanAnchorSignals": ["multi_hour_language", "similar_ai_tasks_finished_fast"],
  "stopWhenDone": true
}
```

### Run Record

```json
{
  "schemaVersion": 1,
  "id": "run_...",
  "estimateId": "est_...",
  "taskId": "task_...",
  "createdAt": "2026-05-10T00:30:00Z",
  "projectId": "aiplus",
  "taskType": "compact-hardening",
  "repoArea": "aiplus-public",
  "agentRole": "ceo",
  "runtime": "opencode",
  "model": "kimi-k2.7",
  "workflowLevel": "HEAVY",
  "originalEstimateMinutes": 300,
  "humanBaselineMinutes": 300,
  "actualActiveMinutes": 20,
  "actualTimeSource": "manual|start_complete_delta|result_packet|unknown",
  "wallClockMinutes": 24,
  "toolWaitMinutes": 3,
  "blockedMinutes": 0,
  "outcome": "pass",
  "verificationDepth": "targeted",
  "qualityVerdict": "pass",
  "reworkCount": 0,
  "ownerGateHit": false,
  "overestimateRatio": 15.0,
  "humanTimeBias": true,
  "slowReason": "none",
  "redactionStatus": "pass"
}
```

### Multiplier Bucket

```json
{
  "schemaVersion": 1,
  "updatedAt": "2026-05-10T00:00:00Z",
  "buckets": {
    "model:kimi-k2.7|project:aiplus|task:compact-hardening|workflow:HEAVY": {
      "modelKey": "kimi-k2.7",
      "modelFamily": "kimi",
      "sampleCount": 14,
      "actualAiP50Minutes": 28,
      "actualAiP80Minutes": 55,
      "overestimateRatioP50": 7.8,
      "observedSpeedupP50": 10.2,
      "humanBiasRate": 0.71,
      "confidence": "medium",
      "staleForCurrentModel": false,
      "lastUpdatedAt": "2026-05-10T00:00:00Z",
      "recentActualMinutes": [20, 24, 31, 35, 42]
    }
  }
}
```

## 11. CLI Commands

### init

```bash
aiplus velocity init
```

Creates `.aiplus/velocity/` and default config.

### estimate

```bash
aiplus velocity estimate \
  --task-type compact-hardening \
  --human-estimate 5h \
  --model kimi-k2.7 \
  --workflow HEAVY
```

Output:

```text
VELOCITY_ESTIMATE_STATUS=PASS
HUMAN_ANCHOR_DETECTED=yes
HUMAN_ESTIMATE_MINUTES=300
AI_NATIVE_ESTIMATE_P50_MINUTES=45
AI_NATIVE_ESTIMATE_P90_MINUTES=90
EXPECTED_SPEEDUP=4x-12x
MATCHED_RECORDS=12
CONFIDENCE=medium
STOP_WHEN_DONE=yes
```

### complete

```bash
aiplus velocity complete \
  --task-id task_123 \
  --actual 20m \
  --outcome pass
```

Output:

```text
VELOCITY_COMPLETE_STATUS=PASS
ACTUAL_ACTIVE_MINUTES=20
OVERESTIMATE_RATIO=15.0
HUMAN_TIME_BIAS=detected
RETENTION_STATUS=applied
```

### bias

```bash
aiplus velocity bias --task task_123
```

Output:

```text
VELOCITY_BIAS_STATUS=PASS
BIAS_TYPE=OVERESTIMATE
OVERESTIMATE_RATIO=15.0
HUMAN_TIME_BIAS_FOUND=yes
NEXT_ESTIMATE_ADJUSTMENT=shorten_same_type
```

### report

```bash
aiplus velocity report
```

Shows recent calibration summary:

```text
VELOCITY_REPORT_STATUS=PASS
CALIBRATION_WINDOW=latest_200
MATCHED_RECORDS=...
MEDIAN_OVERESTIMATE_RATIO=...
HUMAN_TIME_BIAS_RATE=...
```

### doctor

```bash
aiplus velocity doctor
```

Checks:

- velocity directory exists
- JSON and JSONL parse
- schema versions supported
- record IDs are unique
- required fields exist
- time values are non-negative
- `actualActiveMinutes <= wallClockMinutes`
- multiplier values are finite
- file size and record count under threshold
- rotation needed or not
- multipliers stale or not
- no secret/private/raw transcript/provider payload patterns
- no memory integration drift

Output:

```text
VELOCITY_DOCTOR_STATUS=PASS|NEEDS_ROTATION|NEEDS_FIX
records_count=...
rotation_needed=yes|no
bad_jsonl_lines=...
secret_values=none|found
raw_content_found=no|yes
```

### purge

```bash
aiplus velocity purge --yes
```

Deletes local velocity records. This is local cleanup only.

## 12. Estimation Algorithm

Estimate flow:

1. Read config and recent records.
2. Match by bucket:
   - model + project + task type + workflow
   - model + task type + workflow
   - task type + workflow
   - task type
   - global
3. If enough samples exist, use AI-native bucket.
4. If not enough samples exist, use broad range with low confidence.
5. Treat human estimate as an anchor, not as the default center.
6. Output p50/p90 range and confidence.

Bucket fallback order:

```text
model:project:taskType:workflow
model:taskType:workflow
taskType:workflow
taskType
global
```

Sample rules:

```text
sampleCount >= 20: high confidence possible
sampleCount 8-19: medium confidence possible
sampleCount 3-7: low confidence, broad range only
sampleCount < 3: fallback bucket required
```

Model freshness rule:

Do not let old-model data dominate estimates for a newer, stronger model.

```text
same model bucket: primary when sampleCount >= 8
same model family bucket: fallback with confidence penalty
global task bucket: fallback only, never high confidence
old model data: weak prior only
```

If recent runs for the current model are at least 2x faster than the older bucket, mark:

```text
staleForCurrentModel=true
```

## 13. Bias Detection

Human-time bias is likely when:

```text
originalEstimateMinutes >= 2 * aiNativePriorMinutes
actualActiveMinutes <= aiNativeP75Minutes
noMajorSlowReason=true
qualityVerdict=pass
```

When `humanBaselineMinutes` is unavailable, the CLI can still compute `overestimateRatio`, but it must not claim `observedAiSpeedup` or strong human-time bias. It should output:

```text
HUMAN_BASELINE_STATUS=missing
HUMAN_TIME_BIAS_CONFIDENCE=low
```

Large overestimate:

```text
originalEstimateMinutes >= 240
actualActiveMinutes <= 30
overestimateRatio >= 5
```

True slow should be classified separately when actual active time is high and there is a clear slow reason:

- scope expanded
- ambiguous requirements
- test failures
- tool failure
- external dependency
- Owner gate
- review rework
- security gate
- large context recovery

## 14. Fast-Finish Discipline

If actual time is less than 50% of the AI estimate p50, run fast-finish calibration.

Fast finish does not mean "fill the remaining time."

It means:

1. Verify acceptance criteria.
2. Run relevant tests/checks.
3. Confirm docs/evidence if required.
4. Confirm safety boundaries.
5. If complete, stop and record calibration.
6. If incomplete, only fix the missing acceptance gap.

Required output:

```text
FAST_FINISH_CALIBRATION
ORIGINAL_ESTIMATE=...
ACTUAL_TIME=...
ACCEPTANCE_STATUS=pass|needs_fix
ERROR_RATIO=...
CAUSE=human_time_anchoring|scope_smaller_than_expected|acceptance_gap|tooling_fast
NEXT_ESTIMATE_ADJUSTMENT=...
```

## 15. Safety And Privacy

Allowed:

- task metadata
- relative repo area
- task type
- model/runtime label
- estimate and actual minutes
- status and verification depth
- short redacted calibration notes

Forbidden:

- raw prompt
- raw transcript
- provider request/response
- secret values
- API keys
- auth headers
- private keys
- full private paths
- full stdout logs
- source file contents
- full diffs
- private user profile content

Velocity records must not be automatically injected into memory context.

Only a reviewed aggregate insight may later be proposed as memory, for example:

```text
In this repo, compact hardening tasks with clear tests are usually overestimated 5x-10x.
```

## 16. Integration With Other AiPlus Modules

### Auto Team Consultant

Before CEO task assignment:

- run velocity estimate
- include AI-native p50/p90 in Task Card
- treat human estimate as anchor only

After completion:

- record actual agent time
- update bias and multiplier records

### Auto Compact

Compact checkpoints may reference active velocity task ID.

Resume should continue the same task run, not create a new fake run.

### Agent Memory

Velocity records are separate from memory.

Memory may only receive reviewed aggregate insights, never raw velocity events.

### Profile Bundle

Profile preferences may include user preference such as:

```text
Prefer AI-native estimates over human engineer estimates.
```

But profile files should not store raw velocity history.

## 17. Naming And Messaging

Project name:

```text
aiplus-agent-velocity
```

Recommended public phrase:

```text
Agent Velocity
```

Recommended tagline:

```text
Calibrate AI agent time, not human hours.
```

Avoid:

- productivity tracker
- performance dashboard
- engineer-hours saved
- model benchmark
- KPI

## 18. North America Naming Evaluation

`Agent Velocity` is understandable to technical audiences in North America, especially engineers, founders, and AI-tool users.

Strengths:

- "velocity" is familiar in software teams and startup language.
- It suggests speed, throughput, and iteration.
- It is short and brandable.
- It fits AiPlus naming style.

Weaknesses:

- Some programmers may associate "velocity" with Agile story points.
- Non-technical users may not immediately know it means time calibration.
- It could be mistaken for a productivity tracker unless the tagline is clear.
- "Velocity" alone does not explicitly communicate "human-time bias correction."

Recommendation:

Keep the name, but always pair it with a clear subtitle:

```text
Agent Velocity
AI-native time calibration for agent work.
```

or:

```text
Agent Velocity
Stop estimating AI agents like human engineers.
```

Best public messaging:

```text
Agent Velocity converts human-hour guesses into AI-native task estimates.
```

## 19. MVP Acceptance Criteria

v0.1 is complete when:

- `aiplus velocity init` creates local storage.
- `aiplus velocity estimate` accepts human estimate and outputs AI-native p50/p90.
- `aiplus velocity complete` records actual time.
- `aiplus velocity bias` detects 5h -> 20m as a 15x overestimate.
- `aiplus velocity report` summarizes recent bias.
- `aiplus velocity doctor` validates storage, schema, retention, and redaction.
- Retention keeps latest 200 normal records.
- Rare cases are limited to latest 20.
- No raw prompts, transcripts, secrets, provider payloads, or full paths are stored.
- No SQLite is introduced.
- No network calls are required.
- No global config is modified.

## 20. Release Packet Fields

Recommended final packet:

```text
VERDICT=PASS|NEEDS_FIX|BLOCKED
PROJECT=aiplus-agent-velocity
VERSION=0.1.0
SCOPE=human-time-bias correction for AI agent estimates
STORAGE=JSONL_ONLY
SQLITE_USED=NO
RETENTION_POLICY=latest_200_plus_rare_20
COMMANDS_VERIFIED=[init, estimate, complete, bias, report, doctor]
HUMAN_TIME_BIAS_FIXTURE_STATUS=PASS|NEEDS_FIX
FIVE_HOURS_TO_TWENTY_MINUTES_STATUS=PASS|NEEDS_FIX
OVERESTIMATE_RATIO_STATUS=PASS|NEEDS_FIX
RETENTION_STATUS=PASS|NEEDS_FIX
RARE_CASE_STATUS=PASS|NEEDS_FIX
REDACTION_STATUS=PASS|NEEDS_FIX
SECRET_PRIVATE_BOUNDARY_STATUS=PASS|NEEDS_FIX
GLOBAL_CONFIG_STATUS=UNTOUCHED|TOUCHED
TELEMETRY_STATUS=ABSENT|PRESENT
NETWORK_STATUS=NONE|USED
MEMORY_INTEGRATION_STATUS=DISABLED_BY_DEFAULT
KNOWN_LIMITATIONS=[...]
REQUIRED_FIXES=[...]
READY_FOR_PLATFORM_CEO=YES|NO
```

## 21. First Implementation Plan

Phase 1: Docs and schemas

- write module README
- define JSON schemas
- define retention policy
- define CLI output contracts

Phase 2: Core model

- add `velocity.rs` to `aiplus-core`
- implement record structs
- implement time parser
- implement overestimate ratio
- implement retention
- implement redaction checks

Phase 3: CLI

- add `aiplus velocity init`
- add `estimate`
- add `complete`
- add `bias`
- add `report`
- add `doctor`

Phase 4: Tests

- 5h -> 20m fixture
- invalid time parsing
- retention trims latest records
- rare case retention
- corrupt JSONL doctor
- redaction blocks sensitive content
- no SQLite dependency

Phase 5: Dogfood

- use recent AiPlus tasks as redacted fixtures
- demonstrate overestimate detection
- demonstrate future estimate adjustment

## 22. Final Design Decision

Use a lightweight local ledger, not a database.

Keep the name `aiplus-agent-velocity`, but explain it as AI-native time calibration.

The project succeeds if it makes CEO agents stop saying "this will take 5 hours" when local evidence shows a strong AI agent usually completes that kind of work in 20-60 minutes.
