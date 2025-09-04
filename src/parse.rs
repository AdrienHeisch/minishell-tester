use crate::{test::Test, Cli};
use std::{
    fs::{self, File},
    io,
    num::ParseIntError,
    path::Path,
};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub enum ParseTestError {
    Io(#[from] io::Error),
    Csv(#[from] csv::Error),
    Ignore(#[from] IgnoreError),
}

#[derive(Debug, Error)]
pub enum IgnoreError {
    #[error("Failed to parse ignore file: {0}")]
    Parse(#[from] ParseIntError),
    #[error("Failed to read ignore file: {0}")]
    Io(#[from] io::Error),
}

fn read_ignore_file(path: &Path) -> Result<Vec<usize>, IgnoreError> {
    match fs::read_to_string(path) {
        Ok(ignore) => ignore
            .split('\n')
            .take_while(|id| !id.is_empty())
            .filter_map(|id| id.split('#').next())
            .filter(|id| !id.is_empty())
            .map(|id| id.trim().parse::<usize>().map_err(Into::into))
            .collect(),
        Err(err) if matches!(err.kind(), io::ErrorKind::NotFound) => Ok(vec![]),
        Err(err) => Err(err.into()),
    }
}

pub fn parse_tests(path: &Path, cli: &Cli) -> Result<(Vec<Test>, usize), ParseTestError> {
    let mut ignore_path = path.to_owned();
    ignore_path.set_extension("ignore");
    let ignore = match cli.no_ignore {
        false => read_ignore_file(&ignore_path)?,
        true => vec![],
    };
    let mut reader = csv::Reader::from_reader(File::open(path)?);
    let mut tests = vec![];
    let mut n_ignored_tests = 0;
    for (id, test) in reader.deserialize::<Test>().enumerate() {
        if id < cli.start {
            continue ;
        }
        let mut test = test?;
        if ignore.contains(&id) {
            n_ignored_tests += 1;
            continue;
        }
        test.id = id;
        if cli.level < test.level {
            n_ignored_tests += 1;
            continue;
        }
        tests.push(test);
    }
    Ok((tests, n_ignored_tests))
}
