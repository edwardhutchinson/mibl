use super::*;
use crate::reader::{Pid, TableLoad, Vpd};

#[derive(Clone)]
struct Cursor {
    bits: Option<i128>,
    relative: bool,
    constraints: Vec<RuntimeDeclaration>,
    problems: Vec<Problem>,
    sources: Vec<Source>,
}

impl Catalog {
    pub(super) fn variable_layout(
        &self,
        packet: &Row<Pid>,
    ) -> Info<Vec<Layout<ParameterOccurrence>>> {
        let bits = packet
            .cells
            .dfhsize
            .value
            .filter(|n| (0..=99).contains(n))
            .map(|n| i128::from(n) * 8);
        self.variable_structure(
            packet.cells.tpsd.value.unwrap(),
            bits,
            &packet.definition.source,
        )
    }

    fn variable_structure(
        &self,
        tpsd: i64,
        bits: Option<i128>,
        source: &Source,
    ) -> Info<Vec<Layout<ParameterOccurrence>>> {
        let key = tpsd.to_string();
        let mut rows: Vec<_> = self
            .related(Table::Vpd, key.clone())
            .iter()
            .map(|id| &self.records.vpd.rows()[id.0])
            .collect();
        if !matches!(self.records.vpd, TableLoad::Read { .. }) || rows.is_empty() {
            return missing(
                Reference::Supporting {
                    table: Table::Vpd,
                    key,
                },
                source,
            );
        }
        rows.sort_by_key(|r| (r.cells.pos.value, r.definition.source.clone()));
        let mut cursor = Cursor {
            bits,
            relative: false,
            constraints: vec![],
            problems: vec![],
            sources: vec![source.clone()],
        };
        let duplicates = duplicate_positions(&rows);
        let layout = self.variable_group(&rows, &mut cursor, &[], &duplicates);
        info(Some(layout), source)
    }

    pub(super) fn add_variable_occurrences(
        &self,
        name: &ParameterName,
        result: &mut Info<Vec<PacketOccurrences>>,
    ) {
        for ((table, key), ids) in &self.supporting {
            if *table == Table::Pid && self.related(Table::Vpd, key.clone()).is_empty() {
                let source = &self.records.pid.rows()[ids[0].0].definition.source;
                result.problems.extend(
                    missing::<()>(
                        Reference::Supporting {
                            table: Table::Vpd,
                            key: key.clone(),
                        },
                        source,
                    )
                    .problems,
                );
            }
        }
        result.problems.sort_by_key(|p| p.sources.clone());
        if !matches!(self.records.vpd, TableLoad::Read { .. }) {
            return;
        }
        let tpsds: std::collections::BTreeSet<_> = self
            .related(Table::Vpd, format!("name:{}", name.0))
            .iter()
            .map(|id| self.records.vpd.rows()[id.0].cells.tpsd.value.unwrap())
            .collect();
        let mut grouped = std::collections::BTreeMap::<PacketSpid, Vec<ParameterOccurrence>>::new();
        for tpsd in tpsds {
            let packets = self.related(Table::Pid, tpsd.to_string());
            if packets.is_empty() {
                let row = &self.records.vpd.rows()[self.related(Table::Vpd, tpsd.to_string())[0].0];
                let source = &row.definition.source;
                let mut occurrences = Vec::new();
                if let Some(layout) = self.variable_structure(tpsd, None, source).value {
                    collect_occurrences(layout, name, &mut occurrences);
                }
                if !occurrences.is_empty() {
                    result
                        .value
                        .get_or_insert_with(Vec::new)
                        .push(PacketOccurrences {
                            packet: missing(
                                Reference::Supporting {
                                    table: Table::Pid,
                                    key: format!("TPSD {tpsd}"),
                                },
                                source,
                            ),
                            occurrences,
                        });
                }
            }
            for id in packets {
                let packet = &self.records.pid.rows()[id.0];
                let layout = self.variable_layout(packet);
                let occurrences = grouped.entry(packet.cells.spid.value.unwrap()).or_default();
                if let Some(layout) = layout.value {
                    collect_occurrences(layout, name, occurrences);
                }
                result.problems.extend(layout.problems);
            }
        }
        let entries = result.value.get_or_insert_with(Vec::new);
        for (spid, occurrences) in grouped {
            if occurrences.is_empty() {
                continue;
            }
            let rows: Vec<_> = self.packets[&spid]
                .iter()
                .map(|id| &self.records.pid.rows()[id.0])
                .collect();
            let packet = resolve(
                &rows,
                Reference::Root(Identity::Packet(spid)),
                &occurrences[0].definition.source,
                |r| Some(self.packet_summary(r)),
            );
            if let Some(existing) = entries.iter_mut().find(|e| {
                e.packet
                    .sources
                    .iter()
                    .any(|s| rows.iter().any(|r| &r.definition.source == s))
            }) {
                existing.occurrences.extend(occurrences);
            } else {
                entries.push(PacketOccurrences {
                    packet,
                    occurrences,
                });
            }
        }
        entries.sort_by_key(|e| {
            e.packet.value.as_ref().map(|p| p.spid).or_else(|| {
                e.packet.problems.iter().find_map(|p| match &p.kind {
                    ProblemKind::MissingReference {
                        reference: Reference::Root(Identity::Packet(spid)),
                    }
                    | ProblemKind::AmbiguousReference {
                        reference: Reference::Root(Identity::Packet(spid)),
                        ..
                    } => Some(*spid),
                    _ => None,
                })
            })
        });
    }

    fn variable_group(
        &self,
        rows: &[&Row<Vpd>],
        cursor: &mut Cursor,
        enclosing: &[Enclosure],
        duplicates: &HashMap<Source, Problem>,
    ) -> Vec<Layout<ParameterOccurrence>> {
        let mut layout = Vec::new();
        let mut index = 0;
        while index < rows.len() {
            let row = rows[index];
            let source = &row.definition.source;
            if let Some(problem) = duplicates.get(source) {
                cursor.bits = None;
                cursor.problems.push(problem.clone());
            }
            let size = row.cells.grpsize.value.unwrap();
            let fixed = row.cells.fixrep.value.unwrap();
            if size > 0 && enclosing.len() < 64 {
                // GRPSIZE counts following physical records, including nested markers.
                let end = index
                    .saturating_add(1)
                    .saturating_add(size as usize)
                    .min(rows.len());
                if fixed == 0 {
                    layout.push(Layout::Element(
                        self.variable_occurrence(row, cursor, enclosing),
                    ));
                }
                let declaration = self.variable_declaration(
                    row,
                    format!(
                        "Repeat group {}",
                        if fixed > 0 {
                            format!("with fixed count {fixed}")
                        } else {
                            format!(
                                "using value of {}",
                                row.cells.name.value.as_ref().unwrap().0
                            )
                        }
                    ),
                );
                let mut repetition = if fixed > 0 {
                    info(
                        Some(Repetition::Fixed {
                            count: fixed as u64,
                            stride_bits: info(None, source),
                        }),
                        source,
                    )
                } else if fixed == 0 {
                    runtime_info(Some(Repetition::Runtime(declaration.clone())), &declaration)
                } else {
                    unavailable(source, "VPD_FIXREP negative extension is unsupported")
                };
                let complete = end - index - 1 == size as usize;
                if !complete {
                    repetition.problems.push(group_problem(
                        row,
                        "VPD_GRPSIZE extends beyond its enclosing group",
                    ));
                }
                let mut nested = enclosing.to_vec();
                nested.push(Enclosure::Repetition(repetition.clone()));
                let mut child_cursor = Cursor {
                    bits: Some(0),
                    relative: true,
                    constraints: vec![self.variable_declaration(
                        row,
                        format!("Location relative to the start of each repetition; group begins at {}{} bits", if cursor.relative { "relative " } else { "packet " }, cursor.bits.map_or_else(|| "runtime-dependent".into(), |n| n.to_string())),
                    )],
                    problems: cursor.problems.clone(),
                    sources: cursor.sources.clone(),
                };
                let mut children = self.variable_group(
                    &rows[index + 1..end],
                    &mut child_cursor,
                    &nested,
                    duplicates,
                );
                let stride = child_cursor.bits.and_then(|n| u64::try_from(n).ok());
                if let Some(Repetition::Fixed { stride_bits, .. }) = &mut repetition.value {
                    *stride_bits = if let Some(stride) = stride {
                        info(Some(stride), source)
                    } else {
                        unavailable(
                            source,
                            "Repetition stride depends on child widths or runtime structure",
                        )
                    };
                }
                update_enclosure(&mut children, enclosing.len(), &repetition);
                cursor.bits = if complete && fixed > 0 {
                    cursor.bits.zip(stride).and_then(|(start, stride)| {
                        start.checked_add(i128::from(stride) * i128::from(fixed))
                    })
                } else {
                    None
                };
                if cursor.bits.is_none() {
                    cursor.constraints.push(declaration.clone());
                    cursor.problems.extend(repetition.problems.clone());
                }
                layout.push(Layout::Repeat {
                    definition: row.definition.clone(),
                    repetition,
                    children,
                });
                index = end;
            } else {
                let mut occurrence = self.variable_occurrence(row, cursor, enclosing);
                if size != 0 || fixed != 0 {
                    let problem =
                        group_problem(row, "Invalid or unsupported VPD repetition structure");
                    occurrence.location.position.problems.push(problem.clone());
                    cursor.problems.push(problem);
                    cursor.bits = None;
                }
                layout.push(Layout::Element(occurrence));
                index += 1;
            }
            if row.cells.choice.value.as_deref() == Some("Y") {
                let condition = self.variable_declaration(
                    row,
                    format!(
                        "Value of {} selects the following VPD TPSD structure",
                        row.cells.name.value.as_ref().unwrap().0
                    ),
                );
                cursor.bits = None;
                cursor.constraints.push(condition.clone());
                layout.push(Layout::Conditional {
                    definition: row.definition.clone(),
                    condition,
                    children: vec![],
                });
            }
        }
        layout
    }

    fn variable_declaration(&self, row: &Row<Vpd>, expression: String) -> RuntimeDeclaration {
        let name = row.cells.name.value.as_ref().unwrap();
        self.parameter_declaration(name, &row.definition.source, expression)
    }

    fn parameter_declaration(
        &self,
        name: &ParameterName,
        source: &Source,
        expression: String,
    ) -> RuntimeDeclaration {
        let reference = Reference::Root(Identity::Parameter(name.clone()));
        let rows: Vec<_> = self
            .parameters
            .get(name)
            .into_iter()
            .flatten()
            .map(|id| &self.records.pcf.rows()[id.0])
            .collect();
        let mut targets = resolve(&rows, reference.clone(), source, |r| {
            Some(vec![Target {
                reference: reference.clone(),
                definition: r.definition.clone(),
            }])
        });
        if !rows.is_empty() {
            targets.value = Some(
                rows.iter()
                    .map(|r| Target {
                        reference: reference.clone(),
                        definition: r.definition.clone(),
                    })
                    .collect(),
            );
        }
        RuntimeDeclaration {
            expression,
            dependencies: vec![Dependency { reference, targets }],
            sources: vec![source.clone()],
        }
    }

    fn variable_occurrence(
        &self,
        row: &Row<Vpd>,
        cursor: &mut Cursor,
        enclosing: &[Enclosure],
    ) -> ParameterOccurrence {
        let source = &row.definition.source;
        let name = row.cells.name.value.as_ref().unwrap();
        let rows: Vec<_> = self
            .parameters
            .get(name)
            .into_iter()
            .flatten()
            .map(|id| &self.records.pcf.rows()[id.0])
            .collect();
        let parameter = resolve(
            &rows,
            Reference::Root(Identity::Parameter(name.clone())),
            source,
            |r| Some(describe_parameter(r).parameter),
        );
        let widths: Vec<_> = rows
            .iter()
            .map(|p| describe_parameter(p).parameter.encoding.encoded_bits)
            .collect();
        let mut encoded_bits = consensus_width(&widths, &parameter);
        if let [p] = rows.as_slice()
            && p.cells.ptc.value == Some(11)
        {
            let related = ParameterName(p.cells.related.value.clone().unwrap_or_default());
            let declaration = self.parameter_declaration(
                &related,
                &p.definition.source,
                format!(
                    "Encoded type and width use PCF_PID selected by {} (PCF_RELATED)",
                    related.0
                ),
            );
            encoded_bits = runtime_info(None, &declaration);
        }
        let slots: Vec<_> = rows
            .iter()
            .zip(&widths)
            .map(|(p, width)| {
                if p.cells.width.value.is_some() {
                    unsigned(&p.cells.width, "PCF_WIDTH")
                } else {
                    width.clone()
                }
            })
            .collect();
        let padded = consensus_width(&slots, &parameter);
        let padding_conflict = padded
            .value
            .zip(encoded_bits.value)
            .is_some_and(|(slot, width)| slot < width);
        let start = cursor
            .bits
            .and_then(|n| n.checked_add(i128::from(row.cells.offset.value.unwrap())));
        let end = start
            .zip(padded.value)
            .and_then(|(n, w)| n.checked_add(i128::from(w)));
        let value_start = end
            .zip(encoded_bits.value)
            .filter(|_| !padding_conflict)
            .map(|(n, w)| n - i128::from(w));
        let value = value_start.and_then(|n| {
            if cursor.relative {
                i64::try_from(n).ok().map(Position::RelativeBits)
            } else {
                u64::try_from(n).ok().map(|n| Position::PacketAbsolute {
                    byte: n / 8,
                    bit: (n % 8) as u8,
                })
            }
        });
        let mut position = if value.is_some() {
            info(value, source)
        } else {
            unavailable(
                source,
                "Variable position requires a usable preceding location and width",
            )
        };
        position.sources.extend(cursor.sources.clone());
        position.sources.extend(parameter.sources.clone());
        position.sources.sort();
        position.sources.dedup();
        position.problems.extend(padded.problems.clone());
        if padding_conflict {
            position.problems.push(Problem {
                kind: ProblemKind::InconsistentDefinition {
                    fields: vec!["PCF_WIDTH".into(), "PCF_PTC".into(), "PCF_PFC".into()],
                    values: vec![Scalar::Unsigned(padded.value.unwrap()), Scalar::Unsigned(encoded_bits.value.unwrap())],
                    available: rows.iter().map(|p| Target { reference: Reference::Root(Identity::Parameter(name.clone())), definition: p.definition.clone() }).collect(),
                },
                sources: parameter.sources.clone(),
                explanation: "PCF padded width is smaller than the encoded value; the declared slot end remains usable".into(),
            });
        }
        if encoded_bits.value.is_none() {
            position.problems.extend(encoded_bits.problems.clone());
        }
        position.problems.extend(cursor.problems.clone());
        for declaration in &cursor.constraints {
            position
                .problems
                .extend(runtime_info::<()>(None, declaration).problems);
        }
        let mut constraints = cursor.constraints.clone();
        if row.cells.pidref.value.as_deref() == Some("Y") {
            constraints.push(self.variable_declaration(
                row,
                format!("Value of {} identifies a parameter by PCF_PID", name.0),
            ));
        }
        let occurrence = ParameterOccurrence {
            reference: name.clone(),
            parameter,
            definition: row.definition.clone(),
            location: Location {
                position,
                encoded_bits,
                constraints,
            },
            enclosing: enclosing.to_vec(),
        };
        cursor.sources.extend(occurrence.parameter.sources.clone());
        cursor.sources.sort();
        cursor.sources.dedup();
        cursor.bits = end;
        if end.is_none() {
            cursor.problems.extend(padded.problems.clone());
        }
        if end.is_none() {
            cursor.constraints.push(self.variable_declaration(
                row,
                format!("Location follows the padded field for {}", name.0),
            ));
        }
        occurrence
    }
}

fn runtime_info<T>(value: Option<T>, declaration: &RuntimeDeclaration) -> Info<T> {
    Info {
        value,
        sources: declaration.sources.clone(),
        problems: vec![Problem {
            kind: ProblemKind::RuntimeDependent {
                declaration: declaration.clone(),
            },
            sources: declaration.sources.clone(),
            explanation: declaration.expression.clone(),
        }],
    }
}
fn group_problem(row: &Row<Vpd>, explanation: &str) -> Problem {
    Problem {
        kind: ProblemKind::InconsistentDefinition {
            fields: vec!["VPD_GRPSIZE".into(), "VPD_FIXREP".into()],
            values: vec![
                Scalar::Integer(row.cells.grpsize.value.unwrap()),
                Scalar::Integer(row.cells.fixrep.value.unwrap()),
            ],
            available: vec![Target {
                reference: Reference::Supporting {
                    table: Table::Vpd,
                    key: row.cells.tpsd.value.unwrap().to_string(),
                },
                definition: row.definition.clone(),
            }],
        },
        sources: vec![row.definition.source.clone()],
        explanation: explanation.into(),
    }
}
fn update_enclosure(
    layout: &mut [Layout<ParameterOccurrence>],
    depth: usize,
    repetition: &Info<Repetition>,
) {
    for node in layout {
        match node {
            Layout::Element(o) => o.enclosing[depth] = Enclosure::Repetition(repetition.clone()),
            Layout::Repeat { children, .. } | Layout::Conditional { children, .. } => {
                update_enclosure(children, depth, repetition)
            }
        }
    }
}

fn collect_occurrences(
    layout: Vec<Layout<ParameterOccurrence>>,
    name: &ParameterName,
    result: &mut Vec<ParameterOccurrence>,
) {
    for node in layout {
        match node {
            Layout::Element(o) if &o.reference == name => result.push(o),
            Layout::Repeat { children, .. } | Layout::Conditional { children, .. } => {
                collect_occurrences(children, name, result)
            }
            _ => {}
        }
    }
}

fn duplicate_positions(rows: &[&Row<Vpd>]) -> HashMap<Source, Problem> {
    let mut result = HashMap::new();
    let mut start = 0;
    while start < rows.len() {
        let pos = rows[start].cells.pos.value;
        let end = (start + 1..rows.len())
            .find(|i| rows[*i].cells.pos.value != pos)
            .unwrap_or(rows.len());
        if end - start > 1 {
            let problem = Problem {
                kind: ProblemKind::InconsistentDefinition { fields: vec!["VPD_POS".into()], values: vec![Scalar::Integer(pos.unwrap())], available: rows[start..end].iter().map(|r| Target { reference: Reference::Supporting { table: Table::Vpd, key: r.cells.tpsd.value.unwrap().to_string() }, definition: r.definition.clone() }).collect() },
                sources: rows[start..end].iter().map(|r| r.definition.source.clone()).collect(),
                explanation: "Duplicate VPD positions leave extraction order inconsistent; source order is retained for display".into(),
            };
            for row in &rows[start..end] {
                result.insert(row.definition.source.clone(), problem.clone());
            }
        }
        start = end;
    }
    result
}

fn consensus_width(widths: &[Info<u64>], parameter: &Info<ParameterSummary>) -> Info<u64> {
    let value = widths.first().and_then(|first| {
        first
            .value
            .filter(|_| widths.iter().all(|w| w.value == first.value))
    });
    Info {
        value,
        sources: parameter.sources.clone(),
        problems: parameter
            .problems
            .iter()
            .cloned()
            .chain(widths.iter().flat_map(|w| w.problems.clone()))
            .collect(),
    }
}
