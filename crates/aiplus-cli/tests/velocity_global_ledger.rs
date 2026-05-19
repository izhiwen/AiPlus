//! Spec v2 acceptance tests for the cross-project global velocity ledger.
//!
//! Four orthogonal properties:
//!
//! 1. **Privacy structural** — a `--task "Sensitive ACME contract review"`
//!    must not surface anywhere in the global ledger's JSONL, and the
//!    `task` field must be structurally absent (not just empty/hashed).
//! 2. **Migration idempotency** — importing a project with N records
//!    twice yields exactly N global records.
//! 3. **Schema forward-compat** — a config.json with an unknown
//!    `future_field` parses cleanly and round-trips without complaint.
//! 4. **Concurrency stress** — 8 concurrent `aiplus velocity complete`
//!    invocations × 50 writes each → global ledger has exactly 400
//!    parseable records with no duplicate ids.
//!
//! Each test isolates its global ledger via `XDG_CONFIG_HOME=<tempdir>`.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type CompletionFailures = Vec<(i32, String)>;
type CompletionWrites = Vec<(String, String, String)>;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn setup_project(tempdir: &Path) -> PathBuf {
    let project = tempdir.join("project");
    fs::create_dir_all(&project).unwrap();
    fs::create_dir_all(tempdir.join("fake-home")).unwrap();
    fs::create_dir_all(tempdir.join("fake-xdg")).unwrap();
    fs::create_dir_all(tempdir.join("fake-codex-home")).unwrap();
    project
}

fn env_vars(tempdir: &Path) -> [(&'static str, PathBuf); 3] {
    [
        ("HOME", tempdir.join("fake-home")),
        ("XDG_CONFIG_HOME", tempdir.join("fake-xdg")),
        ("CODEX_HOME", tempdir.join("fake-codex-home")),
    ]
}

fn run_velocity(project: &Path, env: &[(&str, PathBuf)], args: &[&str]) -> (i32, String, String) {
    let out = Command::new(bin())
        .args(["velocity"])
        .args(args)
        .current_dir(project)
        .envs(env.iter().map(|(k, v)| (*k, v.as_os_str())))
        .output()
        .expect("run aiplus velocity");
    let code = out.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    (code, stdout, stderr)
}

fn estimate_id_from(out: &str) -> Option<String> {
    out.lines()
        .find_map(|l| l.strip_prefix("ESTIMATE_ID="))
        .map(|s| s.trim().to_string())
}

/// Spec v2 §7: free-text task is NEVER stored in the global ledger.
/// Not hashed, not redacted-in-place — structurally absent.
#[test]
fn global_ledger_drops_free_text_task() {
    let tmp = tempfile::tempdir().unwrap();
    let project = setup_project(tmp.path());
    let env_arr = env_vars(tmp.path());
    let env: Vec<(&str, PathBuf)> = env_arr.iter().map(|(k, v)| (*k, v.clone())).collect();

    run_velocity(&project, &env, &["init"]);
    let (code, out, _) = run_velocity(
        &project,
        &env,
        &[
            "estimate",
            "--task-type",
            "feature",
            "--human-estimate",
            "1h",
            "--model",
            "claude-haiku-4-5",
            "--workflow",
            "MEDIUM",
            "--task",
            "Sensitive ACME Corp contract review",
        ],
    );
    assert_eq!(code, 0, "estimate failed: {out}");
    let est_id = estimate_id_from(&out).expect("ESTIMATE_ID in output");

    let (code, out, _) = run_velocity(
        &project,
        &env,
        &[
            "complete",
            "--task-id",
            &est_id.replace("est_", "task_"),
            "--actual",
            "10m",
            "--outcome",
            "pass",
        ],
    );
    // complete may report needs_fix on fresh fake env but it should still write
    let _ = code;
    let _ = out;

    let global_dir = tmp.path().join("fake-xdg/aiplus/velocity");
    assert!(
        global_dir.exists(),
        "global velocity dir should exist after complete"
    );
    for f in [
        "runs.jsonl",
        "estimates.jsonl",
        "anchor-signals.jsonl",
        "rare-cases.jsonl",
    ] {
        let p = global_dir.join(f);
        if !p.exists() {
            continue;
        }
        let body = fs::read_to_string(&p).unwrap();
        // No literal sensitive strings.
        let lower = body.to_lowercase();
        assert!(
            !lower.contains("acme") && !lower.contains("contract") && !lower.contains("sensitive"),
            "global {f} leaked sensitive task text:\n{body}"
        );
        // task field is structurally absent. Parse each line and assert
        // the JSON object has no "task" key at all (not "task":"" — *absent*).
        for line in body.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let v: serde_json::Value = serde_json::from_str(line).expect("parse jsonl line");
            if let Some(obj) = v.as_object() {
                assert!(
                    !obj.contains_key("task"),
                    "global {f} record carries `task` field (must be absent, not empty): {line}"
                );
            }
        }
    }
}

/// Spec §6: re-running migration on the same project produces zero
/// new global records.
#[test]
fn import_from_project_is_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let project = setup_project(tmp.path());
    let env_arr = env_vars(tmp.path());
    let env: Vec<(&str, PathBuf)> = env_arr.iter().map(|(k, v)| (*k, v.clone())).collect();

    run_velocity(&project, &env, &["init"]);
    // Seed the project with 3 estimate+complete rounds.
    for i in 0..3 {
        let (_, out, _) = run_velocity(
            &project,
            &env,
            &[
                "estimate",
                "--task-type",
                "feature",
                "--human-estimate",
                "1h",
                "--model",
                "claude-haiku-4-5",
                "--workflow",
                "MEDIUM",
                "--task",
                &format!("seed task {i}"),
            ],
        );
        let est_id = estimate_id_from(&out).expect("ESTIMATE_ID");
        let task_id = est_id.replace("est_", "task_");
        run_velocity(
            &project,
            &env,
            &[
                "complete",
                "--task-id",
                &task_id,
                "--actual",
                "10m",
                "--outcome",
                "pass",
            ],
        );
    }

    // Snapshot global runs.jsonl after seeding.
    let global_runs = tmp.path().join("fake-xdg/aiplus/velocity/runs.jsonl");
    let body_before = fs::read_to_string(&global_runs).unwrap_or_default();
    let lines_before = body_before.lines().filter(|l| !l.trim().is_empty()).count();

    // Run import-from-project twice with the same source.
    for _ in 0..2 {
        let (code, _, stderr) = run_velocity(
            &project,
            &env,
            &["import-from-project", project.to_str().unwrap()],
        );
        assert_eq!(code, 0, "import-from-project failed: {stderr}");
    }

    let body_after = fs::read_to_string(&global_runs).unwrap_or_default();
    let lines_after = body_after.lines().filter(|l| !l.trim().is_empty()).count();
    assert_eq!(
        lines_after, lines_before,
        "import-from-project must be idempotent; before={lines_before} after={lines_after}"
    );
}

/// Spec §5: schemas must NOT use `deny_unknown_fields`. A future
/// config carrying `future_field` must still parse cleanly.
#[test]
fn config_tolerates_unknown_future_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let project = setup_project(tmp.path());
    let env_arr = env_vars(tmp.path());
    let env: Vec<(&str, PathBuf)> = env_arr.iter().map(|(k, v)| (*k, v.clone())).collect();

    run_velocity(&project, &env, &["init"]);
    // Overwrite config with a future-shaped JSON.
    let cfg_path = project.join(".aiplus/velocity/config.json");
    let future_cfg = r#"{
        "schemaVersion": "0.99.0",
        "maxRecords": 200,
        "rareCaseMaxRecords": 20,
        "maxBytesPerJsonl": 1048576,
        "retainDays": 90,
        "minBucketSamples": 8,
        "rawContentAllowed": false,
        "memoryIntegration": "disabled",
        "shareToGlobalMode": "read_write",
        "futureField": "some_v3_value",
        "anotherUnknown": 42
    }"#;
    fs::write(&cfg_path, future_cfg).unwrap();

    // Any CLI op that reads config should still succeed.
    let (code, _, stderr) = run_velocity(&project, &env, &["doctor"]);
    assert_eq!(
        code, 0,
        "doctor must tolerate unknown config fields; stderr={stderr}"
    );
}

/// Spec §11: 8 concurrent processes × 50 writes → global has exactly
/// 400 records, all parseable, no duplicate ids. Runs in serial here
/// (spawning 8 threads of subprocess Command) — the property under
/// test is the file-locking and O_APPEND atomicity, not thread sched.
#[test]
fn global_ledger_concurrency_stress() {
    let tmp = tempfile::tempdir().unwrap();
    let env_arr = env_vars(tmp.path());
    let env: Vec<(&str, PathBuf)> = env_arr.iter().map(|(k, v)| (*k, v.clone())).collect();

    // 8 separate project roots (different cwds) all sharing the same
    // fake XDG_CONFIG_HOME, so they all write to the same global ledger.
    let projects: Vec<PathBuf> = (0..8)
        .map(|i| {
            let p = tmp.path().join(format!("proj-{i}"));
            fs::create_dir_all(&p).unwrap();
            p
        })
        .collect();

    // Init each. Serial bootstrap is fine — we're stress-testing
    // `complete`, not `init`.
    for p in &projects {
        let _ = run_velocity(p, &env, &["init"]);
    }

    // Pre-estimate each: 50 estimates per project = 400 task_ids ready
    // for complete. (Doing estimate inside the parallel loop would
    // race the project-local files too, which is out of scope for
    // this stress test.)
    let mut task_ids: Vec<(PathBuf, String)> = Vec::new();
    for p in &projects {
        for i in 0..50 {
            let (_, out, _) = run_velocity(
                p,
                &env,
                &[
                    "estimate",
                    "--task-type",
                    "feature",
                    "--human-estimate",
                    "30m",
                    "--model",
                    "claude-haiku-4-5",
                    "--workflow",
                    "MEDIUM",
                    "--task",
                    &format!("stress-{i}"),
                ],
            );
            let est_id = estimate_id_from(&out).expect("ESTIMATE_ID");
            task_ids.push((p.clone(), est_id.replace("est_", "task_")));
        }
    }

    // Now the concurrent completes. Spawn one thread per project; each
    // thread drains its 50 task_ids via serial `complete` calls. 8
    // threads writing to the same global runs.jsonl exercises the
    // O_APPEND-atomic invariant.
    let env_pairs: Vec<(String, String)> = env
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string_lossy().to_string()))
        .collect();
    let mut grouped: std::collections::BTreeMap<PathBuf, Vec<String>> =
        std::collections::BTreeMap::new();
    for (p, t) in task_ids {
        grouped.entry(p).or_default().push(t);
    }
    let mut handles = Vec::new();
    for (p, tids) in grouped {
        let env_pairs = env_pairs.clone();
        let h = std::thread::spawn(move || -> (CompletionFailures, CompletionWrites) {
            let mut fails = Vec::new();
            let mut writes = Vec::new();
            for tid in tids {
                let out = Command::new(bin())
                    .args([
                        "velocity",
                        "complete",
                        "--task-id",
                        &tid,
                        "--actual",
                        "10m",
                        "--outcome",
                        "pass",
                    ])
                    .current_dir(&p)
                    .envs(env_pairs.iter().map(|(k, v)| (k.as_str(), v.as_str())))
                    .output()
                    .expect("run aiplus velocity complete");
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                let global_write = stdout
                    .lines()
                    .find_map(|l| l.strip_prefix("GLOBAL_WRITE="))
                    .unwrap_or("?")
                    .to_string();
                writes.push((tid.clone(), global_write, stderr.clone()));
                if !out.status.success() {
                    fails.push((out.status.code().unwrap_or(-1), stderr));
                }
            }
            (fails, writes)
        });
        handles.push(h);
    }
    let mut all_fails: Vec<(i32, String)> = Vec::new();
    let mut all_writes: Vec<(String, String, String)> = Vec::new();
    for h in handles {
        let (f, w) = h.join().unwrap();
        all_fails.extend(f);
        all_writes.extend(w);
    }
    assert!(
        all_fails.is_empty(),
        "{} complete invocations failed; first: code={} stderr={}",
        all_fails.len(),
        all_fails[0].0,
        all_fails[0].1
    );
    let appended = all_writes
        .iter()
        .filter(|(_, w, _)| w == "appended")
        .count();
    let skipped = all_writes.iter().filter(|(_, w, _)| w == "skipped").count();
    let dup = all_writes
        .iter()
        .filter(|(_, w, _)| w == "skipped_duplicate")
        .count();
    let failed = all_writes.iter().filter(|(_, w, _)| w == "failed").count();
    let other = all_writes.iter().filter(|(_, w, _)| w == "?").count();
    eprintln!(
        "GLOBAL_WRITE breakdown: appended={appended} skipped={skipped} dup={dup} failed={failed} other={other} total={}",
        all_writes.len()
    );
    for (tid, w, stderr) in &all_writes {
        if w != "appended" {
            eprintln!("  non-appended tid={tid} write={w} stderr={stderr:?}");
        }
    }

    let global_runs = tmp.path().join("fake-xdg/aiplus/velocity/runs.jsonl");
    let body = fs::read_to_string(&global_runs).expect("global runs.jsonl exists");
    let mut seen: HashSet<String> = HashSet::new();
    let mut count = 0usize;
    for (lineno, line) in body.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line).unwrap_or_else(|e| {
            panic!(
                "global runs.jsonl line {} unparseable: {e}\nline={line}",
                lineno + 1
            )
        });
        let id = v
            .get("id")
            .and_then(|s| s.as_str())
            .expect("id field present")
            .to_string();
        if !seen.insert(id.clone()) {
            panic!("duplicate id in global runs.jsonl: {id}");
        }
        count += 1;
    }
    assert_eq!(
        count, 400,
        "expected 400 records in global runs.jsonl, got {count}"
    );
}
