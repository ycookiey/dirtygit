use crate::setup::{SetupFocus, SetupState};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn draw(f: &mut Frame, state: &SetupState) {
    let area = f.area();

    let width = 60u16.min(area.width.saturating_sub(4));
    let height = 20u16.min(area.height.saturating_sub(2));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let panel = Rect::new(x, y, width, height);

    let block = Block::default()
        .title(" dirtygit - Setup ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    let inner = block.inner(panel);
    f.render_widget(block, panel);

    let warning_lines = if state.warning.is_some() { 1 } else { 0 };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),              // Title
            Constraint::Length(1),              // Input field
            Constraint::Length(warning_lines),  // Warning
            Constraint::Length(1),              // List label
            Constraint::Min(3),                // Directory list
            Constraint::Length(1),              // Footer
        ])
        .split(inner);

    draw_title(f, chunks[0]);
    draw_input(f, state, chunks[1]);
    if state.warning.is_some() {
        draw_warning(f, state, chunks[2]);
    }
    draw_list_label(f, state, chunks[3]);
    draw_dir_list(f, state, chunks[4]);
    draw_footer(f, state, chunks[5]);

    if state.completion.menu_open && !state.completion.candidates.is_empty() {
        draw_completion_menu(f, state, chunks[1]);
    }
}

fn draw_title(f: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            " Add directories containing git repos",
            Style::default().fg(Color::White),
        )),
    ];
    f.render_widget(Paragraph::new(lines), area);
}

fn draw_input(f: &mut Frame, state: &SetupState, area: Rect) {
    let focused = state.focus == SetupFocus::Input;
    let prompt_style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let line = if state.input.is_empty() && focused {
        Line::from(vec![
            Span::styled(" > ", prompt_style),
            Span::styled("Tab to autocomplete...", Style::default().fg(Color::DarkGray)),
        ])
    } else {
        let input_style = if focused {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        Line::from(vec![
            Span::styled(" > ", prompt_style),
            Span::styled(&state.input, input_style),
        ])
    };
    f.render_widget(Paragraph::new(line), area);

    if focused {
        let cursor_x = area.x + 3 + state.input.len() as u16;
        f.set_cursor_position((cursor_x, area.y));
    }
}

fn draw_warning(f: &mut Frame, state: &SetupState, area: Rect) {
    if let Some(ref msg) = state.warning {
        let line = Line::from(Span::styled(
            format!(" ! {}", msg),
            Style::default().fg(Color::Yellow),
        ));
        f.render_widget(Paragraph::new(line), area);
    }
}

fn draw_list_label(f: &mut Frame, state: &SetupState, area: Rect) {
    let text = if state.dirs.is_empty() {
        ""
    } else {
        " Directories:"
    };
    let line = Line::from(Span::styled(text, Style::default().fg(Color::DarkGray)));
    f.render_widget(Paragraph::new(line), area);
}

fn draw_dir_list(f: &mut Frame, state: &SetupState, area: Rect) {
    let lines: Vec<Line> = state
        .dirs
        .iter()
        .enumerate()
        .map(|(i, dir)| {
            let is_selected = state.focus == SetupFocus::List && i == state.selected_dir;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let marker = if is_selected { " > " } else { "   " };
            Line::from(Span::styled(format!("{}{}", marker, dir), style))
        })
        .collect();

    f.render_widget(Paragraph::new(lines), area);
}

fn draw_footer(f: &mut Frame, state: &SetupState, area: Rect) {
    let key = Style::default().fg(Color::Cyan);
    let dim = Style::default().fg(Color::DarkGray);

    // Enter label changes based on context
    let enter_label = if state.focus == SetupFocus::List
        || (state.focus == SetupFocus::Input && state.input.trim().is_empty() && !state.dirs.is_empty())
    {
        ":save & start "
    } else {
        ":add "
    };

    let mut spans = vec![Span::raw(" ")];

    if state.focus == SetupFocus::Input {
        spans.extend([
            Span::styled("Tab", key),
            Span::styled(":complete ", dim),
        ]);
    } else {
        spans.extend([
            Span::styled("j/k", key),
            Span::styled(":move ", dim),
            Span::styled("d", key),
            Span::styled(":delete ", dim),
            Span::styled("Tab", key),
            Span::styled(":input ", dim),
        ]);
    }

    spans.extend([
        Span::styled("Enter", key),
        Span::styled(enter_label, dim),
        Span::styled("Ctrl+C", key),
        Span::styled(":quit", dim),
    ]);

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn draw_completion_menu(f: &mut Frame, state: &SetupState, input_area: Rect) {
    let candidates = &state.completion.candidates;
    let max_visible = 8usize;
    let visible_count = candidates.len().min(max_visible);
    let menu_height = visible_count as u16 + 2;

    let menu_x = input_area.x + 3;
    let menu_y = input_area.y + 1;
    let menu_width = input_area.width.saturating_sub(3).min(40);

    let menu_area = Rect::new(menu_x, menu_y, menu_width, menu_height);

    f.render_widget(Clear, menu_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));
    let inner = block.inner(menu_area);
    f.render_widget(block, menu_area);

    let sep = std::path::MAIN_SEPARATOR;
    let lines: Vec<Line> = candidates
        .iter()
        .enumerate()
        .take(max_visible)
        .map(|(i, name)| {
            let is_selected = i == state.completion.selected;
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Cyan)
            } else {
                Style::default().fg(Color::White)
            };
            Line::from(Span::styled(format!(" {}{}", name, sep), style))
        })
        .collect();

    f.render_widget(Paragraph::new(lines), inner);
}
