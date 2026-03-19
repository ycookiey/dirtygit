mod app;
mod cache;
mod config;
mod event;
mod git;
mod scanner;
mod setup;
mod types;
mod ui;

use app::{ActivePane, App, InputMode};
use config::Config;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use setup::SetupAction;
use crossterm::event::EnableMouseCapture;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use event::Event;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> io::Result<()> {
    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(EnableMouseCapture)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut app = App::new();

    // Load config
    let config = if !Config::config_path().exists() {
        app.setup_state = Some(setup::SetupState::new());
        None
    } else {
        match Config::load() {
            Ok(cfg) => Some(cfg),
            Err(msg) => {
                app.config_error = Some(msg);
                None
            }
        }
    };

    // Load cache for instant display
    if config.is_some() {
        if let Some(cached) = cache::load_cache() {
            app.load_from_cache(cached);
        }
    }

    // Set up event channel
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Spawn crossterm event reader
    let event_stop = Arc::new(AtomicBool::new(false));
    let mut event_handle = event::spawn_event_reader(tx.clone(), 250, Arc::clone(&event_stop));

    // Spawn initial background scan
    if let Some(ref cfg) = config {
        app.scanning = true;
        spawn_scan(cfg, &tx, &app.repos);
    }

    // Spawn periodic background scan
    if let Some(ref cfg) = config {
        let interval = cfg.interval_secs;
        let scan_tx = tx.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(interval)).await;
                if let Ok(cfg) = Config::load() {
                    scanner::scan_all_parallel(&cfg, scan_tx.clone(), &[]).await;
                }
            }
        });
    }

    // Spawn render timer (30fps)
    {
        let render_tx = tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(33));
            loop {
                interval.tick().await;
                if render_tx.send(Event::Render).is_err() {
                    break;
                }
            }
        });
    }

    // Initial draw
    terminal.draw(|f| ui::draw(f, &mut app))?;

    // Main loop
    loop {
        if let Some(event) = rx.recv().await {
            match event {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    // Setup mode
                    if let Some(ref mut setup) = app.setup_state {
                        match setup.handle_key(key.code, key.modifiers) {
                            SetupAction::Save(dirs) => {
                                match Config::save(&dirs) {
                                    Ok(cfg) => {
                                        app.setup_state = None;
                                        app.scanning = true;
                                        spawn_scan(&cfg, &tx, &app.repos);
                                        let interval = cfg.interval_secs;
                                        let scan_tx = tx.clone();
                                        tokio::spawn(async move {
                                            loop {
                                                tokio::time::sleep(
                                                    tokio::time::Duration::from_secs(interval),
                                                )
                                                .await;
                                                if let Ok(cfg) = Config::load() {
                                                    scanner::scan_all_parallel(
                                                        &cfg,
                                                        scan_tx.clone(),
                                                        &[],
                                                    )
                                                    .await;
                                                }
                                            }
                                        });
                                        app.flash_message = Some((
                                            format!(
                                                "Config saved: {}",
                                                Config::config_path().display()
                                            ),
                                            std::time::Instant::now(),
                                        ));
                                    }
                                    Err(e) => {
                                        if let Some(ref mut s) = app.setup_state {
                                            s.warning = Some(format!("Save failed: {}", e));
                                        }
                                    }
                                }
                                continue;
                            }
                            SetupAction::Quit => {
                                app.running = false;
                                continue;
                            }
                            SetupAction::None => continue,
                        }
                    }

                    handle_key(&mut app, key.code, key.modifiers, &tx);

                    // Launch external program (lazygit) if requested
                    if let Some(path) = app.pending_external.take() {
                        // Stop event reader thread completely
                        event_stop.store(true, Ordering::Relaxed);
                        let _ = event_handle.join();

                        launch_external(path.to_string_lossy().as_ref());

                        // Refresh the repo that was just opened in lazygit
                        if let Some(status) = git::get_repo_status(&path) {
                            app.update_repo(status);
                            let _ = cache::save_cache(&app.repos);
                        }

                        // Drain stale events from channel
                        while rx.try_recv().is_ok() {}

                        // Spawn fresh event reader thread
                        event_stop.store(false, Ordering::Relaxed);
                        event_handle = event::spawn_event_reader(
                            tx.clone(),
                            250,
                            Arc::clone(&event_stop),
                        );
                        terminal.clear()?;
                    }
                }
                Event::Mouse(mouse) => {
                    handle_mouse(&mut app, mouse);
                }
                Event::Tick => {}
                Event::Render => {
                    terminal.draw(|f| ui::draw(f, &mut app))?;
                }
                Event::RepoUpdated(status) => {
                    app.update_repo(*status);
                }
                Event::ScanComplete => {
                    app.scanning = false;
                    app.last_scan_time =
                        Some(chrono::Local::now().format("%H:%M:%S").to_string());
                    // Save cache with fresh data
                    let _ = cache::save_cache(&app.repos);
                }
            }
        }

        if !app.running {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    terminal.backend_mut().execute(crossterm::event::DisableMouseCapture)?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;

    Ok(())
}

fn spawn_scan(
    cfg: &Config,
    tx: &mpsc::UnboundedSender<Event>,
    cached_repos: &[types::RepoStatus],
) {
    let scan_tx = tx.clone();
    let cfg_clone = cfg.clone();
    let cached: Vec<types::RepoStatus> = cached_repos.to_vec();
    tokio::spawn(async move {
        scanner::scan_all_parallel(&cfg_clone, scan_tx, &cached).await;
    });
}

fn handle_key(
    app: &mut App,
    code: KeyCode,
    modifiers: KeyModifiers,
    tx: &mpsc::UnboundedSender<Event>,
) {
    // Handle help overlay
    if app.show_help {
        match code {
            KeyCode::Char('?') | KeyCode::Esc => app.show_help = false,
            _ => {}
        }
        return;
    }

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
        // Page navigation
        KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => match app.active_pane {
            ActivePane::RepoList => app.move_down_n(app.list_height / 2),
            ActivePane::Detail => {
                app.detail_scroll = app.detail_scroll.saturating_add(app.list_height / 2);
            }
        },
        KeyCode::Char('u') if modifiers.contains(KeyModifiers::CONTROL) => match app.active_pane {
            ActivePane::RepoList => app.move_up_n(app.list_height / 2),
            ActivePane::Detail => {
                app.detail_scroll = app.detail_scroll.saturating_sub(app.list_height / 2);
            }
        },
        KeyCode::Char('f') if modifiers.contains(KeyModifiers::CONTROL) => match app.active_pane {
            ActivePane::RepoList => app.move_down_n(app.list_height),
            ActivePane::Detail => {
                app.detail_scroll = app.detail_scroll.saturating_add(app.list_height);
            }
        },
        KeyCode::Char('b') if modifiers.contains(KeyModifiers::CONTROL) => match app.active_pane {
            ActivePane::RepoList => app.move_up_n(app.list_height),
            ActivePane::Detail => {
                app.detail_scroll = app.detail_scroll.saturating_sub(app.list_height);
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

        // Copy path to clipboard
        KeyCode::Char('c') => {
            if let Some(repo) = app.selected_repo() {
                let path_str = repo.path.display().to_string();
                let copied = copy_to_clipboard(&path_str);
                app.flash_message = Some((
                    if copied {
                        format!("Copied: {}", path_str)
                    } else {
                        "Failed to copy to clipboard".to_string()
                    },
                    std::time::Instant::now(),
                ));
            }
        }

        // Help
        KeyCode::Char('?') => {
            app.show_help = true;
        }

        // Search
        KeyCode::Char('/') => {
            app.input_mode = InputMode::Search;
            app.search_query.clear();
        }

        // Refresh
        KeyCode::Char('r') => {
            if let Ok(cfg) = Config::load() {
                app.scanning = true;
                spawn_scan(&cfg, tx, &app.repos);
            }
        }

        // Open lazygit
        KeyCode::Enter => {
            if let Some(repo) = app.selected_repo() {
                app.pending_external = Some(repo.path.clone());
            }
        }

        _ => {}
    }
}

fn handle_mouse(app: &mut App, mouse: crossterm::event::MouseEvent) {
    use crossterm::event::{MouseEventKind, MouseButton};

    let (lx, ly, _lw, lh) = app.repo_list_area;
    // Content starts at ly+2 (border + header row)
    let content_y = ly + 2;
    let content_end = ly + lh.saturating_sub(1); // exclude bottom border

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            if mouse.row >= content_y && mouse.row < content_end && mouse.column >= lx {
                let row_idx = (mouse.row - content_y) as usize;
                if row_idx < app.filtered_indices.len() {
                    app.active_pane = ActivePane::RepoList;
                    app.selected = row_idx;
                    app.detail_scroll = 0;
                }
            }
        }
        MouseEventKind::ScrollUp => {
            match app.active_pane {
                ActivePane::RepoList => app.move_up(),
                ActivePane::Detail => {
                    app.detail_scroll = app.detail_scroll.saturating_sub(3);
                }
            }
        }
        MouseEventKind::ScrollDown => {
            match app.active_pane {
                ActivePane::RepoList => app.move_down(),
                ActivePane::Detail => {
                    app.detail_scroll = app.detail_scroll.saturating_add(3);
                }
            }
        }
        _ => {}
    }
}

fn launch_external(path: &str) {
    let _ = disable_raw_mode();
    let _ = io::stdout().execute(crossterm::event::DisableMouseCapture);
    let _ = io::stdout().execute(LeaveAlternateScreen);

    // Emit OSC 7 so terminal emulators (e.g. WezTerm) track the CWD
    emit_osc7(path);

    let _ = std::process::Command::new("lazygit")
        .current_dir(path)
        .status();

    let _ = enable_raw_mode();
    let _ = io::stdout().execute(EnterAlternateScreen);
    let _ = io::stdout().execute(crossterm::event::EnableMouseCapture);
}

/// Emit OSC 7 escape sequence to notify the terminal of the current working directory.
fn emit_osc7(path: &str) {
    let canonical = std::path::Path::new(path)
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from(path));
    let path_str = canonical.to_string_lossy();
    // On Windows, strip the \\?\ prefix from canonicalize()
    let path_str = path_str.strip_prefix(r"\\?\").unwrap_or(&path_str);
    let encoded = url_encode_path(path_str);
    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().into_owned())
        .unwrap_or_default();
    print!("\x1b]7;file://{hostname}/{encoded}\x1b\\");
    let _ = io::stdout().flush();
}

fn url_encode_path(path: &str) -> String {
    let mut result = String::with_capacity(path.len());
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'/' | b':' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

fn copy_to_clipboard(text: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::process::{Command, Stdio};
        if let Ok(mut child) = Command::new("clip")
            .stdin(Stdio::piped())
            .spawn()
        {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                let _ = stdin.write_all(text.as_bytes());
            }
            return child.wait().map(|s| s.success()).unwrap_or(false);
        }
        false
    }
    #[cfg(target_os = "macos")]
    {
        use std::process::{Command, Stdio};
        if let Ok(mut child) = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
        {
            use std::io::Write;
            if let Some(ref mut stdin) = child.stdin {
                let _ = stdin.write_all(text.as_bytes());
            }
            return child.wait().map(|s| s.success()).unwrap_or(false);
        }
        false
    }
    #[cfg(target_os = "linux")]
    {
        use std::process::{Command, Stdio};
        // Try xclip first, then xsel
        for cmd in &["xclip", "xsel"] {
            let args: Vec<&str> = if *cmd == "xclip" {
                vec!["-selection", "clipboard"]
            } else {
                vec!["--clipboard", "--input"]
            };
            if let Ok(mut child) = Command::new(cmd)
                .args(&args)
                .stdin(Stdio::piped())
                .spawn()
            {
                use std::io::Write;
                if let Some(ref mut stdin) = child.stdin {
                    let _ = stdin.write_all(text.as_bytes());
                }
                if child.wait().map(|s| s.success()).unwrap_or(false) {
                    return true;
                }
            }
        }
        false
    }
}
