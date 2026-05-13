use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

/// Fail modes for fixture categorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FailMode {
    Absent,
    Partial,
    Corrupted,
    Behavioral,
}

impl std::str::FromStr for FailMode {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "absent" => Ok(FailMode::Absent),
            "partial" => Ok(FailMode::Partial),
            "corrupted" => Ok(FailMode::Corrupted),
            "behavioral" => Ok(FailMode::Behavioral),
            _ => Err(format!("Unknown fail mode: {}", s)),
        }
    }
}

impl std::fmt::Display for FailMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailMode::Absent => write!(f, "absent"),
            FailMode::Partial => write!(f, "partial"),
            FailMode::Corrupted => write!(f, "corrupted"),
            FailMode::Behavioral => write!(f, "behavioral"),
        }
    }
}

/// Validation result for a single `.test.sh` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureValidation {
    pub pass_count: usize,
    pub fail_count: usize,
    pub fail_modes: HashSet<FailMode>,
    pub behavioral_substitution: Option<Vec<FailMode>>,
    pub reviewer_is_author: bool,
}

impl FixtureValidation {
    /// Returns true when all fixture diversity rules are satisfied.
    pub fn is_valid(&self) -> bool {
        self.pass_count >= 1
            && self.fail_count >= 4
            && self.fail_modes.len() >= 4
            && !self.reviewer_is_author
    }
}

/// Tracks statistics across multiple `.test.sh` files.
#[derive(Debug, Clone, Default)]
pub struct FixtureRunnerStats {
    pub total_scripts: usize,
    pub not_applicable_behavioral: usize,
}

impl FixtureRunnerStats {
    /// Percentage of scripts where behavioral is marked not_applicable.
    pub fn not_applicable_percentage(&self) -> f64 {
        if self.total_scripts == 0 {
            0.0
        } else {
            (self.not_applicable_behavioral as f64 / self.total_scripts as f64) * 100.0
        }
    }

    /// Returns true when the percentage exceeds the given threshold.
    pub fn exceeds_threshold(&self, threshold: f64) -> bool {
        self.not_applicable_percentage() > threshold
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FixtureKind {
    Pass,
    Fail,
}

#[derive(Debug, Clone)]
pub struct ParsedFixture {
    pub kind: FixtureKind,
    pub mode: Option<FailMode>,
    pub not_applicable: bool,
}

/// Runner that parses `.test.sh` fixtures and enforces diversity rules.
#[derive(Debug, Clone, Default)]
pub struct FixtureRunner {
    stats: FixtureRunnerStats,
}

impl FixtureRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stats(&self) -> &FixtureRunnerStats {
        &self.stats
    }

    /// Parse a `.test.sh` file and validate its fixtures.
    pub fn validate_test_sh(
        &mut self,
        test_sh_path: &Path,
        subagent_invocations_path: Option<&Path>,
    ) -> Result<FixtureValidation> {
        let content = std::fs::read_to_string(test_sh_path)
            .with_context(|| format!("Failed to read {:?}", test_sh_path))?;

        let fixtures = Self::parse_fixtures(&content);
        let pass_count = fixtures
            .iter()
            .filter(|f| f.kind == FixtureKind::Pass)
            .count();
        let fail_fixtures: Vec<_> = fixtures
            .iter()
            .filter(|f| f.kind == FixtureKind::Fail)
            .collect();
        let fail_count = fail_fixtures.len();

        let mut fail_modes = HashSet::new();
        let mut behavioral_not_applicable = false;
        let mut behavioral_substitution = None;

        for fixture in &fail_fixtures {
            if let Some(mode) = fixture.mode {
                fail_modes.insert(mode);
                if mode == FailMode::Behavioral && fixture.not_applicable {
                    behavioral_not_applicable = true;
                }
            }
        }

        // Parse not_applicable_substitution when behavioral is marked not_applicable.
        if behavioral_not_applicable {
            behavioral_substitution = Self::parse_substitution(&content);
        }

        // Validate reviewer != author using git blame and subagent-invocations.jsonl.
        let reviewer_is_author =
            Self::check_reviewer_is_author(test_sh_path, subagent_invocations_path)?;

        self.stats.total_scripts += 1;
        if behavioral_not_applicable {
            self.stats.not_applicable_behavioral += 1;
        }

        Ok(FixtureValidation {
            pass_count,
            fail_count,
            fail_modes,
            behavioral_substitution,
            reviewer_is_author,
        })
    }

    fn parse_fixtures(content: &str) -> Vec<ParsedFixture> {
        let mut fixtures = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(fixture_decl) = Self::extract_fixture_declaration(trimmed) {
                let parts: Vec<&str> = fixture_decl.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }

                let kind = match parts[0].to_lowercase().as_str() {
                    "pass" => FixtureKind::Pass,
                    "fail" => FixtureKind::Fail,
                    _ => continue,
                };

                let mut mode = None;
                let mut not_applicable = false;

                if kind == FixtureKind::Fail && parts.len() >= 2 {
                    if let Ok(m) = parts[1].parse::<FailMode>() {
                        mode = Some(m);
                        if m == FailMode::Behavioral && parts.len() >= 3 {
                            not_applicable = parts[2].to_lowercase() == "not_applicable";
                        }
                    }
                }

                fixtures.push(ParsedFixture {
                    kind,
                    mode,
                    not_applicable,
                });
            }
        }

        fixtures
    }

    fn extract_fixture_declaration(line: &str) -> Option<&str> {
        if let Some(pos) = line.find("FIXTURE:") {
            let after_marker = &line[pos + "FIXTURE:".len()..];
            return Some(after_marker.trim());
        }
        None
    }

    fn parse_substitution(content: &str) -> Option<Vec<FailMode>> {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(pos) = trimmed.find("NOT_APPLICABLE_SUBSTITUTION:") {
                let after_marker = &trimmed[pos + "NOT_APPLICABLE_SUBSTITUTION:".len()..];
                let modes: Vec<FailMode> = after_marker
                    .split(',')
                    .filter_map(|s| s.trim().parse::<FailMode>().ok())
                    .collect();
                if !modes.is_empty() {
                    return Some(modes);
                }
            }
        }
        None
    }

    /// Returns true if the reviewer is the same as the author (invalid).
    fn check_reviewer_is_author(
        test_sh_path: &Path,
        subagent_invocations_path: Option<&Path>,
    ) -> Result<bool> {
        let author = Self::get_git_author(test_sh_path)?;

        let reviewer = if let Some(invocations_path) = subagent_invocations_path {
            Self::get_reviewer_from_invocations(invocations_path, test_sh_path)?
        } else {
            None
        };

        Ok(match (author, reviewer) {
            (Some(a), Some(r)) => a == r,
            // If we cannot determine either party, conservatively assume they are different.
            _ => false,
        })
    }

    fn get_git_author(path: &Path) -> Result<Option<String>> {
        let output = Command::new("git")
            .args([
                "blame",
                "--line-porcelain",
                "-L",
                "1,1",
                path.to_str().unwrap_or(""),
            ])
            .output()
            .context("Failed to run git blame")?;

        if !output.status.success() {
            return Ok(None);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.starts_with("author ") {
                return Ok(Some(line["author ".len()..].trim().to_string()));
            }
        }

        Ok(None)
    }

    fn get_reviewer_from_invocations(
        invocations_path: &Path,
        test_sh_path: &Path,
    ) -> Result<Option<String>> {
        #[derive(Debug, Deserialize)]
        struct InvocationRecord {
            agent_id: Option<String>,
            file_path: Option<String>,
            action: Option<String>,
        }

        let content = std::fs::read_to_string(invocations_path)
            .context("Failed to read subagent-invocations.jsonl")?;

        let test_path_str = test_sh_path.to_string_lossy().to_string();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let record: InvocationRecord =
                serde_json::from_str(line).context("Failed to parse invocation record")?;

            if let (Some(agent_id), Some(file_path), Some(action)) =
                (record.agent_id, record.file_path, record.action)
            {
                if file_path == test_path_str && action.to_lowercase().contains("review") {
                    return Ok(Some(agent_id));
                }
            }
        }

        Ok(None)
    }

    /// Generate an alert message if the not_applicable percentage exceeds the threshold.
    pub fn check_threshold_alert(&self, threshold: f64) -> Option<String> {
        if self.stats.exceeds_threshold(threshold) {
            Some(format!(
                "Alert: {:.1}% of audit scripts mark behavioral as not_applicable, exceeding {:.1}% threshold",
                self.stats.not_applicable_percentage(),
                threshold
            ))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_parse_fixtures_basic() {
        let content = r#"
# FIXTURE: pass
# FIXTURE: fail absent
# FIXTURE: fail partial
# FIXTURE: fail corrupted
# FIXTURE: fail behavioral
"#;
        let fixtures = FixtureRunner::parse_fixtures(content);
        assert_eq!(fixtures.len(), 5);
        assert_eq!(
            fixtures
                .iter()
                .filter(|f| f.kind == FixtureKind::Pass)
                .count(),
            1
        );
        assert_eq!(
            fixtures
                .iter()
                .filter(|f| f.kind == FixtureKind::Fail)
                .count(),
            4
        );
    }

    #[test]
    fn test_parse_fixtures_behavioral_not_applicable() {
        let content = r#"
# FIXTURE: pass
# FIXTURE: fail absent
# FIXTURE: fail partial
# FIXTURE: fail corrupted
# FIXTURE: fail behavioral not_applicable
# NOT_APPLICABLE_SUBSTITUTION: absent,partial
"#;
        let fixtures = FixtureRunner::parse_fixtures(content);
        let behavioral = fixtures
            .iter()
            .find(|f| f.mode == Some(FailMode::Behavioral))
            .unwrap();
        assert!(behavioral.not_applicable);

        let substitution = FixtureRunner::parse_substitution(content);
        assert_eq!(substitution.unwrap().len(), 2);
    }

    #[test]
    fn test_fixture_validation_valid() {
        let validation = FixtureValidation {
            pass_count: 1,
            fail_count: 4,
            fail_modes: [
                FailMode::Absent,
                FailMode::Partial,
                FailMode::Corrupted,
                FailMode::Behavioral,
            ]
            .iter()
            .cloned()
            .collect(),
            behavioral_substitution: None,
            reviewer_is_author: false,
        };
        assert!(validation.is_valid());
    }

    #[test]
    fn test_fixture_validation_insufficient_pass() {
        let validation = FixtureValidation {
            pass_count: 0,
            fail_count: 4,
            fail_modes: [
                FailMode::Absent,
                FailMode::Partial,
                FailMode::Corrupted,
                FailMode::Behavioral,
            ]
            .iter()
            .cloned()
            .collect(),
            behavioral_substitution: None,
            reviewer_is_author: false,
        };
        assert!(!validation.is_valid());
    }

    #[test]
    fn test_fixture_validation_reviewer_is_author() {
        let validation = FixtureValidation {
            pass_count: 1,
            fail_count: 4,
            fail_modes: [
                FailMode::Absent,
                FailMode::Partial,
                FailMode::Corrupted,
                FailMode::Behavioral,
            ]
            .iter()
            .cloned()
            .collect(),
            behavioral_substitution: None,
            reviewer_is_author: true,
        };
        assert!(!validation.is_valid());
    }

    #[test]
    fn test_stats_threshold() {
        let mut stats = FixtureRunnerStats::default();
        stats.total_scripts = 10;
        stats.not_applicable_behavioral = 4;
        assert!(stats.exceeds_threshold(30.0));

        stats.not_applicable_behavioral = 3;
        assert!(!stats.exceeds_threshold(30.0));
    }

    #[test]
    fn test_validate_test_sh() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let test_sh = temp_dir.path().join("test.test.sh");

        let mut file = std::fs::File::create(&test_sh)?;
        writeln!(file, "# FIXTURE: pass")?;
        writeln!(file, "# FIXTURE: fail absent")?;
        writeln!(file, "# FIXTURE: fail partial")?;
        writeln!(file, "# FIXTURE: fail corrupted")?;
        writeln!(file, "# FIXTURE: fail behavioral")?;

        let mut runner = FixtureRunner::new();
        let validation = runner.validate_test_sh(&test_sh, None)?;

        assert_eq!(validation.pass_count, 1);
        assert_eq!(validation.fail_count, 4);
        assert_eq!(validation.fail_modes.len(), 4);
        assert!(validation.behavioral_substitution.is_none());
        assert!(!validation.reviewer_is_author);

        Ok(())
    }

    #[test]
    fn test_validate_test_sh_not_applicable() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let test_sh = temp_dir.path().join("test.test.sh");

        let mut file = std::fs::File::create(&test_sh)?;
        writeln!(file, "# FIXTURE: pass")?;
        writeln!(file, "# FIXTURE: fail absent")?;
        writeln!(file, "# FIXTURE: fail partial")?;
        writeln!(file, "# FIXTURE: fail corrupted")?;
        writeln!(file, "# FIXTURE: fail behavioral not_applicable")?;
        writeln!(file, "# NOT_APPLICABLE_SUBSTITUTION: absent,partial")?;

        let mut runner = FixtureRunner::new();
        let validation = runner.validate_test_sh(&test_sh, None)?;

        assert_eq!(validation.pass_count, 1);
        assert_eq!(validation.fail_count, 4);
        assert!(validation.behavioral_substitution.is_some());
        let sub = validation.behavioral_substitution.unwrap();
        assert_eq!(sub.len(), 2);
        assert!(sub.contains(&FailMode::Absent));
        assert!(sub.contains(&FailMode::Partial));

        Ok(())
    }

    #[test]
    fn test_validate_test_sh_insufficient_fail() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let test_sh = temp_dir.path().join("test.test.sh");

        let mut file = std::fs::File::create(&test_sh)?;
        writeln!(file, "# FIXTURE: pass")?;
        writeln!(file, "# FIXTURE: fail absent")?;

        let mut runner = FixtureRunner::new();
        let validation = runner.validate_test_sh(&test_sh, None)?;

        assert_eq!(validation.pass_count, 1);
        assert_eq!(validation.fail_count, 1);
        assert!(!validation.is_valid());

        Ok(())
    }

    #[test]
    fn test_threshold_alert() {
        let mut runner = FixtureRunner::new();
        runner.stats.total_scripts = 10;
        runner.stats.not_applicable_behavioral = 4;

        let alert = runner.check_threshold_alert(30.0);
        assert!(alert.is_some());
        assert!(alert.unwrap().contains("40.0%"));

        let no_alert = runner.check_threshold_alert(50.0);
        assert!(no_alert.is_none());
    }

    #[test]
    fn test_parse_substitution_empty() {
        let content = "# NOT_APPLICABLE_SUBSTITUTION:";
        assert!(FixtureRunner::parse_substitution(content).is_none());
    }

    #[test]
    fn test_parse_fixtures_invalid_mode() {
        let content = "# FIXTURE: fail unknown_mode";
        let fixtures = FixtureRunner::parse_fixtures(content);
        assert_eq!(fixtures.len(), 1);
        assert_eq!(fixtures[0].kind, FixtureKind::Fail);
        assert!(fixtures[0].mode.is_none());
    }
}
