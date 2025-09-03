use crate::{test::Test, Cli};
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
) -> io::Result<Output> {
    let mut args = options.to_vec();
    args.extend_from_slice(&["-c", commands]);
    if leak_check {
        let mut command = Command::new("valgrind");
        command
            .arg("--leak-check=full")
            .arg("--show-leak-kinds=all")
            .arg("--error-exitcode=1")
            .arg("--exit-on-first-error=yes")
            .arg(program)
            .args(args);
        command.output()
    } else {
        let mut command = Command::new(program);
        command.args(args);
        command.env_clear();
        command.env("PATH", "");
        command.env("TERM", "");
        command.env("SHELL", "");
        command.output()
    }
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
        exec(bash_path, &test.commands, &bash_options, cli.leak_check),
        "# BASH FAILED TO RUN! ##"
    )?;

    make_err!(clear_dir(&current_dir))?;
    let minishell = make_err!(
        exec(program_path, &test.commands, &[], cli.leak_check),
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
                msg += &String::from_utf8_lossy(&minishell.stderr);
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

    let bash_stdout = String::from_utf8_lossy(&bash.stdout);
    let minishell_stdout = String::from_utf8_lossy(&minishell.stdout);
    if bash_stdout != minishell_stdout {
        msg += "######## FAILED ########\n";
        msg += "Expected output:\n";
        msg += &bash_stdout;
        msg += "Tested output:\n";
        msg += &minishell_stdout;
        if !minishell.stderr.is_empty() {
            msg += "Error:\n";
            msg += &String::from_utf8_lossy(&minishell.stderr);
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
    }
    msg += "########################\n";
    Ok((msg, true))
}
