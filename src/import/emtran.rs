use super::{DownloadError, ImportError, ImportSource, ParseTestError};
use crate::test::{Level, Test};
use reqwest::{blocking::Response, IntoUrl, Url};
use std::{
    fs::{self, OpenOptions},
    io,
    path::Path,
};

const ORIGINAL_URL: &str = "https://docs.google.com/spreadsheets/d/1uJHQu0VPsjjBkR4hxOeCMEt3AOM1Hp_SmUzPFhAH-nA/edit#gid=0";
const HEADER_SIZE: usize = 24;
const BONUS_RANGES: &[std::ops::RangeInclusive<usize>] = &[549..=574, 575..=612, 737..=742];

fn download_file(url: &(impl IntoUrl + Clone)) -> Result<Response, DownloadError> {
    let mut url = url.clone().into_url()?;
    {
        let mut path = url
            .path_segments_mut()
            .map_err(|_| DownloadError::InvalidUrl)?;
        path.pop();
        path.push("export");
    }
    url.query_pairs_mut().append_pair("format", "csv");
    println!("Downloading tests...");
    Ok(reqwest::blocking::get(url)?)
}

fn get_reader(source: &ImportSource) -> Result<Box<dyn io::Read>, ImportError> {
    Ok(match source {
        ImportSource::Path(path) => {
            println!("Importing tests...");
            Box::new(fs::File::open(path).map_err(ImportError::ReadSource)?)
        }
        ImportSource::Url(url) => Box::new(download_file(url)?),
        ImportSource::Default => Box::new(download_file(&Url::parse(ORIGINAL_URL)?)?),
    })
}

fn parse(reader: impl io::Read, header_size: &Option<usize>) -> Result<Vec<Test>, ParseTestError> {
    let mut reader = csv::Reader::from_reader(reader);
    let mut tests = vec![];
    for (id, record) in reader
        .records()
        .skip(header_size.unwrap_or(HEADER_SIZE))
        .enumerate()
    {
        let record = record?;
        let level = if BONUS_RANGES.iter().any(|range| range.contains(&id)) {
            Level::Bonus
        } else {
            match record.get(2) {
                Some(str) if !str.is_empty() => Level::More,
                _ => Level::Mandatory,
            }
        };
        let commands = record
            .get(1)
            .unwrap_or("")
            .replace("(touche entrée)", "\n")
            .replace("[que des espaces]", "           ")
            .replace("[que des tabulations]", "\t\t\t\t\t\t\t\t")
            .replace("$UID", "$SHELL")
            .replace(" [$TERM],", " \"[$TERM]\",")
            .replace("sleep 3", "sleep 0");
        let mut lines = Vec::new();
        for line in commands.lines() {
            let stripped = line.strip_prefix("$> ");
            match stripped {
                Some(line) => lines.push(line.to_owned()),
                None => match lines.last_mut() {
                    Some(prev) => prev.push_str(line),
                    None => lines.push(line.to_owned()),
                },
            }
        }
        let commands = lines.join("\n");
        tests.push(Test {
            id,
            level,
            commands,
        });
    }
    Ok(tests)
}

pub fn import(
    source: &ImportSource,
    output: &Path,
    header_size: &Option<usize>,
) -> Result<(), ImportError> {
    let tests = parse(get_reader(source)?, header_size)?;
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output)
        .map_err(ImportError::WriteOutput)?;
    let mut writer = csv::Writer::from_writer(file);
    for test in tests {
        writer.serialize(test)?;
    }
    println!("Done !");
    Ok(())
}
