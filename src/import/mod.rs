pub mod emtran;
pub use emtran::import as import_emtran;
use url::Url;

use std::{io, path::PathBuf};
use thiserror::Error;

use crate::ImportSourceArgs;

#[derive(Debug, Error)]
#[error("{0}")]
pub enum ImportError {
    #[error("Failed to read source file: {0}")]
    Io(#[from] io::Error),
    #[error("Failed to read source file: {0}")]
    ReadSource(io::Error),
    #[error("Failed to write to output file: {0}")]
    WriteOutput(io::Error),
    #[error("Failed to parse url: {0}")]
    ParseUrl(#[from] url::ParseError),
    Download(#[from] DownloadError),
    ParseTest(#[from] ParseTestError),
    #[error("Failed to serialize tests file: {0}")]
    SerializeTests(#[from] csv::Error),
}

#[derive(Debug, Error)]
#[error("{0}")]
pub enum DownloadError {
    #[error("Failed to download tests file: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Failed to parse url")]
    InvalidUrl,
}

#[derive(Debug, Error)]
#[error("{0}")]
pub enum ParseTestError {
    #[error("Failed to parse tests file: {0}")]
    Csv(#[from] csv::Error),
}

pub enum ImportSource {
    Path(PathBuf),
    Url(Url),
    Default,
}

impl From<&ImportSourceArgs> for ImportSource {
    fn from(value: &ImportSourceArgs) -> Self {
        match value {
            ImportSourceArgs {
                file: Some(path), ..
            } => Self::Path(path.clone()),
            ImportSourceArgs { url: Some(url), .. } => Self::Url(url.clone()),

            _ => Self::Default,
        }
    }
}
