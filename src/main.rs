//! CLI boundary declarations; running this placeholder always fails visibly.
#![allow(dead_code)]
use mibl::{LoadError, Mib, model::*};
use std::{ffi::OsString, io, path::PathBuf, process::ExitCode};
struct Configuration {
    directory: PathBuf,
    debug: bool,
    request: Request,
}
enum Request {
    Parameter(ParameterName),
    Packet(PacketSpid),
    Command(CommandName),
    Search { query: String, scope: SearchScope },
}
enum ConfigurationError {
    MissingMibDir,
    EmptyMibDir,
    InvalidArguments(String),
}
enum CliError {
    Configuration(ConfigurationError),
    Load(LoadError),
    Output(io::Error),
}
/// Future syntax: [--debug] parameter NAME | packet SPID | command NAME |
/// search QUERY [--scope parameters|packets|commands|all]. Default scope: all.
/// MIB_DIR is required and nonempty; non-Unicode directory paths are accepted.
fn configure(
    _arguments: Vec<OsString>,
    _mib_dir: Option<OsString>,
) -> Result<Configuration, ConfigurationError> {
    todo!("interface only")
}
/// Owns stderr tracing subscriber, filtering and formatting; debug enables library events.
fn setup_tracing(_debug: bool) {
    todo!("interface only")
}
enum Response {
    Parameter(Lookup<ParameterDescription>),
    Packet(Lookup<PacketDescription>),
    Command(Lookup<CommandDescription>),
    Search(Vec<Candidate>),
}
fn query(_mib: &Mib, _request: &Request) -> Response {
    todo!("interface only")
}
/// Found: full details, status 0. Ambiguous: candidate table, status 0.
/// NotFound: no output, status 1. Search: table even when empty, status 0.
/// Candidate columns: kind, identity, name, description. Problems appear beside fields.
fn render(_response: &Response, _stdout: &mut dyn io::Write) -> Result<ExitCode, io::Error> {
    todo!("interface only")
}
/// Configuration/load/output failures: visible stderr error, status 2.
fn report_error(_error: &CliError, _stderr: &mut dyn io::Write) -> ExitCode {
    todo!("interface only")
}
fn main() -> ExitCode {
    eprintln!("mibl: interface-only scaffold; viewer functionality is not implemented");
    ExitCode::from(2)
}
