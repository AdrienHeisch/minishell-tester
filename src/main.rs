mod exec;
mod parse;

use clap::Parser;
use colored::Colorize;
use exec::{exec_test, ExecError};
use parse::{parse_tests, ParseTestError};
use std::{
    env, fs, io,
    num::ParseIntError,
    path::{Path, PathBuf},
};
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
    #[arg(short, long, default_value = "mandatory")]
    level: Level,
    #[arg(short, long, default_value = "0")]
    start: usize,
    #[arg(short, long, default_value = "../minishell")]
    program: PathBuf,
    #[arg(long, default_value = "bash")]
    bash: PathBuf,
    #[arg(short, long, default_value = "tests.csv")]
    tests: PathBuf,
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
    Blacklist(#[from] BlacklistError),
    ParseTest(#[from] ParseTestError),
    ExecTest(#[from] ExecError),
}

#[derive(Debug, Error)]
enum BlacklistError {
    #[error("Failed to parse blacklist: {0}")]
    Parse(#[from] ParseIntError),
    #[error("Failed to read blacklist file: {0}")]
    Io(#[from] io::Error),
}

fn read_blacklist(path: &Path) -> Result<Vec<usize>, BlacklistError> {
    match fs::read_to_string(path) {
        Ok(blacklist) => blacklist
            .split('\n')
            .take_while(|id| !id.is_empty())
            .map(|id| id.parse::<usize>().map_err(Into::into))
            .collect(),
        Err(_) if path.as_os_str() == DEFAULT_BLACKLIST_PATH => Ok(vec![]),
        Err(err) => Err(err.into()),
    }
}

fn run_tests(cli: &Cli) -> Result<(), Error> {
    let path = env::current_dir().map_err(Error::CurrentDir)?;
    let blacklist = read_blacklist(&cli.blacklist)?;
    let tests_path = path.join(&cli.tests);
    let program_path = path.join(&cli.program);
    let bash_path = &cli.bash;
    fs::create_dir(path.join(TMP_DIR)).ok();
    env::set_current_dir(path.join(TMP_DIR)).map_err(Error::SetCurrentDir)?;
    let (tests, n_ignored_tests) = parse_tests(&tests_path, cli, &blacklist)?;
    for test in tests.iter().skip_while(|test| test.id != cli.start) {
        match exec_test(test, &program_path, bash_path) {
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
