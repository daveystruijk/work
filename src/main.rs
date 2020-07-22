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

fn render_tasks<W>(w: &mut W, tasks: &Vec<Task>, selected_task_index: usize, start_working_time: &DateTime<Local>, animation_counter: usize, mode: &Mode) -> Result<()>
where
    W: Write,
{
    let (columns, rows) = terminal::size()?;
    let start_of_day = Local::today().and_hms(0, 0, 0);

    let mut eta = Local::now();
    for (i, task) in tasks.iter().enumerate() {
        let duration = Duration::seconds(task.seconds_estimated);
        eta = eta + duration;

        let text = format!("{}", &task.text);
        let duration_str = format!("{}", (start_of_day + duration).format("%-H:%M"));
        let target_str = format!("{}", eta.format("%H:%M"));

        // Draw bullet + name
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
                        style::Print(" "),
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
        }

        // Draw seconds_estimated + eta
        queue!(
            w,
            SetForegroundColor(Color::DarkGrey),
            cursor::MoveToColumn(columns - (target_str.len() - 1) as u16),
            style::Print(&target_str),
            SetForegroundColor(Color::DarkGrey),
            cursor::MoveToColumn(columns - 7 - (duration_str.len() - 1) as u16),
            style::Print(&duration_str),
        )?;

        match mode {
            Mode::Working => {
                if i == 0 {
                    let spent_in_current_session = Local::now() - start_working_time.clone();
                    let spent_seconds = Duration::seconds(task.seconds_spent) + spent_in_current_session;
                    if spent_seconds.num_seconds() < task.seconds_estimated {
                        let timer = start_of_day + Duration::seconds(task.seconds_estimated) - spent_seconds;
                        let timer_str = format!("{}", timer.format("%-H:%M:%S"));
                        queue!(
                            w,
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
                            w,
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
                                w,
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
                                w,
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
                            w,
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
                            w,
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

        queue!(
            w,
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
    let (columns, rows) = terminal::size()?;
    let now = Local::now();
    let now_str = format!("{}", now.format("%-H:%M"));

    queue!(
        w,
        SetForegroundColor(Color::Green),
        cursor::MoveToColumn(columns - 0 - (now_str.len() - 1) as u16),
        style::Print(&now_str),
        style::ResetColor,
        cursor::MoveToNextLine(2)
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
    };

    queue!(
        w,
        cursor::MoveTo(0, rows),
        style::Print(format!("-- {} --", &mode_str))
    )?;

    Ok(())
}

fn edit_task<W>(w: &mut W, tasks: &Vec<Task>, selected_task_index: usize) -> Result<()>
where
    W: Write,
{


    Ok(())
}

fn render<W>(w: &mut W, tasks: &Vec<Task>, selected_task_index: usize, mode: &Mode, start_working_time: &DateTime<Local>, animation_counter: usize) -> Result<()>
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
    // render_current_task(w, &tasks[0])?;
    render_timer(w)?;
    render_tasks(w, &tasks, selected_task_index, start_working_time, animation_counter, mode)?;
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
    let mut start_working_time = Local::now();

    let mut selected_task_index: usize = 0;
    let mut animation_counter: usize = 0;

    execute!(w, terminal::EnterAlternateScreen)?;
    terminal::enable_raw_mode()?;
    render(w, &app_data.tasks, selected_task_index, &mode, &start_working_time, animation_counter)?;

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
                            Mode::Normal => {
                                mode = Mode::Working;
                                start_working_time = Local::now();
                            }
                            Mode::Working => {
                                mode = Mode::Normal;
                                let seconds_spent = Local::now() - start_working_time;
                                app_data.tasks[0].seconds_spent += seconds_spent.num_seconds();
                            }
                        }
                    }
                    if key_event.code == KeyCode::Esc {
                        mode = Mode::Normal;
                    }
                    if key_event.code == KeyCode::Char('J') {
                        match mode {
                            Mode::Normal => {
                                if selected_task_index + 1 < app_data.tasks.len() {
                                    app_data.tasks.swap(selected_task_index, selected_task_index + 1);
                                    selected_task_index = selected_task_index + 1;
                                }
                            }
                            _ => {}
                        }
                    }
                    if key_event.code == KeyCode::Char('K') {
                        match mode {
                            Mode::Normal => {
                                if selected_task_index != 0 {
                                    app_data.tasks.swap(selected_task_index, selected_task_index - 1);
                                    selected_task_index = selected_task_index - 1;
                                }
                            }
                            _ => {}
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
                        app_data.tasks[selected_task_index].seconds_estimated += 15 * 60;
                    }
                    if key_event.code == KeyCode::Char('-') || key_event.code == KeyCode::Char('_') {
                        app_data.tasks[selected_task_index].seconds_estimated = max(15 * 60, app_data.tasks[selected_task_index].seconds_estimated - 15 * 60);
                    }
                    if key_event.code == KeyCode::Char('n') {
                        match mode {
                            Mode::Normal => {
                                selected_task_index = app_data.tasks.len();
                                app_data.tasks.push(Task {
                                    text: String::from(""),
                                    seconds_estimated: 15 * 60,
                                    seconds_spent: 0,
                                });
                                // TODO: edit task
                            }
                            _ => {}
                        }
                    }
                    if key_event.code == KeyCode::Char('e') {
                        match mode {
                            Mode::Normal => {
                                edit_task(w, &app_data.tasks, selected_task_index)?;
                            }
                            _ => {}
                        }
                    }
                    if key_event.code == KeyCode::Char('x') {
                        match mode {
                            Mode::Normal => {
                                app_data.tasks.remove(selected_task_index);
                                if selected_task_index >= app_data.tasks.len() {
                                    selected_task_index = app_data.tasks.len() - 1;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {

                }
            }
        } else {
            animation_counter += 1;
            if animation_counter >= WORKING_ANIMATION_FRAMES.len() {
                animation_counter = 0;
            }
        }

        render(w, &app_data.tasks, selected_task_index, &mode, &start_working_time, animation_counter)?;
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
