use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Test {
    #[serde(skip)]
    pub id: usize,
    pub commands: String,
}
