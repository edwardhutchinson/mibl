//! Owned descriptions. All vectors retain duplicates. See docs/interfaces/contract.md.
use std::{num::NonZeroUsize, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ParameterName(pub String);
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandName(pub String);
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PacketSpid(pub u64);
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Identity {
    Parameter(ParameterName),
    Packet(PacketSpid),
    Command(CommandName),
}
#[derive(Clone, Copy, Debug)]
pub enum SearchScope {
    Parameters,
    Packets,
    Commands,
    All,
}
#[derive(Clone, Debug)]
pub enum NotFoundReason {
    NoMatchingIdentity,
    DefinitionsUnavailable,
}
#[derive(Clone, Debug)]
pub enum Lookup<T> {
    NotFound(NotFoundReason),
    Found(T),
    Ambiguous(AtLeastTwo<Candidate>),
}
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
#[derive(Clone, Debug)]
pub struct Candidate {
    pub identity: Identity,
    pub service_type: Option<u16>,
    pub service_subtype: Option<u16>,
    pub name: Info<String>,
    pub description: Info<String>,
    pub source: Source,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Table {
    Pcf,
    Pid,
    Tpcf,
    Pic,
    Plf,
    Vpd,
    Cur,
    Caf,
    Cap,
    Mcf,
    Lgf,
    Txf,
    Txp,
    Ccf,
    Cdf,
    Cpc,
    Cca,
    Ccs,
    Paf,
    Pas,
    Prf,
    Prv,
    Tcp,
    Pcdf,
    Pcpc,
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
#[derive(Clone, Debug)]
pub struct Encoding {
    pub ptc: Info<u16>,
    pub pfc: Info<u32>,
    pub endian: Info<String>,
    pub encoded_bits: Info<u64>,
}
#[derive(Clone, Debug)]
pub struct ParameterSummary {
    pub name: ParameterName,
    pub definition: Definition,
    pub description: Info<String>,
    pub encoding: Encoding,
    pub units: Info<String>,
    pub calibrations: Info<Vec<CalibrationAlternative>>,
}
#[derive(Clone, Debug)]
pub struct ParameterDescription {
    pub parameter: ParameterSummary,
    pub occurrences: Info<Vec<PacketOccurrences>>,
}
#[derive(Clone, Debug)]
pub struct PacketSummary {
    /// All matching TPCF definitions in source order, including duplicates.
    pub characteristics: Info<Vec<Definition>>,
    pub spid: PacketSpid,
    pub name: Info<String>,
    pub description: Info<String>,
    pub definition: Definition,
}
#[derive(Clone, Debug)]
pub struct PacketOccurrences {
    pub packet: Info<PacketSummary>,
    pub occurrences: Vec<ParameterOccurrence>,
}
#[derive(Clone, Debug)]
pub enum Position {
    PacketAbsolute {
        byte: u64,
        bit: u8,
    },
    /// CDF_BIT is after the header with repetition counts taken as one.
    ApplicationDeclaredBit(u64),
    HeaderBit(u64),
    RelativeBits(i64),
    Runtime(RuntimeDeclaration),
}
#[derive(Clone, Debug)]
pub struct Location {
    pub position: Info<Position>,
    pub encoded_bits: Info<u64>,
    pub constraints: Vec<RuntimeDeclaration>,
}
#[derive(Clone, Debug)]
pub enum Repetition {
    Fixed { count: u64, stride_bits: Info<u64> },
    Runtime(RuntimeDeclaration),
}
#[derive(Clone, Debug)]
pub enum Enclosure {
    Repetition(Info<Repetition>),
    Condition(RuntimeDeclaration),
}
#[derive(Clone, Debug)]
pub struct ParameterOccurrence {
    pub reference: ParameterName,
    pub parameter: Info<ParameterSummary>,
    pub definition: Definition,
    pub location: Location,
    pub enclosing: Vec<Enclosure>,
    /// Recorded declarations for the enclosure path, outermost first.
    pub enclosing_definitions: Vec<Definition>,
}
/// Siblings in declared order; duplicate positions survive with source tie-breaks.
#[derive(Clone, Debug)]
pub enum Layout<T> {
    Element(T),
    Repeat {
        definition: Definition,
        repetition: Info<Repetition>,
        children: Vec<Layout<T>>,
    },
    Conditional {
        definition: Definition,
        condition: RuntimeDeclaration,
        children: Vec<Layout<T>>,
    },
}
#[derive(Clone, Debug)]
pub struct IdentificationCriterion {
    pub expected: Info<u64>,
    pub extraction: Location,
    pub definitions: Vec<Definition>,
}
#[derive(Clone, Debug)]
pub struct PacketIdentification {
    /// Matching PIC rows, including definitions that disable additional criteria.
    pub definitions: Vec<Definition>,
    pub apid: Info<u32>,
    pub service_type: Info<u16>,
    pub service_subtype: Info<u16>,
    pub criteria: Info<Vec<IdentificationCriterion>>,
}
#[derive(Clone, Debug)]
pub struct PacketDescription {
    pub packet: PacketSummary,
    pub identification: PacketIdentification,
    pub layout: Info<Vec<Layout<ParameterOccurrence>>>,
}
#[derive(Clone, Debug)]
pub struct CalibrationAlternative {
    pub condition: Option<RuntimeDeclaration>,
    pub selection: Option<Definition>,
    pub calibration: Info<Calibration>,
}
#[derive(Clone, Debug)]
pub struct Calibration {
    pub reference: Reference,
    pub definition: Definition,
    pub form: Info<CalibrationForm>,
}
#[derive(Clone, Debug)]
pub struct CalibrationPoint {
    pub raw: Info<Scalar>,
    pub engineering: Info<Scalar>,
    pub definition: Definition,
}
#[derive(Clone, Debug)]
pub struct TextInterval {
    pub low: Info<Scalar>,
    pub high: Info<Scalar>,
    pub text: Info<String>,
    pub definition: Definition,
}
#[derive(Clone, Debug)]
pub enum CalibrationForm {
    Numerical {
        points: Info<Vec<CalibrationPoint>>,
        interpolation: Info<String>,
    },
    Polynomial {
        coefficients: Vec<Info<Scalar>>,
    },
    Logarithmic {
        coefficients: Vec<Info<Scalar>>,
    },
    Textual {
        intervals: Info<Vec<TextInterval>>,
    },
    CommandConversion {
        points: Info<Vec<CalibrationPoint>>,
        interpolation: Info<String>,
    },
}
#[derive(Clone, Debug)]
pub enum ValueSource {
    Literal(Scalar),
    Telemetry {
        parameter: ParameterName,
        declaration: RuntimeDeclaration,
    },
    Runtime(RuntimeDeclaration),
}
#[derive(Clone, Debug)]
pub struct ArgumentValue {
    pub source: ValueSource,
    pub representation: Info<String>,
    pub definition: Definition,
}
#[derive(Clone, Debug)]
pub struct AllowedRange {
    pub low: Info<Scalar>,
    pub high: Info<Scalar>,
    pub representation: Info<String>,
    pub definition: Definition,
}
#[derive(Clone, Debug)]
pub struct Alias {
    pub raw: Info<Scalar>,
    pub text: Info<String>,
    pub definition: Definition,
}
#[derive(Clone, Debug)]
pub struct ValueRules {
    pub default: Info<ArgumentValue>,
    pub element_value: Info<ArgumentValue>,
    pub ranges: Info<Vec<AllowedRange>>,
    pub aliases: Info<Vec<Alias>>,
    pub calibrations: Info<Vec<CalibrationAlternative>>,
    pub supporting_definitions: Vec<Definition>,
}
#[derive(Clone, Debug)]
pub struct CommandArgument {
    pub reference: Reference,
    pub definition: Info<Definition>,
    pub element: Definition,
    pub description: Info<String>,
    pub encoding: Encoding,
    pub units: Info<String>,
    pub location: Location,
    pub rules: ValueRules,
}
#[derive(Clone, Debug)]
pub struct FixedArea {
    pub definition: Definition,
    pub location: Location,
    pub value: Info<ArgumentValue>,
}
#[derive(Clone, Debug)]
pub enum CommandElement {
    Argument(Box<CommandArgument>),
    Fixed(Box<FixedArea>),
}
#[derive(Clone, Debug)]
pub struct HeaderField {
    pub definition: Definition,
    pub parameter: Info<Target>,
    pub location: Location,
    pub value: Info<ArgumentValue>,
    pub field_kind: Info<String>,
}
#[derive(Clone, Debug)]
pub struct CommandHeader {
    pub definition: Definition,
    pub fields: Info<Vec<HeaderField>>,
}
#[derive(Clone, Debug)]
pub struct CommandDescription {
    pub name: CommandName,
    pub definition: Definition,
    pub description: Info<String>,
    pub arguments: Info<Vec<Layout<CommandElement>>>,
    pub header: Info<CommandHeader>,
}
