//! Owns all retained records and duplicate-preserving indexes.
use crate::{model::*, reader::Records};
use std::collections::HashMap;
/// Stable within this snapshot; points into retained root rows, never a public handle.
pub(crate) struct RowId(usize);
pub(crate) struct Catalog {
    records: Records,
    parameters: HashMap<ParameterName, Vec<RowId>>,
    packets: HashMap<PacketSpid, Vec<RowId>>,
    commands: HashMap<CommandName, Vec<RowId>>,
    supporting: HashMap<(Table, String), Vec<RowId>>,
}
impl Catalog {
    /// Takes ownership. Relationship failures become Info problems, not load errors.
    pub(crate) fn new(_records: Records) -> Self {
        todo!("interface only")
    }
    pub(crate) fn parameter(&self, _name: &ParameterName) -> Lookup<ParameterDescription> {
        todo!("interface only")
    }
    pub(crate) fn packet(&self, _spid: PacketSpid) -> Lookup<PacketDescription> {
        todo!("interface only")
    }
    pub(crate) fn command(&self, _name: &CommandName) -> Lookup<CommandDescription> {
        todo!("interface only")
    }
    pub(crate) fn search(&self, _query: &str, _scope: SearchScope) -> Vec<Candidate> {
        todo!("interface only")
    }
}
