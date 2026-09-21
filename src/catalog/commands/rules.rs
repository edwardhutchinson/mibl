//! The argument rules a CPC reference resolves: PRF/PRV ranges, PAF/PAS aliases
//! and CCA/CCS conversions.

use super::super::Catalog;
use super::super::encoding::number;
use super::super::resolution::{
    ambiguity, count_disagreement, info, missing, missing_problems, reference, target,
};
use crate::model::{
    Alias, AllowedRange, Calibration, CalibrationAlternative, CalibrationForm, CalibrationPoint,
    Definition, Info, Problem, Scalar, Source, Table,
};
use crate::reader::{Cca, Cpc, Row};

/// The supporting rules the CPC reference cells resolve for one argument.
pub(super) struct ArgumentRules {
    pub(super) ranges: Info<Vec<AllowedRange>>,
    pub(super) aliases: Info<Vec<Alias>>,
    pub(super) calibrations: Info<Vec<CalibrationAlternative>>,
    pub(super) definitions: Vec<Definition>,
}

impl Catalog {
    /// PRF/PRV, PAF/PAS and CCA/CCS join through the CPC reference cells. A reference the row
    /// does not declare contributes no rules, while a declared reference without a usable target
    /// keeps a typed problem and never removes the argument.
    pub(super) fn argument_rules(&self, cpc: &Row<Cpc>) -> ArgumentRules {
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
