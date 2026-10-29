use crate::app::{ActivePane, App};
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;

fn branch_color(branch: &str) -> Color {
    if branch == "main" || branch == "master" {
        Color::Green
    } else if branch.starts_with("hotfix") || branch.starts_with("release") {
        Color::Yellow
    } else if branch == "HEAD" || branch == "(detached)" {
        Color::Red
    } else {
        Color::Magenta
    }
}

pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    // borders(2) + header(1) = 3 lines of chrome
    app.list_height = area.height.saturating_sub(3) as usize;
    app.repo_list_area = (area.x, area.y, area.width, area.height);
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

    if app.filtered_indices.is_empty() {
        let msg = if app.repos.is_empty() {
            if app.scanning {
                "Scanning repositories..."
            } else {
                "No repositories found"
            }
        } else if app.dirty_filter {
            "All repositories are clean!"
        } else {
            "No repositories match search"
        };
        let empty = Paragraph::new(msg)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(block);
        f.render_widget(empty, area);
        return;
    }

    let inner_width = area.width.saturating_sub(2);
    let header_labels: Vec<&str> = if inner_width < 50 {
        vec!["", "Name", "Changes"]
    } else if inner_width < 70 {
        vec!["", "Name", "Branch", "Changes", "↑↓"]
    } else {
        vec!["", "Name", "Branch", "Changes", "↑↓", "Last Commit"]
    };
    let header_cells = header_labels.iter().map(|h| {
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

            let (dirty_indicator, dirty_color) = if repo.stale {
                ("◌", Color::DarkGray) // cached, not yet refreshed
            } else if repo.is_dirty {
                ("●", Color::Red)
            } else {
                ("○", Color::Green)
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

            let cells = vec![
                Cell::from(dirty_indicator).style(Style::default().fg(dirty_color)),
                Cell::from(repo.name.clone()),
                Cell::from(repo.branch.clone()).style(Style::default().fg(branch_color(&repo.branch))),
                Cell::from(changes),
                Cell::from(sync_status),
                Cell::from(if repo.stale {
                    format!("{} ⟳", repo.last_commit_time)
                } else {
                    repo.last_commit_time.clone()
                }).style(Style::default().fg(Color::DarkGray)),
            ];

            let row = Row::new(cells);
            if repo.stale {
                row.style(Style::default().fg(Color::DarkGray))
            } else {
                row
            }
        })
        .collect();

    use ratatui::layout::Constraint as C;
    let inner_width = area.width.saturating_sub(2); // borders
    let widths: Vec<C> = if inner_width < 50 {
        // Narrow: indicator, name, changes only
        vec![C::Length(2), C::Min(10), C::Length(14)]
    } else if inner_width < 70 {
        // Medium: hide last commit
        vec![C::Length(2), C::Min(12), C::Length(12), C::Length(14), C::Length(6)]
    } else {
        // Full
        vec![C::Length(2), C::Min(15), C::Length(15), C::Length(18), C::Length(6), C::Length(10)]
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .bg(Color::Indexed(237))
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = TableState::default();
    state.select(Some(app.selected));

    f.render_stateful_widget(table, area, &mut state);
}
