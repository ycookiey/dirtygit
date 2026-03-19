mod detail;
mod repo_list;
mod setup;

use crate::app::{App, InputMode};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &mut App) {
    // Setup wizard (first run)
    if let Some(ref state) = app.setup_state {
        setup::draw(f, state);
        return;
    }

    // If there's a config error, show it full-screen
    if let Some(ref err) = app.config_error {
        draw_config_error(f, err);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(0),   // Main area
            Constraint::Length(1), // Footer
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_main(f, app, chunks[1]);
    draw_footer(f, app, chunks[2]);

    if app.show_help {
        draw_help_overlay(f);
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let total_count = app.repos.len();
    let dirty_count = app.repos.iter().filter(|r| r.is_dirty).count();
    let stale_count = app.stale_count();

    let bg = Color::DarkGray;
    let mut spans: Vec<Span> = Vec::new();

    // App summary
    spans.push(Span::styled(
        format!(" {}/{} dirty", dirty_count, total_count),
        Style::default().fg(Color::White).bg(bg).add_modifier(Modifier::BOLD),
    ));

    // Scan status
    if app.scanning {
        let done = total_count - stale_count;
        let bar_width = 10usize;
        let filled = if total_count > 0 { done * bar_width / total_count } else { 0 };
        let empty = bar_width - filled;
        spans.push(Span::styled(
            format!(" [{}{}] {}/{}",
                "█".repeat(filled), "░".repeat(empty), done, total_count),
            Style::default().fg(Color::Yellow).bg(bg),
        ));
    } else if let Some(ref t) = app.last_scan_time {
        spans.push(Span::styled(
            format!(" │ {}", t),
            Style::default().fg(Color::DarkGray).bg(bg),
        ));
    }

    // Sort badge
    let sort_label = match app.sort_mode {
        crate::app::SortMode::DirtyFirst => "dirty↑",
        crate::app::SortMode::Name => "name",
        crate::app::SortMode::LastCommit => "time",
    };
    spans.push(Span::styled(
        format!("  sort:{}", sort_label),
        Style::default().fg(Color::Cyan).bg(bg),
    ));

    // Filter badge
    if app.dirty_filter {
        spans.push(Span::styled(
            " [dirty only]",
            Style::default().fg(Color::Red).bg(bg).add_modifier(Modifier::BOLD),
        ));
    }

    // Showing count (if filtered)
    let shown = app.filtered_indices.len();
    if shown != total_count {
        spans.push(Span::styled(
            format!("  showing {}", shown),
            Style::default().fg(Color::White).bg(bg),
        ));
    }

    let header = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(bg));

    f.render_widget(header, area);
}

fn draw_main(f: &mut Frame, app: &mut App, area: Rect) {
    // Responsive: narrow terminals get more space for list
    let (left_pct, right_pct) = if area.width < 100 {
        (55, 45)
    } else {
        (45, 55)
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(left_pct),
            Constraint::Percentage(right_pct),
        ])
        .split(area);

    repo_list::draw(f, app, chunks[0]);
    detail::draw(f, app, chunks[1]);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    // Show flash message if active (3 second duration)
    if let Some((ref msg, instant)) = app.flash_message {
        if instant.elapsed().as_secs() < 3 {
            let flash = Paragraph::new(Line::from(Span::styled(
                format!(" {}", msg),
                Style::default().fg(Color::Red).bg(Color::DarkGray).add_modifier(Modifier::BOLD),
            )))
            .style(Style::default().bg(Color::DarkGray));
            f.render_widget(flash, area);
            return;
        }
    }

    let spans: Vec<Span> = match app.input_mode {
        InputMode::Search => {
            let match_count = app.filtered_indices.len();
            vec![
                Span::styled(" /", Style::default().fg(Color::Yellow).bg(Color::DarkGray)),
                Span::styled(
                    format!("{}█", app.search_query),
                    Style::default().fg(Color::White).bg(Color::DarkGray),
                ),
                Span::styled(
                    format!(" ({} matches)", match_count),
                    Style::default().fg(Color::DarkGray).bg(Color::DarkGray),
                ),
            ]
        }
        InputMode::Normal => {
            let mut parts: Vec<Span> = Vec::new();

            // Show active search filter
            if !app.search_query.is_empty() {
                parts.push(Span::styled(
                    format!(" /{}  ", app.search_query),
                    Style::default().fg(Color::Yellow).bg(Color::DarkGray),
                ));
            }

            let sort_label = match app.sort_mode {
                crate::app::SortMode::DirtyFirst => "dirty↑",
                crate::app::SortMode::Name => "name",
                crate::app::SortMode::LastCommit => "time",
            };

            let mut status = format!(" s:sort({}) f:filter", sort_label);
            if app.dirty_filter {
                status.push_str("(on)");
            }
            status.push_str("  ?:help  q:quit");

            parts.push(Span::styled(
                status,
                Style::default().fg(Color::White).bg(Color::DarkGray),
            ));
            parts
        }
    };

    let footer = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::DarkGray));

    f.render_widget(footer, area);
}

fn draw_help_overlay(f: &mut Frame) {
    let area = f.area();
    let width = 50u16.min(area.width.saturating_sub(4));
    let height = 20u16.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup = Rect::new(x, y, width, height);

    f.render_widget(Clear, popup);

    let help_lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("  j/k ↑/↓  ", Style::default().fg(Color::Cyan)),
            Span::raw("Move up/down"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+d/u ", Style::default().fg(Color::Cyan)),
            Span::raw("Half page down/up"),
        ]),
        Line::from(vec![
            Span::styled("  Ctrl+f/b ", Style::default().fg(Color::Cyan)),
            Span::raw("Full page down/up"),
        ]),
        Line::from(vec![
            Span::styled("  g / G    ", Style::default().fg(Color::Cyan)),
            Span::raw("Go to top / bottom"),
        ]),
        Line::from(vec![
            Span::styled("  Tab      ", Style::default().fg(Color::Cyan)),
            Span::raw("Switch pane"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  /        ", Style::default().fg(Color::Cyan)),
            Span::raw("Search repositories"),
        ]),
        Line::from(vec![
            Span::styled("  s        ", Style::default().fg(Color::Cyan)),
            Span::raw("Cycle sort mode"),
        ]),
        Line::from(vec![
            Span::styled("  f        ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle dirty filter"),
        ]),
        Line::from(vec![
            Span::styled("  r        ", Style::default().fg(Color::Cyan)),
            Span::raw("Refresh scan"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Enter    ", Style::default().fg(Color::Cyan)),
            Span::raw("Open lazygit"),
        ]),
        Line::from(vec![
            Span::styled("  c        ", Style::default().fg(Color::Cyan)),
            Span::raw("Copy repo path"),
        ]),
        Line::from(vec![
            Span::styled("  q        ", Style::default().fg(Color::Cyan)),
            Span::raw("Quit"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "  Press ? or Esc to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Block::default()
        .title(" Help ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(help_lines).block(block);
    f.render_widget(paragraph, popup);
}

fn draw_config_error(f: &mut Frame, error: &str) {
    let block = Block::default()
        .title(" dirtygit - Configuration Required ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let lines: Vec<Line> = error
        .lines()
        .map(|l| Line::from(Span::styled(l.to_string(), Style::default().fg(Color::White))))
        .collect();

    let mut all_lines = vec![Line::from("")];
    all_lines.extend(lines);
    all_lines.push(Line::from(""));
    all_lines.push(Line::from(Span::styled(
        "Press q to quit",
        Style::default()
            .fg(Color::Gray)
            .add_modifier(Modifier::ITALIC),
    )));

    let paragraph = Paragraph::new(all_lines).block(block);

    f.render_widget(paragraph, f.area());
}
