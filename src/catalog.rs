//! Owns all retained records and duplicate-preserving indexes.
use crate::{
    model::*,
    reader::{Pcf, Records, Row},
};
use std::collections::HashMap;
mod calibrations;
mod commands;
mod packets;
mod pus;
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
        let mut parameters = HashMap::<ParameterName, Vec<RowId>>::new();
        for (index, row) in records.pcf.rows().iter().enumerate() {
            if let Some(name) = &row.cells.name.value {
                parameters
                    .entry(name.clone())
                    .or_default()
                    .push(RowId(index));
            }
        }
        let mut catalog = Self {
            records,
            parameters,
            packets: HashMap::new(),
            commands: HashMap::new(),
            pus: Default::default(),
            supporting: HashMap::new(),
        };
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

    fn describe_parameter(&self, row: &Row<Pcf>) -> ParameterDescription {
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

fn info<T>(value: Option<T>, source: &Source) -> Info<T> {
    Info {
        value,
        problems: vec![],
        sources: vec![source.clone()],
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

/// The problem a declared reference reports when nothing resolves it.
fn missing_problems(reference: Reference, source: &Source) -> Vec<Problem> {
    missing::<()>(reference, source).problems
}

/// A problem naming every target when one reference resolves to several definitions.
fn ambiguity(
    reference: Reference,
    source: &Source,
    targets: Vec<Target>,
    explanation: &str,
) -> Option<Problem> {
    (targets.len() > 1).then(|| Problem {
        sources: std::iter::once(source.clone())
            .chain(targets.iter().map(|t| t.definition.source.clone()))
            .collect(),
        kind: ProblemKind::AmbiguousReference {
            reference,
            alternatives: at_least_two(targets),
        },
        explanation: explanation.into(),
    })
}

/// One retained supporting row as a typed candidate target.
fn target<T>(table: Table, key: &str, row: &Row<T>) -> Target {
    Target {
        reference: reference(table, key),
        definition: row.definition.clone(),
    }
}

/// The static encoded width each retained PCF candidate establishes on its own. PTC/PFC
/// determines it; PCF_WIDTH is a padding declaration and never supplies a candidate's
/// encoded width.
fn candidate_widths(rows: &[&Row<Pcf>]) -> Vec<Info<u64>> {
    rows.iter()
        .map(|r| base_parameter(r).parameter.encoding.encoded_bits)
        .collect()
}

/// One independently established width when every candidate establishes that same width.
/// Candidates that disagree, a candidate without a width, and no candidates at all each leave
/// the value unavailable. The reference's own problems and sources survive unchanged, so an
/// ambiguous reference keeps every candidate beside a width they agree on.
fn consensus_width(widths: &[Info<u64>], reference: &Info<ParameterSummary>) -> Info<u64> {
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

/// A declared count and the value rows it covers disagree; both declarations stay available.
fn count_disagreement(
    declared: &str,
    count: i64,
    retained: &str,
    rows: usize,
    available: Vec<Target>,
) -> Problem {
    Problem {
        sources: available
            .iter()
            .map(|t| t.definition.source.clone())
            .collect(),
        kind: ProblemKind::InconsistentDefinition {
            fields: vec![declared.into(), retained.into()],
            values: vec![Scalar::Integer(count), Scalar::Unsigned(rows as u64)],
            available,
        },
        explanation: format!("{declared} disagrees with the number of retained {retained} rows"),
    }
}

/// A supporting table reference under the key its rows declare.
fn reference(table: Table, key: &str) -> Reference {
    Reference::Supporting {
        table,
        key: key.into(),
    }
}

/// Every candidate of an ambiguous reference, in source order.
fn at_least_two(targets: Vec<Target>) -> AtLeastTwo<Target> {
    let mut targets = targets.into_iter();
    AtLeastTwo {
        first: Box::new(targets.next().expect("ambiguity has a first target")),
        second: Box::new(targets.next().expect("ambiguity has a second target")),
        rest: targets.collect(),
    }
}

/// Text-valued SCOS cells take their interpretation from the declared format and radix.
fn number(cell: &Info<String>, format: Option<&str>, radix: Option<&str>) -> Info<Scalar> {
    let Some(text) = &cell.value else {
        return info(None, &cell.sources[0]);
    };
    let radix = match radix {
        Some("H") => 16,
        Some("O") => 8,
        _ => 10,
    };
    let value = match format {
        Some("U") => u64::from_str_radix(text, radix).ok().map(Scalar::Unsigned),
        Some("I") => text.parse::<i64>().ok().map(Scalar::Integer),
        Some("R") => text
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite())
            .map(|_| Scalar::Decimal(text.clone())),
        _ => None,
    };
    if value.is_none() {
        unavailable(
            &cell.sources[0],
            "Recorded number cannot be interpreted with its declared format",
        )
    } else {
        info(value, &cell.sources[0])
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

fn parameter_candidate(row: &Row<Pcf>) -> Candidate {
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

fn encoded_bits(ptc: Option<u16>, pfc: Option<u32>) -> Option<u64> {
    match (ptc, pfc) {
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
    }
}

fn unavailable<T>(source: &Source, explanation: &str) -> Info<T> {
    Info {
        value: None,
        sources: vec![source.clone()],
        problems: vec![Problem {
            kind: ProblemKind::UnsupportedInterpretation {
                column: None,
                meanings: vec![],
            },
            sources: vec![source.clone()],
            explanation: explanation.into(),
        }],
    }
}
fn unsigned<T: TryFrom<i64>>(cell: &Info<i64>, field: &str) -> Info<T> {
    let value = cell.value.and_then(|v| T::try_from(v).ok());
    if cell.value.is_some() && value.is_none() {
        unavailable(
            &cell.sources[0],
            &format!("{field} is outside the supported unsigned range"),
        )
    } else {
        Info {
            value,
            problems: cell.problems.clone(),
            sources: cell.sources.clone(),
        }
    }
}
fn missing<T>(reference: Reference, source: &Source) -> Info<T> {
    Info {
        value: None,
        sources: vec![source.clone()],
        problems: vec![Problem {
            explanation: format!("Missing reference: {reference:?}"),
            kind: ProblemKind::MissingReference { reference },
            sources: vec![source.clone()],
        }],
    }
}
fn resolve<T, U>(
    rows: &[&Row<T>],
    reference: Reference,
    source: &Source,
    describe: impl FnOnce(&Row<T>) -> Option<U>,
) -> Info<U> {
    match rows {
        [] => missing(reference, source),
        [row] => Info {
            value: describe(row),
            sources: vec![source.clone(), row.definition.source.clone()],
            problems: vec![],
        },
        [first, second, rest @ ..] => {
            let target = |r: &Row<T>| Target {
                reference: reference.clone(),
                definition: r.definition.clone(),
            };
            Info {
                value: None,
                sources: std::iter::once(source.clone())
                    .chain(rows.iter().map(|r| r.definition.source.clone()))
                    .collect(),
                problems: vec![Problem {
                    explanation: format!("Ambiguous reference: {reference:?}"),
                    sources: std::iter::once(source.clone())
                        .chain(rows.iter().map(|r| r.definition.source.clone()))
                        .collect(),
                    kind: ProblemKind::AmbiguousReference {
                        reference: reference.clone(),
                        alternatives: AtLeastTwo {
                            first: Box::new(target(first)),
                            second: Box::new(target(second)),
                            rest: rest.iter().map(|r| target(r)).collect(),
                        },
                    },
                }],
            }
        }
    }
}
