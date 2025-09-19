mod exec;
mod parse;

pub use parse::parse_tests;

use crate::Run;
use crate::{show, test::Test};
use exec::{exec_test, ExecError};
use parse::ParseTestError;
use rayon::prelude::*;
use std::{env, fs, io};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub enum RunError {
    #[error("Failed to get current directory: {0}")]
    CurrentDir(io::Error),
    #[error("Failed to create directory: {0}")]
    CreateDir(io::Error),
    #[error("Failed to clear current directory: {0}")]
    ClearCurrentDir(io::Error),
    ParseTest(#[from] ParseTestError),
    Exec(#[from] ExecError),
}

const TMP_DIR: &str = "tmp";

#[derive(Clone, Default)]
pub enum TestResult {
    #[default]
    None,
    Error(String),
    Failed(String),
    Passed(String),
}

pub fn run_tests(tests: &[Test], cli: &Run, do_show: bool) -> Result<Vec<TestResult>, RunError> {
    let base_path = env::current_dir().map_err(RunError::CurrentDir)?;
    let tmp_path = base_path.join(TMP_DIR);
    match fs::remove_dir_all(&tmp_path) {
        Err(err) if err.kind() == io::ErrorKind::NotFound => (),
        Err(err) => Err(RunError::ClearCurrentDir(err))?,
        Ok(()) => (),
    }
    fs::create_dir(base_path.join(TMP_DIR)).map_err(RunError::CreateDir)?;

    let mut tests = tests
        .iter()
        .map(|test| (test, TestResult::None))
        .collect::<Vec<_>>();

    let run_test = |test: &Test, res: &mut TestResult| -> Result<(), Option<RunError>> {
        let exec_path = tmp_path.join(format!("{}", test.id));
        fs::create_dir(&exec_path).map_err(RunError::CreateDir)?;
        let mut output = vec![];
        let is_success = exec_test(test, cli, &base_path, &exec_path, &mut output);
        let output = String::from_utf8_lossy(&output);
        match is_success {
            Ok(true) => {
                *res = TestResult::Passed(output.to_string());
                if do_show {
                    show(cli, res, |res| println!("{res}"));
                }
                fs::remove_dir_all(&exec_path).map_err(RunError::ClearCurrentDir)?;
            }
            Ok(false) => {
                *res = TestResult::Failed(output.to_string());
                if do_show {
                    show(cli, res, |res| println!("{res}"));
                }
                if !cli.keep_going {
                    Err(None)?
                }
            }
            Err(err) => {
                let err = format!("{output}{err}\n########################");
                *res = TestResult::Error(err);
                if do_show {
                    show(cli, res, |res| println!("{res}"));
                }
                Err(None)?
            }
        }
        Ok(())
    };

    let res = if cli.parallel {
        tests
            .par_iter_mut()
            .try_for_each(|(test, res)| run_test(test, res))
    } else {
        tests
            .iter_mut()
            .try_for_each(|(test, res)| run_test(test, res))
    };

    match fs::remove_dir(&tmp_path) {
        Err(err) if err.kind() == io::ErrorKind::DirectoryNotEmpty => (),
        Err(err) => Err(RunError::ClearCurrentDir(err))?,
        Ok(()) => (),
    }

    if let Err(Some(err)) = res {
        Err(err)?
    }

    Ok(tests.into_iter().map(|(_, res)| res).collect())
}
