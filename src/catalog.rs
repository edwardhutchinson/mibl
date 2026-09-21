//! Owns all retained records and duplicate-preserving indexes. `Catalog::new` takes
//! ownership and builds every index in turn: parameters, calibrations, packets, commands,
//! then the PUS listing. A query reads one index and expands only the relationships its
//! retained rows declare; the index maps below are the catalog's only retained state.

use crate::{
    model::*,
    reader::{Records, Row},
};
use std::collections::HashMap;

mod calibrations;
mod commands;
mod encoding;
mod packets;
mod parameters;
mod pus;
mod resolution;
mod search;
mod variable;

/// Stable within this snapshot; points into retained root rows, never a public handle.
pub(crate) struct RowId(usize);

enum PusRow {
    Packet(RowId),
    Command(RowId),
}

pub(crate) struct Catalog {
    records: Records,
    parameters: HashMap<ParameterName, Vec<RowId>>,
    packets: HashMap<PacketSpid, Vec<RowId>>,
    commands: HashMap<CommandName, Vec<RowId>>,
    pus: std::collections::BTreeMap<(u16, Option<u16>), Vec<PusRow>>,
    supporting: HashMap<(Table, String), Vec<RowId>>,
}

impl Catalog {
    /// Takes ownership. Relationship failures become Info problems, not load errors.
    pub(crate) fn new(records: Records) -> Self {
        let mut catalog = Self {
            records,
            parameters: HashMap::new(),
            packets: HashMap::new(),
            commands: HashMap::new(),
            pus: Default::default(),
            supporting: HashMap::new(),
        };
        catalog.index_parameters();
        catalog.index_calibrations();
        catalog.index_packets();
        catalog.index_commands();
        catalog.index_pus();
        catalog
    }

    fn related(&self, table: Table, key: String) -> &[RowId] {
        self.supporting
            .get(&(table, key))
            .map_or(&[], Vec::as_slice)
    }

    /// What the load attempt recorded for each supported table, in code order.
    pub(crate) fn tables(&self) -> Vec<TableReport> {
        self.records.reports.clone()
    }
}

/// Records every row's declared key under its table, keeping duplicates in source order.
fn index_rows<T>(
    index: &mut HashMap<(Table, String), Vec<RowId>>,
    table: Table,
    rows: &[Row<T>],
    key: impl Fn(&T) -> Option<&str>,
) {
    for (i, r) in rows.iter().enumerate() {
        if let Some(key) = key(&r.cells) {
            index
                .entry((table, key.to_owned()))
                .or_default()
                .push(RowId(i));
        }
    }
}
