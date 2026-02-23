mod detail;
mod repo_list;

use crate::app::{App, InputMode};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &App) {
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
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let repo_count = app.filtered_indices.len();
    let total_count = app.repos.len();
    let dirty_count = app.repos.iter().filter(|r| r.is_dirty).count();

    let scan_status = if app.scanning {
        " [scanning...]".to_string()
    } else if let Some(ref t) = app.last_scan_time {
        format!(" [last scan: {}]", t)
    } else {
        String::new()
    };

    let header_text = format!(
        " git-all-status │ {} repos ({} dirty) │ showing {}{}",
        total_count, dirty_count, repo_count, scan_status
    );

    let header = Paragraph::new(Line::from(vec![Span::styled(
        header_text,
        Style::default()
            .fg(Color::White)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    )]))
    .style(Style::default().bg(Color::DarkGray));

    f.render_widget(header, area);
}

fn draw_main(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45), // Left pane
            Constraint::Percentage(55), // Right pane
        ])
        .split(area);

    repo_list::draw(f, app, chunks[0]);
    detail::draw(f, app, chunks[1]);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let footer_text = match app.input_mode {
        InputMode::Search => {
            format!(" /{}█", app.search_query)
        }
        InputMode::Normal => {
            let sort_label = match app.sort_mode {
                crate::app::SortMode::DirtyFirst => "dirty↑",
                crate::app::SortMode::Name => "name",
                crate::app::SortMode::LastCommit => "time",
            };
            let filter_label = if app.dirty_filter { " [dirty only]" } else { "" };
            format!(
                " j/k:move  Tab:pane  s:sort({})  f:filter{}  /:search  r:refresh  Enter:lazygit  q:quit",
                sort_label, filter_label
            )
        }
    };

    let footer = Paragraph::new(Line::from(vec![Span::styled(
        footer_text,
        Style::default().fg(Color::White).bg(Color::DarkGray),
    )]))
    .style(Style::default().bg(Color::DarkGray));

    f.render_widget(footer, area);
}

fn draw_config_error(f: &mut Frame, error: &str) {
    let block = Block::default()
        .title(" git-all-status - Configuration Required ")
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
