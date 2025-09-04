use crate::{test::Test, Cli};
use regex::Regex;
use std::{
    env,
    ffi::OsStr,
    fs, io,
    path::Path,
    process::{Command, Output},
};
use thiserror::Error;

fn clear_dir(dir: &Path) -> io::Result<()> {
    fs::remove_dir_all(dir)?;
    fs::create_dir(dir)?;
    env::set_current_dir(dir)?;
    Ok(())
}

fn exec(
    program: impl AsRef<OsStr>,
    commands: &str,
    options: &[&str],
    leak_check: bool,
    bubblewrap: Option<&Path>,
) -> io::Result<Output> {
    let mut command;
    match (bubblewrap, leak_check) {
        (Some(bubblewrap), true) => {
            command = Command::new(bubblewrap);
            command
                .args(["--bind", env::current_dir().unwrap().to_str().unwrap(), "/"])
                .args(["--proc", "/proc"])
                .args(["--ro-bind", "/usr", "/usr"])
                .args(["--ro-bind", "/lib64", "/lib64"])
                .args(["--chdir", "/"])
                .arg("--unshare-pid")
                .arg("--new-session");
            command.args([
                "valgrind",
                "--leak-check=full",
                "--show-leak-kinds=all",
                "--error-exitcode=1",
                "--exit-on-first-error=yes",
            ]);
            command.arg("program");
        }
        (Some(bubblewrap), false) => {
            command = Command::new(bubblewrap);
            command
                .args(["--bind", env::current_dir().unwrap().to_str().unwrap(), "/"])
                .args(["--proc", "/proc"])
                .args(["--ro-bind", "/usr", "/usr"])
                .args(["--ro-bind", "/lib64", "/lib64"])
                .args(["--chdir", "/"])
                .arg("--unshare-pid")
                .arg("--new-session");
            command.arg("program");
        }
        (None, true) => {
            command = Command::new("valgrind");
            command.args([
                "--leak-check=full",
                "--show-leak-kinds=all",
                "--error-exitcode=1",
                "--exit-on-first-error=yes",
            ]);
            command.arg("program");
        }
        (None, false) => {
            command = Command::new(program);
            command
                .env_clear()
                .env("PATH", "/usr/bin")
                .env("TERM", "")
                .env("SHELL", "");
        }
    };
    command.args(options).args(["-c", commands]);
    command.output()
}

type ExecOk = (String, bool);

#[derive(Debug, Error)]
#[error("{0}\n{1}\n######################")]
pub struct ExecError(pub String, pub io::Error);

pub fn exec_test(test: &Test, cli: &Cli, base_path: &Path) -> Result<ExecOk, ExecError> {
    let program_path = base_path.join(&cli.program);
    let bash_path = &cli.bash;

    let mut msg = String::new();
    macro_rules! make_err {
        ($e:expr) => {
            $e.map_err(|err| ExecError(msg.clone(), err))
        };
        ($e:expr, $err_msg:expr) => {
            $e.map_err(|err| ExecError(msg.clone() + $err_msg, err))
        };
    }

    let current_dir = make_err!(env::current_dir())?;
    msg += &format!("\n##### TEST {:>7} #####\n", test.id);
    msg += &format!("{}\n", test.commands);

    make_err!(clear_dir(&current_dir))?;
    let mut bash_options = Vec::new();
    if cli.bash_posix {
        bash_options.push("--posix");
    }
    let bash = make_err!(
        exec(
            bash_path,
            &test.commands,
            &bash_options,
            cli.leak_check,
            cli.bubblewrap.as_deref()
        ),
        "# BASH FAILED TO RUN! ##"
    )?;

    make_err!(clear_dir(&current_dir))?;
    if cli.bubblewrap.is_some() {
        fs::copy(&program_path, current_dir.join("minishell")).unwrap();
    }
    let minishell = make_err!(
        exec(
            if cli.bubblewrap.is_some() {
                OsStr::new("/minishell")
            } else {
                OsStr::new(&program_path)
            },
            &test.commands,
            &[],
            cli.leak_check,
            cli.bubblewrap.as_deref()
        ),
        "#### FAILED TO RUN! ####"
    )?;

    if cli.leak_check {
        if !minishell.status.success() {
            msg += "##### MEMORY LEAK  #####\n";
            return Ok((msg, false));
        }
        msg += "#### NO LEAK FOUND #####\n";
        return Ok((msg, true));
    }

    match (bash.status.code(), minishell.status.code()) {
        (Some(bash_code), Some(minishell_code)) => {
            if bash_code != minishell_code {
                msg += "######## FAILED ########\n";
                msg += &format!("Expected status {bash_code}, got {minishell_code}\n");
                if !minishell.stdout.is_empty() {
                    msg += "Output:\n";
                    msg += &String::from_utf8_lossy(&minishell.stdout);
                    if !msg.ends_with('\n') {
                        msg += "\n";
                    }
                }
                if !minishell.stderr.is_empty() {
                    msg += "Error:\n";
                    msg += &String::from_utf8_lossy(&minishell.stderr);
                    if !msg.ends_with('\n') {
                        msg += "\n";
                    }
                }
                msg += "########################";
                return Ok((msg, false));
            }
        }
        (None, _) => {
            msg += "#### BASH CRASHED! #####\n";
            return Ok((msg, false));
        }
        (_, None) => {
            msg += "### PROGRAM CRASHED! ###\n";
            return Ok((msg, false));
        }
    }

    fn sort_env(str: &str) -> String {
        let mut list = vec![];
        let mut output = vec![];
        let regex_env = Regex::new(r"^\S+=\S*$").unwrap();
        let regex_export = Regex::new(r"^declare -x \S+$").unwrap();
        for line in str.lines() {
            if regex_env.is_match(line) || regex_export.is_match(line) {
                list.push(line);
            } else {
                if !list.is_empty() {
                    list.sort();
                    for line in list {
                        output.push(line);
                    }
                    list = vec![];
                }
                output.push(line);
            }
        }
        list.sort();
        for line in list {
            output.push(line);
        }
        output.join("\n")
    }
    let bash_stdout = String::from_utf8_lossy(&bash.stdout);
    let bash_stdout = bash_stdout.replace("/usr/bin/env", "env");
    let bash_stdout = sort_env(&bash_stdout);
    let minishell_stdout = String::from_utf8_lossy(&minishell.stdout);
    let minishell_stdout = sort_env(&minishell_stdout);
    if bash_stdout != minishell_stdout {
        msg += "######## FAILED ########\n";
        msg += "Expected output:\n";
        msg += &bash_stdout;
        if !msg.ends_with('\n') {
            msg += "\n";
        }
        msg += "Tested output:\n";
        msg += &minishell_stdout;
        if !msg.ends_with('\n') {
            msg += "\n";
        }
        if !minishell.stderr.is_empty() {
            msg += "Error:\n";
            msg += &String::from_utf8_lossy(&minishell.stderr);
            if !msg.ends_with('\n') {
                msg += "\n";
            }
        }
        msg += "########################\n";
        return Ok((msg, false));
    }

    msg += "####### SUCCESS! #######\n";
    if let Some(minishell_code) = minishell.status.code() {
        msg += &format!("Status: {minishell_code}\n")
    }
    if !minishell_stdout.is_empty() {
        msg += "Output:\n";
        msg += &minishell_stdout;
        if !msg.ends_with('\n') {
            msg += "\n";
        }
    }
    if !minishell.stderr.is_empty() {
        msg += "Error:\n";
        msg += &String::from_utf8_lossy(&minishell.stderr);
        if !msg.ends_with('\n') {
            msg += "\n";
        }
    }
    msg += "########################\n";
    Ok((msg, true))
}
