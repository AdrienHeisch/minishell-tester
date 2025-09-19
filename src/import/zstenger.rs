use super::{write_to_file, DownloadError, ImportError, ImportSource, ParseTestError};
use crate::test::Test;
use reqwest::{blocking::Response, IntoUrl};
use std::io;

const FILENAME_TEMPLATE: &str = "zstenger_{}.csv";
const ORIGINAL_URL_BASE: &str =
    "https://raw.githubusercontent.com/zstenger93/42_minishell_tester/refs/heads/master/cmds/";
const ROUTES: &[(&str, &[&str])] = &[
    (
        "bonus",
        &[
            "1_groups.sh",
            "1_operators.sh",
            "1_wildcards.sh",
            "2_corretion.sh",
            "8_syntax_errors.sh",
            "9_go_wild.sh",
        ],
    ),
    (
        "mand",
        &[
            "0_compare_parsing.sh",
            "10_parsing_hell.sh",
            "1_builtins.sh",
            "1_pipelines.sh",
            "1_redirs.sh",
            "1_scmds.sh",
            "1_variables.sh",
            "2_correction.sh",
            "2_path_check.sh",
            "8_syntax_errors.sh",
            "9_go_wild.sh",
        ],
    ),
    ("mini_death", &["1_segfault.sh"]),
    ("no_env", &["10_no_env.sh"]),
];

fn download_file(
    base: &(impl IntoUrl + Clone),
    route: &str,
    file: &str,
) -> Result<Response, DownloadError> {
    let base = base.clone().into_url()?;
    let route = format!("{route}/");
    let url = base.join(&route)?.join(file)?;
    Ok(reqwest::blocking::get(url)?)
}

fn get_reader(
    source: &ImportSource,
    route: &str,
    file: &str,
) -> Result<Box<dyn io::Read>, ImportError> {
    Ok(match source {
        ImportSource::Path(_) => todo!(),
        ImportSource::Url(url) => Box::new(download_file(url, route, file)?),
        ImportSource::Default => Box::new(download_file(&ORIGINAL_URL_BASE, route, file)?),
    })
}

fn parse(mut reader: impl io::Read) -> Result<Vec<Test>, ParseTestError> {
    let reader = {
        let mut str = String::new();
        reader.read_to_string(&mut str)?;
        str
    };
    let mut commands = vec![];
    let mut tests = vec![];
    for line in reader
        .lines()
        .skip_while(|l| l.starts_with('#') || l.is_empty())
    {
        if line.trim_start().starts_with('#') {
            continue;
        }
        if line.is_empty() {
            if !commands.is_empty() {
                let id = tests.len();
                tests.push(Test {
                    id,
                    commands: commands.join("\n"),
                });
                commands = vec![];
            }
        } else {
            commands.push(line);
        }
    }
    Ok(tests)
}

pub fn import(source: &ImportSource) -> Result<(), ImportError> {
    println!("Importing tests...");
    for (route, files) in ROUTES.iter() {
        let mut tests = Vec::new();
        for file in files.iter() {
            tests.append(&mut parse(get_reader(source, route, file)?)?);
        }
        write_to_file(&tests, FILENAME_TEMPLATE, route)?;
    }
    println!("Done !");
    Ok(())
}
