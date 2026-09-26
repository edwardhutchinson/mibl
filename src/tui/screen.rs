//! Ratatui rendering, with navigation state independent of a real terminal.
use super::{App, Filter, InputKind, View, document::DocumentKind};
use mibl::model::*;
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Paragraph, Tabs},
};

impl App<'_> {
    pub(crate) fn draw(&mut self, frame: &mut Frame) {
        let [tabs, heading, body, help] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .areas(frame.area());
        let document_kind = self.document.as_ref().map(|document| &document.kind);
        let scope = match document_kind {
            Some(DocumentKind::Definition(scope)) => Some(*scope),
            Some(DocumentKind::Tables | DocumentKind::Help) => None,
            None if self.view == View::Pus
                || matches!(self.filter, Filter::Pus { .. } | Filter::PusGroup(_)) =>
            {
                None
            }
            None => Some(self.scope),
        };
        let selected = match (document_kind, scope) {
            (Some(DocumentKind::Tables), _) => Some(3),
            (None, _) if self.view == View::Pus || matches!(self.filter, Filter::PusGroup(_)) => {
                Some(4)
            }
            (_, Some(SearchScope::Packets)) => Some(0),
            (_, Some(SearchScope::Parameters)) => Some(1),
            (_, Some(SearchScope::Commands)) => Some(2),
            _ => None,
        };
        frame.render_widget(
            Tabs::new([
                "P Packets",
                "p Parameters",
                "c Commands",
                "t Tables",
                "u PUS",
            ])
            .select(selected)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            tabs,
        );
        let title = match &self.filter {
            Filter::Inventory => format!("{:?}", self.scope),
            Filter::PusGroup(coordinates) => format!("PUS {coordinates}"),
            Filter::Search(query) => format!("Search {:?}: {query}", self.scope),
            Filter::Pus { service, subtype } => format!(
                "PUS {service}{}",
                subtype.map(|s| format!(",{s}")).unwrap_or_default()
            ),
        };
        let status = match document_kind {
            Some(DocumentKind::Tables) => "Supported-table load reports".into(),
            Some(DocumentKind::Help) => "Keyboard help".into(),
            None if self.view == View::Pus => {
                "PUS services and subtypes | Enter browse definitions".into()
            }
            _ => format!("mibl | {title} | {} definitions", self.candidates.len()),
        };
        let status =
            if let Some(section) = self.document.as_ref().and_then(|doc| doc.current_section()) {
                format!("{status} | {section}  [/] sections")
            } else {
                status
            };
        frame.render_widget(Paragraph::new(status), heading);
        let block = Block::bordered();
        let inner = block.inner(body);
        self.page_height = usize::from(inner.height).max(1);
        frame.render_widget(block, body);
        if let Some(document) = &mut self.document {
            document.draw(frame, inner);
        } else if self.view == View::Pus {
            self.page_height = usize::from(inner.height.saturating_sub(2)).max(1);
            self.pus.draw(frame, inner);
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
            self.page_height = usize::from(inner.height.saturating_sub(2)).max(1);
            self.draw_candidates(frame, inner);
        }
        if let Some(input) = &self.input {
            let prompt = match input.kind {
                InputKind::Search(scope) => format!("Search {scope:?}"),
                InputKind::Pus => "PUS SERVICE[,SUBTYPE]".into(),
            };
            frame.render_widget(Paragraph::new(format!("{prompt}: {}\nEnter apply  Esc cancel  Backspace delete  Tab search scope  Ctrl-C quit\n{}", input.value, input.error.unwrap_or(""))), help);
            return;
        }
        frame.render_widget(Paragraph::new("? help  P packets  p parameters  c/C commands  Tab scope  / search  f filter  t tables  u PUS\nEnter open  Esc back  ↑/↓ j/k move  PgUp/PgDn page  Home/End  [/] sections\n←/→ h/l columns / wide text  q / Ctrl-C quit"), help);
    }
}
