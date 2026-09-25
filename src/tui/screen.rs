//! Ratatui rendering, with navigation state independent of a real terminal.
use super::{App, Filter, InputKind};
use mibl::model::*;
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Style},
    widgets::{Block, List, ListItem, Paragraph},
};

impl App<'_> {
    pub(crate) fn draw(&mut self, frame: &mut Frame) {
        let [heading, body, help] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .areas(frame.area());
        let title = match &self.filter {
            Filter::Inventory => format!("{:?}", self.scope),
            Filter::Search(query) => format!("Search {:?}: {query}", self.scope),
            Filter::Pus { service, subtype } => format!(
                "PUS {service}{}",
                subtype.map(|s| format!(",{s}")).unwrap_or_default()
            ),
        };
        frame.render_widget(
            Paragraph::new(format!(
                "mibl | {title} | {} definitions",
                self.candidates.len()
            )),
            heading,
        );
        let block = Block::bordered();
        let inner = block.inner(body);
        self.page_height = usize::from(inner.height).max(1);
        frame.render_widget(block, body);
        if let Some(document) = &mut self.document {
            document.top = document
                .top
                .min(document.lines.len().saturating_sub(self.page_height));
            let lines = document
                .lines
                .iter()
                .skip(document.top)
                .take(self.page_height)
                .map(|s| {
                    ratatui::text::Line::raw(s.chars().skip(document.left).collect::<String>())
                })
                .collect::<Vec<_>>();
            frame.render_widget(Paragraph::new(lines), inner);
        } else if self.candidates.is_empty() {
            frame.render_widget(
                Paragraph::new(if self.roots_available {
                    "No matching definitions."
                } else {
                    "Definitions unavailable: no usable roots of this kind."
                }),
                inner,
            );
        } else {
            let items = self.candidates.iter().map(|c| {
                let identity = match &c.identity {
                    Identity::Parameter(n) => format!("parameter {}", n.0),
                    Identity::Packet(n) => format!("packet {}", n.0),
                    Identity::Command(n) => format!("command {}", n.0),
                };
                ListItem::new(crate::render::text(&format!(
                    "{} | {} | {} | {}:{}",
                    identity,
                    c.name.value.as_deref().unwrap_or("unavailable"),
                    c.description.value.as_deref().unwrap_or("unavailable"),
                    c.source.file.display(),
                    c.source.line
                )))
            });
            frame.render_stateful_widget(
                List::new(items)
                    .highlight_symbol("> ")
                    .highlight_style(Style::default().fg(Color::Yellow)),
                inner,
                &mut self.selection,
            );
        }
        if let Some(input) = &self.input {
            let prompt = match input.kind {
                InputKind::Search(scope) => format!("Search {scope:?}"),
                InputKind::Pus => "PUS SERVICE[,SUBTYPE]".into(),
            };
            frame.render_widget(Paragraph::new(format!("{prompt}: {}\nEnter apply  Esc cancel  Backspace delete  Tab search scope  Ctrl-C quit\n{}", input.value, input.error.unwrap_or(""))), help);
            return;
        }
        frame.render_widget(Paragraph::new("? help  1 packets  2 parameters  3 commands  Tab scope  / search  p PUS  t tables\nEnter inspect  Esc list  ↑/↓ j/k move  PgUp/PgDn page  Home/End\n←/→ h/l scroll wide text  q / Ctrl-C quit"), help);
    }
}
