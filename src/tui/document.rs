//! Full definition text built directly from typed library results.
use crate::render;
use crossterm::event::KeyCode;
use mibl::{Mib, model::*};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::Paragraph,
};
use std::io::{self, Write};

pub(super) enum DocumentKind {
    Definition(SearchScope),
    Tables,
    Help,
}

enum LineKind {
    Plain,
    Content,
    Heading,
    Bottom,
}

struct DisplayLine {
    text: String,
    style: Style,
    kind: LineKind,
}

pub(super) struct Document {
    pub kind: DocumentKind,
    lines: Vec<DisplayLine>,
    sections: Vec<usize>,
    pub top: usize,
    pub left: usize,
}

impl Document {
    pub fn definition(mib: &Mib, identity: &Identity) -> io::Result<Self> {
        let mut out = render::Sections::default();
        match identity {
            Identity::Parameter(name) => {
                definition(mib.parameter(name), render::parameter_sections, &mut out)?
            }
            Identity::Packet(spid) => {
                definition(mib.packet(*spid), render::packet_sections, &mut out)?
            }
            Identity::Command(name) => {
                definition(mib.command(name), render::command_sections, &mut out)?
            }
        }
        let scope = match identity {
            Identity::Packet(_) => SearchScope::Packets,
            Identity::Parameter(_) => SearchScope::Parameters,
            Identity::Command(_) => SearchScope::Commands,
        };
        Ok(Self::from_sections(out, DocumentKind::Definition(scope)))
    }

    pub fn help() -> Self {
        Self::from_bytes(b"Keyboard controls\n\nP / 1: packets\np / 2: parameters\nc / C / 3: commands\nTab: next list or search scope\n/: search, Tab chooses scope\nf: PUS service[,subtype]\nt: supported-table load reports\nEnter: inspect selected identity\nEsc: cancel input / return to list\nUp/Down or j/k: select / scroll\nPageUp/PageDown: move one page\nHome/End: first / last page\n[ / ]: previous / next section\nLeft/Right or h/l: scroll wide text\n?: this help\nq: quit outside text input\nCtrl-C: quit from any screen\n".to_vec(), DocumentKind::Help)
    }

    pub fn tables(mib: &Mib) -> io::Result<Self> {
        let mut out = Vec::new();
        render::tables(mib.tables().iter(), &mut out)?;
        Ok(Self::from_bytes(out, DocumentKind::Tables))
    }

    fn from_bytes(bytes: Vec<u8>, kind: DocumentKind) -> Self {
        Self {
            kind,
            sections: Vec::new(),
            lines: String::from_utf8(bytes)
                .expect("formatters write UTF-8")
                .lines()
                .map(|s| DisplayLine {
                    text: s.to_owned(),
                    style: Style::default(),
                    kind: LineKind::Plain,
                })
                .collect(),
            top: 0,
            left: 0,
        }
    }

    fn from_sections(sections: render::Sections, kind: DocumentKind) -> Self {
        let mut doc = Self {
            kind,
            lines: Vec::new(),
            sections: Vec::new(),
            top: 0,
            left: 0,
        };
        for section in sections.0 {
            doc.sections.push(doc.lines.len());
            let color = match section.role {
                render::Role::Problem | render::Role::Unavailable => Color::Yellow,
                render::Role::Reference => Color::Magenta,
                _ => Color::Cyan,
            };
            let style = Style::default().fg(color).add_modifier(Modifier::BOLD);
            doc.lines.push(DisplayLine {
                text: section.title.into(),
                style,
                kind: LineKind::Heading,
            });
            for line in String::from_utf8(section.content)
                .expect("formatters write UTF-8")
                .lines()
            {
                let style = match section.role {
                    render::Role::Problem | render::Role::Unavailable => {
                        Style::default().fg(Color::Yellow)
                    }
                    render::Role::Summary => Style::default().fg(Color::White),
                    _ => Style::default(),
                };
                doc.lines.push(DisplayLine {
                    text: line.into(),
                    style,
                    kind: LineKind::Content,
                });
            }
            doc.lines.push(DisplayLine {
                text: String::new(),
                style: Style::default().fg(color),
                kind: LineKind::Bottom,
            });
            doc.lines.push(DisplayLine {
                text: String::new(),
                style: Style::default(),
                kind: LineKind::Plain,
            });
        }
        doc
    }

    pub fn current_section(&self) -> Option<&str> {
        self.sections
            .iter()
            .rev()
            .find(|&&start| start <= self.top)
            .map(|&start| self.lines[start].text.as_str())
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        self.top = self.top.min(
            self.lines
                .len()
                .saturating_sub(usize::from(area.height).max(1)),
        );
        for (offset, line) in self
            .lines
            .iter()
            .skip(self.top)
            .take(usize::from(area.height))
            .enumerate()
        {
            let row = Rect::new(area.x, area.y + offset as u16, area.width, 1);
            let width = usize::from(area.width);
            match line.kind {
                LineKind::Heading => {
                    let border = format!(
                        "╭─ {} {}╮",
                        line.text,
                        "─".repeat(width.saturating_sub(line.text.len() + 5))
                    );
                    frame.render_widget(Paragraph::new(border).style(line.style), row);
                }
                LineKind::Bottom => frame.render_widget(
                    Paragraph::new(format!("╰{}╯", "─".repeat(width.saturating_sub(2))))
                        .style(line.style),
                    row,
                ),
                LineKind::Content if area.width >= 4 => {
                    frame.render_widget(
                        Paragraph::new("│").style(Style::default().fg(Color::DarkGray)),
                        Rect::new(row.x, row.y, 1, 1),
                    );
                    frame.render_widget(
                        Paragraph::new("│").style(Style::default().fg(Color::DarkGray)),
                        Rect::new(row.right() - 1, row.y, 1, 1),
                    );
                    let visible: String = line.text.chars().skip(self.left).collect();
                    frame.render_widget(
                        Paragraph::new(visible).style(line.style),
                        Rect::new(row.x + 2, row.y, row.width - 4, 1),
                    );
                }
                _ => frame.render_widget(
                    Paragraph::new(line.text.chars().skip(self.left).collect::<String>())
                        .style(line.style),
                    row,
                ),
            }
        }
    }

    pub fn navigate(&mut self, code: KeyCode, height: usize) {
        let last = self.lines.len().saturating_sub(height.max(1));
        self.top = super::vertical_position(code, self.top, last, height);
        match code {
            KeyCode::Char(']') => {
                if let Some(next) = self.sections.iter().find(|&&start| start > self.top) {
                    self.top = (*next).min(last);
                    self.left = 0;
                }
            }
            KeyCode::Char('[') => {
                if let Some(prev) = self.sections.iter().rev().find(|&&start| start < self.top) {
                    self.top = *prev;
                    self.left = 0;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => self.left = self.left.saturating_add(8),
            KeyCode::Left | KeyCode::Char('h') => self.left = self.left.saturating_sub(8),
            KeyCode::Home => self.left = 0,
            _ => {}
        }
    }
}

fn definition<T>(
    lookup: Lookup<T>,
    render: fn(&T, bool, &mut dyn render::Output) -> io::Result<()>,
    out: &mut render::Sections,
) -> io::Result<()> {
    match lookup {
        Lookup::Found(value) => render(&value, true, out),
        Lookup::NotFound(NotFoundReason::NoMatchingIdentity) => {
            render::Output::section(out, "Not found", render::Role::Unavailable)?;
            writeln!(out, "No matching identity.")
        }
        Lookup::NotFound(NotFoundReason::DefinitionsUnavailable) => {
            render::Output::section(out, "Unavailable", render::Role::Unavailable)?;
            writeln!(
                out,
                "Definitions unavailable: no usable roots of this kind."
            )
        }
        Lookup::Ambiguous(candidates) => {
            render::Output::section(out, "Ambiguity", render::Role::Problem)?;
            writeln!(out, "Ambiguous identity: no duplicate definition selected.")?;
            render::candidates(
                std::iter::once(candidates.first.as_ref())
                    .chain(std::iter::once(candidates.second.as_ref()))
                    .chain(candidates.rest.iter()),
                out,
            )
        }
    }
}
