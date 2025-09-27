mod import;
mod run;
mod test;
mod tui;
mod watch;

use clap::{Args, Parser, Subcommand};
use colored::Colorize;
use import::{import_emtran, import_zstenger, ImportError};
use run::{parse_tests, run_tests, RunError, TestResult};
use std::{
    env,
    fmt::Debug,
    fs::{self, Permissions},
    io,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};
use thiserror::Error;
use url::Url;
use watch::WatchError;

#[derive(Parser)]
/// MAXITEST FOR MINISHELL
///
/// Feature highlight: parallel execution, sandboxing, watch for recompilation, TUI
/// interface
///
/// Recommended usage: Run ./maxitest run -bpqw *.csv in a dedicated terminal. This will run your
/// tests every time you recompile your program, printing only the first few errors.
///
/// Don't forget to ./maxitest run --help
///
/// Tests are stored in csv files. Use a spreadsheet editor for convenience.
///
/// For any xxx.csv file, a xxx.ignore file can contain a list of test ids to ignore. One id per
/// line, use # to add comments.
///
/// Try the import-emtran subcommand to get a few hundred tests.
///
/// -> ./maxitest import-emtran && ./maxitext run emtran_mandatory.csv
struct Cli {
    #[command(subcommand)]
    command: Subcommands,
}

#[derive(Subcommand)]
enum Subcommands {
    /// Not available yet
    Example,
    /// Run tests from listed files. Each test will be run with both minishell and bash, and
    /// outputs will be compared
    Run(Run),
    /// Open in TUI mode (work in progress)
    Tui(ExecPaths),
    /// Import emtran's test (default source at https://github.com/vietdu91/42_minishell, thank you
    /// emtran !)
    ImportEmtran(ImportEmtran),
    /// Import zstenger's test (default source at https://github.com/zstenger93/42_minishell_tester, thank you
    /// zstenger !)
    ImportZstenger(ImportEmtran),
}

#[derive(Clone, Default, Args)]
struct ExecPaths {
    /// Path to minishell executable
    #[arg(short, long, default_value = "../minishell")]
    minishell: PathBuf,
    /// Path to bash executable
    #[arg(long, default_value = "/usr/bin/bash")]
    bash: PathBuf,
    /// Path to bwrap executable. Will extract an embedded version if not found and feature
    /// "embed-bwrap" was enabled at compilation
    #[arg(long, default_value = "/usr/bin/bwrap")]
    bwrap_path: PathBuf,
    // TODO use these
    // #[arg(long, default_value = "valgrind")]
    // valgrind_path: PathBuf,
    // #[arg(long, default_value = "funcheck")]
    // funcheck_path: PathBuf,
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
    valgrind: bool,
    /// Use valgrind to check for memory leaks
    #[arg(short, long)]
    funcheck: bool,
    /// Check for correct error messages
    #[arg(short, long)]
    error_check: bool,
    /// Ignore the ignore list
    #[arg(short = 'i', long)]
    no_ignore: bool,
    /// Use bubblewrap to isolate tests in a sandbox. Enforces consistent environment, prevents
    /// accidental file deletion
    #[arg(short, long)]
    bwrap: bool,
    /// Run tests in parallel (random order, needs bubblewrap). Known issue:
    /// some tests might fail when this is enabled, double check with normal iteration.
    #[arg(short, long)]
    parallel: bool,
    /// Watch minishell executable file and run tests on change
    #[arg(short, long)]
    watch: bool,
    /// Paths to tests csv files. Individual test results will be hidden if there are multiple
    /// files, so *.csv with --keep-going is a good option for a full test
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
    #[error("Failed to extract bwrap executable: {0}")]
    Bwrap(io::Error),
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();
    match cli.command {
        Subcommands::Example => todo!(),
        Subcommands::Run(cli) => {
            if cli.parallel && !cli.bwrap {
                panic!("--parallel needs --bwrap !");
            }
            if cli.valgrind && cli.funcheck {
                panic!("--valgrind conflicts with --funcheck !");
            }
            let run_test_files = {
                let cli = cli.clone();
                let do_show = cli.tests.len() == 1;
                move || {
                    cli.tests.iter().try_for_each(|file| {
                        println!();
                        println!("Running tests from {file:?}");
                        let (tests, ignored) = parse_tests(file, &cli)?;
                        let results = run_tests(&tests, &cli, do_show)?;
                        println!("{}", recap(tests.len(), ignored, &results));
                        Ok(())
                    })
                }
            };
            run_test_files()?;
            if cli.watch {
                watch::blocking(&cli, run_test_files)?;
            }
        }
        Subcommands::Tui(exec_paths) => {
            tui::run(exec_paths.clone()).unwrap();
        }
        Subcommands::ImportEmtran(ImportEmtran {
            source,
            header_size,
        }) => import_emtran(&(&source).into(), header_size)?,
        Subcommands::ImportZstenger(ImportEmtran { source, .. }) => {
            import_zstenger(&(&source).into())?
        }
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
