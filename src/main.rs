use serde_json;
use serde::{Serialize, Deserialize};
use chrono::{Datelike, DateTime, Timelike, Duration, Local};


use std::{
    io::{self, stdout, Write},
    time::{self, Instant},
    cmp::{min, max},
    fs::{File},
    path::{Path},
};

use crossterm::{
    cursor::{self, position},
    event::{self, poll, read, Event, KeyCode, KeyEvent, KeyModifiers},
    style::{self, SetForegroundColor, SetBackgroundColor, Color},
    terminal::{self, ClearType},
    execute,
    ExecutableCommand,
    QueueableCommand,
    queue,
    Result,
};

// As a user:
// - start the day, add a couple tasks
// - arrange day

// Keys:
// - j/k: navigate
// - n: new task
// - enter: complete task
// - x: delete task
// - backspace: ignore task
// - >: defer task
// - u: undo
// - +/-: increase/decrease duration by 15m
// - e/i: edit

// Features:
// - 'scrollback' to completed tasks
// - Timer
// - lunch e.d.
// - persistent db (serialization of tasks/events)
// - Monthly log (jump to day)
// - Nice (random) animation on task completion
// - Flicker on task selection / work start
// - Auto git (branch creation) and merge request + ticket integration

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
    EditTask,
}

#[derive(Serialize, Deserialize, Debug)]
struct AppData {
    tasks: Vec<Task>,

}

#[derive(Serialize, Deserialize, Debug)]
struct Task {
    text: String,
    minutes: i64,
}

fn default_app_data() -> AppData {
    let tasks: Vec<Task> = vec![
        Task {
            text: String::from("Task 1"),
            minutes: 30,
        },
        Task {
            text: String::from("Task 2"),
            minutes: 60,
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
    let serialized = serde_json::to_writer(&file, &app_data).unwrap();
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
            }
            _ => {}
        }
    }

    return Ok(line);
}

fn render_current_task<W>(w: &mut W, task: &Task) -> Result<()>
where
    W: Write,
{


    Ok(())
}

fn render_tasks<W>(w: &mut W, tasks: &Vec<Task>, selected_task_index: usize, start_time: &DateTime<Local>, animation_counter: usize, mode: &Mode) -> Result<()>
where
    W: Write,
{
    let (columns, rows) = terminal::size()?;
    let start_of_day = Local::today().and_hms(0, 0, 0);

    let mut head = start_time.clone();
    for (i, task) in tasks.iter().enumerate() {
        let duration = Duration::minutes(task.minutes);
        head = head + duration;

        let text = format!("{}", &task.text);
        let duration_str = format!("{}", (start_of_day + duration).format("%-H:%M"));
        let target_str = format!("{}", head.format("%H:%M"));

        match mode {
            Mode::Normal => {
                if i == selected_task_index {
                    queue!(
                        w,
                        SetForegroundColor(Color::Yellow),
                        style::Print("• "),
                        style::Print(&text),
                    )?;
                } else {
                    queue!(
                        w,
                        style::Print("· "),
                        style::Print(&text),
                    )?;
                }
            }
            Mode::Working => {
                if i == 0 {
                    queue!(
                        w,
                        SetForegroundColor(Color::Green),
                        style::Print(WORKING_ANIMATION_FRAMES[animation_counter]),
                        style::Print(&text),
                    )?;
                } else {
                    queue!(
                        w,
                        style::Print("· "),
                        style::Print(&text),
                    )?;
                }
            }
            Mode::EditTask => {
            }
        }

        queue!(
            w,
            SetForegroundColor(Color::DarkGrey),
            cursor::MoveToColumn(columns - (target_str.len() - 1) as u16),
            style::Print(&target_str),
            SetForegroundColor(Color::DarkGrey),
            cursor::MoveToColumn(columns - 8 - (duration_str.len() - 1) as u16),
            style::Print(&duration_str),
            style::ResetColor,
            cursor::MoveToNextLine(2)
        )?;
    }

    Ok(())
}

fn render_timer<W>(w: &mut W) -> Result<()>
where
    W: Write,
{
    let now = Local::now();

    queue!(
        w,
        SetForegroundColor(Color::Green),
        style::Print(now.format("%H:%M:%S").to_string()),
        style::ResetColor
    )?;

    Ok(())
}

fn render_mode<W>(w: &mut W, mode: &Mode) -> Result<()>
where
    W: Write,
{
    let (columns, rows) = terminal::size()?;

    let mode_str = match mode {
        Mode::Normal => "NORMAL",
        Mode::Working => "WORKING",
        Mode::EditTask => "EDITING",
    };

    queue!(
        w,
        cursor::MoveTo(0, rows),
        style::Print(format!("-- {} --", &mode_str))
    )?;

    Ok(())
}

fn render<W>(w: &mut W, tasks: &Vec<Task>, selected_task_index: usize, current_task_index: usize, mode: &Mode, start_time: &DateTime<Local>, animation_counter: usize) -> Result<()>
where
    W: Write,
{
    queue!(
        w,
        style::ResetColor,
        terminal::Clear(ClearType::All),
        cursor::Hide,
        cursor::MoveTo(0, 1)
    )?;
    render_current_task(w, &tasks[current_task_index])?;
    render_tasks(w, &tasks, selected_task_index, start_time, animation_counter, mode)?;
    render_timer(w)?;
    render_mode(w, mode)?;
    w.flush()?;

    Ok(())
}

fn run<W>(w: &mut W) -> Result<()>
where
    W: Write,
{
    // If no app data exists, create the file with default values
    if !Path::new(APP_DATA_FILENAME).exists() {
        save_app_data(default_app_data());
    }

    let mut app_data: AppData = load_app_data();

    let mut mode: Mode = Mode::Normal;
    let start_time = Local::now();

    let mut current_task_index: usize = 0;
    let mut selected_task_index: usize = 0;
    let mut animation_counter: usize = 0;

    execute!(w, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;
    render(w, &app_data.tasks, selected_task_index, current_task_index, &mode, &start_time, animation_counter)?;

    loop {
        // Wait up to 1s for another event
        if poll(time::Duration::from_millis(1_000))? {
            let event = read()?;

            match event {
                Event::Key(key_event) => {
                    if key_event.modifiers == KeyModifiers::CONTROL && key_event.code == KeyCode::Char('c') {
                        save_app_data(app_data);
                        break;
                    }
                    if key_event.code == KeyCode::Enter {
                        match mode {
                            Mode::Normal => { mode = Mode::Working }
                            _ => { mode = Mode::Normal }
                        }
                    }
                    if key_event.code == KeyCode::Esc {
                        mode = Mode::Normal;
                    }
                    if key_event.code == KeyCode::Char('J') {
                        if selected_task_index + 1 < app_data.tasks.len() {
                            app_data.tasks.swap(selected_task_index, selected_task_index + 1);
                            selected_task_index = selected_task_index + 1;
                        }
                    }
                    if key_event.code == KeyCode::Char('K') {
                        if selected_task_index != 0 {
                            app_data.tasks.swap(selected_task_index, selected_task_index - 1);
                            selected_task_index = selected_task_index - 1;
                        }
                    }
                    if key_event.code == KeyCode::Char('j') {
                        match mode {
                            Mode::Normal => {
                                if selected_task_index + 1 < app_data.tasks.len() {
                                    selected_task_index = selected_task_index + 1;
                                }
                            }
                            _ => {}
                        }
                    }
                    if key_event.code == KeyCode::Char('k') {
                        match mode {
                            Mode::Normal => {
                                if selected_task_index != 0 {
                                    selected_task_index = selected_task_index - 1;
                                }
                            }
                            _ => {}
                        }
                    }
                    if key_event.code == KeyCode::Char('+') || key_event.code == KeyCode::Char('=') {
                        app_data.tasks[selected_task_index].minutes += 15;
                    }
                    if key_event.code == KeyCode::Char('-') {
                        app_data.tasks[selected_task_index].minutes = max(15, app_data.tasks[selected_task_index].minutes - 15);
                    }
                    if key_event.code == KeyCode::Char('n') {
                        mode = Mode::EditTask;
                        selected_task_index = app_data.tasks.len();
                        app_data.tasks.push(Task {
                            text: String::from(""),
                            minutes: 15,
                        });
                    }
                    if key_event.code == KeyCode::Char('e') {
                        mode = Mode::EditTask;
                        let input = read_line();
                    }
                }
                _ => {
                    animation_counter += 1;
                    if animation_counter >= WORKING_ANIMATION_FRAMES.len() {
                        animation_counter = 0;
                    }
                }
            }
        }

        render(w, &app_data.tasks, selected_task_index, current_task_index, &mode, &start_time, animation_counter)?;
    }

    // Cleanup
    execute!(
        w,
        style::ResetColor,
        cursor::Show,
        terminal::LeaveAlternateScreen
    )?;
    terminal::disable_raw_mode()
}

fn main() -> Result<()> {
    let mut stderr = io::stdout();
    run(&mut stderr)
}
