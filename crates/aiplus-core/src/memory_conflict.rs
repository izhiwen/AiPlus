use crate::memory::MemoryRecord;

#[derive(Debug, Clone, PartialEq)]
pub struct ConflictReport {
    pub record_id: String,
    pub conflict_type: String,
    pub description: String,
    pub related_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StaleReport {
    pub record_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default)]
pub struct ConflictDetector;

impl ConflictDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detect_conflicts(&self, records: &[MemoryRecord]) -> Vec<ConflictReport> {
        let mut reports = Vec::new();
        let by_id: std::collections::HashMap<&str, &MemoryRecord> =
            records.iter().map(|r| (r.id.as_str(), r)).collect();

        // Check for conflict groups with divergent content
        let mut groups: std::collections::HashMap<&str, Vec<&MemoryRecord>> =
            std::collections::HashMap::new();
        for record in records {
            if let Some(ref group) = record.conflict_group {
                groups.entry(group).or_default().push(record);
            }
        }
        for (group_id, group_records) in groups {
            if group_records.len() < 2 {
                continue;
            }
            let first_summary = &group_records[0].summary;
            for record in group_records.iter().skip(1) {
                if record.summary != *first_summary {
                    reports.push(ConflictReport {
                        record_id: record.id.clone(),
                        conflict_type: "conflict_group_divergence".to_string(),
                        description: format!(
                            "Record in conflict group '{}' has divergent summary",
                            group_id
                        ),
                        related_ids: group_records
                            .iter()
                            .filter(|r| r.id != record.id)
                            .map(|r| r.id.clone())
                            .collect(),
                    });
                }
            }
        }

        // Check for missing superseded records
        for record in records {
            for supersedes_id in &record.supersedes {
                if !by_id.contains_key(supersedes_id.as_str()) {
                    reports.push(ConflictReport {
                        record_id: record.id.clone(),
                        conflict_type: "missing_superseded".to_string(),
                        description: format!(
                            "Record claims to supersede '{}' which does not exist",
                            supersedes_id
                        ),
                        related_ids: vec![supersedes_id.clone()],
                    });
                }
            }
        }

        // Check for circular superseding
        for record in records {
            if record.supersedes.iter().any(|id| {
                by_id
                    .get(id.as_str())
                    .map(|target| target.supersedes.contains(&record.id))
                    .unwrap_or(false)
            }) {
                reports.push(ConflictReport {
                    record_id: record.id.clone(),
                    conflict_type: "circular_supersede".to_string(),
                    description: "Circular superseding detected".to_string(),
                    related_ids: record.supersedes.clone(),
                });
            }
        }

        reports
    }

    pub fn detect_stale(&self, records: &[MemoryRecord]) -> Vec<StaleReport> {
        records
            .iter()
            .filter(|r| r.is_stale())
            .map(|r| {
                let reason = if r.confidence == "stale" {
                    "confidence_marked_stale"
                } else if r.is_expired() {
                    "expired"
                } else {
                    "stale_after_elapsed"
                };
                StaleReport {
                    record_id: r.id.clone(),
                    reason: reason.to_string(),
                }
            })
            .collect()
    }
}

pub fn detect_conflicts(records: &[MemoryRecord]) -> Vec<ConflictReport> {
    ConflictDetector::new().detect_conflicts(records)
}

pub fn detect_stale(records: &[MemoryRecord]) -> Vec<StaleReport> {
    ConflictDetector::new().detect_stale(records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryRecord;

    fn make_record(id: &str, summary: &str, group: Option<&str>) -> MemoryRecord {
        MemoryRecord {
            id: id.to_string(),
            summary: summary.to_string(),
            status: "active".to_string(),
            conflict_group: group.map(|s| s.to_string()),
            ..MemoryRecord::default()
        }
    }

    #[test]
    fn detect_conflict_group_divergence() {
        let records = vec![
            make_record("mem_1", "Value A", Some("group_1")),
            make_record("mem_2", "Value B", Some("group_1")),
            make_record("mem_3", "Value A", Some("group_1")),
        ];
        let conflicts = ConflictDetector::new().detect_conflicts(&records);
        assert!(!conflicts.is_empty());
        assert!(conflicts
            .iter()
            .any(|c| c.conflict_type == "conflict_group_divergence"));
    }

    #[test]
    fn detect_missing_superseded() {
        let mut r1 = make_record("mem_1", "New value", None);
        r1.supersedes = vec!["mem_old".to_string()];
        let records = vec![r1];
        let conflicts = ConflictDetector::new().detect_conflicts(&records);
        assert!(conflicts
            .iter()
            .any(|c| c.conflict_type == "missing_superseded"));
    }

    #[test]
    fn detect_circular_supersede() {
        let mut r1 = make_record("mem_1", "A", None);
        r1.supersedes = vec!["mem_2".to_string()];
        let mut r2 = make_record("mem_2", "B", None);
        r2.supersedes = vec!["mem_1".to_string()];
        let records = vec![r1, r2];
        let conflicts = ConflictDetector::new().detect_conflicts(&records);
        assert!(conflicts
            .iter()
            .any(|c| c.conflict_type == "circular_supersede"));
    }

    #[test]
    fn detect_stale_records() {
        let past = (crate::memory::epoch_millis() - 1000).to_string();
        let mut r1 = make_record("mem_1", "Old", None);
        r1.stale_after = Some(past);
        let records = vec![r1];
        let stale = ConflictDetector::new().detect_stale(&records);
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].reason, "stale_after_elapsed");
    }

    #[test]
    fn detect_stale_by_confidence() {
        let mut r1 = make_record("mem_1", "Old", None);
        r1.confidence = "stale".to_string();
        let records = vec![r1];
        let stale = ConflictDetector::new().detect_stale(&records);
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].reason, "confidence_marked_stale");
    }
}
