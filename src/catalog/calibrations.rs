use super::encoding::number;
use super::resolution::{ambiguity, info, missing, reference, resolve};
use super::*;
use crate::reader::Pcf;

/// The calibration namespace PCF_CATEG declares for its PCF_CURTX or CUR_SELECT reference.
/// Status parameters name textual TXF definitions; every other category names the numerical
/// CAF, MCF or LGF families. The namespaces share one key space, so the category decides
/// which definitions a reference means.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Family {
    Numerical,
    Textual,
}

impl Family {
    fn declared(category: Option<&str>) -> Self {
        if category == Some("S") {
            Self::Textual
        } else {
            Self::Numerical
        }
    }

    fn other(self) -> Self {
        match self {
            Self::Numerical => Self::Textual,
            Self::Textual => Self::Numerical,
        }
    }

    /// The family a resolved calibration belongs to.
    fn of(form: &Option<CalibrationForm>) -> Self {
        if matches!(form, Some(CalibrationForm::Textual { .. })) {
            Self::Textual
        } else {
            Self::Numerical
        }
    }

    /// The family's own reference table, named when the key resolves nowhere.
    fn primary(self) -> Table {
        match self {
            Self::Numerical => Table::Caf,
            Self::Textual => Table::Txf,
        }
    }

    /// Every table the family may resolve a reference in, in schema order.
    fn tables(self) -> &'static [Table] {
        match self {
            Self::Numerical => &[Table::Caf, Table::Mcf, Table::Lgf],
            Self::Textual => &[Table::Txf],
        }
    }
}

impl Catalog {
    pub(super) fn index_calibrations(&mut self) {
        let (index, records) = (&mut self.supporting, &self.records);
        // Composite supporting keys stay collision-free per table, so each table gets its own key space.
        index_rows(index, Table::Cur, records.cur.rows(), |c| {
            c.pname.value.as_ref().map(|name| name.0.as_str())
        });
        index_rows(index, Table::Txp, records.txp.rows(), |c| {
            c.numbr.value.as_deref()
        });
        index_rows(index, Table::Txf, records.txf.rows(), |c| {
            c.numbr.value.as_deref()
        });
        index_rows(index, Table::Lgf, records.lgf.rows(), |c| {
            c.ident.value.as_deref()
        });
        index_rows(index, Table::Mcf, records.mcf.rows(), |c| {
            c.ident.value.as_deref()
        });
        index_rows(index, Table::Caf, records.caf.rows(), |c| {
            c.numbr.value.as_deref()
        });
        index_rows(index, Table::Cap, records.cap.rows(), |c| {
            c.numbr.value.as_deref()
        });
    }

    pub(super) fn calibrations(&self, row: &Row<Pcf>) -> Info<Vec<CalibrationAlternative>> {
        let source = &row.definition.source;
        let name = row
            .cells
            .name
            .value
            .as_ref()
            .expect("retained PCF has a required name")
            .0
            .clone();
        let mut result = info(Some(Vec::new()), source);
        let mut selections: Vec<_> = self
            .related(Table::Cur, name.clone())
            .iter()
            .map(|i| &self.records.cur.rows()[i.0])
            .collect();
        selections.sort_by_key(|r| (r.cells.pos.value, r.definition.source.line));
        let declared = Family::declared(row.cells.categ.value.as_deref());
        // The ICD leaves PCF_CURTX null whenever CUR rows select the calibrations, and names the
        // family from PCF_CATEG either way, so a simultaneous declaration keeps both sets of
        // definitions in the declared family with the disagreement attached.
        let simultaneous = !selections.is_empty() && row.cells.curtx.value.is_some();
        let mut positions = std::collections::BTreeMap::new();
        for r in &selections {
            positions
                .entry(r.cells.pos.value)
                .or_insert_with(Vec::new)
                .push(*r);
        }
        for (position, competing) in &positions {
            if competing.len() > 1 {
                let reference = reference(Table::Cur, &name);
                result.problems.push(Problem {
                    kind: ProblemKind::InconsistentDefinition {
                        fields: vec!["CUR_POS".into()],
                        values: vec![Scalar::Integer(
                            position.expect("retained CUR rows have a required CUR_POS"),
                        )],
                        available: competing
                            .iter()
                            .map(|r| Target {
                                reference: reference.clone(),
                                definition: r.definition.clone(),
                            })
                            .collect(),
                    },
                    sources: competing
                        .iter()
                        .map(|r| r.definition.source.clone())
                        .collect(),
                    explanation:
                        "Duplicate conditional calibration positions; source line breaks the ordering tie"
                            .into(),
                });
            }
        }
        for r in selections {
            let name = r.cells.rlchk.value.as_ref().unwrap();
            let dependency_reference = Reference::Root(Identity::Parameter(name.clone()));
            let rows: Vec<_> = self
                .parameters
                .get(name)
                .into_iter()
                .flatten()
                .map(|i| &self.records.pcf.rows()[i.0])
                .collect();
            let mut targets = resolve(
                &rows,
                dependency_reference.clone(),
                &r.definition.source,
                |p| {
                    Some(vec![Target {
                        reference: dependency_reference.clone(),
                        definition: p.definition.clone(),
                    }])
                },
            );
            if rows.len() > 1 {
                targets.value = Some(
                    rows.iter()
                        .map(|p| Target {
                            reference: dependency_reference.clone(),
                            definition: p.definition.clone(),
                        })
                        .collect(),
                );
            }
            let condition = RuntimeDeclaration {
                expression: format!("raw({}) = {}", name.0, r.cells.valpar.value.unwrap()),
                dependencies: vec![Dependency {
                    reference: dependency_reference,
                    targets,
                }],
                sources: vec![r.definition.source.clone()],
            };
            let mut alternatives = self.select_calibrations(
                r.cells.select.value.as_deref().unwrap(),
                &r.definition.source,
                declared,
            );
            for a in &mut alternatives {
                a.selection = Some(r.definition.clone());
                a.condition = Some(condition.clone());
                a.calibration.problems.push(Problem { kind: ProblemKind::RuntimeDependent { declaration: condition.clone() }, sources: condition.sources.clone(), explanation: "Conditional calibration requires a runtime raw value; first matching condition in CUR_POS order applies".into() });
            }
            result.value.as_mut().unwrap().extend(alternatives);
        }
        if let Some(key) = row.cells.curtx.value.as_deref() {
            result
                .value
                .as_mut()
                .unwrap()
                .extend(self.select_calibrations(key, source, declared));
        }
        let alternatives = result.value.as_mut().unwrap();
        for a in alternatives {
            if a.calibration.value.is_none() {
                let key = a
                    .selection
                    .as_ref()
                    .and_then(|d| text_at(d, "CUR_SELECT"))
                    .or(row.cells.curtx.value.as_deref())
                    .unwrap_or("");
                let tables = declared.tables();
                a.calibration
                    .problems
                    .retain(|p| !matches!(p.kind, ProblemKind::MissingReference { .. }));
                for table in tables {
                    a.calibration.problems.extend(
                        missing::<Calibration>(reference(*table, key), &a.calibration.sources[0])
                            .problems,
                    );
                }
            }
            // PCF_CATEG is one of N, S or T, and the ICD permits a calibration reference only
            // in the N and S cases, so a text parameter keeps its definitions as a disagreement.
            let category_conflict = a.calibration.value.as_ref().is_some_and(|c| {
                !matches!(
                    (row.cells.categ.value.as_deref(), Family::of(&c.form.value)),
                    (Some("S"), Family::Textual) | (Some("N"), Family::Numerical)
                )
            });
            // String and time encodings cannot carry a calibration either.
            let prohibited_type = matches!(row.cells.ptc.value, Some(7..=10));
            if simultaneous || category_conflict || prohibited_type {
                let available = a
                    .calibration
                    .value
                    .iter()
                    .map(|c| Target {
                        reference: c.reference.clone(),
                        definition: c.definition.clone(),
                    })
                    .collect();
                let mut sources = vec![source.clone()];
                sources.extend(a.calibration.sources.clone());
                a.calibration.problems.push(Problem {
                    kind: ProblemKind::InconsistentDefinition {
                        fields: vec!["PCF_CATEG".into(), "PCF_CURTX".into(), "PCF_PTC".into(), "CUR_SELECT".into()],
                        values: vec![Scalar::Code(row.cells.categ.value.clone().unwrap()), Scalar::Text(row.cells.curtx.value.clone().unwrap_or_default())],
                        available,
                    },
                    sources,
                    explanation: "Calibration category, parameter type, or simultaneous direct and conditional declarations disagree; available definitions are retained".into(),
                });
            }
        }
        if row.cells.categ.value.as_deref() == Some("S") && row.cells.curtx.value.is_none() {
            result.problems.push(Problem {
                kind: ProblemKind::InconsistentDefinition {
                    fields: vec!["PCF_CATEG".into(), "PCF_CURTX".into()],
                    values: vec![Scalar::Code("S".into())],
                    available: vec![],
                },
                sources: vec![source.clone()],
                explanation: "Status parameter has no required PCF_CURTX reference".into(),
            });
        }
        result
    }

    /// Every definition `key` names in one calibration namespace, in schema order.
    fn family_values(&self, key: &str, family: Family) -> Vec<Calibration> {
        let mut values = Vec::new();
        if family == Family::Numerical {
            for i in self.related(Table::Caf, key.into()) {
                values.push(self.curve(&self.records.caf.rows()[i.0]));
            }
            for i in self.related(Table::Mcf, key.into()) {
                let r = &self.records.mcf.rows()[i.0];
                let c = &r.cells;
                values.push(coefficients(
                    Table::Mcf,
                    key,
                    &r.definition,
                    [&c.pol1, &c.pol2, &c.pol3, &c.pol4, &c.pol5],
                    |coefficients| CalibrationForm::Polynomial { coefficients },
                ));
            }
            for i in self.related(Table::Lgf, key.into()) {
                let r = &self.records.lgf.rows()[i.0];
                let c = &r.cells;
                values.push(coefficients(
                    Table::Lgf,
                    key,
                    &r.definition,
                    [&c.pol1, &c.pol2, &c.pol3, &c.pol4, &c.pol5],
                    |coefficients| CalibrationForm::Logarithmic { coefficients },
                ));
            }
        } else {
            for i in self.related(Table::Txf, key.into()) {
                let r = &self.records.txf.rows()[i.0];
                let source = &r.definition.source;
                let rows = self.related(Table::Txp, key.into());
                let intervals = if rows.is_empty() {
                    missing(reference(Table::Txp, key), source)
                } else {
                    info(
                        Some(
                            rows.iter()
                                .map(|i| {
                                    let p = &self.records.txp.rows()[i.0];
                                    TextInterval {
                                        low: number(
                                            &p.cells.from,
                                            r.cells.rawfmt.value.as_deref(),
                                            Some("D"),
                                        ),
                                        high: number(
                                            &p.cells.to,
                                            r.cells.rawfmt.value.as_deref(),
                                            Some("D"),
                                        ),
                                        text: p.cells.altxt.clone(),
                                        definition: p.definition.clone(),
                                    }
                                })
                                .collect(),
                        ),
                        source,
                    )
                };
                values.push(Calibration {
                    reference: reference(Table::Txf, key),
                    definition: r.definition.clone(),
                    form: info(Some(CalibrationForm::Textual { intervals }), source),
                });
            }
        }
        values
    }

    fn select_calibrations(
        &self,
        key: &str,
        source: &Source,
        declared: Family,
    ) -> Vec<CalibrationAlternative> {
        // PCF_CATEG names the family its reference means, so only that family resolves it.
        // A reference that resolves only in the other family keeps its usable definitions
        // instead of removing them, with the disagreement attached by the caller.
        let values = self.family_values(key, declared);
        let values = if values.is_empty() {
            self.family_values(key, declared.other())
        } else {
            values
        };
        if values.is_empty() {
            return vec![CalibrationAlternative {
                condition: None,
                selection: None,
                calibration: missing(reference(declared.primary(), key), source),
            }];
        }
        let targets: Vec<_> = values
            .iter()
            .map(|c| Target {
                reference: c.reference.clone(),
                definition: c.definition.clone(),
            })
            .collect();
        let ambiguity = targets.first().and_then(|first| {
            ambiguity(
                first.reference.clone(),
                source,
                targets.clone(),
                "Calibration reference has multiple targets; every available definition is retained",
            )
        });
        values
            .into_iter()
            .map(|c| CalibrationAlternative {
                condition: None,
                selection: None,
                calibration: Info {
                    sources: vec![source.clone(), c.definition.source.clone()],
                    value: Some(c),
                    problems: ambiguity.iter().cloned().collect(),
                },
            })
            .collect()
    }

    fn curve(&self, row: &Row<crate::reader::Caf>) -> Calibration {
        let c = &row.cells;
        let key = c.numbr.value.as_deref().unwrap();
        let source = &row.definition.source;
        let rows = self.related(Table::Cap, key.into());
        let points = if rows.is_empty() {
            missing(reference(Table::Cap, key), source)
        } else {
            info(
                Some(
                    rows.iter()
                        .map(|i| {
                            let r = &self.records.cap.rows()[i.0];
                            CalibrationPoint {
                                raw: number(
                                    &r.cells.xvals,
                                    c.rawfmt.value.as_deref(),
                                    c.radix.value.as_deref(),
                                ),
                                engineering: number(
                                    &r.cells.yvals,
                                    c.engfmt.value.as_deref(),
                                    Some("D"),
                                ),
                                definition: r.definition.clone(),
                            }
                        })
                        .collect(),
                ),
                source,
            )
        };
        Calibration {
            reference: reference(Table::Caf, key),
            definition: row.definition.clone(),
            form: info(
                Some(CalibrationForm::Numerical {
                    points,
                    interpolation: c.r#inter.clone(),
                }),
                source,
            ),
        }
    }
}

/// MCF and LGF rows declare the same five coefficients in the same format and radix.
fn coefficients(
    table: Table,
    key: &str,
    definition: &Definition,
    cells: [&Info<String>; 5],
    form: impl FnOnce(Vec<Info<Scalar>>) -> CalibrationForm,
) -> Calibration {
    Calibration {
        reference: reference(table, key),
        definition: definition.clone(),
        form: info(
            Some(form(
                cells.map(|v| number(v, Some("R"), Some("D"))).to_vec(),
            )),
            &definition.source,
        ),
    }
}

fn text_at<'a>(definition: &'a Definition, name: &str) -> Option<&'a str> {
    definition
        .fields
        .iter()
        .find(|f| f.meanings.iter().any(|m| m.schema_name == name))
        .and_then(|f| match &f.presence {
            Presence::Text(s) => Some(s.as_str()),
            _ => None,
        })
}
