//! Command lookup and indexing, with the helpers its layout, argument, rule and
//! header modules share.

use super::{Catalog, RowId, encoding::unsigned, index_rows, resolution::info};
use crate::model::{
    AtLeastTwo, Candidate, CommandDescription, CommandName, Identity, Lookup, NotFoundReason,
    Reference, Table, Target,
};
use crate::reader::{Ccf, Cdf, Row};

mod arguments;
mod header;
mod layout;
mod rules;

impl Catalog {
    pub(super) fn index_commands(&mut self) {
        for (index, row) in self.records.cdf.rows().iter().enumerate() {
            if let Some(name) = &row.cells.cname.value {
                self.supporting
                    .entry((Table::Cdf, name.0.clone()))
                    .or_default()
                    .push(RowId(index));
            }
        }
        for (index, row) in self.records.cpc.rows().iter().enumerate() {
            if let Some(name) = &row.cells.name.value {
                self.supporting
                    .entry((Table::Cpc, name.clone()))
                    .or_default()
                    .push(RowId(index));
            }
        }
        for (index, row) in self.records.ccf.rows().iter().enumerate() {
            if let Some(name) = &row.cells.cname.value {
                self.commands
                    .entry(name.clone())
                    .or_default()
                    .push(RowId(index));
            }
        }
        let (index, records) = (&mut self.supporting, &self.records);
        // TCP roots by identifier, PCDF rows by their TCP and PCPC rows by parameter name.
        index_rows(index, Table::Tcp, records.tcp.rows(), |c| {
            c.id.value.as_deref()
        });
        index_rows(index, Table::Pcdf, records.pcdf.rows(), |c| {
            c.tcname.value.as_deref()
        });
        index_rows(index, Table::Pcpc, records.pcpc.rows(), |c| {
            c.pname.value.as_deref()
        });
        // The command value rules key on NUMBR, in the same per-table key space as the calibrations.
        index_rows(index, Table::Cca, records.cca.rows(), |c| {
            c.numbr.value.as_deref()
        });
        index_rows(index, Table::Ccs, records.ccs.rows(), |c| {
            c.numbr.value.as_deref()
        });
        index_rows(index, Table::Paf, records.paf.rows(), |c| {
            c.numbr.value.as_deref()
        });
        index_rows(index, Table::Pas, records.pas.rows(), |c| {
            c.numbr.value.as_deref()
        });
        index_rows(index, Table::Prf, records.prf.rows(), |c| {
            c.numbr.value.as_deref()
        });
        index_rows(index, Table::Prv, records.prv.rows(), |c| {
            c.numbr.value.as_deref()
        });
    }

    pub(crate) fn command(&self, name: &CommandName) -> Lookup<CommandDescription> {
        tracing::debug!(command = %name.0, matches = self.commands.get(name).map_or(0, Vec::len), "exact command lookup");
        match self.commands.get(name).map(Vec::as_slice) {
            Some([id]) => {
                let row = &self.records.ccf.rows()[id.0];
                Lookup::Found(CommandDescription {
                    name: name.clone(),
                    definition: row.definition.clone(),
                    description: row.cells.descr.clone(),
                    arguments: self.command_layout(row),
                    header: self.command_header(row),
                })
            }
            Some([first, second, rest @ ..]) => Lookup::Ambiguous(AtLeastTwo {
                first: Box::new(command_candidate(&self.records.ccf.rows()[first.0])),
                second: Box::new(command_candidate(&self.records.ccf.rows()[second.0])),
                rest: rest
                    .iter()
                    .map(|id| command_candidate(&self.records.ccf.rows()[id.0]))
                    .collect(),
            }),
            _ => {
                let reason = if self.commands.is_empty() {
                    NotFoundReason::DefinitionsUnavailable
                } else {
                    NotFoundReason::NoMatchingIdentity
                };
                tracing::debug!(?reason, "command not found");
                Lookup::NotFound(reason)
            }
        }
    }
}
pub(super) fn command_candidate(row: &Row<Ccf>) -> Candidate {
    Candidate {
        service_type: unsigned(&row.cells.r#type, "Service type").value,
        service_subtype: unsigned(&row.cells.stype, "Service subtype").value,
        identity: Identity::Command(
            row.cells
                .cname
                .value
                .clone()
                .expect("retained CCF has a name"),
        ),
        name: info(
            row.cells.cname.value.as_ref().map(|n| n.0.clone()),
            &row.definition.source,
        ),
        description: row.cells.descr.clone(),
        source: row.definition.source.clone(),
    }
}

/// A retained CDF row as a typed target. CNAME and BIT are its declared key, and the
/// argument construction and the group evidence share it.
fn cdf_target(row: &Row<Cdf>) -> Target {
    Target {
        reference: Reference::Supporting {
            table: Table::Cdf,
            key: format!(
                "{}:{}",
                row.cells.cname.value.as_ref().unwrap().0,
                row.cells.bit.value.unwrap()
            ),
        },
        definition: row.definition.clone(),
    }
}
