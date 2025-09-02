use crate::Test;
use std::{
    io::{self},
    path::Path,
    process::Command,
};

fn clear_dir(dir: &Path) -> io::Result<()> {
    if !dir.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "path is not a directory",
        ));
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && !path.symlink_metadata()?.file_type().is_symlink() {
            std::fs::remove_dir_all(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}

pub fn exec_test(test: &Test, program_path: &Path) -> io::Result<bool> {
    let current_dir = std::env::current_dir()?;
    println!();
    println!("##### TEST {:>7} #####", test.id);
    println!("{}", test.commands);
    clear_dir(&current_dir)?;
    let bash = match Command::new("bash").args(["-c", &test.commands]).output() {
        Ok(minishell) => minishell,
        Err(_) => {
            println!("##### INVALID TEST #####");
            return Ok(false);
        }
    };
    clear_dir(&current_dir)?;
    let minishell = match Command::new(program_path)
        .args(["-c", &test.commands])
        .output()
    {
        Ok(minishell) => minishell,
        Err(_) => {
            println!("#### FAILED TO RUN! ####");
            return Ok(false);
        }
    };
    match (bash.status.code(), minishell.status.code()) {
        (Some(bash_code), Some(minishell_code)) => {
            if bash_code != minishell_code {
                println!("######## FAILED ########");
                println!("Expected status {bash_code}, got {minishell_code}");
                println!("{}", String::from_utf8_lossy(&minishell.stderr));
                println!("########################");
                return Ok(false);
            }
        }
        (None, _) => {
            println!("#### FAILED TO RUN! ####");
            return Ok(false);
        }
        (_, None) => {
            println!("### PROGRAM CRASHED! ###");
            return Ok(false);
        }
    }
    let bash_stdout = String::from_utf8_lossy(&bash.stdout);
    let minishell_stdout = String::from_utf8_lossy(&minishell.stdout);
    if bash_stdout != minishell_stdout {
        println!("######## FAILED ########");
        println!("Expected output:");
        println!("{bash_stdout}");
        println!("Tested output:");
        println!("{minishell_stdout}");
        if !minishell.stderr.is_empty() {
            println!("Error:");
            println!("{}", String::from_utf8_lossy(&minishell.stderr));
        }
        println!("########################");
        return Ok(false);
    }
    println!("####### SUCCESS! #######");
    Ok(true)
}
