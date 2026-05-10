use crate::memory::MemoryRecord;

#[derive(Debug, Clone, Default)]
pub struct ContextBudget {
    pub max_chars: usize,
}

impl ContextBudget {
    pub fn new(max_chars: usize) -> Self {
        Self { max_chars }
    }
}

pub fn select_records(records: &[MemoryRecord], budget: usize) -> Vec<&MemoryRecord> {
    let mut eligible: Vec<&MemoryRecord> = records
        .iter()
        .filter(|r| {
            // Exclude rejected, forgotten, or stale status
            if r.status == "rejected" || r.status == "forgotten" || r.status == "stale" {
                return false;
            }
            // Exclude expired records
            if r.is_expired() {
                return false;
            }
            true
        })
        .collect();

    // Sort by priority
    eligible.sort_by_key(|r| type_priority(&r.record_type));

    let mut selected = Vec::new();
    let mut used = 0usize;
    for record in eligible {
        let line = format!(
            "- [{}] {} / {}: {}",
            record.id, record.scope, record.record_type, record.summary
        );
        if used + line.len() > budget {
            break;
        }
        used += line.len();
        selected.push(record);
    }
    selected
}

fn type_priority(record_type: &str) -> u8 {
    match record_type {
        "owner_gate" => 1,
        "project_decision" => 2,
        "risk" => 3,
        "owner_preference" => 4,
        "workflow_rule" => 5,
        "project_fact" => 6,
        "handoff_note" => 7,
        "verification_evidence" => 8,
        "role_identity" => 9,
        "skill_candidate" => 10,
        "deprecated_memory" => 11,
        // Fallback for v0.1.0 kinds mapped to approximate priorities
        "decision_summary" => 2,
        "preference" => 4,
        "constraint" => 5,
        "evidence_pointer" => 8,
        _ => 12,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(id: &str, record_type: &str, status: &str, summary: &str) -> MemoryRecord {
        MemoryRecord {
            id: id.to_string(),
            record_type: record_type.to_string(),
            status: status.to_string(),
            summary: summary.to_string(),
            ..MemoryRecord::default()
        }
    }

    #[test]
    fn select_records_respects_budget() {
        let records = vec![
            make_record("mem_1", "project_fact", "active", "Short"),
            make_record(
                "mem_2",
                "project_fact",
                "active",
                "This is a much longer summary that takes up more space",
            ),
        ];
        let selected = select_records(&records, 50);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "mem_1");
    }

    #[test]
    fn select_records_excludes_rejected() {
        let records = vec![
            make_record("mem_1", "project_fact", "active", "Active fact"),
            make_record("mem_2", "project_fact", "rejected", "Rejected fact"),
        ];
        let selected = select_records(&records, 1000);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "mem_1");
    }

    #[test]
    fn select_records_priority_ordering() {
        let records = vec![
            make_record("mem_1", "project_fact", "active", "Fact"),
            make_record("mem_2", "owner_gate", "active", "Gate"),
            make_record("mem_3", "project_decision", "active", "Decision"),
        ];
        let selected = select_records(&records, 1000);
        assert_eq!(selected.len(), 3);
        assert_eq!(selected[0].id, "mem_2"); // owner_gate first
        assert_eq!(selected[1].id, "mem_3"); // project_decision second
        assert_eq!(selected[2].id, "mem_1"); // project_fact last
    }

    #[test]
    fn select_records_excludes_expired() {
        let past = (crate::memory::epoch_millis() - 1000).to_string();
        let mut expired = make_record("mem_1", "project_fact", "active", "Expired fact");
        expired.expires_at = Some(past);

        let records = vec![
            expired,
            make_record("mem_2", "project_fact", "active", "Fresh fact"),
        ];
        let selected = select_records(&records, 1000);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].id, "mem_2");
    }
}
