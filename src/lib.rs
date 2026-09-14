//! Read-only SCOS MIB snapshots. Parameter, fixed packet and basic command lookups, plus fuzzy search.
#![allow(dead_code)] // Declarations are intentionally unused until implementation tickets.
mod catalog;
pub mod model;
mod reader;

use model::*;
use std::{
    io,
    path::{Path, PathBuf},
};

/// Owns one immutable snapshot. Queries never read files and return owned data.
/// The library never reads MIB_DIR or installs a tracing subscriber.
pub struct Mib {
    catalog: catalog::Catalog,
}

#[derive(Debug)]
pub enum LoadError {
    InaccessibleDirectory {
        directory: PathBuf,
        cause: io::Error,
    },
    NoUsableSupportedRows {
        directory: PathBuf,
    },
}
impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InaccessibleDirectory { directory, cause } => write!(
                f,
                "cannot read MIB directory {}: {cause}",
                directory.display()
            ),
            Self::NoUsableSupportedRows { directory } => {
                write!(f, "no usable supported rows in {}", directory.display())
            }
        }
    }
}
impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InaccessibleDirectory { cause, .. } => Some(cause),
            Self::NoUsableSupportedRows { .. } => None,
        }
    }
}
impl Mib {
    /// Reads supported rows once. Missing support files allow partial results.
    pub fn load(directory: &Path) -> Result<Self, LoadError> {
        Ok(Self {
            catalog: catalog::Catalog::new(reader::load(directory)?),
        })
    }
    /// Case-sensitive identity; duplicate root rows return Ambiguous.
    pub fn parameter(&self, name: &ParameterName) -> Lookup<ParameterDescription> {
        self.catalog.parameter(name)
    }
    pub fn packet(&self, spid: PacketSpid) -> Lookup<PacketDescription> {
        self.catalog.packet(spid)
    }
    pub fn command(&self, name: &CommandName) -> Lookup<CommandDescription> {
        self.catalog.command(name)
    }
    /// List packet and command definitions by PUS service and optional subtype.
    /// Order: subtype ascending, missing subtype last, kind, identity, source.
    pub fn pus(&self, service: u16, subtype: Option<u16>) -> Vec<Candidate> {
        self.catalog.pus(service, subtype)
    }
    /// Case-insensitive names/descriptions and SPIDs. No cap. Blank query is empty.
    /// Rank exact identities, prefixes, then fuzzy matches, names before descriptions.
    /// Ties: parameter/packet/command, identity (SPIDs numeric), source.
    pub fn search(&self, query: &str, scope: SearchScope) -> Vec<Candidate> {
        self.catalog.search(query, scope)
    }
}
