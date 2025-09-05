mod exec;
mod parse;

use crate::Run;
use colored::Colorize;
use exec::exec_test;
use parse::{parse_tests, ParseTestError};
use std::path::Path;
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
}

const TMP_DIR: &str = "tmp";

pub fn run_tests(file_path: &Path, cli: &Run) -> Result<(), RunError> {
    let path = env::current_dir().map_err(RunError::CurrentDir)?;
    let tests_path = path.join(file_path);
    let (tests, ignored) = parse_tests(&tests_path, cli)?;
    let (mut passed, mut failed) = (0usize, 0usize);

    println!();
    println!("Running tests from {file_path:?}");

    fs::create_dir(path.join(TMP_DIR)).ok();
    env::set_current_dir(path.join(TMP_DIR)).map_err(RunError::SetCurrentDir)?;
    for test in tests.iter() {
        let mut output = vec![];
        match exec_test(test, cli, &path, &mut output) {
            Ok(success) => {
                if success {
                    passed += 1;
                    if !cli.quiet {
                        println!("{}", String::from_utf8_lossy(&output).green());
                    }
                    if cli.one {
                        break;
                    }
                } else {
                    failed += 1;
                    println!("{}", String::from_utf8_lossy(&output).red());
                    if cli.one || !cli.keep_going {
                        break;
                    }
                }
            }
            Err(err) => {
                println!("{}", String::from_utf8_lossy(&output).red());
                println!("{}", format!("{err}").red());
                println!("{}", "########################".red());
                break;
            }
        }
    }

    println!(
        "{}{}{}{}",
        format!("{passed} passed, ").green(),
        format!("{failed} failed, ").red(),
        format!("{ignored} ignored, ").yellow(),
        format!("{} not run", tests.len() - passed - failed).white(),
    );
    fs::remove_dir_all(path.join(TMP_DIR)).map_err(RunError::ClearCurrentDir)?;

    Ok(())
}
