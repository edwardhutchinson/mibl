//! Owns all retained records and duplicate-preserving indexes.
use crate::{
    model::*,
    reader::{Pcf, Records, Row},
};
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
    pub(crate) fn new(records: Records) -> Self {
        let mut parameters = HashMap::<ParameterName, Vec<RowId>>::new();
        for (index, row) in records.pcf.rows().iter().enumerate() {
            if let Some(name) = &row.cells.name.value {
                parameters
                    .entry(name.clone())
                    .or_default()
                    .push(RowId(index));
            }
        }
        Self {
            records,
            parameters,
            packets: HashMap::new(),
            commands: HashMap::new(),
            supporting: HashMap::new(),
        }
    }
    pub(crate) fn parameter(&self, name: &ParameterName) -> Lookup<ParameterDescription> {
        tracing::debug!(parameter = %name.0, matches = self.parameters.get(name).map_or(0, Vec::len), definitions = self.parameters.len(), "exact parameter lookup");
        match self.parameters.get(name).map(Vec::as_slice) {
            Some([row]) => Lookup::Found(describe_parameter(&self.records.pcf.rows()[row.0])),
            Some([first, second, rest @ ..]) => Lookup::Ambiguous(AtLeastTwo {
                first: Box::new(parameter_candidate(&self.records.pcf.rows()[first.0])),
                second: Box::new(parameter_candidate(&self.records.pcf.rows()[second.0])),
                rest: rest
                    .iter()
                    .map(|id| parameter_candidate(&self.records.pcf.rows()[id.0]))
                    .collect(),
            }),
            _ => {
                let reason = if self.parameters.is_empty() {
                    NotFoundReason::DefinitionsUnavailable
                } else {
                    NotFoundReason::NoMatchingIdentity
                };
                tracing::debug!(?reason, "parameter not found");
                Lookup::NotFound(reason)
            }
        }
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

fn info<T>(value: Option<T>, source: &Source) -> Info<T> {
    Info {
        value,
        problems: vec![],
        sources: vec![source.clone()],
    }
}

fn unsupported<T>(row: &Row<Pcf>, columns: &[usize], explanation: &str) -> Info<T> {
    let meanings = columns
        .iter()
        .flat_map(|i| row.definition.fields[*i].meanings.clone())
        .collect();
    Info {
        value: None,
        sources: vec![row.definition.source.clone()],
        problems: vec![Problem {
            kind: ProblemKind::UnsupportedInterpretation {
                column: None,
                meanings,
            },
            sources: vec![row.definition.source.clone()],
            explanation: explanation.into(),
        }],
    }
}

fn describe_parameter(row: &Row<Pcf>) -> ParameterDescription {
    let p = &row.cells;
    let source = &row.definition.source;
    let ptc = p.ptc.value.and_then(|n| u16::try_from(n).ok());
    let pfc = p.pfc.value.and_then(|n| u32::try_from(n).ok());
    let bits = match (ptc, pfc) {
        (Some(1), Some(0)) => Some(1),
        (Some(2 | 6), Some(n @ 1..=32)) => Some(u64::from(n)),
        (Some(3 | 4), Some(n @ 0..=12)) => Some(u64::from(n) + 4),
        (Some(3 | 4), Some(13)) => Some(24),
        (Some(3 | 4), Some(14)) => Some(32),
        (Some(3 | 4), Some(15)) => Some(48),
        (Some(3 | 4), Some(16)) => Some(64),
        (Some(5), Some(1 | 3)) => Some(32),
        (Some(5), Some(2)) => Some(64),
        (Some(5), Some(4)) => Some(48),
        (Some(7 | 8), Some(n @ 1..)) => Some(u64::from(n) * 8),
        (Some(9), Some(1)) => Some(48),
        (Some(9), Some(2 | 30)) => Some(64),
        (Some(9 | 10), Some(n @ 3..=18)) => {
            // CUC formats enumerate one to four coarse octets, each with zero to three fine octets.
            Some(u64::from(1 + (n - 3) / 4 + (n - 3) % 4) * 8)
        }
        _ => None,
    };
    ParameterDescription {
        parameter: ParameterSummary {
            name: p
                .name
                .value
                .clone()
                .expect("retained PCF has a required name"),
            definition: row.definition.clone(),
            description: p.descr.clone(),
            units: p.unit.clone(),
            encoding: Encoding {
                ptc: if ptc.is_some() {
                    info(ptc, source)
                } else {
                    unsupported(
                        row,
                        &[4],
                        "PTC cannot be represented as an unsigned type code",
                    )
                },
                pfc: if pfc.is_some() {
                    info(pfc, source)
                } else {
                    unsupported(
                        row,
                        &[5],
                        "PFC cannot be represented as an unsigned format code",
                    )
                },
                endian: p.endian.clone(),
                encoded_bits: if bits.is_some() {
                    info(bits, source)
                } else {
                    unsupported(
                        row,
                        &[4, 5],
                        "No supported static encoded width for this PTC/PFC combination",
                    )
                },
            },
            calibrations: unsupported(
                row,
                &[9, 11],
                "Calibration expansion is not implemented yet",
            ),
        },
        occurrences: unsupported(row, &[], "Packet occurrences are not implemented yet"),
    }
}

fn parameter_candidate(row: &Row<Pcf>) -> Candidate {
    Candidate {
        identity: Identity::Parameter(
            row.cells
                .name
                .value
                .clone()
                .expect("retained PCF has a required name"),
        ),
        name: info(
            row.cells.name.value.as_ref().map(|name| name.0.clone()),
            &row.definition.source,
        ),
        description: row.cells.descr.clone(),
        source: row.definition.source.clone(),
    }
}
