use std::path::MAIN_SEPARATOR;

use derive_getters::Getters;
use rat_ftable::TableData;
use rat_widget::paragraph::Paragraph;
use ratatui::prelude::Line;
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::symbols::line::{ROUNDED_BOTTOM_LEFT, VERTICAL_RIGHT};
use ratatui::{text::Span, widgets::Widget};
use russh_sftp::protocol::{FileAttributes, FileType};

#[derive(Getters, Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub type_: FileType,
    pub attributes: FileAttributes,
}

impl From<russh_sftp::client::fs::DirEntry> for FileEntry {
    fn from(value: russh_sftp::client::fs::DirEntry) -> Self {
        let name = value.file_name();
        let type_ = value.file_type();
        let attributes = value.metadata();
        Self {
            name: name.to_string(),
            type_,
            attributes,
        }
    }
}

impl FileEntry {
    pub fn is_dir(&self) -> bool {
        self.type_ == FileType::Dir
    }

    pub fn is_file(&self) -> bool {
        self.type_ == FileType::File
    }
    pub fn is_symlink(&self) -> bool {
        self.type_ == FileType::Symlink
    }
}

impl From<FileEntry> for Paragraph<'_> {
    fn from(value: FileEntry) -> Self {
        let type_str = if value.is_dir() {
            "DIR"
        } else if value.is_file() {
            "FILE"
        } else if value.is_symlink() {
            "SYMLINK"
        } else {
            "UNKNOWN"
        };
        let size = value.attributes.size.unwrap_or_default();
        let size_string = human_readable_size(size);
        let perms = value.attributes.permissions();

        let perms_spans = perms
            .to_string()
            .chars()
            .map(|c| match c {
                'r' => Span::styled("r", ratatui::style::Color::Green).add_modifier(Modifier::DIM),
                'w' => Span::styled("w", ratatui::style::Color::Red).add_modifier(Modifier::DIM),
                'x' => Span::styled("x", ratatui::style::Color::Yellow).add_modifier(Modifier::DIM),
                's' => Span::styled("s", ratatui::style::Color::Blue).add_modifier(Modifier::DIM),
                't' => Span::styled("t", ratatui::style::Color::Blue).add_modifier(Modifier::DIM),
                _ => Span::styled(c.to_string(), ratatui::style::Color::DarkGray)
                    .add_modifier(Modifier::DIM),
            })
            .collect::<Vec<_>>();

        let mut temp_vec = vec![
            Span::styled(format!("{} ", type_str), ratatui::style::Color::Yellow)
                .style(Style::default().add_modifier(Modifier::DIM)),
            Span::styled(format!("'{}'", value.name), ratatui::style::Color::White)
                .add_modifier(Modifier::BOLD),
            Span::styled("  ", ratatui::style::Color::White),
        ];
        temp_vec.extend(perms_spans);
        let title = Line::from(temp_vec);
        let heading = Line::styled("Metadata", ratatui::style::Color::Yellow);
        let metadata_lines = [
            Line::from(vec![
                Span::styled("size: ", ratatui::style::Color::White),
                Span::styled(size_string, ratatui::style::Color::White),
            ]),
            Line::from(vec![
                Span::styled("modification time: ", ratatui::style::Color::White),
                match format_timestamp(value.attributes().mtime) {
                    Some(timestamp_string) => Span::from(timestamp_string),
                    None => Span::from("N/A"),
                },
            ]),
            Line::from(vec![
                Span::styled("owner: ", ratatui::style::Color::White),
                Span::styled(
                    value.attributes.clone().user.unwrap_or("N/A".to_string()),
                    ratatui::style::Color::White,
                ),
            ]),
        ];
        let mut vec = vec![title, Line::from(""), heading];
        vec.extend_from_slice(&metadata_lines);

        Paragraph::new(vec)
    }
}

pub struct FileDataSlice<'a>(pub &'a [FileEntry]);

impl FileEntry {
    pub fn from_file(name: String, type_: FileType, attributes: FileAttributes) -> Self {
        let name = name.to_string();

        Self {
            name,
            type_,
            attributes,
        }
    }
}

pub struct ProgressDataSlice<'a>(pub &'a [FileEntry]);

impl<'a> TableData<'a> for ProgressDataSlice<'a> {
    fn rows(&self) -> usize {
        self.0.len()
    }
    fn render_cell(
        &self,
        _ctx: &rat_ftable::TableContext,
        column: usize,
        row: usize,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        let entry = &self.0[row];
        match column {
            0 => {
                let name = entry.name();
                let span = Span::from(name);
                span.render(area, buf);
            }
            1 => {
                let size = entry.attributes.size.unwrap_or_default();
                let size_string = human_readable_size(size);
                let span = Span::from(size_string);
                span.render(area, buf);
            }
            _ => {}
        }
    }
}

impl<'a> TableData<'a> for FileDataSlice<'a> {
    fn rows(&self) -> usize {
        self.0.len()
    }
    fn render_cell(
        &self,
        _ctx: &rat_ftable::TableContext,
        column: usize,
        row: usize,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
    ) {
        let entry = &self.0[row];
        match column {
            0 => {
                let perms = entry.attributes.permissions();
                let perms_string = perms
                    .to_string()
                    .chars()
                    .map(|c| match c {
                        'r' => Span::styled("r", ratatui::style::Color::Green),
                        'w' => Span::styled("w", ratatui::style::Color::Red),
                        'x' => Span::styled("x", ratatui::style::Color::Yellow),
                        's' => Span::styled("s", ratatui::style::Color::Blue),
                        't' => Span::styled("t", ratatui::style::Color::Blue),
                        _ => Span::styled(c.to_string(), ratatui::style::Color::DarkGray),
                    })
                    .collect::<Vec<_>>();
                let line = Line::from(perms_string);
                line.render(area, buf);
            }
            1 => {
                let vertical_right = if self.rows() - 1 != row {
                    VERTICAL_RIGHT.to_string()
                } else {
                    ROUNDED_BOTTOM_LEFT.to_string()
                };
                let vertical_line_symbol = if _ctx.selected_row {
                    vertical_right + "> "
                } else {
                    vertical_right + "  "
                };
                let span_prefix = match entry.type_() {
                    FileType::Dir => "ð ",
                    FileType::File => "ƒ ",
                    FileType::Symlink => "§ ",
                    _ => "├ █ ",
                };
                let mut span =
                    Span::from(vertical_line_symbol.clone() + span_prefix + " " + &entry.name);

                if _ctx.selected_row {
                    span = Span::from(format!(
                        "{}{}[{}]",
                        vertical_line_symbol, span_prefix, &entry.name
                    ))
                    .style(Style::default().add_modifier(Modifier::BOLD));
                }
                span.render(area, buf);
            }
            2 => {
                if !entry.is_dir() {
                    let size = entry.attributes.size.unwrap_or_default();
                    let size_string = human_readable_size(size);
                    let span = Span::from(size_string);
                    span.render(area, buf);
                }
            }
            3 => match format_timestamp(entry.attributes().mtime) {
                Some(timestamp_string) => {
                    Span::from(timestamp_string).render(area, buf);
                }
                None => {
                    Span::from("N/A").render(area, buf);
                }
            },
            _ => {}
        }
    }
}

fn human_readable_size(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KB", "MB", "GB", "TB", "PB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let exp = (bytes as f64).log10() / 1024f64.log10();
    let idx = exp.floor() as usize;

    let idx = idx.min(UNITS.len() - 1);
    let size = bytes as f64 / 1024f64.powi(idx as i32);

    if size < 10.0 {
        format!("{:.2} {}", size, UNITS[idx])
    } else if size < 100.0 {
        format!("{:.1} {}", size, UNITS[idx])
    } else {
        format!("{:.0} {}", size, UNITS[idx])
    }
}

fn format_timestamp(timestamp: Option<u32>) -> Option<String> {
    let timestamp = timestamp?;
    let datetime = chrono::DateTime::from_timestamp(timestamp.into(), 0)?;
    let fmt_datetime = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
    Some(fmt_datetime)
}

pub trait JoinablePaths {
    fn join(&self, other: &str) -> String;
}

impl JoinablePaths for String {
    fn join(&self, child: &str) -> String {
        let base = self;

        // Normalize the separator for this platform
        let sep = MAIN_SEPARATOR;

        // Handle empty base or child cases
        if base.is_empty() {
            return child.to_string();
        }
        if child.is_empty() {
            return base.to_string();
        }

        // If the child is absolute, return it as-is
        if child.starts_with(sep) || child.starts_with('/') || child.starts_with('\\') {
            return child.to_string();
        }

        // Trim trailing separator from base, and leading separator from child
        let mut result = base.trim_end_matches(['/', '\\']).to_string();
        result.push(sep);
        result.push_str(child.trim_start_matches(['/', '\\']));
        result
    }
}
