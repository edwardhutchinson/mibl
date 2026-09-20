//! Monitoring parameter summaries and their containing packet occurrences.

use super::{
    calibrations::CalibrationAlternative,
    evidence::{Definition, Info},
    layout::Encoding,
    lookup::ParameterName,
    packets::PacketOccurrences,
};

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
