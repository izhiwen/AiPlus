use serde::{Deserialize, Serialize};

pub const REMINDER_STATE_SCHEMA_VERSION: u32 = 1;
pub const CONTEXT_CAPSULE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReminderState {
    pub schema_version: u32,
    pub project_id: String,
    pub last_watch_at: Option<String>,
    pub last_reminder_decision: String,
    pub last_reminder_level: String,
    pub last_handoff_state: String,
    pub last_recovery_confidence: String,
    pub snooze_until: Option<String>,
    pub manual_compact_only: bool,
    pub host_compact_triggered: bool,
    pub watch_count: u64,
    pub remind_count: u64,
}

impl Default for ReminderState {
    fn default() -> Self {
        Self {
            schema_version: REMINDER_STATE_SCHEMA_VERSION,
            project_id: String::new(),
            last_watch_at: None,
            last_reminder_decision: "unknown".to_string(),
            last_reminder_level: "unknown".to_string(),
            last_handoff_state: "unknown".to_string(),
            last_recovery_confidence: "unknown".to_string(),
            snooze_until: None,
            manual_compact_only: true,
            host_compact_triggered: false,
            watch_count: 0,
            remind_count: 0,
        }
    }
}

impl ReminderState {
    pub fn new(project_id: impl Into<String>) -> Self {
        Self {
            project_id: project_id.into(),
            ..Self::default()
        }
    }

    pub fn is_snooze_active(&self) -> bool {
        if let Some(ref until) = self.snooze_until {
            if let Ok(until_millis) = until.parse::<u128>() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                return now < until_millis;
            }
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WatchConfig {
    pub mode: WatchMode,
    pub interval_seconds: u64,
    pub max_iterations: Option<u64>,
    pub json_output: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WatchMode {
    Once,
    Interval,
}

impl Default for WatchConfig {
    fn default() -> Self {
        Self {
            mode: WatchMode::Once,
            interval_seconds: 600,
            max_iterations: None,
            json_output: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WatchResult {
    pub status: String,
    pub watch_mode: String,
    pub iteration: u64,
    pub reminder_decision: String,
    pub reminder_level: String,
    pub handoff_state: String,
    pub recovery_confidence: String,
    pub manual_compact_recommended: bool,
    pub host_compact_triggered: bool,
    pub secret_values_printed: bool,
    pub raw_transcript_captured: bool,
    pub context_capsule_status: String,
    pub next_action: String,
    pub reason: String,
}

impl Default for WatchResult {
    fn default() -> Self {
        Self {
            status: "PASS".to_string(),
            watch_mode: "once".to_string(),
            iteration: 0,
            reminder_decision: "unknown".to_string(),
            reminder_level: "unknown".to_string(),
            handoff_state: "unknown".to_string(),
            recovery_confidence: "unknown".to_string(),
            manual_compact_recommended: false,
            host_compact_triggered: false,
            secret_values_printed: false,
            raw_transcript_captured: false,
            context_capsule_status: "skipped".to_string(),
            next_action: String::new(),
            reason: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct RedactionInfo {
    pub secret_values_printed: bool,
    pub raw_transcript_captured: bool,
    pub private_paths_included: bool,
}

pub fn parse_watch_interval(value: &str) -> anyhow::Result<u64> {
    let value = value.trim();
    if value.is_empty() {
        return Err(anyhow::anyhow!("interval is empty"));
    }
    let (number, multiplier) = match value.chars().last().unwrap_or('s') {
        'm' | 'M' => (&value[..value.len() - 1], 60),
        'h' | 'H' => (&value[..value.len() - 1], 3_600),
        's' | 'S' => (&value[..value.len() - 1], 1),
        _ => {
            if value.parse::<u64>().is_ok() {
                (value, 1)
            } else {
                return Err(anyhow::anyhow!("invalid interval: {value}"));
            }
        }
    };
    let amount = number
        .parse::<u64>()
        .map_err(|_| anyhow::anyhow!("invalid interval: {value}"))?;
    if amount == 0 {
        return Err(anyhow::anyhow!("interval must be positive"));
    }
    Ok(amount.saturating_mul(multiplier))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reminder_state_default() {
        let state = ReminderState::default();
        assert_eq!(state.schema_version, 1);
        assert!(state.manual_compact_only);
        assert!(!state.host_compact_triggered);
        assert!(!state.is_snooze_active());
    }

    #[test]
    fn reminder_state_snooze_logic() {
        let future = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            + 60_000)
            .to_string();
        let state = ReminderState {
            snooze_until: Some(future),
            ..ReminderState::default()
        };
        assert!(state.is_snooze_active());

        let past = "1000".to_string();
        let state = ReminderState {
            snooze_until: Some(past),
            ..ReminderState::default()
        };
        assert!(!state.is_snooze_active());
    }

    #[test]
    fn parse_interval_variants() {
        assert_eq!(parse_watch_interval("10s").unwrap(), 10);
        assert_eq!(parse_watch_interval("5m").unwrap(), 300);
        assert_eq!(parse_watch_interval("1h").unwrap(), 3600);
        assert_eq!(parse_watch_interval("30").unwrap(), 30);
        assert!(parse_watch_interval("0s").is_err());
        assert!(parse_watch_interval("").is_err());
        assert!(parse_watch_interval("abc").is_err());
    }

    #[test]
    fn watch_result_markers() {
        let result = WatchResult::default();
        assert!(!result.secret_values_printed);
        assert!(!result.raw_transcript_captured);
        assert!(!result.host_compact_triggered);
    }
}
