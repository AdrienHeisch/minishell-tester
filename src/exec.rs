use thiserror::Error;

use crate::Test;
use std::{
    env,
    ffi::OsStr,
    fs, io,
    path::Path,
    process::{Command, Output},
};

fn clear_dir(dir: &Path) -> io::Result<()> {
    fs::remove_dir_all(dir)?;
    fs::create_dir(dir)?;
    env::set_current_dir(dir)?;
    Ok(())
}

fn exec(program: impl AsRef<OsStr>, commands: &str) -> io::Result<Output> {
    Command::new(program).args(["-c", commands]).output()
}

pub struct ExecOk(pub String, pub bool);

#[derive(Debug, Error)]
#[error("{0}\n{1}\n######################")]
pub struct ExecError(pub String, pub io::Error);

pub fn exec_test(test: &Test, program_path: &Path) -> Result<ExecOk, ExecError> {
    let mut msg = String::new();
    let current_dir = env::current_dir().map_err(|err| ExecError(msg.clone(), err))?;
    msg += &format!("\n##### TEST {:>7} #####\n", test.id);
    msg += &format!("{}\n", test.commands);
    clear_dir(&current_dir).map_err(|err| ExecError(msg.clone(), err))?;
    let bash = exec("bash", &test.commands)
        .map_err(|err| ExecError(msg.clone() + "##### BASH FAILED! #####", err))?;
    clear_dir(&current_dir).map_err(|err| ExecError(msg.clone(), err))?;
    let minishell = exec(program_path, &test.commands)
        .map_err(|err| ExecError(msg.clone() + "#### FAILED TO RUN! ####", err))?;
    match (bash.status.code(), minishell.status.code()) {
        (Some(bash_code), Some(minishell_code)) => {
            if bash_code != minishell_code {
                msg += "######## FAILED ########\n";
                msg += &format!("Expected status {bash_code}, got {minishell_code}\n");
                msg += &String::from_utf8_lossy(&minishell.stderr);
                msg += "########################";
                return Ok(ExecOk(msg, false));
            }
        }
        (None, _) => {
            msg += "#### BASH  CRASHED! ####\n";
            return Ok(ExecOk(msg, false));
        }
        (_, None) => {
            msg += "### PROGRAM CRASHED! ###\n";
            return Ok(ExecOk(msg, false));
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
        return Ok(ExecOk(msg, false));
    }
    msg += "####### SUCCESS! #######\n";
    Ok(ExecOk(msg, true))
}
