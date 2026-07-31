//! Minimal terminal styling: color + box-drawing for the CLI's output.
//!
//! Deliberately hand-rolled instead of pulling in a crate (comfy-table,
//! owo-colors, ...) — the whole surface needed here is "wrap some text in
//! an ANSI SGR code" and "draw a rectangle of box-drawing characters
//! around some lines", which is a couple dozen lines either way, and this
//! project already has a stated preference for a small, auditable
//! dependency tree (see deny.toml / the dependency-audit CI job).
//!
//! Colors and box-drawing are skipped automatically when stdout isn't a
//! terminal (piped into another program, redirected to a file) or when
//! `NO_COLOR` is set, per <https://no-color.org/> — `gai`'s output is
//! also meant to be greppable/scriptable, and nothing here should get in
//! the way of that.

use std::io::IsTerminal;
use std::net::IpAddr;

#[derive(Clone, Copy)]
pub struct Style {
    color: bool,
}

impl Style {
    pub fn detect() -> Self {
        let no_color = std::env::var_os("NO_COLOR").is_some();
        Self {
            color: !no_color && std::io::stdout().is_terminal(),
        }
    }

    fn paint(&self, code: &str, text: &str) -> String {
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }

    pub fn green(&self, text: &str) -> String {
        self.paint("32", text)
    }
    pub fn red(&self, text: &str) -> String {
        self.paint("31", text)
    }
    pub fn dim(&self, text: &str) -> String {
        self.paint("2", text)
    }

    /// `code` is a raw SGR color code ("32" green, "33" yellow, "31" red,
    /// ...) — used where the caller picks the color dynamically (e.g. a
    /// diagnosis panel whose severity isn't known until it's computed).
    pub fn accent(&self, code: &str, text: &str) -> String {
        self.paint(code, text)
    }
}

pub const GREEN: &str = "32";
pub const YELLOW: &str = "33";
pub const RED: &str = "31";

/// Comma-joined address list, the one repeated bit of formatting every
/// command needs.
pub fn format_addrs(addrs: &[IpAddr]) -> String {
    if addrs.is_empty() {
        "(none)".to_string()
    } else {
        addrs
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

const PANEL_WIDTH: usize = 74;

/// A titled, color-accented box around one or more lines of text,
/// word-wrapped to fit. `accent` is one of the SGR codes above (or any
/// raw code) and both colors the border and hints severity — green for
/// "no issue", yellow for "informational", red for "worth a look".
pub fn panel(style: &Style, title: &str, body: &[String], accent: &str) {
    let top = format!("┌─ {title} ");
    let dashes = PANEL_WIDTH.saturating_sub(top.chars().count() + 1);
    println!(
        "  {}",
        style.accent(accent, &format!("{top}{}┐", "─".repeat(dashes)))
    );
    for line in body {
        for wrapped in wrap(line, PANEL_WIDTH.saturating_sub(4)) {
            let pad = PANEL_WIDTH
                .saturating_sub(2)
                .saturating_sub(wrapped.chars().count());
            println!(
                "  {} {}{} {}",
                style.accent(accent, "│"),
                wrapped,
                " ".repeat(pad),
                style.accent(accent, "│")
            );
        }
    }
    println!(
        "  {}",
        style.accent(accent, &format!("└{}┘", "─".repeat(PANEL_WIDTH)))
    );
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let extra = if line.is_empty() { 0 } else { 1 };
        if !line.is_empty() && line.chars().count() + extra + word.chars().count() > width {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        out.push(line);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}
