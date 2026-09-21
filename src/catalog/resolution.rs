//! Reference resolution: the shared construction of an `Info` result, the missing,
//! ambiguous, unsupported and inconsistent problems a declared reference reports, and
//! the retained rows those problems name as targets.

use crate::model::{
    AtLeastTwo, Info, Problem, ProblemKind, Reference, Scalar, Source, Table, Target,
};
use crate::reader::Row;

pub(super) fn info<T>(value: Option<T>, source: &Source) -> Info<T> {
    Info {
        value,
        problems: vec![],
        sources: vec![source.clone()],
    }
}

pub(super) fn missing<T>(reference: Reference, source: &Source) -> Info<T> {
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

/// The problem a declared reference reports when nothing resolves it.
pub(super) fn missing_problems(reference: Reference, source: &Source) -> Vec<Problem> {
    missing::<()>(reference, source).problems
}

pub(super) fn unavailable<T>(source: &Source, explanation: &str) -> Info<T> {
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

/// A problem naming every target when one reference resolves to several definitions.
pub(super) fn ambiguity(
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
pub(super) fn target<T>(table: Table, key: &str, row: &Row<T>) -> Target {
    Target {
        reference: reference(table, key),
        definition: row.definition.clone(),
    }
}

/// A supporting table reference under the key its rows declare.
pub(super) fn reference(table: Table, key: &str) -> Reference {
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

pub(super) fn resolve<T, U>(
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

/// A declared count and the value rows it covers disagree; both declarations stay available.
pub(super) fn count_disagreement(
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
