#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod app;
mod database;

use crate::app::App;
use crate::database::{Database, Task, save_data, load_data, default_data};

use std::{
    io::{self},
    path::{Path},
};

use crossterm::{
    cursor::{self},
    execute, queue,
    style::{self, Color},
    terminal::{self, ClearType},
    Result,
};

use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders},
    Terminal,
};

const DB_FILENAME: &str = "/Users/daveystruijk/work.json";

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
// [ ] audio alerts
// [ ] use tui-rs
// [ ] length of task -> longer (more lines) if it takes more time?
// [ ] progress bar for current task (est + actual)
// [ ] 'scrollback' to completed tasks
// [x] Timer
// [ ] lunch e.d.
// [x] persistent db (serialization of tasks/events)
// [ ] Monthly log (jump to day)
// [ ] Nice (random) animation on task completion
// [ ] Flicker on task selection / work start
// [ ] Auto git (branch creation) and merge request + ticket integration
// [ ] reporting/finalizing, using:
//     - command history during time
//     - browser history?

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

fn main() -> Result<()> {
    // Setup terminal
    terminal::enable_raw_mode()?;
    execute!(io::stdout(), terminal::EnterAlternateScreen)?;
    execute!(io::stdout(), terminal::Clear(ClearType::All)).unwrap();
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend).unwrap();

    // If no database exists, create the file with default values
    if !Path::new(DB_FILENAME).exists() {
        save_data(DB_FILENAME, default_data());
    }

    // Load data & Initialize app
    let mut db = load_data(DB_FILENAME);
    let mut app = App::new();

    // Main loop
    loop {
        app.update(&db)?;
        app.render(&db, &terminal)?;
    }

    // Persist data on exit
    save_data(db);

    // Cleanup terminal
    execute!(
        io::stdout(),
        style::ResetColor,
        cursor::Show,
        terminal::LeaveAlternateScreen
    )?;
    terminal::disable_raw_mode()
}
