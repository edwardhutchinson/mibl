use super::*;
use crate::reader::{Pid, Plf};

impl Catalog {
    pub(super) fn index_packets(&mut self) {
        for (i, row) in self.records.pid.rows().iter().enumerate() {
            if row.cells.tpsd.value != Some(-1) && self.variable_packet_source.is_none() {
                self.variable_packet_source = Some(row.definition.source.clone());
            }
            self.packets
                .entry(row.cells.spid.value.unwrap())
                .or_default()
                .push(RowId(i));
        }
        for (i, row) in self.records.tpcf.rows().iter().enumerate() {
            self.supporting
                .entry((Table::Tpcf, row.cells.spid.value.unwrap().0.to_string()))
                .or_default()
                .push(RowId(i));
        }
        for (i, row) in self.records.pic.rows().iter().enumerate() {
            self.supporting
                .entry((
                    Table::Pic,
                    pic_key(row.cells.r#type.value, row.cells.stype.value),
                ))
                .or_default()
                .push(RowId(i));
        }
        for (i, row) in self.records.plf.rows().iter().enumerate() {
            self.supporting
                .entry((
                    Table::Plf,
                    format!("spid:{}", row.cells.spid.value.unwrap().0),
                ))
                .or_default()
                .push(RowId(i));
            self.supporting
                .entry((
                    Table::Plf,
                    format!("name:{}", row.cells.name.value.as_ref().unwrap().0),
                ))
                .or_default()
                .push(RowId(i));
        }
    }

    fn related(&self, table: Table, key: String) -> &[RowId] {
        self.supporting
            .get(&(table, key))
            .map_or(&[], Vec::as_slice)
    }

    fn packet_summary(&self, row: &Row<Pid>) -> PacketSummary {
        let spid = row.cells.spid.value.unwrap();
        let reference = Reference::Supporting {
            table: Table::Tpcf,
            key: spid.0.to_string(),
        };
        let rows: Vec<_> = self
            .related(Table::Tpcf, spid.0.to_string())
            .iter()
            .map(|id| &self.records.tpcf.rows()[id.0])
            .collect();
        let name = resolve(&rows, reference.clone(), &row.definition.source, |r| {
            r.cells.name.value.clone()
        });
        let characteristics = if rows.is_empty() {
            missing(reference, &row.definition.source)
        } else {
            info(
                Some(rows.iter().map(|r| r.definition.clone()).collect()),
                &row.definition.source,
            )
        };
        PacketSummary {
            spid,
            name,
            characteristics,
            description: row.cells.descr.clone(),
            definition: row.definition.clone(),
        }
    }

    fn packet_candidate(&self, row: &Row<Pid>) -> Candidate {
        let summary = self.packet_summary(row);
        Candidate {
            identity: Identity::Packet(summary.spid),
            name: summary.name,
            description: summary.description,
            source: summary.definition.source,
        }
    }

    pub(crate) fn packet(&self, spid: PacketSpid) -> Lookup<PacketDescription> {
        tracing::debug!(spid = spid.0, "exact packet lookup");
        match self.packets.get(&spid).map(Vec::as_slice) {
            Some([id]) => {
                let row = &self.records.pid.rows()[id.0];
                Lookup::Found(PacketDescription {
                    packet: self.packet_summary(row),
                    identification: self.identification(row),
                    layout: self.fixed_layout(row),
                })
            }
            Some([first, second, rest @ ..]) => Lookup::Ambiguous(AtLeastTwo {
                first: Box::new(self.packet_candidate(&self.records.pid.rows()[first.0])),
                second: Box::new(self.packet_candidate(&self.records.pid.rows()[second.0])),
                rest: rest
                    .iter()
                    .map(|id| self.packet_candidate(&self.records.pid.rows()[id.0]))
                    .collect(),
            }),
            _ => Lookup::NotFound(if self.packets.is_empty() {
                NotFoundReason::DefinitionsUnavailable
            } else {
                NotFoundReason::NoMatchingIdentity
            }),
        }
    }

    fn identification(&self, row: &Row<Pid>) -> PacketIdentification {
        let p = &row.cells;
        let source = &row.definition.source;
        let key = pic_key(p.r#type.value, p.stype.value);
        let rows: Vec<_> = self
            .related(Table::Pic, key.clone())
            .iter()
            .map(|id| &self.records.pic.rows()[id.0])
            .filter(|r| r.cells.apid.value == p.apid.value || r.cells.apid.value == Some(99999))
            .collect();
        let reference = Reference::Supporting {
            table: Table::Pic,
            key,
        };
        let mut criteria = resolve(&rows, reference.clone(), source, |_| Some(Vec::new()));
        let mut values = Vec::new();
        let definitions: Vec<_> = rows.iter().map(|r| r.definition.clone()).collect();
        for (expected, offset_field, width_field, cells) in [
            (
                &p.pi1_val,
                "PIC_PI1_OFF",
                "PIC_PI1_WID",
                rows.iter()
                    .map(|r| (&r.cells.pi1_off, &r.cells.pi1_wid))
                    .collect::<Vec<_>>(),
            ),
            (
                &p.pi2_val,
                "PIC_PI2_OFF",
                "PIC_PI2_WID",
                rows.iter()
                    .map(|r| (&r.cells.pi2_off, &r.cells.pi2_wid))
                    .collect::<Vec<_>>(),
            ),
        ] {
            let (offsets, widths): (Vec<_>, Vec<_>) = cells.into_iter().unzip();
            if !offsets.is_empty() && offsets.iter().all(|offset| offset.value == Some(-1)) {
                continue;
            }
            let offset = reconcile_cells(&offsets, &definitions, &reference, source, offset_field);
            let width = reconcile_cells(&widths, &definitions, &reference, source, width_field);
            let mut position = packet_position(&offset, &info(Some(0), source));
            position.problems.extend(offset.problems);
            values.push(IdentificationCriterion {
                expected: unsigned(expected, "PID_PI value"),
                extraction: Location {
                    position,
                    encoded_bits: unsigned(&width, "PIC width"),
                    constraints: vec![],
                },
                definitions: std::iter::once(row.definition.clone())
                    .chain(definitions.clone())
                    .collect(),
            });
        }
        criteria.value = Some(values);
        PacketIdentification {
            definitions,
            apid: unsigned(&p.apid, "PID_APID"),
            service_type: unsigned(&p.r#type, "PID_TYPE"),
            service_subtype: unsigned(&p.stype, "PID_STYPE"),
            criteria,
        }
    }

    fn fixed_layout(&self, row: &Row<Pid>) -> Info<Vec<Layout<ParameterOccurrence>>> {
        let source = &row.definition.source;
        if row.cells.tpsd.value != Some(-1) {
            return unavailable(source, "Variable packet layouts are not implemented yet");
        }
        let rows: Vec<_> = self
            .related(
                Table::Plf,
                format!("spid:{}", row.cells.spid.value.unwrap().0),
            )
            .iter()
            .map(|id| &self.records.plf.rows()[id.0])
            .collect();
        if !matches!(self.records.plf, crate::reader::TableLoad::Read { .. }) {
            return missing(
                Reference::Supporting {
                    table: Table::Plf,
                    key: row.cells.spid.value.unwrap().0.to_string(),
                },
                source,
            );
        }
        info(
            Some(
                self.ordered_occurrences(&rows)
                    .into_iter()
                    .map(Layout::Element)
                    .collect(),
            ),
            source,
        )
    }

    fn ordered_occurrences(&self, rows: &[&Row<Plf>]) -> Vec<ParameterOccurrence> {
        let mut occurrences = Vec::new();
        for row in rows {
            let mut occurrence = self.occurrence(row);
            let count = row.cells.nbocc.value.unwrap();
            let stride = unsigned::<u64>(&row.cells.lgocc, "PLF_LGOCC");
            if !(1..=9999).contains(&count) {
                occurrence.location.position.problems.extend(
                    unavailable::<()>(&row.definition.source, "PLF_NBOCC must be in 1..=9999")
                        .problems,
                );
                occurrences.push(occurrence);
                continue;
            }
            if count > 1 {
                occurrence.enclosing.push(Enclosure::Repetition(info(
                    Some(Repetition::Fixed {
                        count: count as u64,
                        stride_bits: stride.clone(),
                    }),
                    &row.definition.source,
                )));
            }
            for index in 0..count as u64 {
                let mut expanded = occurrence.clone();
                if index > 0 {
                    expanded.location.position =
                        match (position_key(&occurrence.location.position), stride.value) {
                            (Some((byte, bit)), Some(stride)) => {
                                // Wide intermediate arithmetic keeps every independently representable byte offset.
                                let bits = u128::from(byte) * 8
                                    + u128::from(bit)
                                    + u128::from(index) * u128::from(stride);
                                match u64::try_from(bits / 8) {
                                    Ok(byte) => info(
                                        Some(Position::PacketAbsolute {
                                            byte,
                                            bit: (bits % 8) as u8,
                                        }),
                                        &row.definition.source,
                                    ),
                                    Err(_) => unavailable(
                                        &row.definition.source,
                                        "Repeated position exceeds the supported byte range",
                                    ),
                                }
                            }
                            _ => unavailable(
                                &row.definition.source,
                                "Repeated position requires a usable start and stride",
                            ),
                        };
                }
                occurrences.push(expanded);
            }
        }
        occurrences.sort_by_key(|o| {
            (
                position_key(&o.location.position),
                o.definition.source.clone(),
            )
        });
        let mut start = 0;
        while start < occurrences.len() {
            let key = position_key(&occurrences[start].location.position);
            let end = (start + 1..occurrences.len())
                .find(|i| position_key(&occurrences[*i].location.position) != key)
                .unwrap_or(occurrences.len());
            if let Some((byte, bit)) = key
                && end - start > 1
            {
                let sources = occurrences[start..end]
                    .iter()
                    .map(|o| o.definition.source.clone())
                    .collect();
                let available = occurrences[start..end]
                    .iter()
                    .map(|o| Target {
                        reference: Reference::Supporting {
                            table: Table::Plf,
                            key: o.reference.0.clone(),
                        },
                        definition: o.definition.clone(),
                    })
                    .collect();
                let problem = Problem {
                    kind: ProblemKind::InconsistentDefinition {
                        fields: vec!["PLF_OFFBY".into(), "PLF_OFFBI".into(), "PLF_LGOCC".into()],
                        values: vec![Scalar::Unsigned(byte), Scalar::Unsigned(u64::from(bit))],
                        available,
                    },
                    sources,
                    explanation: "Multiple fixed occurrences occupy the same position".into(),
                };
                for o in &mut occurrences[start..end] {
                    o.location.position.problems.push(problem.clone());
                }
            }
            start = end;
        }
        occurrences
    }

    fn occurrence(&self, row: &Row<Plf>) -> ParameterOccurrence {
        let p = &row.cells;
        let source = &row.definition.source;
        let name = p.name.value.as_ref().unwrap();
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
        let encoded_bits = parameter.value.as_ref().map_or_else(
            || Info {
                value: None,
                problems: parameter.problems.clone(),
                sources: parameter.sources.clone(),
            },
            |p| p.encoding.encoded_bits.clone(),
        );
        ParameterOccurrence {
            reference: name.clone(),
            parameter,
            definition: row.definition.clone(),
            location: Location {
                position: packet_position(&p.offby, &p.offbi),
                encoded_bits,
                constraints: vec![],
            },
            enclosing: vec![],
        }
    }

    pub(super) fn parameter_occurrences(
        &self,
        name: &ParameterName,
    ) -> Info<Vec<PacketOccurrences>> {
        let rows = self.related(Table::Plf, format!("name:{}", name.0));
        let source = self.records.pcf.rows()[self.parameters[name][0].0]
            .definition
            .source
            .clone();
        if !matches!(self.records.plf, crate::reader::TableLoad::Read { .. }) {
            return missing(
                Reference::Supporting {
                    table: Table::Plf,
                    key: name.0.clone(),
                },
                &source,
            );
        }
        let mut grouped = std::collections::BTreeMap::<PacketSpid, Vec<&Row<Plf>>>::new();
        for id in rows {
            let row = &self.records.plf.rows()[id.0];
            grouped
                .entry(row.cells.spid.value.unwrap())
                .or_default()
                .push(row);
        }
        let mut result = info(
            Some(
                grouped
                    .into_iter()
                    .map(|(spid, rows)| {
                        let packets: Vec<_> = self
                            .packets
                            .get(&spid)
                            .into_iter()
                            .flatten()
                            .map(|id| &self.records.pid.rows()[id.0])
                            .collect();
                        let all_rows: Vec<_> = self
                            .related(Table::Plf, format!("spid:{}", spid.0))
                            .iter()
                            .map(|id| &self.records.plf.rows()[id.0])
                            .collect();
                        let mut packet = resolve(
                                &packets,
                                Reference::Root(Identity::Packet(spid)),
                                &rows[0].definition.source,
                                |r| Some(self.packet_summary(r)),
                            );
                        for variable in packets.iter().filter(|p| p.cells.tpsd.value != Some(-1)) {
                            packet.problems.push(Problem {
                                kind: ProblemKind::InconsistentDefinition {
                                    fields: vec!["PID_TPSD".into(), "PLF_SPID".into()],
                                    values: variable.cells.tpsd.value.map(Scalar::Integer).into_iter().collect(),
                                    available: std::iter::once(Target { reference: Reference::Root(Identity::Packet(spid)), definition: variable.definition.clone() }).chain(rows.iter().map(|r| Target { reference: Reference::Supporting { table: Table::Plf, key: spid.0.to_string() }, definition: r.definition.clone() })).collect(),
                                },
                                sources: std::iter::once(variable.definition.source.clone()).chain(rows.iter().map(|r| r.definition.source.clone())).collect(),
                                explanation: "Fixed PLF occurrences reference a PID declaring a variable layout".into(),
                            });
                        }
                        PacketOccurrences {
                            packet,
                            occurrences: self
                                .ordered_occurrences(&all_rows)
                                .into_iter()
                                .filter(|o| &o.reference == name)
                                .collect(),
                        }
                    })
                    .collect(),
            ),
            &source,
        );
        if let Some(source) = &self.variable_packet_source {
            result.problems.extend(unavailable::<()>(source, "Variable packet containment is not implemented yet; fixed occurrences are shown").problems);
        }
        result
    }
}

fn pic_key(service: Option<i64>, subtype: Option<i64>) -> String {
    format!("{service:?}/{subtype:?}")
}
fn position_key(position: &Info<Position>) -> Option<(u64, u8)> {
    match position.value {
        Some(Position::PacketAbsolute { byte, bit }) => Some((byte, bit)),
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
fn packet_position(byte: &Info<i64>, bit: &Info<i64>) -> Info<Position> {
    match (byte.value.and_then(|v| u64::try_from(v).ok()), bit.value) {
        (Some(byte_value), Some(bit_value @ 0..=7)) => info(
            Some(Position::PacketAbsolute {
                byte: byte_value,
                bit: bit_value as u8,
            }),
            &byte.sources[0],
        ),
        _ => unavailable(&byte.sources[0], "Invalid packet byte/bit position"),
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

fn reconcile_cells(
    cells: &[&Info<i64>],
    definitions: &[Definition],
    reference: &Reference,
    source: &Source,
    field: &str,
) -> Info<i64> {
    let Some(first) = cells.first() else {
        return missing(reference.clone(), source);
    };
    let sources: Vec<_> = std::iter::once(source.clone())
        .chain(definitions.iter().map(|d| d.source.clone()))
        .collect();
    if cells.iter().all(|cell| cell.value == first.value) {
        return Info {
            value: first.value,
            problems: cells
                .iter()
                .flat_map(|cell| cell.problems.clone())
                .collect(),
            sources,
        };
    }
    Info {
        value: None,
        sources: sources.clone(),
        problems: vec![Problem {
            kind: ProblemKind::InconsistentDefinition {
                fields: vec![field.into()],
                values: cells
                    .iter()
                    .filter_map(|cell| cell.value.map(Scalar::Integer))
                    .collect(),
                available: definitions
                    .iter()
                    .map(|definition| Target {
                        reference: reference.clone(),
                        definition: definition.clone(),
                    })
                    .collect(),
            },
            sources,
            explanation: format!("Conflicting declarations for {field}"),
        }],
    }
}
