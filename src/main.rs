mod import;
mod parse;
mod run;
mod test;

use clap::{Args, Parser, Subcommand};
use import::import_emtran;
use run::{run_tests, RunError};
use std::{fmt::Debug, path::PathBuf};
use test::Level;
use thiserror::Error;
use url::Url;

const DEFAULT_TESTS_PATH: &str = "tests.csv";
const DEFAULT_IGNORE_PATH: &str = ".testignore";

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    subcommand: Option<Subcommands>,

    /// Levels are in order here on the right, higher levels will still run lower level tests
    #[arg(short, long, default_value = "mandatory")]
    level: Level,
    /// Use this to skip tests
    #[arg(short, long, default_value = "0")]
    start: usize,
    /// Path to minishell executable
    #[arg(short, long, default_value = "../minishell")]
    program: PathBuf,
    /// Path to bash executable
    #[arg(long, default_value = "/usr/bin/bash")]
    bash: PathBuf,
    /// Run bash as bash --posix
    #[arg(long)]
    bash_posix: bool,
    /// Use valgrind to check for memory leaks
    #[arg(short = 'm', long)]
    leak_check: bool,
    /// Path to tests csv file
    #[arg(short, long, default_value = DEFAULT_TESTS_PATH)]
    tests: PathBuf,
    /// Path to ignore file, should contain test ids to be ignored, each followed by a line break. Lines starting with # are commented
    #[arg(short, long, default_value = DEFAULT_IGNORE_PATH)]
    ignore: PathBuf,
    /// Ignore the ignore list
    #[arg(short, long)]
    no_ignore: bool,
}

#[derive(Subcommand)]
enum Subcommands {
    /// Import emtran's test (default source can be found at https://github.com/vietdu91/42_minishell)
    ImportEmtran {
        #[command(flatten)]
        source: ImportSourceArgs,
        /// Output file
        #[arg(short, long, default_value = DEFAULT_TESTS_PATH)]
        output: PathBuf,
        /// Number of lines before first test
        #[arg(short = 's', long)]
        header_size: Option<usize>,
    },
}

#[derive(Debug, Clone, Args)]
#[group(required = false, multiple = false)]
struct ImportSourceArgs {
    /// Set source url
    #[arg(short, long)]
    url: Option<Url>,
    /// Set source file
    #[arg(short, long)]
    file: Option<PathBuf>,
}

#[derive(Error)]
#[error("{0}")]
enum Error {
    Run(#[from] RunError),
    Import(#[from] import::ImportError),
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    if let Some(subcommand) = &cli.subcommand {
        match subcommand {
            Subcommands::ImportEmtran {
                source,
                output,
                header_size,
            } => import_emtran(&source.into(), output, header_size).map_err(Into::into),
        }
    } else {
        run_tests(&cli).map_err(Into::into)
    }
}
