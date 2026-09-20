//! Telecommand definitions, their arguments, value rules and expanded headers.

use super::{
    calibrations::CalibrationAlternative,
    evidence::{Definition, Info, Reference, RuntimeDeclaration, Scalar, Target},
    layout::{Encoding, Layout, Location},
    lookup::{CommandName, ParameterName},
};

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
