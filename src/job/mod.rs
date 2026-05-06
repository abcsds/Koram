use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    pub status: String,             // "idle" | "running" | "completed" | "cancelled" | "error"
    pub processed: u32,
    pub total: u32,
    pub current_person_id: Option<String>,
    pub current_person_name: Option<String>,
    pub message: Option<String>,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            status: "idle".into(),
            processed: 0,
            total: 0,
            current_person_id: None,
            current_person_name: None,
            message: None,
        }
    }
}
