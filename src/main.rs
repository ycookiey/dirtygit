mod app;
mod config;
mod event;
mod git;
mod scanner;
mod types;
mod ui;

use app::{ActivePane, App, InputMode};
use config::Config;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use event::Event;
use std::io;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> io::Result<()> {
    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut app = App::new();

    // Load config
    let config = match Config::load() {
        Ok(cfg) => Some(cfg),
        Err(msg) => {
            app.config_error = Some(msg);
            None
        }
    };

    // Initial scan if config is available
    if let Some(ref cfg) = config {
        app.scanning = true;
        let repos = scanner::scan_all(cfg);
        app.set_repos(repos);
        app.scanning = false;
        app.last_scan_time = Some(chrono::Local::now().format("%H:%M:%S").to_string());
    }

    // Set up event channel
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Spawn crossterm event reader
    event::spawn_event_reader(tx.clone(), 250);

    // Spawn periodic background scan
    if let Some(cfg) = config {
        let interval = cfg.interval_secs;
        let scan_tx = tx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
                let cfg_clone = Config::load();
                if let Ok(cfg) = cfg_clone {
                    let repos = tokio::task::spawn_blocking(move || scanner::scan_all(&cfg))
                        .await
                        .unwrap_or_default();
                    let _ = scan_tx.send(Event::ScanComplete(repos));
                }
            }
        });
    }

    // Main loop
    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        if let Some(event) = rx.recv().await {
            match event {
                Event::Key(key) => {
                    // Only handle key press events (not release/repeat)
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    handle_key(&mut app, key.code, key.modifiers, &tx);
                }
                Event::Tick => {}
                Event::ScanComplete(repos) => {
                    app.set_repos(repos);
                    app.scanning = false;
                    app.last_scan_time =
                        Some(chrono::Local::now().format("%H:%M:%S").to_string());
                }
            }
        }

        if !app.running {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;

    Ok(())
}

fn handle_key(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
    tx: &mpsc::UnboundedSender<Event>,
) {
    // Handle search mode input
    if app.input_mode == InputMode::Search {
        match code {
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                app.search_query.clear();
                app.update_filtered();
            }
            KeyCode::Enter => {
                app.input_mode = InputMode::Normal;
                // Keep the search query active
            }
            KeyCode::Backspace => {
                app.search_query.pop();
                app.update_filtered();
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
                app.update_filtered();
            }
            _ => {}
        }
        return;
    }

    // Normal mode
    match code {
        KeyCode::Char('q') => app.running = false,
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => app.running = false,

        // Navigation
        KeyCode::Char('j') | KeyCode::Down => match app.active_pane {
            ActivePane::RepoList => app.move_down(),
            ActivePane::Detail => {
                app.detail_scroll = app.detail_scroll.saturating_add(1);
            }
        },
        KeyCode::Char('k') | KeyCode::Up => match app.active_pane {
            ActivePane::RepoList => app.move_up(),
            ActivePane::Detail => {
                app.detail_scroll = app.detail_scroll.saturating_sub(1);
            }
        },
        KeyCode::Char('g') => {
            if app.active_pane == ActivePane::RepoList {
                app.selected = 0;
                app.detail_scroll = 0;
            } else {
                app.detail_scroll = 0;
            }
        }
        KeyCode::Char('G') => {
            if app.active_pane == ActivePane::RepoList && !app.filtered_indices.is_empty() {
                app.selected = app.filtered_indices.len() - 1;
            }
        }

        // Pane switching
        KeyCode::Tab => {
            app.active_pane = match app.active_pane {
                ActivePane::RepoList => ActivePane::Detail,
                ActivePane::Detail => ActivePane::RepoList,
            };
        }

        // Sort
        KeyCode::Char('s') => app.toggle_sort(),

        // Dirty filter
        KeyCode::Char('f') => app.toggle_dirty_filter(),

        // Search
        KeyCode::Char('/') => {
            app.input_mode = InputMode::Search;
            app.search_query.clear();
        }

        // Refresh
        KeyCode::Char('r') => {
            let scan_tx = tx.clone();
            app.scanning = true;
            tokio::spawn(async move {
                if let Ok(cfg) = Config::load() {
                    let repos = tokio::task::spawn_blocking(move || scanner::scan_all(&cfg))
                        .await
                        .unwrap_or_default();
                    let _ = scan_tx.send(Event::ScanComplete(repos));
                }
            });
        }

        // Open lazygit
        KeyCode::Enter => {
            if let Some(repo) = app.selected_repo() {
                let path = repo.path.clone();
                launch_external(path.to_string_lossy().as_ref());
            }
        }

        _ => {}
    }
}

fn launch_external(path: &str) {
    // Temporarily leave alternate screen and disable raw mode
    let _ = disable_raw_mode();
    let _ = io::stdout().execute(LeaveAlternateScreen);

    // Try lazygit first
    let result = std::process::Command::new("lazygit")
        .current_dir(path)
        .status();

    if result.is_err() {
        eprintln!("lazygit not found. Install lazygit to use this feature.");
        std::thread::sleep(std::time::Duration::from_secs(2));
    }

    // Restore terminal
    let _ = enable_raw_mode();
    let _ = io::stdout().execute(EnterAlternateScreen);
}
