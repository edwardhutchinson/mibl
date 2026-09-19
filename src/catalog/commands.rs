use super::*;
use crate::reader::{Cca, Ccf, Cdf, Cpc};

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

/// CDF_GRPSIZE is a two-digit count of 1 to 99 following elements; a larger declaration is out
/// of range and stays clamped to the elements its group retains.
const MAX_GROUP_SIZE: i64 = 99;
/// CDF groups nest and CDF_GRPSIZE counts following records, so a declaration cycle must stop
/// descending; beyond this depth the repeater stays an element with its problem.
const MAX_GROUP_DEPTH: usize = 64;

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
        // Declared layout order is the declared unexpanded position, then source.
        rows.sort_by_key(|r| (r.cells.bit.value, &r.definition.source));
        let duplicates = duplicate_positions(&rows);
        Info {
            value: Some(self.command_elements(&rows, &[], 0, &duplicates)),
            problems: vec![],
            sources: vec![root.definition.source.clone()],
        }
    }

    /// CDF describes the unexpanded application data, and an element declaring CDF_GRPSIZE is a
    /// repeater whose value repeats the following declared elements. Groups nest, so the tree
    /// follows the declared position order recursively, keeps every member at its declared bit
    /// and never expands a repetition.
    fn command_elements(
        &self,
        rows: &[&Row<Cdf>],
        enclosing: &[RuntimeDeclaration],
        depth: usize,
        duplicates: &HashMap<Source, Problem>,
    ) -> Vec<Layout<CommandElement>> {
        let mut layout = Vec::new();
        // Groups already repeated before the current element, in declaration order.
        let mut preceding: Vec<RuntimeDeclaration> = Vec::new();
        let mut index = 0;
        while index < rows.len() {
            let row = rows[index];
            let source = &row.definition.source;
            let size = row.cells.grpsize.value.unwrap_or(0);
            let mut element = self.command_element(row);
            if let Some(problem) = duplicates.get(source) {
                element_location(&mut element)
                    .position
                    .problems
                    .push(problem.clone());
            }
            constrain(&mut element, enclosing, &preceding);
            if size > 0 && depth < MAX_GROUP_DEPTH {
                // GRPSIZE counts following declared elements, including nested repeaters.
                let end = index
                    .saturating_add(1)
                    .saturating_add(size as usize)
                    .min(rows.len());
                let complete = end - index - 1 == size as usize;
                let group =
                    self.group_repetition(row, &element, size, &rows[index + 1..end], complete);
                let nested: Vec<RuntimeDeclaration> = enclosing
                    .iter()
                    .chain(&preceding)
                    .chain(std::iter::once(&group.position))
                    .cloned()
                    .collect();
                let children =
                    self.command_elements(&rows[index + 1..end], &nested, depth + 1, duplicates);
                preceding.push(group.position);
                layout.push(Layout::Element(element));
                layout.push(Layout::Repeat {
                    definition: row.definition.clone(),
                    repetition: group.repetition,
                    children,
                });
                index = end;
            } else {
                if size != 0 {
                    let problem = invalid_group(row, size, depth >= MAX_GROUP_DEPTH);
                    element_location(&mut element)
                        .position
                        .problems
                        .push(problem);
                }
                layout.push(Layout::Element(element));
                index += 1;
            }
        }
        layout
    }

    /// The declared repetition of one group. CDF has no fixed-count column, so the count is the
    /// repeater element's value at command invocation and stays a declaration with dependencies.
    /// A fixed area cannot supply that value, and a group that outlives its declared elements
    /// keeps them with the disagreement recorded.
    fn group_repetition(
        &self,
        row: &Row<Cdf>,
        element: &CommandElement,
        size: i64,
        members: &[&Row<Cdf>],
        complete: bool,
    ) -> DeclaredGroup {
        let source = &row.definition.source;
        let mut problems = Vec::new();
        if size > MAX_GROUP_SIZE {
            problems.push(group_problem(
                row,
                size,
                members,
                "CDF_GRPSIZE is outside the declared 1 to 99 range and is clamped to the retained elements",
            ));
        } else if !complete {
            problems.push(group_problem(
                row,
                size,
                members,
                "CDF_GRPSIZE declares more elements than its group retains",
            ));
        }
        // Every member and following element carries the unexpanded-position rule. Its typed
        // count source stays on the group itself, so no location repeats that dependency.
        let position = RuntimeDeclaration {
            expression: format!(
                "Declared CDF_BIT is the unexpanded application-data position and shifts with the {size} declared element(s) this group repeats"
            ),
            dependencies: vec![],
            sources: vec![source.clone()],
        };
        let repetition = match element {
            // A fixed area carries no value, so it cannot declare how often the group repeats.
            CommandElement::Fixed(_) => {
                problems.push(group_problem(
                    row,
                    size,
                    members,
                    "A fixed area declares a group size and cannot supply a repetition count",
                ));
                Info {
                    value: None,
                    problems,
                    sources: vec![source.clone()],
                }
            }
            CommandElement::Argument(argument) => {
                let count = repetition_source(row, argument);
                let dependencies = self.repetition_dependencies(row, argument);
                // A source whose declared target never resolved keeps its problem beside the group.
                problems.extend(
                    dependencies
                        .iter()
                        .flat_map(|dependency| dependency.targets.problems.clone()),
                );
                let declaration = RuntimeDeclaration {
                    expression: format!(
                        "CDF_GRPSIZE={size} repeats the following {size} declared element(s) using {count}; declared CDF_BIT positions assume one repetition"
                    ),
                    dependencies,
                    sources: vec![source.clone()],
                };
                problems.push(Problem {
                    kind: ProblemKind::RuntimeDependent {
                        declaration: declaration.clone(),
                    },
                    sources: vec![source.clone()],
                    explanation: format!(
                        "Group of {size} declared element(s) repeats using {count}"
                    ),
                });
                Info {
                    value: Some(Repetition::Runtime(declaration)),
                    problems,
                    sources: vec![source.clone()],
                }
            }
        };
        DeclaredGroup {
            repetition,
            position,
        }
    }

    /// The runtime value that supplies a group's repetition count, typed as the declaration names
    /// it: a telemetry parameter, or the command parameter the operator or the MIB supplies.
    fn repetition_dependencies(
        &self,
        row: &Row<Cdf>,
        argument: &CommandArgument,
    ) -> Vec<Dependency> {
        if let Some(ArgumentValue {
            source: ValueSource::Telemetry { declaration, .. },
            ..
        }) = &argument.rules.element_value.value
        {
            return declaration.dependencies.clone();
        }
        let key = row.cells.pname.value.clone().unwrap_or_default();
        let reference = reference(Table::Cpc, &key);
        let rows: Vec<_> = self
            .related(Table::Cpc, key)
            .iter()
            .map(|id| &self.records.cpc.rows()[id.0])
            .collect();
        let targets = resolve(&rows, reference.clone(), &row.definition.source, |r| {
            Some(vec![Target {
                reference: reference.clone(),
                definition: r.definition.clone(),
            }])
        });
        vec![Dependency { reference, targets }]
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
        let rules = match cpc {
            Some(cpc) => self.argument_rules(cpc),
            None => ArgumentRules {
                ranges: unresolved(&definition),
                aliases: unresolved(&definition),
                calibrations: unresolved(&definition),
                definitions: vec![],
            },
        };
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
                ranges: rules.ranges,
                aliases: rules.aliases,
                calibrations: rules.calibrations,
                supporting_definitions: rules.definitions,
            },
        }))
    }
}

/// The supporting rules the CPC reference cells resolve for one argument.
struct ArgumentRules {
    ranges: Info<Vec<AllowedRange>>,
    aliases: Info<Vec<Alias>>,
    calibrations: Info<Vec<CalibrationAlternative>>,
    definitions: Vec<Definition>,
}

impl Catalog {
    /// PRF/PRV, PAF/PAS and CCA/CCS join through the CPC reference cells. A reference the row
    /// does not declare contributes no rules, while a declared reference without a usable target
    /// keeps a typed problem and never removes the argument.
    fn argument_rules(&self, cpc: &Row<Cpc>) -> ArgumentRules {
        let source = &cpc.definition.source;
        let (ranges, range_sets) = self.range_rules(cpc.cells.prfref.value.as_deref(), source);
        let (aliases, alias_sets) = self.alias_rules(cpc.cells.pafref.value.as_deref(), source);
        let calibrations = self.conversion_rules(cpc.cells.ccaref.value.as_deref(), source);
        ArgumentRules {
            ranges,
            aliases,
            calibrations,
            definitions: range_sets.into_iter().chain(alias_sets).collect(),
        }
    }

    /// PRF names the range set and declares the representation, input format and radix of its PRV
    /// bounds, so every bound is interpreted with its own set's declarations. Duplicate PRF rows
    /// keep their own sets, every PRV row stays in declared order, and an omitted upper bound stays
    /// unavailable instead of becoming an empty range. A declared PRF_NRANGE that disagrees with
    /// the retained rows keeps both declarations.
    fn range_rules(
        &self,
        key: Option<&str>,
        source: &Source,
    ) -> (Info<Vec<AllowedRange>>, Vec<Definition>) {
        let Some(key) = key else {
            return (info(Some(Vec::new()), source), Vec::new());
        };
        let sets: Vec<_> = self
            .related(Table::Prf, key.into())
            .iter()
            .map(|id| &self.records.prf.rows()[id.0])
            .collect();
        if sets.is_empty() {
            return (missing(reference(Table::Prf, key), source), Vec::new());
        }
        let mut ranges = Vec::new();
        let mut problems: Vec<Problem> = ambiguity(
            reference(Table::Prf, key),
            source,
            sets.iter()
                .map(|set| target(Table::Prf, key, set))
                .collect(),
            "Range set reference has multiple targets; every available definition is retained",
        )
        .into_iter()
        .collect();
        for set in &sets {
            let boundaries: Vec<_> = self
                .related(Table::Prv, key.into())
                .iter()
                .map(|id| &self.records.prv.rows()[id.0])
                .collect();
            if boundaries.is_empty() {
                problems.extend(missing_problems(
                    reference(Table::Prv, key),
                    &set.definition.source,
                ));
                continue;
            }
            let format = set.cells.dspfmt.value.as_deref();
            let radix = set.cells.radix.value.as_deref();
            ranges.extend(boundaries.iter().map(|boundary| AllowedRange {
                low: bound(&boundary.cells.minval, format, radix),
                high: bound(&boundary.cells.maxval, format, radix),
                representation: set.cells.r#inter.clone(),
                definition: boundary.definition.clone(),
            }));
            if let Some(declared) = set.cells.nrange.value
                && u64::try_from(declared).ok() != Some(boundaries.len() as u64)
            {
                problems.push(count_disagreement(
                    "PRF_NRANGE",
                    declared,
                    "PRV_NUMBR",
                    boundaries.len(),
                    std::iter::once(target(Table::Prf, key, set))
                        .chain(
                            boundaries
                                .iter()
                                .map(|boundary| target(Table::Prv, key, boundary)),
                        )
                        .collect(),
                ));
            }
        }
        let value = (!ranges.is_empty()).then_some(ranges);
        (
            Info {
                value,
                problems,
                sources: std::iter::once(source.clone())
                    .chain(sets.iter().map(|set| set.definition.source.clone()))
                    .collect(),
            },
            sets.iter().map(|set| set.definition.clone()).collect(),
        )
    }

    /// PAF names the alias set and declares the format of its PAS values. The alias text stays
    /// beside the interpreted raw value, and PAF declares no radix column, so values are decimal.
    /// A declared PAF_NALIAS that disagrees with the retained rows keeps both declarations.
    fn alias_rules(
        &self,
        key: Option<&str>,
        source: &Source,
    ) -> (Info<Vec<Alias>>, Vec<Definition>) {
        let Some(key) = key else {
            return (info(Some(Vec::new()), source), Vec::new());
        };
        let sets: Vec<_> = self
            .related(Table::Paf, key.into())
            .iter()
            .map(|id| &self.records.paf.rows()[id.0])
            .collect();
        if sets.is_empty() {
            return (missing(reference(Table::Paf, key), source), Vec::new());
        }
        let mut aliases = Vec::new();
        let mut problems: Vec<Problem> = ambiguity(
            reference(Table::Paf, key),
            source,
            sets.iter()
                .map(|set| target(Table::Paf, key, set))
                .collect(),
            "Alias set reference has multiple targets; every available definition is retained",
        )
        .into_iter()
        .collect();
        for set in &sets {
            let values: Vec<_> = self
                .related(Table::Pas, key.into())
                .iter()
                .map(|id| &self.records.pas.rows()[id.0])
                .collect();
            if values.is_empty() {
                problems.extend(missing_problems(
                    reference(Table::Pas, key),
                    &set.definition.source,
                ));
                continue;
            }
            aliases.extend(values.iter().map(|value| Alias {
                raw: number(
                    &value.cells.alval,
                    set.cells.rawfmt.value.as_deref(),
                    Some("D"),
                ),
                text: value.cells.altxt.clone(),
                definition: value.definition.clone(),
            }));
            if let Some(declared) = set.cells.nalias.value
                && u64::try_from(declared).ok() != Some(values.len() as u64)
            {
                problems.push(count_disagreement(
                    "PAF_NALIAS",
                    declared,
                    "PAS_NUMBR",
                    values.len(),
                    std::iter::once(target(Table::Paf, key, set))
                        .chain(values.iter().map(|value| target(Table::Pas, key, value)))
                        .collect(),
                ));
            }
        }
        let value = (!aliases.is_empty()).then_some(aliases);
        (
            Info {
                value,
                problems,
                sources: std::iter::once(source.clone())
                    .chain(sets.iter().map(|set| set.definition.source.clone()))
                    .collect(),
            },
            sets.iter().map(|set| set.definition.clone()).collect(),
        )
    }

    /// CCA names a command conversion whose CCS points carry the declared raw and engineering
    /// formats. Several CCA rows for one key keep every conversion with the ambiguity attached,
    /// and a declared CCA_NCURVE that disagrees with the retained points keeps both declarations.
    fn conversion_rules(
        &self,
        key: Option<&str>,
        source: &Source,
    ) -> Info<Vec<CalibrationAlternative>> {
        let Some(key) = key else {
            return info(Some(Vec::new()), source);
        };
        let rows: Vec<_> = self
            .related(Table::Cca, key.into())
            .iter()
            .map(|id| &self.records.cca.rows()[id.0])
            .collect();
        if rows.is_empty() {
            return missing(reference(Table::Cca, key), source);
        }
        let problem = ambiguity(
            reference(Table::Cca, key),
            source,
            rows.iter()
                .map(|row| target(Table::Cca, key, row))
                .collect(),
            "Command conversion reference has multiple targets; every available definition is retained",
        );
        let value = rows
            .iter()
            .map(|row| CalibrationAlternative {
                condition: None,
                selection: None,
                calibration: Info {
                    value: Some(self.command_conversion(row)),
                    problems: problem.iter().cloned().collect(),
                    sources: vec![source.clone(), row.definition.source.clone()],
                },
            })
            .collect();
        Info {
            value: Some(value),
            problems: vec![],
            sources: std::iter::once(source.clone())
                .chain(rows.iter().map(|row| row.definition.source.clone()))
                .collect(),
        }
    }

    fn command_conversion(&self, row: &Row<Cca>) -> Calibration {
        let cells = &row.cells;
        let key = cells
            .numbr
            .value
            .as_deref()
            .expect("retained CCA has a required number");
        let source = &row.definition.source;
        let rows = self.related(Table::Ccs, key.into());
        let points = if rows.is_empty() {
            missing(reference(Table::Ccs, key), source)
        } else {
            let mut problems = Vec::new();
            if let Some(declared) = cells.ncurve.value
                && u64::try_from(declared).ok() != Some(rows.len() as u64)
            {
                problems.push(count_disagreement(
                    "CCA_NCURVE",
                    declared,
                    "CCS_NUMBR",
                    rows.len(),
                    std::iter::once(target(Table::Cca, key, row))
                        .chain(
                            rows.iter()
                                .map(|id| target(Table::Ccs, key, &self.records.ccs.rows()[id.0])),
                        )
                        .collect(),
                ));
            }
            Info {
                value: Some(
                    rows.iter()
                        .map(|id| {
                            let point = &self.records.ccs.rows()[id.0];
                            CalibrationPoint {
                                raw: number(
                                    &point.cells.xvals,
                                    cells.rawfmt.value.as_deref(),
                                    cells.radix.value.as_deref(),
                                ),
                                engineering: number(
                                    &point.cells.yvals,
                                    cells.engfmt.value.as_deref(),
                                    Some("D"),
                                ),
                                definition: point.definition.clone(),
                            }
                        })
                        .collect(),
                ),
                problems,
                sources: vec![source.clone()],
            }
        };
        Calibration {
            reference: reference(Table::Cca, key),
            definition: row.definition.clone(),
            // CCA declares no interpolation, and CPC_INTER names a raw or engineering input
            // rather than an extrapolation rule, so none is inferred.
            form: info(
                Some(CalibrationForm::CommandConversion {
                    points,
                    interpolation: info(None, source),
                }),
                source,
            ),
        }
    }
}

/// PRF_DSPFMT declares a command input format. `A`, `T` and `D` name a character or time token
/// that has no numeric reduction, so those bounds retain their recorded text as a text scalar.
/// The numeric formats interpret the bound with the shared format and radix rules.
fn bound(cell: &Info<String>, format: Option<&str>, radix: Option<&str>) -> Info<Scalar> {
    match (format, cell.value.as_deref()) {
        (Some("A" | "T" | "D"), Some(text)) => Info {
            value: Some(Scalar::Text(text.to_owned())),
            problems: cell.problems.clone(),
            sources: cell.sources.clone(),
        },
        _ => number(cell, format, radix),
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

/// One declared CDF group: how it repeats, and the position rule its members follow.
struct DeclaredGroup {
    repetition: Info<Repetition>,
    /// The unexpanded-position rule shared by every member and following element.
    position: RuntimeDeclaration,
}

/// Every element inside or after a repeated group depends on that group's runtime expansion,
/// so each one keeps the group's position rule in the location's constraints.
fn constrain(
    element: &mut CommandElement,
    enclosing: &[RuntimeDeclaration],
    preceding: &[RuntimeDeclaration],
) {
    let location = element_location(element);
    for declaration in enclosing.iter().chain(preceding) {
        if !location.constraints.contains(declaration) {
            location.constraints.push(declaration.clone());
        }
    }
}

/// Duplicate declared positions keep every element and name every competing definition.
fn duplicate_positions(rows: &[&Row<Cdf>]) -> HashMap<Source, Problem> {
    let mut duplicates = HashMap::new();
    for group in rows.chunk_by(|a, b| a.cells.bit.value == b.cells.bit.value) {
        if group.len() < 2 {
            continue;
        }
        let problem = Problem {
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
        };
        for row in group {
            duplicates.insert(row.definition.source.clone(), problem.clone());
        }
    }
    duplicates
}

/// The declaration a group and its members report: the repeater, its declared group size and
/// every element the group covers.
fn group_problem(row: &Row<Cdf>, size: i64, members: &[&Row<Cdf>], explanation: &str) -> Problem {
    Problem {
        kind: ProblemKind::InconsistentDefinition {
            fields: vec!["CDF_GRPSIZE".into()],
            values: vec![Scalar::Integer(size)],
            available: std::iter::once(cdf_target(row))
                .chain(members.iter().map(|member| cdf_target(member)))
                .collect(),
        },
        sources: vec![row.definition.source.clone()],
        explanation: explanation.into(),
    }
}

/// A declared group size that cannot declare any group: fewer than one element, or nested deeper
/// than any usable declaration. The element and its position stay available.
fn invalid_group(row: &Row<Cdf>, size: i64, too_deep: bool) -> Problem {
    Problem {
        kind: ProblemKind::InconsistentDefinition {
            fields: vec!["CDF_GRPSIZE".into()],
            values: vec![Scalar::Integer(size)],
            available: vec![cdf_target(row)],
        },
        sources: vec![row.definition.source.clone()],
        explanation: if too_deep {
            format!(
                "CDF_GRPSIZE={size} nests groups deeper than the supported depth; the retained elements stay declared at this level"
            )
        } else {
            format!("CDF_GRPSIZE={size} cannot declare a group of fewer than one element")
        },
    }
}

/// How a repeater element's value arrives at command invocation, as CDF_INTER and CDF_ELTYPE
/// declare it. The element's own value rules already interpret the recorded default.
fn repetition_source(row: &Row<Cdf>, argument: &CommandArgument) -> String {
    let key = row.cells.pname.value.clone().unwrap_or_default();
    match argument
        .rules
        .element_value
        .value
        .as_ref()
        .map(|v| &v.source)
    {
        Some(ValueSource::Telemetry { parameter, .. }) => {
            format!("the value of monitoring parameter {}", parameter.0)
        }
        Some(ValueSource::Runtime(_)) => {
            format!("the value an operator supplies for {key} at command invocation")
        }
        Some(ValueSource::Literal(value)) if row.cells.eltype.value.as_deref() == Some("E") => {
            format!(
                "the recorded value {} of {key}, which an operator may replace at command invocation",
                scalar_text(value)
            )
        }
        Some(ValueSource::Literal(value)) => {
            format!("the recorded value {} of {key}", scalar_text(value))
        }
        None => format!("the declared value of {key}"),
    }
}

/// A recorded value named inside a declaration. CLI rendering quotes values, but a declaration
/// sentence keeps the recorded text and lets the renderer escape it.
fn scalar_text(value: &Scalar) -> String {
    match value {
        Scalar::Text(text) | Scalar::Decimal(text) | Scalar::Code(text) => text.clone(),
        Scalar::Integer(number) => number.to_string(),
        Scalar::Unsigned(number) => number.to_string(),
        Scalar::Boolean(value) => value.to_string(),
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
