//! Shared terminal palette for help, results, and errors.
use clap::builder::styling::{AnsiColor, Style, Styles};

pub const HEADING: Style = AnsiColor::Cyan.on_default().bold();
pub const LABEL: Style = AnsiColor::Cyan.on_default();
pub const VALUE: Style = AnsiColor::Green.on_default().bold();
pub const MUTED: Style = Style::new().dimmed();
pub const ERROR: Style = AnsiColor::Red.on_default().bold();
pub const WARNING: Style = AnsiColor::Yellow.on_default();

pub fn help() -> Styles {
    Styles::styled()
        .header(HEADING)
        .usage(HEADING)
        .literal(LABEL.bold())
        .placeholder(MUTED)
        .valid(VALUE)
        .invalid(WARNING)
        .error(ERROR)
}
