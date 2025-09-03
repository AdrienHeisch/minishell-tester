mod exec;
mod parse;

use clap::Parser;
use colored::Colorize;
use exec::{exec_test, ExecError, ExecOk};
use parse::{parse_tests, ParseTestError};
use std::{env, fs, io, num::ParseIntError};
use thiserror::Error;

const TMP_DIR: &str = "tmp";

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
    program: String,
    #[arg(short, long, default_value = "tests.csv")]
    tests: String,
}

#[derive(Debug, Error)]
#[error("{0}")]
enum Error {
    CurrentDir(#[from] CurrentDirError),
    SetCurrentDir(#[from] SetCurrentDirError),
    ClearCurrentDir(#[from] ClearCurrentDirError),
    Blacklist(#[from] BlacklistError),
    ParseTest(#[from] ParseTestError),
    #[error("")]
    ExecTest(#[from] ExecError),
}

#[derive(Debug, Error)]
#[error("Failed to get current directory: {0}")]
struct CurrentDirError(#[from] io::Error);

#[derive(Debug, Error)]
#[error("Failed to set current directory: {0}")]
struct SetCurrentDirError(#[from] io::Error);

#[derive(Debug, Error)]
#[error("Failed to clear current directory: {0}")]
struct ClearCurrentDirError(#[from] io::Error);

#[derive(Debug, Error)]
#[error("Failed to parse blacklist: {0}")]
struct BlacklistError(#[from] ParseIntError);

fn read_blacklist() -> Result<Vec<usize>, BlacklistError> {
    match fs::read_to_string("blacklist") {
        Ok(blacklist) => blacklist
            .split('\n')
            .take_while(|id| !id.is_empty())
            .map(|id| id.parse::<usize>().map_err(BlacklistError))
            .collect(),
        Err(_) => Ok(vec![]),
    }
}

fn run_tests(cli: &Cli) -> Result<(), Error> {
    let path = env::current_dir().map_err(CurrentDirError)?;
    let blacklist = read_blacklist()?;
    let tests_path = path.join(&cli.tests);
    let program_path = path.join(&cli.program);
    fs::create_dir(path.join(TMP_DIR)).ok();
    env::set_current_dir(path.join(TMP_DIR)).map_err(SetCurrentDirError)?;
    let (tests, n_ignored_tests) = parse_tests(&tests_path, cli, &blacklist)?;
    for test in tests.iter().skip_while(|test| test.id != cli.start) {
        match exec_test(test, &program_path) {
            Ok(ExecOk(message, success)) => {
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
                return Err(Error::from(err));
            }
        }
    }
    if n_ignored_tests > 0 {
        println!("\n!!!   {n_ignored_tests} IGNORED TESTS   !!!");
    }
    fs::remove_dir_all(path.join(TMP_DIR)).map_err(ClearCurrentDirError)?;
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run_tests(&cli) {
        eprintln!("{err}");
    }
}
