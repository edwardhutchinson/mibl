//! Ratatui rendering, with navigation state independent of a real terminal.
use super::{App, Filter, InputKind, View, document::DocumentKind};
use mibl::model::*;
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Paragraph, Tabs, Wrap},
};

impl App<'_> {
    pub(crate) fn draw(&mut self, frame: &mut Frame) {
        // Keep short terminals useful without spending extra rows on decoration.
        let compact = frame.area().height < 16;
        let [header, body, notice, help] = Layout::vertical([
            Constraint::Length(if compact { 2 } else { 4 }),
            Constraint::Min(1),
            Constraint::Length(if self.notice.is_some() { 2 } else { 0 }),
            Constraint::Length(if self.input.is_some() { 3 } else { 1 }),
        ])
        .areas(frame.area());
        let header_inner = if compact {
            header
        } else {
            let block = Block::bordered()
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Cyan))
                .title("─ mibl ")
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                );
            let inner = block.inner(header);
            frame.render_widget(block, header);
            inner
        };
        let [tabs, heading] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(header_inner);
        let document_kind = self.document.as_ref().map(|document| &document.kind);
        let scope = match document_kind {
            Some(DocumentKind::Definition(scope)) => Some(*scope),
            Some(DocumentKind::Help) => None,
            None if self.view != View::Definitions
                || matches!(self.filter, Filter::Pus { .. } | Filter::PusGroup(_)) =>
            {
                None
            }
            None => Some(self.scope),
        };
        let selected = match (document_kind, scope) {
            (None, _) if self.view == View::Tables => Some(4),
            (None, _) if self.view == View::Pus || matches!(self.filter, Filter::PusGroup(_)) => {
                Some(3)
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
                "u PUS",
                "t Tables",
            ])
            .select(selected)
            .divider("  ")
            .style(Style::default().fg(Color::Gray))
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
            None if self.view == View::Tables => {
                "Supported-table load reports | Enter edit in $EDITOR".into()
            }
            Some(DocumentKind::Help) => "Keyboard help".into(),
            None if self.view == View::Pus => {
                "PUS services and subtypes | Enter browse definitions".into()
            }
            _ => format!("{title} | {} definitions", self.candidates.len()),
        };
        let status =
            if let Some(section) = self.document.as_ref().and_then(|doc| doc.current_section()) {
                format!("{status} | {section}  [/] sections")
            } else {
                status
            };
        frame.render_widget(
            Paragraph::new(format!(" {status}")).style(Style::default().fg(Color::Cyan)),
            heading,
        );
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray));
        let inner = block.inner(body);
        self.page_height = usize::from(inner.height).max(1);
        frame.render_widget(block, body);
        if let Some(document) = &mut self.document {
            document.draw(frame, inner);
        } else if self.view == View::Tables {
            self.page_height = usize::from(inner.height.saturating_sub(2)).max(1);
            self.tables.draw(frame, inner);
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
        if let Some(message) = &self.notice {
            frame.render_widget(
                Paragraph::new(crate::render::text(message))
                    .style(Style::default().fg(Color::Yellow))
                    .wrap(Wrap { trim: false }),
                notice,
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
        frame.render_widget(
            Paragraph::new("? help").style(Style::default().fg(Color::Cyan)),
            help,
        );
    }
}
