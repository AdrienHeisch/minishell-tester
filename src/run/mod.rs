mod exec;

use exec::{exec_test, ExecError};
use crate::parse::{parse_tests, ParseTestError};
use crate::Cli;
use colored::Colorize;
use std::{env, fs, io};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub enum RunError {
    #[error("Failed to get current directory: {0}")]
    CurrentDir(io::Error),
    #[error("Failed to set current directory: {0}")]
    SetCurrentDir(io::Error),
    #[error("Failed to clear current directory: {0}")]
    ClearCurrentDir(io::Error),
    ParseTest(#[from] ParseTestError),
    ExecTest(#[from] ExecError),
}

const TMP_DIR: &str = "tmp";

pub fn run_tests(cli: &Cli) -> Result<(), RunError> {
    let path = env::current_dir().map_err(RunError::CurrentDir)?;
    let tests_path = path.join(&cli.tests);
    let (tests, n_ignored_tests) = parse_tests(&tests_path, cli)?;

    fs::create_dir(path.join(TMP_DIR)).ok();
    env::set_current_dir(path.join(TMP_DIR)).map_err(RunError::SetCurrentDir)?;
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
        println!("{}", format!("\n!!!   {n_ignored_tests} IGNORED TESTS   !!!").yellow());
    }
    fs::remove_dir_all(path.join(TMP_DIR)).map_err(RunError::ClearCurrentDir)?;

    Ok(())
}
