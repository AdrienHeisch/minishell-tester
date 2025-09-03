use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Serialize, Deserialize)]
pub struct Test {
    #[serde(skip)]
    pub id: usize,
    pub level: Level,
    pub commands: String,
}

#[derive(
    Debug, Clone, PartialEq, PartialOrd, clap::ValueEnum, Serialize_repr, Deserialize_repr,
)]
#[repr(u8)]
pub enum Level {
    #[allow(unused)]
    Mandatory,
    Bonus,
    More,
}
