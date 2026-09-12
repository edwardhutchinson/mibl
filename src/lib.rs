//! Interface-only SCOS MIB viewer. Operations panic until implemented.
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
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!("issue #8: load error formatting contract only")
    }
}
impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        todo!("issue #8: expose underlying I/O cause")
    }
}
impl Mib {
    /// Reads supported rows once. Missing support files allow partial results.
    pub fn load(_directory: &Path) -> Result<Self, LoadError> {
        todo!("interface only")
    }
    /// Case-sensitive identity; duplicate root rows return Ambiguous.
    pub fn parameter(&self, _name: &ParameterName) -> Lookup<ParameterDescription> {
        todo!("interface only")
    }
    pub fn packet(&self, _spid: PacketSpid) -> Lookup<PacketDescription> {
        todo!("interface only")
    }
    pub fn command(&self, _name: &CommandName) -> Lookup<CommandDescription> {
        todo!("interface only")
    }
    /// Case-insensitive names/descriptions and SPIDs. No cap. Blank query is empty.
    /// Rank exact identities, prefixes, then fuzzy matches, names before descriptions.
    /// Ties: parameter/packet/command, identity (SPIDs numeric), source.
    pub fn search(&self, _query: &str, _scope: SearchScope) -> Vec<Candidate> {
        todo!("interface only")
    }
}
