#![warn(clippy::pedantic)]

//! Styled logo rendering widget.
//!
//! This module provides a reusable logo widget with theme-aware styling
//! for the infs TUI application.

use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Text},
    widgets::Paragraph,
};

use crate::tui::theme::Theme;

/// The ASCII art logo for the Inference toolchain.
pub const LOGO_ASCII: &str = r"
  _____       __
 |_   _|     / _| ___ _ __ ___ _ __   ___ ___
   | | _ __ | |_ / _ \ '__/ _ \ '_ \ / __/ _ \
   | || '_ \|  _|  __/ | |  __/ | | | (_|  __/
  _|_||_| |_||_|  \___|_|  \___|_| |_|\___\___|";

/// Returns the logo as styled text lines.
#[must_use]
pub fn logo_lines(theme: &Theme) -> Vec<Line<'static>> {
    LOGO_ASCII
        .lines()
        .skip(1) // Skip the first empty line
        .map(|line| Line::styled(line.to_string(), Style::default().fg(theme.highlight)))
        .collect()
}

/// Returns the logo as a Text widget.
#[must_use]
pub fn logo_text(theme: &Theme) -> Text<'static> {
    Text::from(logo_lines(theme))
}

/// Returns the logo as a Paragraph widget.
#[must_use]
#[allow(dead_code)]
pub fn logo_widget(theme: &Theme) -> Paragraph<'static> {
    Paragraph::new(logo_text(theme))
}

/// Renders the logo at the specified area.
#[allow(dead_code)]
pub fn render_logo(frame: &mut Frame, area: Rect, theme: &Theme) {
    frame.render_widget(logo_widget(theme), area);
}

/// Returns the height of the logo in lines.
#[must_use]
#[allow(dead_code)]
pub const fn logo_height() -> u16 {
    5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logo_lines_returns_correct_count() {
        let theme = Theme::dark();
        let lines = logo_lines(&theme);
        assert_eq!(lines.len(), 5);
    }

    #[test]
    fn logo_ascii_is_not_empty() {
        assert!(!LOGO_ASCII.is_empty());
    }

    #[test]
    fn logo_height_matches_lines() {
        let theme = Theme::dark();
        let lines = logo_lines(&theme);
        #[allow(clippy::cast_possible_truncation)]
        let lines_len = lines.len() as u16;
        assert_eq!(logo_height(), lines_len);
    }

    #[test]
    fn logo_text_has_correct_line_count() {
        let theme = Theme::dark();
        let text = logo_text(&theme);
        assert_eq!(text.lines.len(), 5);
    }

    #[test]
    fn logo_widget_can_be_created() {
        let theme = Theme::dark();
        let widget = logo_widget(&theme);
        // Verify the widget implements Widget trait by using it
        let _ = widget;
    }
}
