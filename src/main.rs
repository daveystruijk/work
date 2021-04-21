use serde_json;
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Duration, Local};


use std::{
    io::{self, Write},
    time::{self},
    cmp::{min, max},
    fs::{File},
    path::{Path},
};

use crossterm::{
    cursor::{self},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    style::{self, SetForegroundColor, Color},
    terminal::{self, ClearType},
    execute,
    queue,
    Result,
};

// As a user:
// - start the day, add a couple tasks
// - arrange day

// Keys:
// [x] j/k: navigate
// [x] n: new task
// [ ] enter: start working
// [ ] x: delete task
// [ ] backspace: ignore task
// [ ] u: undo
// [x] +/-: increase/decrease duration by 15m
// [x] e: edit

// Features:
// [ ] 'scrollback' to completed tasks
// [x] Timer
// [ ] lunch e.d.
// [x] persistent db (serialization of tasks/events)
// [ ] Monthly log (jump to day)
// [ ] Nice (random) animation on task completion
// [ ] Flicker on task selection / work start
// [ ] Auto git (branch creation) and merge request + ticket integration

// DATA STRUCTURES

// list
//
// event: fixed from/to
// task: amount of time planned
//
// • task:
//   amount of time
//   from
//   to
//   completed yes/no
//   subtasks
//   blockers
//   
// ◷ event:
//   amount of time
//   from
//   to
//   preparation time
//

// TODO:
// - Serialize to something more human-editable when format is more stable

const APP_DATA_FILENAME: &str = "/Users/daveystruijk/work.json";

const WORKING_ANIMATION_FRAMES: [char; 4] = ['▖', '▘', '▝', '▗'];


enum Mode {
    Normal,
    Working,
}

struct AppState {
    mode: Mode,
    start_working_time: DateTime<Local>,
    selected_task_index: usize,
    animation_counter: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct AppData {
    tasks: Vec<Task>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Task {
    text: String,
    seconds_estimated: i64,
    seconds_spent: i64,
}

fn default_app_data() -> AppData {
    let tasks: Vec<Task> = vec![
        Task {
            text: String::from("Task 1"),
            seconds_estimated: 30 * 60,
            seconds_spent: 0,
        },
        Task {
            text: String::from("Task 2"),
            seconds_estimated: 30 * 60,
            seconds_spent: 0,
        }
    ];
    let app_data: AppData = AppData { tasks };
    app_data
}

fn load_app_data() -> AppData {
	let file = File::open(APP_DATA_FILENAME).unwrap();
    let deserialized: AppData = serde_json::from_reader(&file).unwrap();
    deserialized
}

fn save_app_data(app_data: AppData) {
	let file = File::create(APP_DATA_FILENAME).unwrap();
    serde_json::to_writer(&file, &app_data).unwrap();
}

pub fn read_line() -> Result<String> {
    let mut line = String::new();
    while let Event::Key(KeyEvent { code, .. }) = event::read()? {
        match code {
            KeyCode::Enter => {
                break;
            }
            KeyCode::Char(c) => {
                line.push(c);
                execute!(
                    io::stdout(),
                    style::Print(c),
                )?;
            }

            KeyCode::Backspace => {
                line.pop();
                execute!(
                    io::stdout(),
                    style::Print("\x08 \x08"),  // Print backspace
                )?;
            }
            _ => {}
        }
    }

    Ok(line)
}

fn render_tasks(data: &AppData, state: &AppState) -> Result<()> {
    let (columns, _) = terminal::size()?;
    let start_of_day = Local::today().and_hms(0, 0, 0);

    let mut eta = Local::now();
    for (i, task) in data.tasks.iter().enumerate() {

        queue!(
            io::stdout(),
            style::ResetColor,
            cursor::MoveToNextLine(2)
        )?;

        let duration = Duration::seconds(task.seconds_estimated);
        let spent_in_current_session = Local::now() - state.start_working_time.clone();
        let spent = match state.mode {
            Mode::Working => {
                if i == 0 {
                    Duration::seconds(task.seconds_spent) + spent_in_current_session
                } else {
                    Duration::seconds(task.seconds_spent)
                }
            }
            _ => { Duration::seconds(task.seconds_spent) }
        };
        eta = eta + duration - spent;

        let text = format!("{}", &task.text);
        let duration_str = format!("{}", (start_of_day + duration).format("%-H:%M"));
        let target_str = format!("{}", eta.format("%H:%M"));

        // Draw dot + task name
        match state.mode {
            Mode::Normal => {
                if i == state.selected_task_index {
                    queue!(
                        io::stdout(),
                        SetForegroundColor(Color::Yellow),
                        style::Print("• "),
                        style::Print(&text),
                    )?;
                } else {
                    queue!(
                        io::stdout(),
                        style::Print("· "),
                        style::Print(&text),
                    )?;
                }
            }
            Mode::Working => {
                if i == 0 {
                    queue!(
                        io::stdout(),
                        SetForegroundColor(Color::Green),
                        style::Print(WORKING_ANIMATION_FRAMES[state.animation_counter]),
                        style::Print(" "),
                        style::Print(&text),
                    )?;
                } else {
                    let hidden_text: String = text.chars().map(|c| match c { 
                        ' ' => ' ', 
                        _ => '-',
                    }).collect();
                    queue!(
                        io::stdout(),
                        SetForegroundColor(Color::DarkGrey),
                        style::Print("· "),
                        style::Print(&hidden_text),
                    )?;
                }
            }
        }

        // Draw seconds_estimated + eta
        queue!(
            io::stdout(),
            SetForegroundColor(Color::DarkGrey),
            cursor::MoveToColumn(columns - (target_str.len() - 1) as u16),
            style::Print(&target_str),
            SetForegroundColor(Color::DarkGrey),
            cursor::MoveToColumn(columns - 7 - (duration_str.len() - 1) as u16),
            style::Print(&duration_str),
        )?;

        match state.mode {
            Mode::Working => {
                if i == 0 {
                    // TODO: refactor a bit
                    let spent_in_current_session = Local::now() - state.start_working_time.clone();
                    let spent_seconds = Duration::seconds(task.seconds_spent) + spent_in_current_session;
                    if spent_seconds.num_seconds() < task.seconds_estimated {
                        let timer = start_of_day + Duration::seconds(task.seconds_estimated) - spent_seconds;
                        let timer_str = format!("{}", timer.format("%-H:%M:%S"));
                        queue!(
                            io::stdout(),
                            cursor::MoveToColumn(columns - 13 - (timer_str.len() - 0) as u16),
                            SetForegroundColor(Color::Green),
                            style::Print("+"),
                            style::Print(&timer_str),
                            style::ResetColor,
                        )?;
                    } else {
                        let timer = start_of_day + (spent_seconds - Duration::seconds(task.seconds_estimated));
                        let timer_str = format!("{}", timer.format("%-H:%M:%S"));
                        queue!(
                            io::stdout(),
                            cursor::MoveToColumn(columns - 13 - (timer_str.len() - 0) as u16),
                            SetForegroundColor(Color::Red),
                            style::Print("-"),
                            style::Print(&timer_str),
                            style::ResetColor,
                        )?;
                    }
                } else {
                    if task.seconds_spent > 0 {
                        let spent_seconds = Duration::seconds(task.seconds_spent);
                        if spent_seconds.num_seconds() < task.seconds_estimated {
                            let timer = start_of_day + Duration::seconds(task.seconds_estimated) - spent_seconds;
                            let timer_str = format!("{}", timer.format("%-H:%M:%S"));
                            queue!(
                                io::stdout(),
                                cursor::MoveToColumn(columns - 13 - (timer_str.len() - 0) as u16),
                                SetForegroundColor(Color::Green),
                                style::Print("+"),
                                style::Print(&timer_str),
                                style::ResetColor,
                            )?;
                        } else {
                            let timer = start_of_day + (spent_seconds - Duration::seconds(task.seconds_estimated));
                            let timer_str = format!("{}", timer.format("%-H:%M:%S"));
                            queue!(
                                io::stdout(),
                                cursor::MoveToColumn(columns - 13 - (timer_str.len() - 0) as u16),
                                SetForegroundColor(Color::Red),
                                style::Print("-"),
                                style::Print(&timer_str),
                                style::ResetColor,
                            )?;
                        }
                    }
                }
            }
            _ => {
                if task.seconds_spent > 0 {
                    let spent_seconds = Duration::seconds(task.seconds_spent);
                    if spent_seconds.num_seconds() < task.seconds_estimated {
                        let timer = start_of_day + Duration::seconds(task.seconds_estimated) - spent_seconds;
                        let timer_str = format!("{}", timer.format("%-H:%M:%S"));
                        queue!(
                            io::stdout(),
                            cursor::MoveToColumn(columns - 13 - (timer_str.len() - 0) as u16),
                            SetForegroundColor(Color::Green),
                            style::Print("+"),
                            style::Print(&timer_str),
                            style::ResetColor,
                        )?;
                    } else {
                        let timer = start_of_day + (spent_seconds - Duration::seconds(task.seconds_estimated));
                        let timer_str = format!("{}", timer.format("%-H:%M:%S"));
                        queue!(
                            io::stdout(),
                            cursor::MoveToColumn(columns - 13 - (timer_str.len() - 0) as u16),
                            SetForegroundColor(Color::Red),
                            style::Print("-"),
                            style::Print(&timer_str),
                            style::ResetColor,
                        )?;
                    }
                }
            }
        }
    }

    Ok(())
}

fn render_timer() -> Result<()> {
    let (columns, _) = terminal::size()?;
    let now = Local::now();
    let now_str = format!("{}", now.format("%-H:%M"));

    queue!(
        io::stdout(),
        SetForegroundColor(Color::Green),
        cursor::MoveToColumn(columns - 0 - (now_str.len() - 1) as u16),
        style::Print(&now_str),
        style::ResetColor
    )?;

    Ok(())
}

fn render_mode(state: &AppState) -> Result<()> {
    let (_, rows) = terminal::size()?;

    let mode_str = match state.mode {
        Mode::Normal => "NORMAL",
        Mode::Working => "WORKING",
    };

    queue!(
        io::stdout(),
        cursor::MoveTo(0, rows),
        style::Print(format!("-- {} --", &mode_str)),
    )?;

    Ok(())
}

fn edit_task() -> Result<String> {
    queue!(
        io::stdout(),
        style::ResetColor,
        cursor::Show,
        cursor::MoveTo(0, 0),
        style::Print("Enter task description:"),
        cursor::MoveToNextLine(1),
    )?;

    io::stdout().flush()?;

    let name = read_line()?;

    queue!(
        io::stdout(),
        style::ResetColor,
        terminal::Clear(ClearType::All),
        cursor::Hide,
        cursor::MoveTo(0, 1)
    )?;

    Ok(name)
}

fn render(data: &AppData, state: &AppState) -> Result<()> {
    queue!(
        io::stdout(),
        style::ResetColor,
        terminal::Clear(ClearType::All),
        cursor::Hide,
        cursor::MoveTo(0, 1)
    )?;
    render_timer()?;
    render_tasks(&data, &state)?;
    render_mode(&state)?;
    io::stdout().flush()?;

    Ok(())
}

fn main() -> Result<()> {
    // If no app data exists, create the file with default values
    if !Path::new(APP_DATA_FILENAME).exists() {
        save_app_data(default_app_data());
    }

    // Load data & Initialize state
    let mut data: AppData = load_app_data();
    let mut state: AppState = AppState {
        mode: Mode::Normal,
        start_working_time: Local::now(),
        selected_task_index: 0,
        animation_counter: 0,
    };

    // Setup terminal
    execute!(io::stdout(), terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;

    // Main loop
    loop {
        render(&data, &state)?;

        // Wait up to 1s for an input event
        if event::poll(time::Duration::from_millis(1_000))? {
            match event::read()? {

                // Ctrl+C -> Exit program
                Event::Key(KeyEvent {
                    modifiers: KeyModifiers::CONTROL,
                    code: KeyCode::Char('c'),
                }) => {
                    match state.mode {
                        Mode::Working => {
                            let seconds_spent = Local::now() - state.start_working_time;
                            data.tasks[0].seconds_spent += seconds_spent.num_seconds();
                        }
                        _ => {}
                    }
                    break;
                }

                // Enter -> Switch mode
                Event::Key(KeyEvent { code: KeyCode::Enter, .. }) => {
                    match state.mode {
                        Mode::Normal => {
                            state.mode = Mode::Working;
                            state.start_working_time = Local::now();
                        }
                        Mode::Working => {
                            state.mode = Mode::Normal;
                            let seconds_spent = Local::now() - state.start_working_time;
                            data.tasks[0].seconds_spent += seconds_spent.num_seconds();
                        }
                    }
                }

                // J
                Event::Key(KeyEvent { code: KeyCode::Char('J'), .. }) => {
                    match state.mode {
                        Mode::Normal => {
                            if state.selected_task_index + 1 < data.tasks.len() {
                                data.tasks.swap(state.selected_task_index, state.selected_task_index + 1);
                                state.selected_task_index = state.selected_task_index + 1;
                            }
                        }
                        _ => {}
                    }
                }

                // K
                Event::Key(KeyEvent { code: KeyCode::Char('K'), .. }) => {
                    match state.mode {
                        Mode::Normal => {
                            if state.selected_task_index != 0 {
                                data.tasks.swap(state.selected_task_index, state.selected_task_index - 1);
                                state.selected_task_index = state.selected_task_index - 1;
                            }
                        }
                        _ => {}
                    }
                }

                // j
                Event::Key(KeyEvent { code: KeyCode::Char('j'), .. }) => {
                    match state.mode {
                        Mode::Normal => {
                            if state.selected_task_index + 1 < data.tasks.len() {
                                state.selected_task_index = state.selected_task_index + 1;
                            }
                        }
                        _ => {}
                    }
                }

                // k
                Event::Key(KeyEvent { code: KeyCode::Char('k'), .. }) => {
                    match state.mode {
                        Mode::Normal => {
                            if state.selected_task_index != 0 {
                                state.selected_task_index = state.selected_task_index - 1;
                            }
                        }
                        _ => {}
                    }
                }

                // + or =
                Event::Key(KeyEvent { code: KeyCode::Char('+'), .. }) |
                Event::Key(KeyEvent { code: KeyCode::Char('='), .. }) => {
                    data.tasks[state.selected_task_index].seconds_estimated += 15 * 60;
                }

                // - or _
                Event::Key(KeyEvent { code: KeyCode::Char('-'), .. }) |
                Event::Key(KeyEvent { code: KeyCode::Char('_'), .. }) => {
                    data.tasks[state.selected_task_index].seconds_estimated = max(15 * 60, data.tasks[state.selected_task_index].seconds_estimated - 15 * 60);
                }

                // n
                Event::Key(KeyEvent { code: KeyCode::Char('n'), .. }) => {
                    match state.mode {
                        Mode::Normal => {
                            state.selected_task_index = data.tasks.len();
                            data.tasks.push(Task {
                                text: String::from(""),
                                seconds_estimated: 15 * 60,
                                seconds_spent: 0,
                            });
                            let text = edit_task()?;
                            data.tasks[state.selected_task_index].text = text;
                        }
                        _ => {}
                    }
                }

                // e
                Event::Key(KeyEvent { code: KeyCode::Char('e'), .. }) => {
                    match state.mode {
                        Mode::Normal => {
                            let text = edit_task()?;
                            data.tasks[state.selected_task_index].text = text;
                        }
                        _ => {}
                    }
                }

                // x
                Event::Key(KeyEvent { code: KeyCode::Char('x'), .. }) => {
                    match state.mode {
                        Mode::Normal => {
                            data.tasks.remove(state.selected_task_index);
                            if state.selected_task_index >= data.tasks.len() {
                                state.selected_task_index = data.tasks.len() - 1;
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        } else {
            state.animation_counter += 1;
            if state.animation_counter >= WORKING_ANIMATION_FRAMES.len() {
                state.animation_counter = 0;
            }
        }
    }

    // Persist data on exit
    save_app_data(data);

    // Cleanup terminal
    execute!(
        io::stdout(),
        style::ResetColor,
        cursor::Show,
        terminal::LeaveAlternateScreen
    )?;
    terminal::disable_raw_mode()
}
