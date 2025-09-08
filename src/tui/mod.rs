mod popup;

use crate::{
    recap,
    run::{parse_tests, run_tests, RunError, TestResult},
    show,
    test::Test,
    watch::{self, WatchRunResult, WatchThreadResult},
    ExecPaths, Run,
};
use popup::Popup;
use ratatui::{
    crossterm::event::{self, KeyCode, KeyEventKind},
    prelude::*,
    widgets::{Block, Borders, List, ListState, Paragraph},
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
        test_file_selected: 0,
        test_selected: 0,
        results: vec![],
        run_options,
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
    test_file_selected: usize,
    test_selected: usize,
    results: Vec<TestResult>,
    run_options: Run,
}

fn process_tests_results(state: &State, res: Result<FullRunResults, String>) -> (String, bool) {
    match res {
        Ok(results) => (
            {
                let mut content = String::new();
                for (file, ignored, results) in results {
                    content += &format!("Running tests from {file:?}");
                    for result in results.iter() {
                        show(&state.run_options, result, |str| content += str);
                    }
                    content += &recap(results.len(), ignored, &results);
                }
                content
            },
            true,
        ),
        Err(err) => (format!("Error while running tests: {err}"), false),
    }
}

fn run_loop(
    mut terminal: DefaultTerminal,
    mut state: State,
    watch_rx: Receiver<WatchRunResult<FullRunResults>>,
) -> io::Result<()> {
    let mut then = Instant::now();
    let mut running_rx: Option<Receiver<Result<FullRunResults, String>>> = None;
    let mut ui = UI::new(
        state
            .test_files
            .lock()
            .unwrap()
            .iter()
            .map(|p| p.file_name().unwrap_or_default().to_string_lossy().into()),
    );
    ui.test_list.set_filename(
        state
            .test_files
            .lock()
            .unwrap()
            .get(state.test_file_selected)
            .map(|p| p.file_name().unwrap_or_default().to_string_lossy().into()),
        &state.run_options,
    );
    ui.test_list.state.select_first();
    run_test_thread(&state, &mut running_rx);

    loop {
        let now = Instant::now();
        let elapsed = now - then;
        if let Some(ref rx) = running_rx {
            loop {
                match rx.try_recv() {
                    Ok(res) => {
                        if let Ok(res) = &res {
                            let state: &mut State = &mut state;
                            let ui: &mut UI = &mut ui;
                            if let Some((_, _, results)) = res.iter().find(|(path, _, _)| {
                                Some(path)
                                    == state
                                        .test_files
                                        .lock()
                                        .unwrap()
                                        .get(state.test_file_selected)
                            }) {
                                state.results = results.clone();
                                if let Some(selected) = state.results.get(state.test_selected) {
                                    ui.update_test_display(selected)
                                }
                            }
                        }
                        let (content, color) = process_tests_results(&state, res);
                        ui.add_popup(content, color);
                        continue;
                    }
                    Err(TryRecvError::Disconnected) => running_rx = None,
                    _ => (),
                }
                break;
            }
        }
        while let Ok(res) = watch_rx.try_recv() {
            let (content, color) =
                process_tests_results(&state, res.map_err(|err| format!("{err}")));
            ui.add_popup(content, color);
        }
        ui.update(elapsed);
        if event::poll(Duration::from_millis(33))? {
            while let event::Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Enter => {
                            run_test_thread(&state, &mut running_rx);
                        }
                        KeyCode::Char('j') => {
                            ui.test_list.state.scroll_down_by(1);
                            state.test_selected = state.test_selected.saturating_add(1);
                            if let Some(selected) = state.results.get(state.test_selected) {
                                ui.update_test_display(selected)
                            }
                        }
                        KeyCode::Char('k') => {
                            ui.test_list.state.scroll_up_by(1);
                            state.test_selected = state.test_selected.saturating_sub(1);
                            if let Some(selected) = state.results.get(state.test_selected) {
                                ui.update_test_display(selected)
                            }
                        }
                        KeyCode::Char('h') => {
                            state.test_file_selected = state.test_file_selected.saturating_sub(1);
                            ui.test_list.set_filename(
                                state
                                    .test_files
                                    .lock()
                                    .unwrap()
                                    .get(state.test_file_selected)
                                    .map(|p| {
                                        p.file_name().unwrap_or_default().to_string_lossy().into()
                                    }),
                                &state.run_options,
                            );
                        }

                        _ => continue,
                    }
                }
                break;
            }
        }
        terminal.draw(|frame| frame.render_widget(&mut ui, frame.area()))?;
        then = now;
    }
}

fn run_test_thread(
    state: &State,
    running_rx: &mut Option<Receiver<Result<Vec<(PathBuf, usize, Vec<TestResult>)>, String>>>,
) {
    let (tx, rx) = std::sync::mpsc::channel();
    let test_files = state.test_files.lock().unwrap().clone();
    let run_options = state.run_options.clone();
    thread::spawn(move || {
        let res = do_run_tests(&test_files, &run_options).map_err(|err| format!("{err}"));
        tx.send(res).ok();
    });
    *running_rx = Some(rx);
}

#[derive(Default)]
struct UI {
    current_file: Option<String>,
    test_list: TestList,
    test_result: String,
    popups: Vec<(String, Color, Duration)>,
}

impl UI {
    const POPUP_DURATION: Duration = Duration::from_secs(0);

    pub fn new(test_files: impl Iterator<Item = String>) -> Self {
        let test_files = test_files.collect::<Vec<_>>();
        Self {
            current_file: test_files.first().cloned(),
            ..Default::default()
        }
    }

    pub fn update(&mut self, elapsed: Duration) {
        for (_, _, duration) in &mut self.popups {
            *duration = duration.saturating_sub(elapsed);
        }
        self.popups.retain(|(_, _, duration)| !duration.is_zero());
    }

    pub fn add_popup(&mut self, content: String, is_success: bool) {
        let color = match is_success {
            true => Color::Green,
            false => Color::Red,
        };
        self.popups.push((content, color, Self::POPUP_DURATION));
    }

    pub fn update_test_display(&mut self, result: &TestResult) {
        self.test_result = match result {
            TestResult::None => "Test not run".to_string(),
            TestResult::Error(err) => err.clone(),
            TestResult::Failed(str) => str.clone(),
            TestResult::Passed(str) => str.clone(),
        }
        .trim()
        .to_string();
    }
}

impl Widget for &mut UI {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let vlayout =
            Layout::vertical(vec![Constraint::Length(1), Constraint::Fill(1)]).split(area);

        let hlayout =
            Layout::horizontal(vec![Constraint::Fill(1), Constraint::Fill(1)]).split(vlayout[1]);

        self.test_list.render(hlayout[0], buf);

        let test =
            Paragraph::new(self.test_result.as_str()).block(Block::new().borders(Borders::ALL));
        test.render(hlayout[1], buf);

        if let Some((content, color, _)) = self.popups.first() {
            let popup_area = Rect {
                x: area.width / 4,
                y: area.height / 3,
                width: area.width / 2,
                height: area.height / 3,
            };
            let popup = Popup::default()
                .content(content.as_str())
                .style(Style::new().fg(*color))
                .title_style(Style::new().white().bold())
                .border_style(Style::new().red());
            popup.render(popup_area, buf);
        }
    }
}

#[derive(Default)]
struct TestList {
    data: Option<(String, Vec<Test>)>,
    state: ListState,
}

impl TestList {
    pub fn set_filename(&mut self, filename: Option<String>, run_options: &Run) {
        self.data = filename.map(|filename| {
            (
                filename.clone(),
                parse_tests(Path::new(&filename), run_options).unwrap().0,
            )
        });
    }
}

impl Widget for &mut TestList {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let list = List::new(
            self.data
                .as_ref()
                .map(|data| {
                    data.1
                        .iter()
                        .map(|t| format!("{:>4} {}", t.id, t.commands.replace("\n", " \\ ")))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        )
        .block(
            Block::bordered().title(
                self.data
                    .as_ref()
                    .map(|data| data.0.as_str())
                    .unwrap_or_default(),
            ),
        )
        .highlight_style(Style::new().reversed());
        StatefulWidget::render(list, area, buf, &mut self.state);
    }
}
