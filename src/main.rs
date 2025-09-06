mod import;
mod run;
mod test;
mod watch;

use clap::{Args, Parser, Subcommand};
use hotwatch::{
    blocking::{Flow, Hotwatch},
    notify::event::AccessKind,
    Event, EventKind,
};
use import::import_emtran;
use run::{run_tests, RunError};
use std::{fmt::Debug, fs, io, path::PathBuf, time::Duration};
use thiserror::Error;
use url::Url;
use watch::WatchError;

#[derive(Parser)]
/// MAXITEST FOR MINISHELL
///
/// Tests are stored in csv files. Use a spreadsheet editor for convenience. Use the example
/// subcommand to have a look at the test format.
///
/// For any xxx.csv file, a xxx.ignore file can contain a list of test ids to ignore. One id per
/// line, use # to add comments.
///
/// Try the import-emtran subcommand to get a few hundred tests.
struct Cli {
    #[command(subcommand)]
    command: Subcommands,
}

#[derive(Subcommand)]
enum Subcommands {
    Example,
    Run(Run),
    ImportEmtran(ImportEmtran),
}

#[derive(Clone, Args)]
/// Run tests from listed files
struct Run {
    /// Use this to skip tests
    #[arg(short, long, default_value = "0")]
    start: usize,
    /// Execute one single test
    #[arg(short, long)]
    one: bool,
    /// Don't stop on failed test
    #[arg(short, long)]
    keep_going: bool,
    /// Don't show passed tests
    #[arg(short, long)]
    quiet: bool,
    /// Path to minishell executable
    #[arg(short, long, default_value = "../minishell")]
    minishell: PathBuf,
    /// Path to bash executable
    #[arg(long, default_value = "/usr/bin/bash")]
    bash: PathBuf,
    /// Run bash as bash --posix
    #[arg(long)]
    bash_posix: bool,
    /// Use valgrind to check for memory leaks
    #[arg(short, long)]
    leak_check: bool,
    /// Check for correct error messages
    #[arg(short, long)]
    error_check: bool,
    /// Ignore the ignore list
    #[arg(short = 'i', long)]
    no_ignore: bool,
    /// Use bubblewrap to isolate tests in a sandbox
    #[arg(short, long)]
    bwrap: bool,
    /// Path to bwrap executable
    #[arg(long, default_value = "/usr/bin/bwrap")]
    bwrap_path: PathBuf,
    /// Run tests in parallel (random order, needs bubblewrap). Some tests might fail when this is
    /// enabled, double check with normal iteration. -pbqk flags recommended
    #[arg(short, long)]
    parallel: bool,
    /// Watch minishell executable file and run tests on change
    #[arg(short, long)]
    watch: bool,
    /// Paths to tests csv files
    #[arg(required = true)]
    tests: Vec<PathBuf>,
}

#[derive(Args)]
/// Import emtran's test (default source at https://github.com/vietdu91/42_minishell)
struct ImportEmtran {
    #[command(flatten)]
    source: ImportSourceArgs,
    /// Number of lines before first test
    #[arg(short = 's', long, default_value = "24")]
    header_size: usize,
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
    Watch(#[from] WatchError),
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();
    match &cli.command {
        Subcommands::Example => todo!(),
        Subcommands::Run(cli) => {
            if cli.parallel && !cli.bwrap {
                panic!("--parallel needs --bwrap !");
            }
            let run_test_files = {
                let cli = cli.clone();
                move || cli.tests.iter().try_for_each(|file| run_tests(file, &cli))
            };
            if cli.watch {
                watch::blocking(cli, run_test_files)?;
            } else {
                run_test_files()?;
            }
        }
        Subcommands::ImportEmtran(ImportEmtran {
            source,
            header_size,
        }) => import_emtran(&source.into(), *header_size)?,
    }
    Ok(())
}
