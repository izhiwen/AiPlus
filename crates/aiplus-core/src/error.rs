use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreError {
    message: String,
}

impl CoreError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn memory_not_found(id: impl AsRef<str>) -> Self {
        Self::new(format!("memory record not found: {}", id.as_ref()))
    }

    pub fn skill_candidate_not_found(id: impl AsRef<str>) -> Self {
        Self::new(format!("skill candidate not found: {}", id.as_ref()))
    }
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CoreError {}
