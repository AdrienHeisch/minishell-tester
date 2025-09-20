use crate::{test::Test, Run};
use regex::Regex;
use std::{
    ffi::OsStr,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub enum ExecError {
    Setup(#[from] SetupError),
    #[error("Error during test execution: {0}")]
    Io(#[from] io::Error),
    #[error("Error during test subcommand execution: {0}")]
    Command(io::Error),
    #[error("Error from bwrap, probably missing executable")]
    Bwrap,
}

#[derive(Debug, Error)]
#[error("Error during setup: {0}")]
pub enum SetupError {
    Io(#[from] io::Error),
}

fn join_path_if_relative(base: &Path, path: &Path) -> PathBuf {
    match path.is_absolute() {
        true => path.to_owned(),
        false => base.join(path),
    }
}

fn setup_test(exec_path: &Path, is_bwrap: bool) -> Result<(), SetupError> {
    let mut exec_path = exec_path.to_owned();
    fs::remove_dir_all(&exec_path)?;
    fs::create_dir(&exec_path)?;
    if is_bwrap {
        fs::create_dir(exec_path.join(".bin"))?;
        exec_path = exec_path.join("home/maxitester");
        fs::create_dir_all(&exec_path)?;
    }
    fs::File::create_new(exec_path.join("a"))?.write_all(b"file a\n")?;
    fs::File::create_new(exec_path.join("b"))?.write_all(b"file b\n")?;
    fs::File::create_new(exec_path.join("c"))?.write_all(b"file c\n")?;
    Ok(())
}

fn sort_env(bytes: &mut Vec<u8>) {
    let str = String::from_utf8_lossy(bytes);
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
    *bytes = output.join("\n").as_bytes().to_vec();
}

fn ensure_newline(bytes: &mut Vec<u8>) {
    match bytes.last_mut() {
        Some(b'\n') | None => (),
        _ => bytes.push(b'\n'),
    }
}

fn exec(
    program: impl AsRef<OsStr>,
    test: &str,
    options: &[&str],
    valgrind: bool,
    funcheck: bool,
    bwrap: Option<&Path>,
    exec_path: &Path,
) -> Result<Output, ExecError> {
    let mut command = if let Some(bwrap) = bwrap {
        let mut command = Command::new(bwrap);
        command
            .args(["--bind", ".", "/"])
            .args(["--dev", "/dev"])
            .args(["--ro-bind", "/usr", "/usr"])
            .args(["--ro-bind", "/bin", "/bin"])
            .args(["--ro-bind", "/lib", "/lib"])
            .args(["--ro-bind", "/lib64", "/lib64"])
            .args(["--tmpfs", "/tmp"])
            .args(["--chdir", "/home/maxitester"])
            .arg("--unshare-all")
            .arg("--die-with-parent")
            .arg("--new-session");
        if valgrind {
            command.args(["--proc", "/proc"]);
        }
        command
    } else if valgrind {
        Command::new("valgrind")
    } else if funcheck {
        Command::new("funcheck")
    } else {
        Command::new(&program)
    };
    if valgrind {
        if bwrap.is_some() {
            command.arg("valgrind");
        }
        command.args([
            "--leak-check=full",
            "--show-leak-kinds=all",
            "--track-origins=yes",
            "--track-fds=yes",
            "--errors-for-leak-kinds=all",
            "--error-exitcode=3", // TODO move 3 to const
            "--suppressions=../../valgrind-suppressions",
        ]);
    }
    if funcheck && bwrap.is_some() {
        command.arg("funcheck");
    }
    if valgrind || funcheck || bwrap.is_some() {
        command.arg(&program);
    }
    command.args(options);
    command.current_dir(exec_path);
    command
        .env_clear()
        .env("PATH", "/usr/bin")
        .env("USER", "maxitester")
        .env("HOME", "/home/maxitester")
        .env("SHELL", "/usr/bin/someshell")
        .env("TERM", "xterm-256color")
        .env("UID", "1000")
        .env("SHLVL", "");
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(ExecError::Command)?;
    let mut stdin = child.stdin.take().unwrap();
    for line in test.lines() {
        stdin.write_all(line.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
    }
    drop(stdin);
    let mut output = child.wait_with_output()?;
    sort_env(&mut output.stdout);
    sort_env(&mut output.stderr);
    ensure_newline(&mut output.stdout);
    ensure_newline(&mut output.stderr);
    Ok(output)
}

fn exec_minishell(
    test: &Test,
    cli: &Run,
    base_path: &Path,
    exec_path: &Path,
) -> Result<Output, ExecError> {
    let program_path = join_path_if_relative(base_path, &cli.exec_paths.minishell);

    setup_test(exec_path, cli.bwrap)?;
    if cli.bwrap {
        fs::copy(&program_path, exec_path.join(".bin/minishell")).unwrap();
    }
    let output = exec(
        if cli.bwrap {
            OsStr::new("/.bin/minishell")
        } else {
            OsStr::new(&program_path)
        },
        &test.commands,
        &[],
        cli.valgrind,
        cli.funcheck,
        cli.bwrap.then_some(&join_path_if_relative(
            base_path,
            &cli.exec_paths.bwrap_path,
        )),
        exec_path,
    );
    if cli.bwrap {
        fs::remove_file(exec_path.join(".bin/minishell")).unwrap();
    }
    output
}

fn adjust_bash_output(bytes: &mut Vec<u8>, bash_path: &Path) {
    let str = String::from_utf8_lossy(bytes)
        .replace("/usr/bin/env", "env")
        .replace(bash_path.to_str().unwrap_or("bash"), "minishell");
    *bytes = str.as_bytes().to_vec();
}

fn exec_bash(
    test: &Test,
    cli: &Run,
    base_path: &Path,
    exec_path: &Path,
) -> Result<Output, ExecError> {
    let bash_path = join_path_if_relative(base_path, &cli.exec_paths.bash);

    setup_test(exec_path, cli.bwrap)?;
    let mut bash_options = Vec::new();
    if cli.bash_posix {
        bash_options.push("--posix");
    }
    let mut output = exec(
        &bash_path,
        &test.commands,
        &bash_options,
        cli.valgrind,
        cli.funcheck,
        cli.bwrap.then_some(&join_path_if_relative(
            base_path,
            &cli.exec_paths.bwrap_path,
        )),
        exec_path,
    )?;
    adjust_bash_output(&mut output.stdout, &bash_path);
    adjust_bash_output(&mut output.stderr, &bash_path);
    Ok(output)
}

pub fn exec_test(
    test: &Test,
    cli: &Run,
    base_path: &Path,
    exec_path: &Path,
    output: &mut impl io::Write,
) -> Result<bool, ExecError> {
    writeln!(output)?;
    writeln!(output, "##### TEST {:>7} #####", test.id)?;
    writeln!(output, "{}", test.commands)?;

    let bash = exec_bash(test, cli, base_path, exec_path)
        .inspect_err(|_| drop(writeln!(output, "# BASH FAILED TO RUN! ##")))?;

    if cli.bwrap
        && !bash.status.success()
        && String::from_utf8_lossy(&bash.stderr).contains("bwrap: execvp")
    {
        writeln!(output, "## BASH FAILED TO RUN! ##")?;
        output.write_all(&bash.stderr)?;
        return Err(ExecError::Bwrap);
    }

    let minishell = exec_minishell(test, cli, base_path, exec_path)
        .inspect_err(|_| drop(writeln!(output, "#### FAILED TO RUN! ####")))?;

    if cli.bwrap
        && !minishell.status.success()
        && String::from_utf8_lossy(&minishell.stderr).contains("bwrap: execvp")
    {
        writeln!(output, "#### FAILED TO RUN! ####")?;
        output.write_all(&minishell.stderr)?;
        return Err(ExecError::Bwrap);
    }

    if cli.valgrind {
        match minishell.status.code() {
            Some(3) => {
                writeln!(output, "#### VALGRIND ERROR ####")?;
                if !minishell.stdout.is_empty() {
                    writeln!(output, "Output:")?;
                    output.write_all(&minishell.stdout)?;
                }
                if !minishell.stderr.is_empty() {
                    writeln!(output, "Error:")?;
                    output.write_all(&minishell.stderr)?;
                }
                writeln!(output, "########################")?;
                return Ok(false);
            }
            Some(_) => {
                writeln!(output, "####### SUCCESS! #######")?;
                return Ok(true); // DESIGN compare with bash instead of success ?
            }
            _ => (),
        }
    }

    if cli.funcheck {
        match minishell.status.code() {
            Some(0) => {
                writeln!(output, "####### SUCCESS! #######")?;
                return Ok(true); // DESIGN compare with bash instead of success ?
            }
            Some(_) => {
                writeln!(output, "#### FUNCHECK ERROR ####")?;
                if !minishell.stdout.is_empty() {
                    writeln!(output, "Output:")?;
                    output.write_all(&minishell.stdout)?;
                }
                if !minishell.stderr.is_empty() {
                    writeln!(output, "Error:")?;
                    output.write_all(&minishell.stderr)?;
                }
                writeln!(output, "########################")?;
                return Ok(false);
            }
            _ => (),
        }
    }

    match (bash.status.code(), minishell.status.code()) {
        (Some(bash_code), Some(minishell_code)) => {
            if bash_code != minishell_code {
                writeln!(output, "######## FAILED ########")?;
                writeln!(output, "Expected status {bash_code}, got {minishell_code}")?;
                if !minishell.stdout.is_empty() {
                    writeln!(output, "Output:")?;
                    output.write_all(&minishell.stdout)?;
                }
                if !minishell.stderr.is_empty() {
                    writeln!(output, "Error:")?;
                    output.write_all(&minishell.stderr)?;
                }
                writeln!(output, "########################")?;
                return Ok(false);
            }
        }
        (None, _) => {
            writeln!(output, "#### BASH CRASHED! #####")?;
            return Ok(false);
        }
        (_, None) => {
            writeln!(output, "### PROGRAM CRASHED! ###")?;
            return Ok(false);
        }
    }

    if bash.stdout != minishell.stdout {
        writeln!(output, "######## FAILED ########")?;
        writeln!(output, "Expected output:")?;
        output.write_all(&bash.stdout)?;
        writeln!(output, "Tested output:")?;
        output.write_all(&minishell.stdout)?;
        if !minishell.stderr.is_empty() {
            writeln!(output, "Error:")?;
            output.write_all(&minishell.stderr)?;
        }
        writeln!(output, "########################")?;
        return Ok(false);
    }

    if cli.error_check && bash.stderr != minishell.stderr {
        writeln!(output, "######## FAILED ########")?;
        if !minishell.stdout.is_empty() {
            writeln!(output, "Output:")?;
            output.write_all(&minishell.stdout)?;
        }
        writeln!(output, "Expected error:")?;
        output.write_all(&bash.stderr)?;
        writeln!(output, "Tested error:")?;
        output.write_all(&minishell.stderr)?;
        writeln!(output, "########################")?;
        return Ok(false);
    }

    writeln!(output, "####### SUCCESS! #######")?;
    if let Some(minishell_code) = minishell.status.code() {
        writeln!(output, "Status: {minishell_code}")?;
    }
    if !minishell.stdout.is_empty() {
        writeln!(output, "Output:")?;
        output.write_all(&minishell.stdout)?;
    }
    if !minishell.stderr.is_empty() {
        writeln!(output, "Error:")?;
        output.write_all(&minishell.stderr)?;
    }
    writeln!(output, "########################")?;
    Ok(true)
}
