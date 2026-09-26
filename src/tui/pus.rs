//! Presentation grouping of the snapshot's packet and command inventory.
use crossterm::event::KeyCode;
use mibl::{
    Mib,
    model::{Candidate, Identity, SearchScope},
};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Paragraph, Row, Table, TableState},
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Coordinates {
    pub service: Option<u16>,
    pub subtype: Option<u16>,
}

impl std::fmt::Display for Coordinates {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{},{}",
            coordinate(self.service),
            coordinate(self.subtype)
        )
    }
}

fn coordinate(value: Option<u16>) -> String {
    value.map_or_else(|| "unavailable".into(), |n| n.to_string())
}

pub(super) struct Group {
    pub coordinates: Coordinates,
    pub candidates: Vec<Candidate>,
}

pub(super) struct Browser {
    groups: Vec<Group>,
    selection: TableState,
}

impl Browser {
    pub fn new(mib: &Mib) -> Self {
        let mut groups: BTreeMap<Coordinates, Vec<Candidate>> = BTreeMap::new();
        for candidate in mib.inventory(SearchScope::All) {
            if matches!(candidate.identity, Identity::Parameter(_)) {
                continue;
            }
            groups
                .entry(Coordinates {
                    service: candidate.service_type,
                    subtype: candidate.service_subtype,
                })
                .or_default()
                .push(candidate);
        }
        // Inventory already provides kind/identity/source ordering inside each coordinate.
        let mut groups: Vec<_> = groups
            .into_iter()
            .map(|(coordinates, candidates)| Group {
                coordinates,
                candidates,
            })
            .collect();
        groups.sort_by_key(|g| {
            (
                g.coordinates.service.is_none(),
                g.coordinates.service,
                g.coordinates.subtype.is_none(),
                g.coordinates.subtype,
            )
        });
        let selection = TableState::default().with_selected((!groups.is_empty()).then_some(0));
        Self { groups, selection }
    }

    pub fn selected(&self) -> Option<&Group> {
        self.selection.selected().and_then(|i| self.groups.get(i))
    }

    pub fn navigate(&mut self, key: KeyCode, height: usize) {
        if let Some(index) = self.selection.selected() {
            self.selection.select(Some(super::vertical_position(
                key,
                index,
                self.groups.len().saturating_sub(1),
                height,
            )));
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        if self.groups.is_empty() {
            frame.render_widget(
                Paragraph::new("PUS definitions unavailable: no packet or command roots."),
                area,
            );
            return;
        }
        let rows = self.groups.iter().map(|group| {
            let packets = group
                .candidates
                .iter()
                .filter(|c| matches!(c.identity, Identity::Packet(_)))
                .count();
            Row::new(vec![
                coordinate(group.coordinates.service),
                coordinate(group.coordinates.subtype),
                packets.to_string(),
                (group.candidates.len() - packets).to_string(),
            ])
        });
        let header = Row::new(["Service", "Subtype", "Packets", "Commands"])
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .bottom_margin(1);
        frame.render_stateful_widget(
            Table::new(rows, [Constraint::Ratio(1, 4); 4])
                .header(header)
                .column_spacing(2)
                .highlight_symbol("> ")
                .row_highlight_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            area,
            &mut self.selection,
        );
    }
}
