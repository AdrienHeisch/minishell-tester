mod exec;
mod parse;

use crate::test::Test;
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

fn run_test(
    test: &Test,
    cli: &Run,
    base_path: &Path,
    output: &mut impl io::Write,
) -> Result<bool, RunError> {
    let current_dir = env::current_dir().map_err(RunError::CurrentDir)?;
    let subdir = current_dir.join(format!("{}", test.id));
    println!("subdir: {subdir:?}");
    fs::create_dir(&subdir).map_err(RunError::CreateDir)?;
    env::set_current_dir(&subdir).map_err(RunError::SetCurrentDir)?;
    exec_test(test, cli, base_path, output).map_err(Into::into)
}

pub fn run_tests(file_path: &Path, cli: &Run) -> Result<(), RunError> {
    let base_path = env::current_dir().map_err(RunError::CurrentDir)?;
    // macro_rules! reset_path_on_err {
    //     ($e:expr, $err:expr) => {
    //         $e.map_err($err).map_err(|err| {
    //             env::set_current_dir(path.join(TMP_DIR))
    //                 .map_err(RunError::SetCurrentDir)
    //                 .err()
    //                 .unwrap_or(err)
    //         })
    //     };
    // }

    let tests_path = base_path.join(file_path);
    let (tests, ignored) = parse_tests(&tests_path, cli)?;
    let (mut passed, mut failed) = (0usize, 0usize);

    println!();
    println!("Running tests from {file_path:?}");

    fs::create_dir(base_path.join(TMP_DIR)).ok();
    for test in tests.iter() {
        env::set_current_dir(base_path.join(TMP_DIR)).map_err(RunError::SetCurrentDir)?;
        let mut output = vec![];
        let is_success = run_test(test, cli, &base_path, &mut output);
        let mut err = None;
        match is_success {
            Ok(true) => {
                passed += 1;
                if !cli.quiet {
                    println!("{}", String::from_utf8_lossy(&output).green());
                }
                if cli.one {
                    break;
                }
            }
            Ok(false) => {
                failed += 1;
                println!("{}", String::from_utf8_lossy(&output).red());
                if cli.one || !cli.keep_going {
                    break;
                }
            }
            Err(RunError::Exec(err)) => {
                println!("{}", String::from_utf8_lossy(&output).red());
                println!("{}", format!("{err}").red());
                println!("{}", "########################".red());
                break;
            }
            Err(e) => err = Some(e),
        }
        if let Some(err) = err {
            return Err(err);
        }
    }

    println!(
        "{}{}{}{}",
        format!("{passed} passed, ").green(),
        format!("{failed} failed, ").red(),
        format!("{ignored} ignored, ").yellow(),
        format!("{} not run", tests.len() - passed - failed).white(),
    );
    fs::remove_dir_all(base_path.join(TMP_DIR)).map_err(RunError::ClearCurrentDir)?;
    env::set_current_dir(base_path).map_err(RunError::SetCurrentDir)?;

    Ok(())
}
