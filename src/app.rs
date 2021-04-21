use crate::database::Database;

use chrono::{DateTime, Duration, Local};

use crossterm::{
    Result,
};

use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Widget},
    Terminal,
};


enum Mode {
    Normal,
    Working,
}

pub struct App {
    mode: Mode,
    start_working_time: DateTime<Local>,
    selected_task_index: usize,
    animation_counter: usize,
}

impl App {
    pub fn new() -> Self {
        Self {
            mode: Mode::Normal,
            start_working_time: Local::now(),
            selected_task_index: 0,
            animation_counter: 0,
        }
    }

    pub fn update(&mut self, db: &Database) -> Result<()> {
        Ok(())
    }

    pub fn render<B>(&mut self, db: &Database, terminal: &mut Terminal<B>) -> Result<()>
    where
        B: Backend,
    {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Percentage(10),
                        Constraint::Percentage(80),
                        Constraint::Percentage(10),
                    ]
                    .as_ref(),
                )
                .split(f.size());
            let block = Block::default().title("Block").borders(Borders::ALL);
            f.render_widget(block, chunks[0]);
            let block = Block::default().title("Block 2").borders(Borders::ALL);
            f.render_widget(block, chunks[1]);
        });
        Ok(())
    }
}

