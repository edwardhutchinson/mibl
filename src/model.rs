//! Owned descriptions. All vectors retain duplicates. See docs/interfaces/contract.md.

mod calibrations;
mod commands;
mod evidence;
mod layout;
mod lookup;
mod packets;
mod parameters;
mod tables;

pub use calibrations::{
    Calibration, CalibrationAlternative, CalibrationForm, CalibrationPoint, TextInterval,
};
pub use commands::{
    Alias, AllowedRange, ArgumentValue, CommandArgument, CommandDescription, CommandElement,
    CommandHeader, FixedArea, HeaderField, ValueRules, ValueSource,
};
pub use evidence::{
    AtLeastTwo, Definition, Dependency, FieldMeaning, Info, Interpretation, InterpretationOrigin,
    Presence, Problem, ProblemKind, RecordedField, Reference, RuntimeDeclaration, Scalar, Source,
    Target,
};
pub use layout::{Enclosure, Encoding, Layout, Location, Position, Repetition};
pub use lookup::{
    Candidate, CommandName, Identity, Lookup, NotFoundReason, PacketSpid, ParameterName,
    SearchScope,
};
pub use packets::{
    IdentificationCriterion, PacketDescription, PacketIdentification, PacketOccurrences,
    PacketSummary, ParameterOccurrence,
};
pub use parameters::{ParameterDescription, ParameterSummary};
pub use tables::{Table, TableReport, TableRoot, TableRows};
