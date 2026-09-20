//! Recorded evidence: sources, interpreted cells, definitions and the problems they raise.

use std::{num::NonZeroUsize, path::PathBuf};

use super::{lookup::Identity, tables::Table};

/// Two required elements make empty/singleton ambiguity unrepresentable.
#[derive(Clone, Debug, PartialEq)]
pub struct AtLeastTwo<T> {
    pub first: Box<T>,
    pub second: Box<T>,
    pub rest: Vec<T>,
}
/// Reader guarantees a relative path inside the supplied directory, no parent traversal.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Source {
    pub file: PathBuf,
    pub line: NonZeroUsize,
}
/// None means no usable interpretation, not necessarily absent recorded text.
/// A usable value and problems may coexist; nested fields remain independent.
#[derive(Clone, Debug, PartialEq)]
pub struct Info<T> {
    pub value: Option<T>,
    pub problems: Vec<Problem>,
    pub sources: Vec<Source>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Presence {
    Omitted,
    Empty,
    Text(String),
}
#[derive(Clone, Debug, PartialEq)]
pub enum Scalar {
    Text(String),
    Integer(i64),
    Unsigned(u64),
    Decimal(String),
    Boolean(bool),
    Code(String),
}
#[derive(Clone, Debug, PartialEq)]
pub enum InterpretationOrigin {
    Recorded,
    DocumentedDefault { rule: String },
}
#[derive(Clone, Debug, PartialEq)]
pub struct Interpretation {
    pub value: Scalar,
    pub origin: InterpretationOrigin,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FieldMeaning {
    pub schema_name: String,
    pub interpretation: Info<Interpretation>,
}
/// Ordered by physical, one-based schema column. Ambiguous columns have multiple meanings.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordedField {
    pub column: NonZeroUsize,
    pub presence: Presence,
    pub meanings: Vec<FieldMeaning>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Definition {
    pub source: Source,
    pub fields: Vec<RecordedField>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Reference {
    Root(Identity),
    Supporting { table: Table, key: String },
    Deferred { concept: String, key: String },
}
#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    pub reference: Reference,
    pub definition: Definition,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Dependency {
    pub reference: Reference,
    pub targets: Info<Vec<Target>>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeDeclaration {
    pub expression: String,
    pub dependencies: Vec<Dependency>,
    pub sources: Vec<Source>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum ProblemKind {
    MissingReference {
        reference: Reference,
    },
    AmbiguousReference {
        reference: Reference,
        alternatives: AtLeastTwo<Target>,
    },
    InconsistentDefinition {
        fields: Vec<String>,
        values: Vec<Scalar>,
        available: Vec<Target>,
    },
    UnsupportedInterpretation {
        column: Option<NonZeroUsize>,
        meanings: Vec<FieldMeaning>,
    },
    RuntimeDependent {
        declaration: RuntimeDeclaration,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    pub kind: ProblemKind,
    pub sources: Vec<Source>,
    pub explanation: String,
}
