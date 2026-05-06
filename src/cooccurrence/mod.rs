pub mod cache;
pub mod compute;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonNode {
    pub id: String,
    pub name: Option<String>,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairCount {
    pub a: String,
    pub b: String,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoOccurrenceResult {
    pub people: Vec<PersonNode>,
    pub pairs: Vec<PairCount>,
    pub computed_at: String,
    pub from: Option<String>,
    pub to: Option<String>,
}
