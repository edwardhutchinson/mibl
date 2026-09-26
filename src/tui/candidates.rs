//! Definition rows and sticky column headers, independent of list navigation.
use super::App;
use crate::render::text;
use mibl::model::Identity;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Row, Table},
};

impl App<'_> {
    fn mixed_kinds(&self) -> bool {
        self.candidates.first().is_some_and(|first| {
            self.candidates.iter().any(|c| {
                std::mem::discriminant(&c.identity) != std::mem::discriminant(&first.identity)
            })
        })
    }

    pub(super) fn candidate_column_count(&self) -> usize {
        if self.mixed_kinds() { 5 } else { 4 }
    }

    pub(super) fn draw_candidates(&mut self, frame: &mut Frame, area: Rect) {
        let mixed = self.mixed_kinds();
        let mut headers = vec!["Identity", "Name", "Description", "Source"];
        if mixed {
            headers.insert(0, "Kind");
        }
        let rows = self.candidates.iter().map(|c| {
            let (kind, identity) = match &c.identity {
                Identity::Packet(n) => ("packet", n.0.to_string()),
                Identity::Parameter(n) => ("parameter", n.0.clone()),
                Identity::Command(n) => ("command", n.0.clone()),
            };
            let mut cells = vec![
                identity,
                c.name.value.clone().unwrap_or_else(|| "unavailable".into()),
                c.description
                    .value
                    .clone()
                    .unwrap_or_else(|| "unavailable".into()),
                format!("{}:{}", c.source.file.display(), c.source.line),
            ];
            if mixed {
                cells.insert(0, kind.into());
            }
            Row::new(cells.into_iter().skip(self.first_column).map(|s| text(&s)))
        });
        let headers = &headers[self.first_column..];
        let weights: Vec<_> = headers
            .iter()
            .map(|h| if *h == "Description" { 2 } else { 1 })
            .collect();
        let total: u32 = weights.iter().sum();
        let widths = weights.into_iter().map(|w| Constraint::Ratio(w, total));
        let header = Row::new(headers.iter().copied())
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .bottom_margin(1);
        frame.render_stateful_widget(
            Table::new(rows, widths)
                .header(header)
                .column_spacing(2)
                .row_highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("> "),
            area,
            &mut self.selection,
        );
    }
}
