// Track C.3 retroactive: regression coverage for PR #69 (D.2
// CHANGELOG hygiene). D.2 set the baseline that CHANGELOG.md must:
//
//   1. Start with `# Changelog` as its only H1.
//   2. Have `## Unreleased` as its FIRST H2 heading — the placeholder
//      that subsequent commits land bullets under.
//   3. List released versions in reverse chronological / descending
//      semver order BELOW Unreleased.
//   4. Have no duplicate version headings (a bug we hit twice in
//      this sprint: the v0.5.16 → v0.5.19 transitions both spawned
//      duplicate `## 0.5.16` and a missing-`## 0.5.17` section that
//      had to be hand-fixed).
//
// These four invariants are cheap to assert in regular CI and would
// have caught both prior CHANGELOG-structure incidents.

use std::path::PathBuf;

fn changelog_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root")
        .join("CHANGELOG.md")
}

#[test]
fn changelog_starts_with_changelog_h1() {
    let body = std::fs::read_to_string(changelog_path()).expect("read CHANGELOG.md");
    let first = body.lines().next().unwrap_or("");
    assert_eq!(
        first.trim(),
        "# Changelog",
        "CHANGELOG.md first line must be `# Changelog`, got {first:?}"
    );
}

#[test]
fn changelog_first_h2_is_unreleased() {
    let body = std::fs::read_to_string(changelog_path()).unwrap();
    let first_h2 = body
        .lines()
        .find(|l| l.starts_with("## "))
        .map(str::trim)
        .unwrap_or("");
    assert_eq!(
        first_h2, "## Unreleased",
        "First `## ` heading must be `## Unreleased`, got {first_h2:?}. \
         The Unreleased section is the placeholder for next-sprint changes \
         (Track D.2 discipline); without it the promote-on-release pattern \
         breaks."
    );
}

#[test]
fn changelog_version_headings_are_reverse_chronological() {
    let body = std::fs::read_to_string(changelog_path()).unwrap();
    let mut versions: Vec<(usize, usize, usize, String)> = Vec::new();
    for line in body.lines() {
        let l = line.trim();
        if !l.starts_with("## ") {
            continue;
        }
        let label = l.trim_start_matches("## ").trim();
        if label == "Unreleased" {
            continue;
        }
        // Parse `X.Y.Z` (allow trailing labels like "-rc1" but ignore them
        // for ordering).
        let mut numeric = label;
        if let Some(idx) = numeric.find(['-', ' ', '(']) {
            numeric = &numeric[..idx];
        }
        let parts: Vec<&str> = numeric.split('.').collect();
        if parts.len() != 3 {
            continue;
        }
        let nums: Option<Vec<usize>> = parts.iter().map(|p| p.parse::<usize>().ok()).collect();
        if let Some(v) = nums {
            versions.push((v[0], v[1], v[2], label.to_string()));
        }
    }
    // Strictly descending — duplicate versions or out-of-order ones fail.
    for pair in versions.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        let a_tuple = (a.0, a.1, a.2);
        let b_tuple = (b.0, b.1, b.2);
        assert!(
            a_tuple > b_tuple,
            "version headings not strictly descending: `## {}` came \
             before `## {}` — duplicate or out-of-order. Full sequence:\n{:?}",
            a.3,
            b.3,
            versions.iter().map(|v| &v.3).collect::<Vec<_>>(),
        );
    }
}

#[test]
fn changelog_has_no_duplicate_version_headings() {
    let body = std::fs::read_to_string(changelog_path()).unwrap();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for line in body.lines() {
        if !line.starts_with("## ") {
            continue;
        }
        let label = line.trim_start_matches("## ").trim().to_string();
        if label == "Unreleased" {
            // Multiple `## Unreleased` would also be a bug, but we
            // assert that in `changelog_first_h2_is_unreleased` + the
            // descending check above.
            continue;
        }
        assert!(
            seen.insert(label.clone()),
            "duplicate version heading `## {label}` in CHANGELOG.md"
        );
    }
}

#[test]
fn changelog_has_a_section_for_current_cargo_version() {
    // Defensive: when Cargo.toml is at version X, CHANGELOG.md should
    // already have a `## X` section (the release commit promotes
    // `## Unreleased` to `## X` BEFORE pushing the tag, per D.2).
    let cli_version = env!("CARGO_PKG_VERSION");
    let body = std::fs::read_to_string(changelog_path()).unwrap();
    let expected = format!("## {cli_version}");
    assert!(
        body.lines().any(|l| l.trim() == expected),
        "Cargo.toml is at {cli_version} but CHANGELOG.md has no `{expected}` section. \
         Per D.2: promote `## Unreleased` to `## {cli_version}` in the same commit \
         that bumps Cargo.toml."
    );
}
