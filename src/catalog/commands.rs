use super::*;
use crate::reader::{Ccf, Cdf, Cpc};

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
                    header: unavailable(
                        &row.definition.source,
                        "Command header expansion is not implemented yet",
                    ),
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

impl Catalog {
    fn command_layout(&self, root: &Row<Ccf>) -> Info<Vec<Layout<CommandElement>>> {
        let name = root.cells.cname.value.as_ref().unwrap();
        let mut rows: Vec<_> = self
            .related(Table::Cdf, name.0.clone())
            .iter()
            .map(|id| &self.records.cdf.rows()[id.0])
            .collect();
        if rows.is_empty() && !matches!(self.records.cdf, crate::reader::TableLoad::Read { .. }) {
            return missing(
                Reference::Supporting {
                    table: Table::Cdf,
                    key: name.0.clone(),
                },
                &root.definition.source,
            );
        }
        let mut problems = Vec::new();
        for row in &rows {
            if row.cells.grpsize.value != Some(0) {
                problems.extend(unavailable::<()>(&row.definition.source, "CDF repetition structure is not implemented yet; elements retain declared unexpanded order").problems);
            }
        }
        rows.sort_by_key(|r| (r.cells.bit.value, &r.definition.source));
        let mut elements = Vec::new();
        for group in rows.chunk_by(|a, b| a.cells.bit.value == b.cells.bit.value) {
            let duplicate = (group.len() > 1).then(|| Problem {
                kind: ProblemKind::InconsistentDefinition {
                    fields: vec!["CDF_BIT".into()],
                    values: group
                        .iter()
                        .filter_map(|r| r.cells.bit.value.map(Scalar::Integer))
                        .collect(),
                    available: group.iter().map(|r| cdf_target(r)).collect(),
                },
                sources: group.iter().map(|r| r.definition.source.clone()).collect(),
                explanation: "Duplicate declared command element positions".into(),
            });
            for row in group {
                let mut element = self.command_element(row);
                if let Some(problem) = &duplicate {
                    element_location(&mut element)
                        .position
                        .problems
                        .push(problem.clone());
                }
                elements.push(Layout::Element(element));
            }
        }
        Info {
            value: Some(elements),
            problems,
            sources: vec![root.definition.source.clone()],
        }
    }

    fn telemetry_value(&self, row: &Row<Cdf>) -> Info<ArgumentValue> {
        let parameter = ParameterName(row.cells.tmid.value.clone().unwrap_or_default());
        let reference = Reference::Root(Identity::Parameter(parameter.clone()));
        let rows: Vec<_> = self
            .parameters
            .get(&parameter)
            .into_iter()
            .flatten()
            .map(|id| &self.records.pcf.rows()[id.0])
            .collect();
        let targets = resolve(&rows, reference.clone(), &row.definition.source, |r| {
            Some(vec![Target {
                reference: reference.clone(),
                definition: r.definition.clone(),
            }])
        });
        let declaration = RuntimeDeclaration {
            expression: format!(
                "CDF_INTER=T takes the dynamic default from monitoring parameter {}",
                parameter.0
            ),
            dependencies: vec![Dependency { reference, targets }],
            sources: vec![row.definition.source.clone()],
        };
        runtime_value(row, declaration, Some(parameter))
    }

    fn command_element(&self, row: &Row<Cdf>) -> CommandElement {
        let source = &row.definition.source;
        let bit: Info<u64> = unsigned(&row.cells.bit, "CDF_BIT");
        let position = Info {
            value: bit.value.map(Position::ApplicationDeclaredBit),
            problems: bit.problems,
            sources: bit.sources,
        };
        if row.cells.eltype.value.as_deref() == Some("A") {
            return CommandElement::Fixed(Box::new(FixedArea {
                definition: row.definition.clone(),
                location: Location {
                    position,
                    encoded_bits: unsigned(&row.cells.ellen, "CDF_ELLEN"),
                    constraints: vec![],
                },
                value: literal(
                    &row.cells.value,
                    &info(Some("R".into()), source),
                    &row.definition,
                ),
            }));
        }
        let reference = Reference::Supporting {
            table: Table::Cpc,
            key: row.cells.pname.value.clone().unwrap_or_default(),
        };
        let rows: Vec<_> = self
            .related(
                Table::Cpc,
                row.cells.pname.value.clone().unwrap_or_default(),
            )
            .iter()
            .map(|id| &self.records.cpc.rows()[id.0])
            .collect();
        let definition = resolve(&rows, reference.clone(), source, |r| {
            Some(r.definition.clone())
        });
        let cpc = match rows.as_slice() {
            [r] => Some(*r),
            _ => None,
        };
        let encoding = match cpc {
            Some(cpc) => command_encoding(cpc),
            None => Encoding {
                ptc: unresolved(&definition),
                pfc: unresolved(&definition),
                endian: unresolved(&definition),
                encoded_bits: unresolved(&definition),
            },
        };
        let default = cpc
            .map(|r| literal(&r.cells.defval, &r.cells.r#inter, &r.definition))
            .unwrap_or_else(|| unresolved(&definition));
        let element_value = match row.cells.r#inter.value.as_deref() {
            Some("T") => self.telemetry_value(row),
            representation
                if row.cells.eltype.value.as_deref() == Some("E")
                    && if representation == Some("D") {
                        default.value.is_none() && default.problems.is_empty()
                    } else {
                        row.cells.value.value.is_none()
                    } =>
            {
                runtime_value(
                    row,
                    RuntimeDeclaration {
                        expression:
                            "An operator must supply this editable argument at command invocation"
                                .into(),
                        dependencies: vec![],
                        sources: vec![source.clone()],
                    },
                    None,
                )
            }
            Some("D") => default.clone(),
            _ => literal(&row.cells.value, &row.cells.r#inter, &row.definition),
        };
        let mut width = encoding.encoded_bits.clone();
        if let (Some(declared), Some(bits), Some(cpc)) = (row.cells.ellen.value, width.value, cpc)
            && u64::try_from(declared).ok() != Some(bits)
        {
            width.problems.push(Problem {
                kind: ProblemKind::InconsistentDefinition {
                    fields: vec!["CDF_ELLEN".into(), "CPC_PTC".into(), "CPC_PFC".into()],
                    values: vec![Scalar::Integer(declared), Scalar::Unsigned(bits)],
                    available: vec![
                        cdf_target(row),
                        Target {
                            reference: reference.clone(),
                            definition: cpc.definition.clone(),
                        },
                    ],
                },
                sources: vec![source.clone(), cpc.definition.source.clone()],
                explanation: "CDF_ELLEN disagrees with the CPC encoded width".into(),
            });
        }
        CommandElement::Argument(Box::new(CommandArgument {
            reference,
            definition,
            element: row.definition.clone(),
            description: cpc
                .map(|r| r.cells.descr.clone())
                .unwrap_or_else(|| info(None, source)),
            units: cpc
                .map(|r| r.cells.unit.clone())
                .unwrap_or_else(|| info(None, source)),
            location: Location {
                position,
                encoded_bits: width,
                constraints: vec![],
            },
            encoding,
            rules: ValueRules {
                default,
                element_value,
                ranges: unavailable(
                    source,
                    "Command argument range expansion is not implemented yet",
                ),
                aliases: unavailable(
                    source,
                    "Command argument alias expansion is not implemented yet",
                ),
                calibrations: unavailable(
                    source,
                    "Command argument calibration expansion is not implemented yet",
                ),
                supporting_definitions: vec![],
            },
        }))
    }
}

fn unresolved<T>(definition: &Info<Definition>) -> Info<T> {
    Info {
        value: None,
        problems: definition.problems.clone(),
        sources: definition.sources.clone(),
    }
}
fn literal(
    value: &Info<String>,
    representation: &Info<String>,
    definition: &Definition,
) -> Info<ArgumentValue> {
    Info {
        value: value.value.as_ref().map(|s| ArgumentValue {
            source: ValueSource::Literal(Scalar::Text(s.clone())),
            representation: representation.clone(),
            definition: definition.clone(),
        }),
        problems: value.problems.clone(),
        sources: value.sources.clone(),
    }
}
fn command_encoding(row: &Row<Cpc>) -> Encoding {
    let ptc = unsigned(&row.cells.ptc, "CPC_PTC");
    let pfc = unsigned(&row.cells.pfc, "CPC_PFC");
    let bits = encoded_bits(ptc.value, pfc.value);
    Encoding {
        ptc,
        pfc,
        endian: row.cells.endian.clone(),
        encoded_bits: if bits.is_some() {
            info(bits, &row.definition.source)
        } else if matches!(
            (row.cells.ptc.value, row.cells.pfc.value),
            (Some(7 | 8), Some(0)) | (Some(11), _)
        ) {
            let declaration = RuntimeDeclaration {
                expression: "Encoded width depends on the argument value or its deduced type at command invocation".into(),
                dependencies: vec![], sources: vec![row.definition.source.clone()],
            };
            Info {
                value: None,
                sources: vec![row.definition.source.clone()],
                problems: vec![Problem {
                    kind: ProblemKind::RuntimeDependent { declaration },
                    sources: vec![row.definition.source.clone()],
                    explanation: "Command argument width requires runtime input".into(),
                }],
            }
        } else {
            unavailable(
                &row.definition.source,
                "No supported static encoded width for this CPC PTC/PFC combination",
            )
        },
    }
}

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
fn element_location(element: &mut CommandElement) -> &mut Location {
    match element {
        CommandElement::Argument(a) => &mut a.location,
        CommandElement::Fixed(f) => &mut f.location,
    }
}

fn runtime_value(
    row: &Row<Cdf>,
    declaration: RuntimeDeclaration,
    parameter: Option<ParameterName>,
) -> Info<ArgumentValue> {
    let source = match parameter {
        Some(parameter) => ValueSource::Telemetry {
            parameter,
            declaration: declaration.clone(),
        },
        None => ValueSource::Runtime(declaration.clone()),
    };
    Info {
        value: Some(ArgumentValue {
            source,
            representation: row.cells.r#inter.clone(),
            definition: row.definition.clone(),
        }),
        problems: vec![Problem {
            kind: ProblemKind::RuntimeDependent { declaration },
            sources: vec![row.definition.source.clone()],
            explanation: "Argument value requires runtime input".into(),
        }],
        sources: vec![row.definition.source.clone()],
    }
}
