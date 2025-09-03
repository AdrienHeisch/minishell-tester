use thiserror::Error;

use crate::{Cli, Level, Test};
use std::{fs::File, io, path::Path};

const BONUS_RANGES: &[std::ops::RangeInclusive<usize>] = &[549..=574, 575..=612]; //, 737..=742];

fn fix_commands(commands: &str) -> String {
    commands
        .replace("(touche entrée)", "\n")
        .replace("[que des espaces]", "           ")
        .replace("[que des tabulations]", "\t\t\t\t\t\t\t\t")
        .replace("$UID", "$SHELL")
        .replace(" [$TERM],", " \"[$TERM]\",")
        .replace("sleep 3", "sleep 0")
        .replace("../", "./")
}

#[derive(Debug, Error)]
#[error("0")]
pub struct ParseTestError(#[from] io::Error);

pub fn parse_tests(
    path: &Path,
    cli: &Cli,
    blacklist: &[usize],
) -> Result<(Vec<Test>, usize), ParseTestError> {
    let file = File::open(path)?;
    let mut n_ignored_tests = 0;
    let mut reader = csv::Reader::from_reader(file);
    let mut tests = vec![];
    for (id, result) in reader.records().skip(24).enumerate() {
        if blacklist.contains(&id) {
            n_ignored_tests += 1;
            continue;
        }
        let record = result.map_err(|err| {
            let out: io::Error = err.into();
            out
        })?;
        if BONUS_RANGES.iter().any(|range| range.contains(&id)) {
            if cli.level < Level::Bonus {
                n_ignored_tests += 1;
                continue;
            }
        } else if cli.level < Level::More {
            match record.get(2) {
                Some(str) if !str.is_empty() => {
                    n_ignored_tests += 1;
                    continue;
                }
                _ => (),
            }
        }
        let mut commands = if let Some(commands) = record.get(1) {
            let mut is_valid = true;
            if commands.contains("Ctlr-")
                || commands.contains("env")
                || commands.contains("export")
                || commands.contains("unset")
            {
                n_ignored_tests += 1;
                continue;
            }
            let mut lines = Vec::new();
            for line in commands.lines() {
                let stripped = line.strip_prefix("$> ");
                match stripped {
                    Some(line) => lines.push(line.to_owned()),
                    None => match lines.last_mut() {
                        Some(prev) => prev.push_str(line),
                        None => {
                            is_valid = false;
                            break;
                        }
                    },
                }
            }
            let commands = lines.join("\n");
            if !is_valid {
                println!("INVALID TEST : {id}");
                if !commands.is_empty() {
                    println!("{commands}");
                }
                n_ignored_tests += 1;
                continue;
            }
            commands
        } else {
            n_ignored_tests += 1;
            continue;
        };
        commands = fix_commands(&commands);
        tests.push(Test { id, commands });
    }
    Ok((tests, n_ignored_tests))
}
