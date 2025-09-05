use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Test {
    pub id: usize,
    pub commands: String,
}
