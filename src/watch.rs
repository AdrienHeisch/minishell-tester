use crate::{run::RunError, Run};
use hotwatch::{
    blocking::{Flow, Hotwatch},
    notify::event::AccessKind,
    Event, EventKind,
};
use std::{fmt::Debug, fs, io, time::Duration};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub enum WatchError {
    #[error("Couldn't find minishell's parent directory")]
    Parent,
    Io(#[from] io::Error),
    Watch(#[from] hotwatch::Error),
}

pub fn blocking<F>(cli: &Run, run_test_files: F) -> Result<(), WatchError>
where
    F: 'static + Fn() -> Result<(), RunError>,
{
    let minishell_path = std::env::current_dir()?
        .join(&cli.exec_paths.minishell)
        .canonicalize()?;
    let dir_path = minishell_path.parent().ok_or(WatchError::Parent)?;
    let hotwatch_hanlder = {
        let minishell_path = minishell_path.clone();
        move |event: Event| {
            if !event.paths.iter().any(|path| path == &minishell_path) {
                return Flow::Continue;
            }
            match event.kind {
                EventKind::Access(AccessKind::Close(
                    hotwatch::notify::event::AccessMode::Write,
                )) => (),
                _ => {
                    return Flow::Continue;
                }
            }
            match fs::exists(&minishell_path) {
                Ok(true) => (),
                Ok(false) => return Flow::Continue,
                Err(err) => {
                    eprintln!("{err}");
                    return Flow::Exit;
                }
            }
            if let Err(err) = run_test_files() {
                eprintln!("{err}");
                return Flow::Exit;
            }
            Flow::Continue
        }
    };
    let mut hotwatch = Hotwatch::new_with_custom_delay(Duration::from_millis(100))?;
    hotwatch.watch(dir_path, hotwatch_hanlder)?;
    println!("Watching file {minishell_path:?}");
    hotwatch.run();
    Ok(())
}
