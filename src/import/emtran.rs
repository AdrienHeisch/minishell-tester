use super::{DownloadError, ImportError, ImportSource, ParseTestError};
use crate::test::Test;
use reqwest::{blocking::Response, IntoUrl, Url};
use std::{
    fs::{self, OpenOptions},
    io,
};

const FILENAME_TEMPLATE: &str = "emtran_{}.csv";
const ORIGINAL_URL: &str = "https://docs.google.com/spreadsheets/d/1uJHQu0VPsjjBkR4hxOeCMEt3AOM1Hp_SmUzPFhAH-nA/edit#gid=0";

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

type Tests = (Vec<Test>, Vec<Test>, Vec<Test>);

fn parse(reader: impl io::Read, header_size: usize) -> Result<Tests, ParseTestError> {
    let mut reader = csv::Reader::from_reader(reader);
    let (mut mandatory, mut bonus, mut more) = Tests::default();
    for record in reader.records().skip(header_size) {
        let record = record?;
        let out = match record.get(9) {
            Some(str) if str.contains("[BONUS]") => &mut bonus,
            _ => match record.get(2) {
                Some(str) if !str.is_empty() => &mut more,
                _ => match record.get(1) {
                    Some("$> echo $hola*") => &mut bonus,
                    _ => &mut mandatory,
                },
            },
        };
        let commands = record.get(1).unwrap_or("");
        if [
            "Ctlr-",
            "env -i",
            "[touche du haut]",
            "!!! Contenu du fichier",
        ]
        .iter()
        .any(|str| commands.contains(str))
        {
            continue;
        }
        let mut expected = record.get(7).unwrap_or_default().lines();
        let mut lines = Vec::<String>::new();
        for line in commands.lines() {
            let line = match line.strip_prefix("$> ") {
                Some(line) => line.to_string(),
                None => match lines.last_mut() {
                    Some(prev) => {
                        prev.push_str(line);
                        continue;
                    }
                    None => line.to_string(),
                },
            };
            lines.push(line.clone());
            if line.contains("<<") {
                expected
                    .by_ref()
                    .map_while(|line| line.strip_prefix("> "))
                    .for_each(|line| lines.push(line.to_string()));
            }
        }
        let commands = lines
            .join("\n")
            .replace("(touche entrée)", "\n")
            .replace("[que des espaces]", "           ")
            .replace("[que des tabulations]", "\t\t\t\t\t\t\t\t")
            .replace("[$TERM]", "\"[$TERM]\"")
            .replace("sleep 3", "sleep 0")
            .replace("vietdu91", "maxitester");
        let id = out.len();
        out.push(Test { id, commands });
    }
    Ok((mandatory, bonus, more))
}

fn write_to_file(tests: &[Test], name: &str) -> Result<(), ImportError> {
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(FILENAME_TEMPLATE.replace("{}", name))
        .map_err(ImportError::WriteOutput)?;
    let mut writer = csv::Writer::from_writer(file);
    for test in tests {
        writer.serialize(test)?;
    }
    Ok(())
}

pub fn import(source: &ImportSource, header_size: usize) -> Result<(), ImportError> {
    let (mandatory, bonus, more) = parse(get_reader(source)?, header_size)?;
    write_to_file(&mandatory, "mandatory")?;
    write_to_file(&bonus, "bonus")?;
    write_to_file(&more, "more")?;
    println!("Done !");
    Ok(())
}
