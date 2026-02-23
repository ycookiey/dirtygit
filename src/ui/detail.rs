use crate::app::{ActivePane, App};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_pane == ActivePane::Detail;
    let border_color = if is_active {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let repo = match app.selected_repo() {
        Some(r) => r,
        None => {
            let block = Block::default()
                .title(" Detail ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color));
            let empty = Paragraph::new("No repository selected").block(block);
            f.render_widget(empty, area);
            return;
        }
    };

    let mut lines: Vec<Line> = Vec::new();

    // Repo name and path
    lines.push(Line::from(vec![
        Span::styled("  Repo: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(&repo.name, Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Path: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            repo.path.display().to_string(),
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Branch: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(&repo.branch, Style::default().fg(Color::Magenta)),
        if repo.ahead > 0 || repo.behind > 0 {
            Span::styled(
                format!("  ↑{} ↓{}", repo.ahead, repo.behind),
                Style::default().fg(Color::Yellow),
            )
        } else {
            Span::styled("  (up to date)", Style::default().fg(Color::Green))
        },
    ]));
    lines.push(Line::from(vec![
        Span::styled("Commit: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(&repo.last_commit_message, Style::default().fg(Color::White)),
        Span::styled(
            format!("  ({})", repo.last_commit_time),
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    lines.push(Line::from(""));

    // Staged changes
    if !repo.staged.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("── Staged ({}) ──", repo.staged.len()),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));
        for change in &repo.staged {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", change.status),
                    Style::default().fg(Color::Green),
                ),
                Span::raw(&change.path),
            ]));
        }
        lines.push(Line::from(""));
    }

    // Unstaged changes
    if !repo.unstaged.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("── Unstaged ({}) ──", repo.unstaged.len()),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )));
        for change in &repo.unstaged {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {} ", change.status),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(&change.path),
            ]));
        }
        lines.push(Line::from(""));
    }

    // Untracked files
    if !repo.untracked.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("── Untracked ({}) ──", repo.untracked.len()),
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )));
        for file in &repo.untracked {
            lines.push(Line::from(vec![
                Span::styled("  ? ", Style::default().fg(Color::Red)),
                Span::raw(file),
            ]));
        }
        lines.push(Line::from(""));
    }

    if !repo.is_dirty {
        lines.push(Line::from(Span::styled(
            "  ✓ Working tree clean",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));
    }

    let total_lines = lines.len();
    // Inner height excludes borders (2 lines)
    let inner_height = area.height.saturating_sub(2) as usize;

    let scroll_info = if total_lines > inner_height {
        format!(
            " [{}/{}] ",
            app.detail_scroll + 1,
            total_lines.saturating_sub(inner_height) + 1
        )
    } else {
        String::new()
    };

    let title = format!(" Detail {}", scroll_info);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((app.detail_scroll as u16, 0));

    f.render_widget(paragraph, area);
}
