use crate::{run::RunError, Run};
use hotwatch::{
    blocking::{Flow, Hotwatch},
    notify::event::{AccessKind, AccessMode},
    Event, EventKind,
};
use std::{
    fmt::Debug,
    fs, io,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
    time::Duration,
};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub enum WatchError {
    #[error("Couldn't find minishell's parent directory")]
    Parent,
    Io(#[from] io::Error),
    Watch(#[from] hotwatch::Error),
}

struct Info {
    watch_path: PathBuf,
    file_path: PathBuf,
    handler: Box<dyn Fn(Event) -> Flow>,
}

pub type WatchRunResult<T> = Result<T, Box<dyn std::error::Error + Send>>;

fn setup<F, T>(
    cli: &Run,
    run_test_files: F,
    on_res: impl 'static + Fn(WatchRunResult<T>),
) -> Result<Info, WatchError>
where
    F: 'static + Fn() -> Result<T, RunError>,
{
    let file_path = std::env::current_dir()?
        .join(&cli.exec_paths.minishell)
        .canonicalize()?;
    let watch_path = file_path.parent().ok_or(WatchError::Parent)?.to_owned();
    let handler = Box::new({
        let file_path = file_path.clone();
        move |event: Event| {
            if !event.paths.iter().any(|path| path == &file_path) {
                return Flow::Continue;
            }
            match event.kind {
                EventKind::Access(AccessKind::Close(AccessMode::Write)) => (),
                _ => {
                    return Flow::Continue;
                }
            }
            match fs::exists(&file_path) {
                Ok(true) => (),
                Ok(false) => return Flow::Continue,
                Err(err) => {
                    on_res(Err(Box::new(err)));
                    return Flow::Exit;
                }
            }
            match run_test_files() {
                Ok(out) => on_res(Ok(out)),
                Err(err) => {
                    on_res(Err(Box::new(err)));
                    return Flow::Exit;
                }
            }
            Flow::Continue
        }
    });
    Ok(Info {
        watch_path,
        file_path,
        handler,
    })
}

pub fn blocking<F, T>(cli: &Run, run_test_files: F) -> Result<(), WatchError>
where
    F: 'static + Fn() -> Result<T, RunError>,
{
    let on_res = |res| {
        if let Err(err) = res {
            eprintln!("{err}");
        }
    };
    let Info {
        watch_path,
        file_path,
        handler,
    } = setup(cli, run_test_files, on_res)?;
    let mut hotwatch = Hotwatch::new_with_custom_delay(Duration::from_millis(100))?;
    hotwatch.watch(watch_path, handler)?;
    println!("Watching file {file_path:?}");
    hotwatch.run();
    Ok(())
}

pub type WatchThreadHandle = JoinHandle<Result<(), WatchError>>;

pub type WatchThreadResult<T> = Result<(WatchThreadHandle, Receiver<WatchRunResult<T>>), WatchError>;

pub fn thread<F, T>(cli: Run, run_test_files: F) -> WatchThreadResult<T>
where
    F: 'static + Fn() -> Result<T, RunError> + Send,
    T: 'static + Send,
{
    let (tx, rx) = mpsc::channel();
    let on_res = move |res| {
        tx.send(res).unwrap();
    };
    let handle = thread::spawn(move || -> Result<(), WatchError> {
        let Info {
            watch_path,
            handler,
            ..
        } = setup(&cli, run_test_files, on_res)?;
        let mut hotwatch = Hotwatch::new_with_custom_delay(Duration::from_millis(100))?;
        hotwatch.watch(watch_path, handler)?;
        hotwatch.run();
        Ok(())
    });
    Ok((handle, rx))
}
