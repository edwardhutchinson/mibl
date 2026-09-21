//! The identities callers address definitions by, and the outcome of addressing them.

use super::evidence::{AtLeastTwo, Info, Source};

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

#[derive(Clone, Debug)]
pub struct Candidate {
    pub identity: Identity,
    pub service_type: Option<u16>,
    pub service_subtype: Option<u16>,
    pub name: Info<String>,
    pub description: Info<String>,
    pub source: Source,
}
