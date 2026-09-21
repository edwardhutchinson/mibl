//! Telemetry packet definitions, identification and their parameter occurrences.

use super::{
    evidence::{Definition, Info},
    layout::{Enclosure, Layout, Location},
    lookup::{PacketSpid, ParameterName},
    parameters::ParameterSummary,
};

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
pub struct ParameterOccurrence {
    pub reference: ParameterName,
    pub parameter: Info<ParameterSummary>,
    pub definition: Definition,
    pub location: Location,
    pub enclosing: Vec<Enclosure>,
    /// Recorded declarations for the enclosure path, outermost first.
    pub enclosing_definitions: Vec<Definition>,
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
