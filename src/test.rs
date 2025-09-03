use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Test {
    #[serde(skip)]
    pub id: usize,
    pub level: Level,
    pub commands: String,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, ValueEnum, Serialize, Deserialize)]
pub enum Level {
    #[allow(unused)]
    Mandatory,
    Bonus,
    More,
}
