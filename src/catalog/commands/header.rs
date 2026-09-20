//! Command header expansion: CCF_PKTID names a TCP packet header whose PCDF records
//! describe the fixed and parameter elements the command source encodes before the
//! application data. ICD 7.0 sections "Packet headers" define every retained meaning.
use super::super::Catalog;
use super::super::encoding::{number, unsigned};
use super::super::resolution::{ambiguity, info, missing, reference, resolve, target};
use crate::model::{
    ArgumentValue, CommandHeader, HeaderField, Info, Location, Position, Problem, ProblemKind,
    Reference, Scalar, Source, Table, Target, ValueSource,
};
use crate::reader::{Ccf, Pcdf, Pcpc, Row, TableLoad, Tcp};
use std::collections::{BTreeMap, BTreeSet, HashMap};

impl Catalog {
    /// The expanded header of one command. A missing, ambiguous or incomplete declaration
    /// leaves the command found: the header section reports the problem beside whatever
    /// usable information the retained rows carry.
    pub(super) fn command_header(&self, root: &Row<Ccf>) -> Info<CommandHeader> {
        let source = &root.definition.source;
        let key = root
            .cells
            .pktid
            .value
            .clone()
            .expect("retained CCF has a required packet identifier");
        let reference = reference(Table::Tcp, &key);
        let rows: Vec<_> = self
            .related(Table::Tcp, key.clone())
            .iter()
            .map(|id| &self.records.tcp.rows()[id.0])
            .collect();
        // CCF_PKTID names a packet header, so a key without a retained row is a missing reference.
        let Some((tcp, _)) = rows.split_first() else {
            return missing(reference, source);
        };
        let problems: Vec<Problem> = ambiguity(
            reference,
            source,
            rows.iter()
                .map(|row| target(Table::Tcp, &key, row))
                .collect(),
            "Packet header reference has multiple targets; every available definition is retained",
        )
        .into_iter()
        .collect();
        let fields = self.header_fields(tcp, &key);
        Info {
            value: Some(CommandHeader {
                definition: tcp.definition.clone(),
                fields,
            }),
            problems,
            sources: vec![source.clone(), tcp.definition.source.clone()],
        }
    }

    /// PCDF describes the header structure: one record per element, keyed by its declared bit
    /// offset. Declared order is the offset then the source line, so elements repeated at one
    /// offset keep every declaration in source order.
    fn header_fields(&self, tcp: &Row<Tcp>, key: &str) -> Info<Vec<HeaderField>> {
        let mut rows: Vec<_> = self
            .related(Table::Pcdf, key.to_owned())
            .iter()
            .map(|id| &self.records.pcdf.rows()[id.0])
            .collect();
        if rows.is_empty() {
            // A readable PCDF holding no record for this header declares no elements.
            return if matches!(self.records.pcdf, TableLoad::Read { .. }) {
                info(Some(Vec::new()), &tcp.definition.source)
            } else {
                missing(reference(Table::Pcdf, key), &tcp.definition.source)
            };
        }
        rows.sort_by_key(|row| (row.cells.bit.value, &row.definition.source));
        let duplicates = duplicate_positions(&rows);
        let lengths = self.element_lengths();
        Info {
            value: Some(
                rows.iter()
                    .map(|row| self.header_field(row, &duplicates, &lengths))
                    .collect(),
            ),
            problems: vec![],
            sources: vec![tcp.definition.source.clone()],
        }
    }

    fn header_field(
        &self,
        row: &Row<Pcdf>,
        duplicates: &HashMap<Source, Problem>,
        lengths: &HashMap<String, Problem>,
    ) -> HeaderField {
        let source = &row.definition.source;
        let (parameter, pcpc) = self.header_parameter(row);
        let bit = unsigned::<u64>(&row.cells.bit, "PCDF_BIT");
        let mut position = Info {
            value: bit.value.map(Position::HeaderBit),
            problems: bit.problems,
            sources: bit.sources,
        };
        if let Some(problem) = duplicates.get(source) {
            position.problems.push(problem.clone());
        }
        let mut encoded_bits = unsigned::<u64>(&row.cells.len, "PCDF_LEN");
        if let Some(name) = row.cells.pname.value.as_deref()
            && let Some(problem) = lengths.get(name)
        {
            encoded_bits.problems.push(problem.clone());
        }
        HeaderField {
            definition: row.definition.clone(),
            parameter,
            location: Location {
                position,
                encoded_bits,
                constraints: vec![],
            },
            value: header_value(row, pcpc),
            field_kind: row.cells.r#type.clone(),
        }
    }

    /// PCDF_PNAME names the PCPC parameter the element is an instance of. ICD 7.0 declares the
    /// name applicable to the parameter kinds and requires it to be absent for a fixed area, so
    /// a contradictory declaration keeps the parameter beside its problem.
    fn header_parameter<'a>(&'a self, row: &Row<Pcdf>) -> (Info<Target>, Option<&'a Row<Pcpc>>) {
        let source = &row.definition.source;
        let kind = row.cells.r#type.value.as_deref();
        let fixed = kind == Some("F");
        let Some(key) = row.cells.pname.value.clone() else {
            return if fixed {
                (info(None, source), None)
            } else {
                let problem = Problem {
                    kind: ProblemKind::InconsistentDefinition {
                        fields: vec!["PCDF_TYPE".into(), "PCDF_PNAME".into()],
                        values: kind
                            .map(|code| Scalar::Code(code.to_owned()))
                            .into_iter()
                            .collect(),
                        available: vec![pcdf_target(row)],
                    },
                    sources: vec![source.clone()],
                    explanation: "A parameter header element declares no PCDF_PNAME, so no packet header parameter is linked".into(),
                };
                (
                    Info {
                        value: None,
                        problems: vec![problem],
                        sources: vec![source.clone()],
                    },
                    None,
                )
            };
        };
        let reference = reference(Table::Pcpc, &key);
        let rows: Vec<_> = self
            .related(Table::Pcpc, key.clone())
            .iter()
            .map(|id| &self.records.pcpc.rows()[id.0])
            .collect();
        let pcpc = match rows.as_slice() {
            [row] => Some(*row),
            _ => None,
        };
        let mut resolved = resolve(&rows, reference.clone(), source, |row| {
            Some(Target {
                reference: reference.clone(),
                definition: row.definition.clone(),
            })
        });
        if fixed {
            resolved.problems.push(Problem {
                kind: ProblemKind::InconsistentDefinition {
                    fields: vec!["PCDF_TYPE".into(), "PCDF_PNAME".into()],
                    values: vec![Scalar::Code("F".into()), Scalar::Text(key.clone())],
                    available: std::iter::once(pcdf_target(row))
                        .chain(rows.iter().map(|row| target(Table::Pcpc, &key, row)))
                        .collect(),
                },
                sources: std::iter::once(source.clone())
                    .chain(rows.iter().map(|row| row.definition.source.clone()))
                    .collect(),
                explanation:
                    "A fixed header area declares a PCDF_PNAME, which ICD 7.0 requires to be absent"
                        .into(),
            });
        }
        (resolved, pcpc)
    }

    /// ICD 7.0 requires every PCDF record declaring the same PCDF_PNAME to declare the same
    /// element length, within one packet header or across several. A disagreement keeps every
    /// declaration and the lengths they state.
    fn element_lengths(&self) -> HashMap<String, Problem> {
        let mut named: BTreeMap<&str, Vec<&Row<Pcdf>>> = BTreeMap::new();
        for row in self.records.pcdf.rows() {
            if let Some(name) = row.cells.pname.value.as_deref() {
                named.entry(name).or_default().push(row);
            }
        }
        let mut problems = HashMap::new();
        for (name, rows) in named {
            let lengths: BTreeSet<i64> =
                rows.iter().filter_map(|row| row.cells.len.value).collect();
            if lengths.len() < 2 {
                continue;
            }
            problems.insert(
                name.to_owned(),
                Problem {
                    kind: ProblemKind::InconsistentDefinition {
                        fields: vec!["PCDF_PNAME".into(), "PCDF_LEN".into()],
                        values: lengths.iter().map(|len| Scalar::Integer(*len)).collect(),
                        available: rows.iter().map(|row| pcdf_target(row)).collect(),
                    },
                    sources: rows
                        .iter()
                        .map(|row| row.definition.source.clone())
                        .collect(),
                    explanation: format!(
                        "PCDF records for element {name} disagree on the declared element length"
                    ),
                },
            );
        }
        problems
    }
}

/// A retained PCDF row as a typed target. TCNAME and BIT are its declared key.
fn pcdf_target(row: &Row<Pcdf>) -> Target {
    Target {
        reference: Reference::Supporting {
            table: Table::Pcdf,
            key: format!(
                "{}:{}",
                row.cells
                    .tcname
                    .value
                    .as_ref()
                    .expect("retained PCDF has a name"),
                row.cells.bit.value.expect("retained PCDF has a bit offset")
            ),
        },
        definition: row.definition.clone(),
    }
}

/// The recorded value of one header element. ICD 7.0 gives a fixed area's PCDF_VALUE in hex
/// and every other element's default in decimal for a signed integer parameter and in
/// PCDF_RADIX for an unsigned one. Without a linked parameter the declared format is unknown,
/// so the value stays unavailable and the link carries the problem.
fn header_value(row: &Row<Pcdf>, pcpc: Option<&Row<Pcpc>>) -> Info<ArgumentValue> {
    let source = &row.definition.source;
    let (interpretation, representation) = match row.cells.r#type.value.as_deref() {
        Some("F") => (
            number(&row.cells.value, Some("U"), Some("H")),
            info(Some("H".into()), source),
        ),
        Some(_) => {
            let Some(pcpc) = pcpc else {
                return info(None, source);
            };
            match pcpc.cells.code.value.as_deref() {
                Some("I") => (
                    number(&row.cells.value, Some("I"), Some("D")),
                    info(Some("D".into()), source),
                ),
                _ => (
                    number(
                        &row.cells.value,
                        Some("U"),
                        row.cells.radix.value.as_deref(),
                    ),
                    row.cells.radix.clone(),
                ),
            }
        }
        None => return info(None, source),
    };
    Info {
        value: interpretation.value.map(|value| ArgumentValue {
            source: ValueSource::Literal(value),
            representation,
            definition: row.definition.clone(),
        }),
        problems: interpretation.problems.clone(),
        sources: interpretation.sources.clone(),
    }
}

/// Duplicate declared header offsets keep every element in declared source order and name
/// every competing declaration.
fn duplicate_positions(rows: &[&Row<Pcdf>]) -> HashMap<Source, Problem> {
    let mut duplicates = HashMap::new();
    for group in rows.chunk_by(|a, b| a.cells.bit.value == b.cells.bit.value) {
        if group.len() < 2 {
            continue;
        }
        let problem = Problem {
            kind: ProblemKind::InconsistentDefinition {
                fields: vec!["PCDF_BIT".into()],
                values: group
                    .iter()
                    .filter_map(|row| row.cells.bit.value.map(Scalar::Integer))
                    .collect(),
                available: group.iter().map(|row| pcdf_target(row)).collect(),
            },
            sources: group
                .iter()
                .map(|row| row.definition.source.clone())
                .collect(),
            explanation: "Duplicate declared packet header element offsets".into(),
        };
        for row in group {
            duplicates.insert(row.definition.source.clone(), problem.clone());
        }
    }
    duplicates
}
