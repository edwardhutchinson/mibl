//! Selectable reports from the loaded snapshot. File editing belongs to the external editor.
use crossterm::event::KeyCode;
use mibl::{
    Mib,
    model::{TableReport, TableRoot, TableRows},
};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Row, Table, TableState},
};

pub(super) struct Browser {
    reports: Vec<TableReport>,
    selection: TableState,
}

impl Browser {
    pub fn new(mib: &Mib) -> Self {
        Self {
            reports: mib.tables(),
            selection: TableState::default().with_selected(Some(0)),
        }
    }

    pub fn selected(&self) -> Option<&TableReport> {
        self.selection.selected().and_then(|i| self.reports.get(i))
    }

    pub fn navigate(&mut self, key: KeyCode, height: usize) {
        if let Some(index) = self.selection.selected() {
            self.selection.select(Some(super::vertical_position(
                key,
                index,
                self.reports.len().saturating_sub(1),
                height,
            )));
        }
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let rows = self.reports.iter().map(|report| {
            Row::new(vec![
                report.table.code().into(),
                report.table.file().into(),
                report.table.meaning().into(),
                match report.table.root() {
                    Some(TableRoot::Parameter) => "parameter",
                    Some(TableRoot::Packet) => "packet",
                    Some(TableRoot::Command) => "command",
                    None => "-",
                }
                .into(),
                match report.rows {
                    TableRows::Missing => "missing".into(),
                    TableRows::Unreadable => "unreadable".into(),
                    TableRows::Read { rows } => rows.to_string(),
                },
            ])
        });
        let header = Row::new(["Code", "File", "Defines", "Lookup", "Rows"])
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .bottom_margin(1);
        frame.render_stateful_widget(
            Table::new(
                rows,
                [
                    Constraint::Length(5),
                    Constraint::Length(10),
                    Constraint::Min(10),
                    Constraint::Length(9),
                    Constraint::Length(10),
                ],
            )
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
