use std::{error::Error, io, io::Stdout};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::prelude::Backend;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::{Frame, Terminal};

use crate::gz::git::{Entry, Git};

pub enum CurrentScreen {
    Add,
}

pub struct App {
    git: Git,
    screen: CurrentScreen,
    entries: Vec<Entry>,
    selected: usize,
    list_state: ListState,
}

impl App {
    pub fn new(screen: CurrentScreen) -> App {
        let git = Git::open();
        let mut app = App {
            git,
            screen,
            entries: Vec::new(),
            selected: 0,
            list_state: ListState::default(),
        };
        app.refresh_entries();
        app
    }

    fn refresh_entries(&mut self) {
        self.entries = self.git.status();

        if self.selected >= self.entries.len() && !self.entries.is_empty() {
            self.selected = self.entries.len() - 1;
        }

        if self.entries.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(self.selected));
        }
    }
}

pub fn run_tui(screen: CurrentScreen) -> Result<(), Box<dyn Error>> {
    let mut manager = GzTerminal::new()?;
    let mut app = App::new(screen);

    run_app(manager.terminal_mut(), &mut app)?;
    Ok(())
}

struct GzTerminal {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl GzTerminal {
    fn new() -> Result<Self, Box<dyn Error>> {
        enable_raw_mode()?;

        let mut stdout = io::stdout();
        if let Err(e) = execute!(stdout, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(e.into());
        }

        let backend = CrosstermBackend::new(stdout);
        match Terminal::new(backend) {
            Ok(terminal) => Ok(Self { terminal }),
            Err(e) => {
                let _ = execute!(io::stdout(), LeaveAlternateScreen);
                let _ = disable_raw_mode();
                Err(e.into())
            }
        }
    }

    fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }
}

impl Drop for GzTerminal {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Release {
                continue;
            }
            match app.screen {
                CurrentScreen::Add => match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Up => {
                        if app.selected > 0 {
                            app.selected -= 1;
                        }
                        if app.selected >= app.entries.len() {
                            app.selected = app.entries.len().saturating_sub(1);
                        }
                        app.list_state.select(Some(app.selected));
                    }
                    KeyCode::Down => {
                        if app.selected + 1 < app.entries.len() {
                            app.selected += 1;
                        }
                        app.list_state.select(Some(app.selected));
                    }
                    KeyCode::Enter => {
                        if let Some(entry) = app.entries.get_mut(app.selected) {
                            if entry.staged {
                                app.git.unstage_paths(&[entry.path.clone()]);
                            } else {
                                app.git.stage_paths(&[entry.path.clone()]);
                            }
                            app.refresh_entries();
                        }
                    }
                    _ => {}
                },
            }
        }
    }
}

fn ui(f: &mut Frame<'_>, app: &mut App) {
    let size = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
        .split(size);

    match app.screen {
        CurrentScreen::Add => {
            let mut items: Vec<ListItem> = Vec::new();
            let mut vis_idx: usize = 0;

            let staged_map = app.git.staged_line_counts();
            let unstaged_map = app.git.unstaged_line_counts();

            for e in app.entries.iter() {
                let file = e.path.to_string_lossy();
                let key = e.path.to_string_lossy().to_string();
                let (a, d) = if e.staged {
                    staged_map.get(&key).cloned().unwrap_or((0, 0))
                } else {
                    unstaged_map.get(&key).cloned().unwrap_or((0, 0))
                };
                let count = format!("+{} -{}  {}", a, d, file);
                let mut style = if e.staged {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                };
                if vis_idx == app.selected {
                    style = style.add_modifier(Modifier::BOLD);
                }
                items.push(ListItem::new(Span::styled(count, style)));
                vis_idx += 1;
            }

            let list = List::new(items).block(Block::default().borders(Borders::ALL).title("Add"));
            f.render_stateful_widget(list, chunks[0], &mut app.list_state);

            let footer = Paragraph::new("Enter: Toggle  •  ↑/↓: Up/Down  •  q: Quit").alignment(Alignment::Center);
            f.render_widget(footer, chunks[1]);
        }
    }
}
