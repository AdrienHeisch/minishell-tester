mod import;
mod run;
mod test;
mod tui;
mod watch;

use clap::{Args, Parser, Subcommand};
use colored::Colorize;
use import::{import_emtran, ImportError};
use run::{parse_tests, run_tests, RunError, TestResult};
use std::{fmt::Debug, path::PathBuf};
use thiserror::Error;
use url::Url;
use watch::WatchError;

#[derive(Parser)]
/// MAXITEST FOR MINISHELL
///
/// Feature highlight: sandboxing, parallel execution, watch for recompilation, TUI
/// interface
///
/// Tests are stored in csv files. Use a spreadsheet editor for convenience.
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
    /// Not available yet
    Example,
    /// Run tests from listed files
    Run(Run),
    /// Open in TUI mode (work in progress)
    Tui(ExecPaths),
    /// Import emtran's test (default source at https://github.com/vietdu91/42_minishell, thank you
    /// emtran !)
    ImportEmtran(ImportEmtran),
}

#[derive(Clone, Default, Args)]
struct ExecPaths {
    /// Path to minishell executable
    #[arg(short, long, default_value = "../minishell")]
    minishell: PathBuf,
    /// Path to bash executable
    #[arg(long, default_value = "/usr/bin/bash")]
    bash: PathBuf,
    /// Path to bwrap executable
    #[arg(long, default_value = "/usr/bin/bwrap")]
    bwrap_path: PathBuf,
}

#[derive(Clone, Default, Args)]
struct Run {
    #[command(flatten)]
    exec_paths: ExecPaths,
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
    Import(#[from] ImportError),
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
                move || {
                    cli.tests.iter().try_for_each(|file| {
                        println!();
                        println!("Running tests from {file:?}");
                        let (tests, ignored) = parse_tests(file, &cli)?;
                        let results = run_tests(&tests, &cli, true)?;
                        println!("{}", recap(tests.len(), ignored, &results));
                        Ok(())
                    })
                }
            };
            if cli.watch {
                watch::blocking(cli, run_test_files)?;
            } else {
                run_test_files()?;
            }
        }
        Subcommands::Tui(exec_paths) => {
            tui::run(exec_paths.clone()).unwrap();
        }
        Subcommands::ImportEmtran(ImportEmtran {
            source,
            header_size,
        }) => import_emtran(&source.into(), *header_size)?,
    }
    Ok(())
}

fn show(cli: &Run, res: &TestResult, mut f: impl FnMut(&str)) {
    match res {
        TestResult::Error(out) | TestResult::Failed(out) => f(&format!("{}", out.red())),
        TestResult::Passed(out) if !cli.quiet => f(&format!("{}", out.green())),
        _ => (),
    }
}

fn recap(n_tests: usize, ignored: usize, results: &[TestResult]) -> String {
    let (passed, failed) = results.iter().fold((0, 0), |(p, f), res| match res {
        TestResult::None => (p, f),
        TestResult::Error(_) => (p, f),
        TestResult::Passed(_) => (p + 1, f),
        TestResult::Failed(_) => (p, f + 1),
    });
    format!(
        "{}{}{}{}",
        format!("{passed} passed, ").green(),
        format!("{failed} failed, ").red(),
        format!("{ignored} ignored, ").yellow(),
        format!("{} not run", n_tests - passed - failed).white(),
    )
}
