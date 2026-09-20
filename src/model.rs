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
impl Table {
    /// Every table the loader reads, ordered by code. The file name orders identically,
    /// because each file name is its lowercase code with a `.dat` suffix.
    pub const ALL: [Table; 25] = [
        Table::Caf,
        Table::Cap,
        Table::Cca,
        Table::Ccf,
        Table::Ccs,
        Table::Cdf,
        Table::Cpc,
        Table::Cur,
        Table::Lgf,
        Table::Mcf,
        Table::Paf,
        Table::Pas,
        Table::Pcdf,
        Table::Pcf,
        Table::Pcpc,
        Table::Pic,
        Table::Pid,
        Table::Plf,
        Table::Prf,
        Table::Prv,
        Table::Tcp,
        Table::Tpcf,
        Table::Txf,
        Table::Txp,
        Table::Vpd,
    ];
    /// The declared code, as it appears in field names, reference labels and rows.
    pub fn code(self) -> &'static str {
        match self {
            Table::Pcf => "PCF",
            Table::Pid => "PID",
            Table::Tpcf => "TPCF",
            Table::Pic => "PIC",
            Table::Plf => "PLF",
            Table::Vpd => "VPD",
            Table::Cur => "CUR",
            Table::Caf => "CAF",
            Table::Cap => "CAP",
            Table::Mcf => "MCF",
            Table::Lgf => "LGF",
            Table::Txf => "TXF",
            Table::Txp => "TXP",
            Table::Ccf => "CCF",
            Table::Cdf => "CDF",
            Table::Cpc => "CPC",
            Table::Cca => "CCA",
            Table::Ccs => "CCS",
            Table::Paf => "PAF",
            Table::Pas => "PAS",
            Table::Prf => "PRF",
            Table::Prv => "PRV",
            Table::Tcp => "TCP",
            Table::Pcdf => "PCDF",
            Table::Pcpc => "PCPC",
        }
    }
    /// The file name the loader reads, relative to the MIB directory.
    pub fn file(self) -> &'static str {
        match self {
            Table::Pcf => "pcf.dat",
            Table::Pid => "pid.dat",
            Table::Tpcf => "tpcf.dat",
            Table::Pic => "pic.dat",
            Table::Plf => "plf.dat",
            Table::Vpd => "vpd.dat",
            Table::Cur => "cur.dat",
            Table::Caf => "caf.dat",
            Table::Cap => "cap.dat",
            Table::Mcf => "mcf.dat",
            Table::Lgf => "lgf.dat",
            Table::Txf => "txf.dat",
            Table::Txp => "txp.dat",
            Table::Ccf => "ccf.dat",
            Table::Cdf => "cdf.dat",
            Table::Cpc => "cpc.dat",
            Table::Cca => "cca.dat",
            Table::Ccs => "ccs.dat",
            Table::Paf => "paf.dat",
            Table::Pas => "pas.dat",
            Table::Prf => "prf.dat",
            Table::Prv => "prv.dat",
            Table::Tcp => "tcp.dat",
            Table::Pcdf => "pcdf.dat",
            Table::Pcpc => "pcpc.dat",
        }
    }
    /// What the table's rows declare, in the glossary's terms.
    pub fn meaning(self) -> &'static str {
        match self {
            Table::Pcf => "Monitoring parameter definitions",
            Table::Pid => "Telemetry packet definitions",
            Table::Tpcf => "Recorded packet names and sizes",
            Table::Pic => "Packet identification criteria",
            Table::Plf => "Parameter occurrence locations",
            Table::Vpd => "Variable packet layout elements",
            Table::Cur => "Conditional calibration alternatives",
            Table::Caf => "Numerical calibration definitions",
            Table::Cap => "Numerical calibration curve points",
            Table::Mcf => "Polynomial calibration coefficients",
            Table::Lgf => "Logarithmic calibration coefficients",
            Table::Txf => "Textual calibration definitions",
            Table::Txp => "Textual calibration intervals",
            Table::Ccf => "Telecommand definitions",
            Table::Cdf => "Telecommand argument elements",
            Table::Cpc => "Command argument definitions",
            Table::Cca => "Command conversion definitions",
            Table::Ccs => "Command conversion curve points",
            Table::Paf => "Alias definitions for argument values",
            Table::Pas => "Alias values",
            Table::Prf => "Allowed range definitions",
            Table::Prv => "Range boundary values",
            Table::Tcp => "Outgoing packet header definitions",
            Table::Pcdf => "Command header field definitions",
            Table::Pcpc => "Command header parameters",
        }
    }
    /// The root kind a table's rows are looked up as, when a lookup addresses them directly.
    pub fn root(self) -> Option<TableRoot> {
        match self {
            Table::Pcf => Some(TableRoot::Parameter),
            Table::Pid => Some(TableRoot::Packet),
            Table::Ccf => Some(TableRoot::Command),
            _ => None,
        }
    }
}
/// The kind of root definition a table's rows are addressed as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TableRoot {
    Parameter,
    Packet,
    Command,
}
/// One table as the loader left it. Every read attempt is reported, not only the failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableReport {
    pub table: Table,
    pub rows: TableRows,
}
/// A readable table is distinguished from an absent one even when it retains no rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TableRows {
    Missing,
    Unreadable,
    Read { rows: usize },
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
