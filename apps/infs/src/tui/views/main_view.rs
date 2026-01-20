#![warn(clippy::pedantic)]

//! Main view rendering for the TUI.
//!
//! This module contains the rendering logic for the main menu screen,
//! including the logo, menu items, command input, and status line.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::tui::menu::{Menu, MENU_ITEMS};
use crate::tui::theme::Theme;

/// Renders the main view.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    menu: &Menu,
    command_input: &str,
    is_command_mode: bool,
    status_message: &str,
) {
    let chunks = Layout::vertical([
        Constraint::Length(8), // Logo and version
        Constraint::Min(6),    // Menu
        Constraint::Length(3), // Input line
        Constraint::Length(1), // Status
    ])
    .split(area);

    render_header(frame, chunks[0], theme);
    render_menu(frame, chunks[1], theme, menu);
    render_input(frame, chunks[2], theme, command_input, is_command_mode);
    render_status(frame, chunks[3], theme, status_message);
}

/// Renders the header with logo and version.
fn render_header(frame: &mut Frame, area: Rect, theme: &Theme) {
    let logo = r"
  _____       __
 |_   _|     / _| ___ _ __ ___ _ __   ___ ___
   | | _ __ | |_ / _ \ '__/ _ \ '_ \ / __/ _ \
   | || '_ \|  _|  __/ | |  __/ | | | (_|  __/
  _|_||_| |_||_|  \___|_|  \___|_| |_|\___\___|
";
    let version = format!("v{}", env!("CARGO_PKG_VERSION"));
    let cwd = std::env::current_dir()
        .map_or_else(|_| String::from("<unknown>"), |p| p.display().to_string());

    let header_text = vec![
        Line::from(logo.trim_start_matches('\n')),
        Line::from(""),
        Line::from(vec![
            Span::styled("Version: ", Style::default().fg(theme.muted)),
            Span::raw(&version),
            Span::raw("  "),
            Span::styled("Directory: ", Style::default().fg(theme.muted)),
            Span::raw(&cwd),
        ]),
    ];

    let header = Paragraph::new(header_text)
        .alignment(Alignment::Left)
        .block(Block::default().borders(Borders::NONE));

    frame.render_widget(header, area);
}

/// Renders the menu with navigation indicators.
fn render_menu(frame: &mut Frame, area: Rect, theme: &Theme, menu: &Menu) {
    let mut lines = Vec::with_capacity(MENU_ITEMS.len() + 2);

    for (idx, item) in MENU_ITEMS.iter().enumerate() {
        let is_selected = idx == menu.selected();

        let prefix = if is_selected { "> " } else { "  " };
        let key_style = if is_selected {
            Style::default()
                .fg(theme.selected)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(theme.highlight)
                .add_modifier(Modifier::BOLD)
        };
        let label_style = if is_selected {
            Style::default()
                .fg(theme.text)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text)
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, label_style),
            Span::styled(format!("[{}] ", item.key), key_style),
            Span::styled(item.label, label_style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(
            "Use arrows or keys to navigate, Enter to select, : for commands",
            Style::default().fg(theme.muted),
        ),
    ]));

    let menu_widget = Paragraph::new(lines).block(
        Block::default()
            .title(" Menu ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border)),
    );

    frame.render_widget(menu_widget, area);
}

/// Renders the command input line.
fn render_input(
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    command_input: &str,
    is_command_mode: bool,
) {
    let (input_text, cursor_style) = if is_command_mode {
        (
            format!(":{command_input}"),
            Style::default().fg(theme.text),
        )
    } else {
        (
            String::from("Press ':' to enter command mode"),
            Style::default().fg(theme.muted),
        )
    };

    let input = Paragraph::new(input_text)
        .style(cursor_style)
        .block(
            Block::default()
                .title(" Input ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border)),
        );

    frame.render_widget(input, area);

    if is_command_mode {
        #[allow(clippy::cast_possible_truncation)]
        let cursor_x = area.x + 1 + command_input.len() as u16 + 1;
        let cursor_y = area.y + 1;
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}

/// Renders the status message line.
fn render_status(frame: &mut Frame, area: Rect, theme: &Theme, status_message: &str) {
    let status = Paragraph::new(status_message).style(Style::default().fg(theme.muted));
    frame.render_widget(status, area);
}
