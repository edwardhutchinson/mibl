//! Parameter lookup and summary construction: the retained PCF index, one
//! parameter's description, and the width its candidate definitions agree on.

use super::{Catalog, RowId, encoding::encoded_bits, resolution::info};
use crate::model::{
    AtLeastTwo, Candidate, Encoding, Identity, Info, Lookup, NotFoundReason, ParameterDescription,
    ParameterName, ParameterSummary, Problem, ProblemKind,
};
use crate::reader::{Pcf, Row};

impl Catalog {
    pub(super) fn index_parameters(&mut self) {
        for (index, row) in self.records.pcf.rows().iter().enumerate() {
            if let Some(name) = &row.cells.name.value {
                self.parameters
                    .entry(name.clone())
                    .or_default()
                    .push(RowId(index));
            }
        }
    }

    pub(super) fn describe_parameter(&self, row: &Row<Pcf>) -> ParameterDescription {
        let mut description = base_parameter(row);
        description.parameter.calibrations = self.calibrations(row);
        description
    }

    pub(crate) fn parameter(&self, name: &ParameterName) -> Lookup<ParameterDescription> {
        tracing::debug!(parameter = %name.0, matches = self.parameters.get(name).map_or(0, Vec::len), definitions = self.parameters.len(), "exact parameter lookup");
        match self.parameters.get(name).map(Vec::as_slice) {
            Some([row]) => {
                let mut description = self.describe_parameter(&self.records.pcf.rows()[row.0]);
                description.occurrences = self.parameter_occurrences(name);
                Lookup::Found(description)
            }
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
}

/// Owned summary with no catalog relationships; `Catalog::describe_parameter` adds
/// the calibrations, and parameter lookup adds the packet occurrences.
fn base_parameter(row: &Row<Pcf>) -> ParameterDescription {
    let p = &row.cells;
    let source = &row.definition.source;
    let ptc = p.ptc.value.and_then(|n| u16::try_from(n).ok());
    let pfc = p.pfc.value.and_then(|n| u32::try_from(n).ok());
    let bits = encoded_bits(ptc, pfc);
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
            // Filled in by Catalog::describe_parameter, which owns the calibration joins.
            calibrations: info(None, source),
        },
        occurrences: unsupported(row, &[], "Packet occurrences are not implemented yet"),
    }
}

pub(super) fn parameter_candidate(row: &Row<Pcf>) -> Candidate {
    Candidate {
        service_type: None,
        service_subtype: None,
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

/// The static encoded width each retained PCF candidate establishes on its own. PTC/PFC
/// determines it; PCF_WIDTH is a padding declaration and never supplies a candidate's
/// encoded width.
pub(super) fn candidate_widths(rows: &[&Row<Pcf>]) -> Vec<Info<u64>> {
    rows.iter()
        .map(|r| base_parameter(r).parameter.encoding.encoded_bits)
        .collect()
}

/// One independently established width when every candidate establishes that same width.
/// Candidates that disagree, a candidate without a width, and no candidates at all each leave
/// the value unavailable. The reference's own problems and sources survive unchanged, so an
/// ambiguous reference keeps every candidate beside a width they agree on.
pub(super) fn consensus_width(
    widths: &[Info<u64>],
    reference: &Info<ParameterSummary>,
) -> Info<u64> {
    let value = widths.first().and_then(|first| {
        first
            .value
            .filter(|_| widths.iter().all(|w| w.value == first.value))
    });
    Info {
        value,
        sources: reference.sources.clone(),
        problems: reference
            .problems
            .iter()
            .cloned()
            .chain(widths.iter().flat_map(|w| w.problems.clone()))
            .collect(),
    }
}
