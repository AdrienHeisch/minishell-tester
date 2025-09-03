use crate::{test::Test, Cli, DEFAULT_BLACKLIST_PATH};
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
    Blacklist(#[from] BlacklistError),
}

#[derive(Debug, Error)]
pub enum BlacklistError {
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
            .filter(|id| !id.starts_with("#"))
            .map(|id| id.parse::<usize>().map_err(Into::into))
            .collect(),
        Err(_) if path.as_os_str() == DEFAULT_BLACKLIST_PATH => Ok(vec![]),
        Err(err) => Err(err.into()),
    }
}

pub fn parse_tests(path: &Path, cli: &Cli) -> Result<(Vec<Test>, usize), ParseTestError> {
    let blacklist = match cli.no_ignore {
        false => read_blacklist(&cli.blacklist)?,
        true => vec![],
    };
    let mut reader = csv::Reader::from_reader(File::open(path)?);
    let mut tests = vec![];
    let mut n_ignored_tests = 0;
    for (id, test) in reader.deserialize::<Test>().enumerate() {
        let mut test = test?;
        if blacklist.contains(&id) {
            n_ignored_tests += 1;
            continue;
        }
        test.id = id;
        if cli.level < test.level {
            n_ignored_tests += 1;
            continue;
        }
        if ["Ctlr-", "env", "export", "unset"]
            .iter()
            .any(|str| test.commands.contains(str))
        {
            n_ignored_tests += 1;
            continue;
        }
        tests.push(test);
    }
    Ok((tests, n_ignored_tests))
}
