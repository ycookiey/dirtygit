use crate::app::{ActivePane, App};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};
use ratatui::Frame;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let is_active = app.active_pane == ActivePane::RepoList;
    let border_color = if is_active {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let block = Block::default()
        .title(" Repositories ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let header_cells = ["", "Name", "Branch", "Changes", "↑↓", "Last Commit"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        });
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = app
        .filtered_indices
        .iter()
        .map(|&i| {
            let repo = &app.repos[i];

            let dirty_indicator = if repo.is_dirty { "●" } else { "○" };
            let dirty_color = if repo.is_dirty {
                Color::Red
            } else {
                Color::Green
            };

            let changes = if repo.is_dirty {
                format!(
                    "{} (+{} ~{} ?{})",
                    repo.total_changes(),
                    repo.staged.len(),
                    repo.unstaged.len(),
                    repo.untracked.len()
                )
            } else {
                "clean".to_string()
            };

            let sync_status = if repo.ahead > 0 || repo.behind > 0 {
                format!("↑{}↓{}", repo.ahead, repo.behind)
            } else {
                "=".to_string()
            };

            Row::new(vec![
                Cell::from(dirty_indicator).style(Style::default().fg(dirty_color)),
                Cell::from(repo.name.clone()),
                Cell::from(repo.branch.clone()).style(Style::default().fg(Color::Magenta)),
                Cell::from(changes),
                Cell::from(sync_status),
                Cell::from(repo.last_commit_time.clone())
                    .style(Style::default().fg(Color::DarkGray)),
            ])
        })
        .collect();

    let widths = [
        ratatui::layout::Constraint::Length(2),
        ratatui::layout::Constraint::Min(15),
        ratatui::layout::Constraint::Length(15),
        ratatui::layout::Constraint::Length(18),
        ratatui::layout::Constraint::Length(6),
        ratatui::layout::Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = TableState::default();
    state.select(Some(app.selected));

    f.render_stateful_widget(table, area, &mut state);
}
