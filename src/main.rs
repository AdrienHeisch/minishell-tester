mod exec;
mod parse;

use clap::Parser;
use exec::exec_test;
use parse::parse_tests;
use std::io::{self};

const BONUS_RANGES: &[std::ops::RangeInclusive<usize>] = &[549..=574, 575..=612]; //, 737..=742];
const BLACKLIST: &[usize] = &[
    2, 3, 24, 68, 92, 102, 103, 405, 407, 418, 424, 425, 427, 734, 48, 49, 50, 51, 120, 123, 360,
];

struct Test {
    id: usize,
    commands: String,
}

#[derive(Clone, PartialEq, PartialOrd, clap::ValueEnum)]
enum Level {
    #[allow(unused)]
    Mandatory,
    Bonus,
    More,
}

#[derive(Parser)]
struct Cli {
    #[arg(short, long, default_value = "mandatory")]
    level: Level,
    #[arg(short, long, default_value_t)]
    skip_n: usize,
    #[arg(short, long, default_value = "../minishell")]
    program: String,
    #[arg(short, long, default_value = "tests.csv")]
    tests: String,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();
    let path = std::env::current_dir()?;
    let tests_path = path.join(&cli.tests);
    let program_path = path.join(&cli.program);
    std::fs::create_dir(path.join("tmp")).ok();
    std::env::set_current_dir(path.join("tmp"))?;
    for test in parse_tests(&tests_path, &cli)?
        .iter()
        .skip_while(|test| test.id < cli.skip_n)
    {
        if !exec_test(test, &program_path)? {
            return Ok(());
        }
    }
    std::fs::remove_dir_all(path.join("tmp"))?;
    Ok(())
}
