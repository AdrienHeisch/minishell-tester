mod exec;
mod parse;

use clap::Parser;
use colored::Colorize;
use exec::{exec_test, ExecError};
use parse::{parse_tests, ParseTestError};
use std::{env, fs, io, path::PathBuf};
use thiserror::Error;

const TMP_DIR: &str = "tmp";
const DEFAULT_BLACKLIST_PATH: &str = "blacklist";

struct Test {
    id: usize,
    commands: String,
}

#[derive(Clone, PartialEq, PartialOrd, clap::ValueEnum)]
enum Level {
    #[allow(unused)]
    Mandatory,
    Bonus,
    More,
}

#[derive(Parser)]
struct Cli {
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
    /// Will not clear some readonly variables in bash -> list with "env -i bash -c set"
    #[arg(long)]
    no_env: bool,
    /// Path to tests csv file
    #[arg(short, long, default_value = "tests.csv")]
    tests: PathBuf,
    /// Path to blacklist file, should contain test ids to be ignored, each followed by a line break
    #[arg(short, long, default_value = DEFAULT_BLACKLIST_PATH)]
    blacklist: PathBuf,
}

#[derive(Debug, Error)]
#[error("{0}")]
enum Error {
    #[error("Failed to get current directory: {0}")]
    CurrentDir(io::Error),
    #[error("Failed to set current directory: {0}")]
    SetCurrentDir(io::Error),
    #[error("Failed to clear current directory: {0}")]
    ClearCurrentDir(io::Error),
    ParseTest(#[from] ParseTestError),
    ExecTest(#[from] ExecError),
}

fn run_tests(cli: &Cli) -> Result<(), Error> {
    let path = env::current_dir().map_err(Error::CurrentDir)?;
    let tests_path = path.join(&cli.tests);
    let (tests, n_ignored_tests) = parse_tests(&tests_path, cli)?;

    fs::create_dir(path.join(TMP_DIR)).ok();
    env::set_current_dir(path.join(TMP_DIR)).map_err(Error::SetCurrentDir)?;
    for test in tests.iter().skip_while(|test| test.id != cli.start) {
        match exec_test(test, cli, &path) {
            Ok((message, success)) => {
                if success {
                    println!("{}", message.green());
                } else {
                    println!("{}", message.red());
                    return Ok(());
                }
            }
            Err(err @ ExecError(_, _)) => {
                println!(
                    "{}",
                    format!("{}\n{}\n######################", err.0, err.1).red()
                );
                return Ok(());
            }
        }
    }
    if n_ignored_tests > 0 {
        println!("\n!!!   {n_ignored_tests} IGNORED TESTS   !!!");
    }
    fs::remove_dir_all(path.join(TMP_DIR)).map_err(Error::ClearCurrentDir)?;

    Ok(())
}

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run_tests(&cli) {
        eprintln!("{err}");
    }
}
