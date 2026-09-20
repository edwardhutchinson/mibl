//! Duplicate-preserving PUS index over retained packet and command definitions.
use super::{Catalog, PusRow, RowId, commands::command_candidate, encoding::unsigned};
use crate::model::*;

impl Catalog {
    pub(super) fn index_pus(&mut self) {
        let packets = self.records.pid.rows().iter().enumerate().map(|(i, row)| {
            (
                &row.cells.r#type,
                &row.cells.stype,
                PusRow::Packet(RowId(i)),
            )
        });
        let commands = self.records.ccf.rows().iter().enumerate().map(|(i, row)| {
            (
                &row.cells.r#type,
                &row.cells.stype,
                PusRow::Command(RowId(i)),
            )
        });
        for (service, subtype, row) in packets.chain(commands) {
            if let Some(service) = unsigned::<u16>(service, "Service type").value {
                let subtype = unsigned::<u16>(subtype, "Service subtype").value;
                self.pus.entry((service, subtype)).or_default().push(row);
            }
        }
    }

    pub(crate) fn pus(&self, service: u16, subtype: Option<u16>) -> Vec<Candidate> {
        let mut candidates: Vec<_> = self
            .pus
            .range((service, None)..=(service, Some(u16::MAX)))
            .filter(|((_, st), _)| subtype.is_none() || *st == subtype)
            .flat_map(|(_, rows)| rows)
            .map(|row| match row {
                PusRow::Packet(id) => self.packet_candidate(&self.records.pid.rows()[id.0]),
                PusRow::Command(id) => command_candidate(&self.records.ccf.rows()[id.0]),
            })
            .collect();
        candidates.sort_by_key(|c| {
            (
                c.service_subtype.is_none(),
                c.service_subtype,
                c.identity.clone(),
                c.source.clone(),
            )
        });
        tracing::debug!(service, ?subtype, matches = candidates.len(), "PUS lookup");
        candidates
    }
}
