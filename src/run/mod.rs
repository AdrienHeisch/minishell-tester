mod exec;
mod parse;

use crate::Run;
use colored::Colorize;
use exec::{exec_test, ExecError};
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
    #[error("Failed to create directory: {0}")]
    CreateDir(io::Error),
    #[error("Failed to clear current directory: {0}")]
    ClearCurrentDir(io::Error),
    ParseTest(#[from] ParseTestError),
    Exec(#[from] ExecError),
}

const TMP_DIR: &str = "tmp";

#[derive(Default)]
enum TestRes {
    #[default]
    None,
    Failed,
    Passed,
}

pub fn run_tests(file_path: &Path, cli: &Run) -> Result<(), RunError> {
    let base_path = env::current_dir().map_err(RunError::CurrentDir)?;
    let tests_path = base_path.join(file_path);
    let tmp_path = base_path.join(TMP_DIR);
    fs::create_dir(base_path.join(TMP_DIR)).map_err(RunError::CreateDir)?;

    let (tests, ignored) = parse_tests(&tests_path, cli)?;
    let mut tests = tests
        .into_iter()
        .map(|test| (test, TestRes::None))
        .collect::<Vec<_>>();

    println!();
    println!("Running tests from {file_path:?}");

    let res = tests.iter_mut().try_for_each(
        |(ref test, ref mut res)| -> Result<(), Option<RunError>> {
            let exec_path = tmp_path.join(format!("{}", test.id));
            fs::create_dir(&exec_path).map_err(RunError::CreateDir)?;
            println!("exec_path: {exec_path:?}");
            let mut output = vec![];
            let is_success =
                exec_test(test, cli, &base_path, &exec_path, &mut output);
            match is_success {
                Ok(true) => {
                    *res = TestRes::Passed;
                    if !cli.quiet {
                        println!("{}", String::from_utf8_lossy(&output).green());
                    }
                    if cli.one {
                        return Err(None);
                    }
                }
                Ok(false) => {
                    *res = TestRes::Failed;
                    println!("{}", String::from_utf8_lossy(&output).red());
                    if cli.one || !cli.keep_going {
                        return Err(None);
                    }
                }
                Err(err) => {
                    println!("{}", String::from_utf8_lossy(&output).red());
                    println!("{}", format!("{err}").red());
                    println!("{}", "########################".red());
                    return Err(None);
                }
            }
            Ok(())
        },
    );

    if let Err(Some(err)) = res {
        return Err(err);
    }

    let passed = tests
        .iter()
        .filter(|(_, res)| matches!(res, TestRes::Passed))
        .count();
    let failed = tests
        .iter()
        .filter(|(_, res)| matches!(res, TestRes::Failed))
        .count();

    println!(
        "{}{}{}{}",
        format!("{passed} passed, ").green(),
        format!("{failed} failed, ").red(),
        format!("{ignored} ignored, ").yellow(),
        format!("{} not run", tests.len() - passed - failed).white(),
    );
    fs::remove_dir_all(base_path.join(TMP_DIR)).map_err(RunError::ClearCurrentDir)?;

    Ok(())
}
