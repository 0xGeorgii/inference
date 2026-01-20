#![warn(clippy::pedantic)]

//! Toolchain view rendering for the TUI.
//!
//! This module contains the rendering logic for the installed toolchains screen,
//! showing a list of toolchain versions with their installation details.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::tui::state::ToolchainsState;
use crate::tui::theme::Theme;

/// Renders the toolchains view.
pub fn render(frame: &mut Frame, area: Rect, theme: &Theme, state: &ToolchainsState) {
    let chunks = Layout::vertical([
        Constraint::Min(6),    // Toolchain list
        Constraint::Length(3), // Help text
    ])
    .split(area);

    render_toolchain_list(frame, chunks[0], theme, state);
    render_help(frame, chunks[1], theme);
}

/// Renders the toolchain list.
fn render_toolchain_list(frame: &mut Frame, area: Rect, theme: &Theme, state: &ToolchainsState) {
    let mut lines = Vec::new();

    if state.toolchains.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "  No toolchains installed.",
            Style::default().fg(theme.muted),
        )]));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            "  Run 'infs install' to install the Inference toolchain.",
            Style::default().fg(theme.muted),
        )]));
    } else {
        for (idx, toolchain) in state.toolchains.iter().enumerate() {
            let is_selected = idx == state.selected;

            let prefix = if is_selected { "> " } else { "  " };

            let version_style = if is_selected {
                Style::default()
                    .fg(theme.selected)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text)
            };

            let default_indicator = if toolchain.is_default {
                Span::styled(" (default)", Style::default().fg(theme.success))
            } else {
                Span::raw("")
            };

            let installed_ago = toolchain
                .metadata
                .as_ref()
                .map_or_else(String::new, |m| format!(" - installed {}", m.installed_ago()));

            lines.push(Line::from(vec![
                Span::styled(prefix, version_style),
                Span::styled(&toolchain.version, version_style),
                default_indicator,
                Span::styled(installed_ago, Style::default().fg(theme.muted)),
            ]));
        }
    }

    let list_widget = Paragraph::new(lines).block(
        Block::default()
            .title(" Installed Toolchains ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border)),
    );

    frame.render_widget(list_widget, area);
}

/// Renders the help text at the bottom.
fn render_help(frame: &mut Frame, area: Rect, theme: &Theme) {
    let help_text = Line::from(vec![
        Span::styled("[Esc] ", Style::default().fg(theme.highlight)),
        Span::styled("Back", Style::default().fg(theme.muted)),
        Span::raw("  "),
        Span::styled("[Up/Down] ", Style::default().fg(theme.highlight)),
        Span::styled("Navigate", Style::default().fg(theme.muted)),
    ]);

    let help = Paragraph::new(help_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border)),
    );

    frame.render_widget(help, area);
}
