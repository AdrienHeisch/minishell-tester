mod popup;

use crate::{
    recap,
    run::{parse_tests, run_tests, RunError, TestResult},
    show,
    watch::{self, WatchRunResult, WatchThreadResult},
    ExecPaths, Run,
};
use popup::Popup;
use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    prelude::*,
    DefaultTerminal,
};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    sync::{
        mpsc::{Receiver, TryRecvError},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

type FullRunResults = Vec<(PathBuf, usize, Vec<TestResult>)>;

fn find_test_files(path: &Path) -> io::Result<Vec<PathBuf>> {
    Ok(fs::read_dir(path)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "csv"))
        .collect())
}

pub fn run(exec_paths: ExecPaths) -> io::Result<()> {
    let run_options = Run {
        exec_paths,
        bwrap: true,
        parallel: true,
        keep_going: true,
        ..Default::default()
    };
    let test_files = Arc::new(Mutex::new(find_test_files(&env::current_dir()?)?));
    let (_watch_handle, watch_rx) = watch_thread(test_files.clone(), &run_options).unwrap();
    let state = State {
        test_files,
        run_options,
        watch_results: vec![],
    };
    let mut terminal = ratatui::init();
    terminal.clear()?;
    let res = run_loop(terminal, state, watch_rx);
    ratatui::restore();
    // TODO stop watch thread ?
    res
}

fn do_run_tests(tests: &[PathBuf], run_options: &Run) -> Result<FullRunResults, RunError> {
    let mut out = vec![];
    for file in tests.iter() {
        let (tests, ignored) = parse_tests(file, run_options)?;
        let results = run_tests(&tests, run_options, false)?;
        out.push((file.clone(), ignored, results));
    }
    Ok(out)
}

fn watch_thread(
    tests: Arc<Mutex<Vec<PathBuf>>>,
    run_options: &Run,
) -> WatchThreadResult<FullRunResults> {
    let run_test_files = {
        let run_options = run_options.clone();
        move || -> Result<_, RunError> {
            let tests = tests.lock().unwrap();
            do_run_tests(&tests, &run_options)
        }
    };
    watch::thread(run_options.clone(), run_test_files)
}

struct State {
    test_files: Arc<Mutex<Vec<PathBuf>>>,
    run_options: Run,
    watch_results: Vec<(Result<FullRunResults, String>, Duration)>,
}

fn run_loop(
    mut terminal: DefaultTerminal,
    mut state: State,
    watch_rx: Receiver<WatchRunResult<FullRunResults>>,
) -> io::Result<()> {
    let mut then = Instant::now();
    let mut running_rx: Option<Receiver<Result<FullRunResults, String>>> = None;
    loop {
        let now = Instant::now();
        let elapsed = now - then;
        if let Some(ref rx) = running_rx {
            loop {
                match rx.try_recv() {
                    Ok(res) => {
                        state.watch_results.push((res, Duration::from_secs(3)));
                        continue;
                    }
                    Err(TryRecvError::Disconnected) => running_rx = None,
                    _ => (),
                }
                break;
            }
        }
        while let Ok(res) = watch_rx.try_recv() {
            let res = res.map_err(|err| format!("{err}"));
            state.watch_results.push((res, Duration::from_secs(3)));
        }
        for (_, duration) in &mut state.watch_results {
            *duration = duration.saturating_sub(elapsed);
        }
        state
            .watch_results
            .retain(|(_, duration)| !duration.is_zero());
        if event::poll(Duration::from_millis(33))? {
            while let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Enter => {
                            let (tx, rx) = std::sync::mpsc::channel();
                            let test_files = state.test_files.lock().unwrap().clone();
                            let run_options = state.run_options.clone();
                            thread::spawn(move || {
                                let res = do_run_tests(&test_files, &run_options)
                                    .map_err(|err| format!("{err}"));
                                tx.send(res).ok();
                            });
                            running_rx = Some(rx);
                        }
                        _ => continue,
                    }
                }
                break;
            }
        }
        terminal.draw(|frame| draw(frame, &state))?;
        then = now;
    }
}

fn draw(frame: &mut Frame, state: &State) {
    let area = frame.area();
    let popup_area = Rect {
        x: area.width / 4,
        y: area.height / 3,
        width: area.width / 2,
        height: area.height / 3,
    };
    if let Some((res, _)) = state.watch_results.first() {
        let (content, color) = match res {
            Ok(results) => (
                {
                    let mut content = String::new();
                    for (file, ignored, results) in results {
                        content += &format!("Running tests from {file:?}");
                        for result in results.iter() {
                            show(&state.run_options, result, |str| content += str);
                        }
                        content += &recap(results.len(), *ignored, results);
                    }
                    content
                },
                Style::new().green(),
            ),
            Err(err) => (
                format!("Error while running tests: {err}"),
                Style::new().red(),
            ),
        };
        let popup = Popup::default()
            .content(content)
            .style(color)
            .title_style(Style::new().white().bold())
            .border_style(Style::new().red());
        frame.render_widget(popup, popup_area);
    }
}
