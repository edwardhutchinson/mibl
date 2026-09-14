//! Application-owned configuration, tracing, rendering and exit status.
#![allow(dead_code)] // Other request/response variants belong to later viewer slices.
mod render;
use mibl::{LoadError, Mib, model::*};
use std::{ffi::OsString, io, path::PathBuf, process::ExitCode};
struct Configuration {
    directory: PathBuf,
    debug: bool,
    details: bool,
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
/// Preserve non-Unicode directory paths.
fn configure(
    arguments: Vec<OsString>,
    mib_dir: Option<OsString>,
) -> Result<Configuration, ConfigurationError> {
    let directory = mib_dir.ok_or(ConfigurationError::MissingMibDir)?;
    if directory.is_empty() {
        return Err(ConfigurationError::EmptyMibDir);
    }
    let mut args = arguments.as_slice();
    let mut debug = false;
    let mut details = false;
    while let Some(flag) = args.first() {
        match flag.to_str() {
            Some("--debug") if !debug => debug = true,
            Some("--details") if !details => details = true,
            _ => break,
        }
        args = &args[1..];
    }
    let request = match args {
        [verb, name]
            if (verb == "parameter" || verb == "command")
                && !name.is_empty()
                && !name.to_string_lossy().starts_with('-') =>
        {
            let name = name
                .to_str()
                .ok_or_else(|| ConfigurationError::InvalidArguments("name must be UTF-8".into()))?;
            if verb == "command" {
                Request::Command(CommandName(name.into()))
            } else {
                Request::Parameter(ParameterName(name.into()))
            }
        }
        [verb, spid] if verb == "packet" => {
            let spid = spid
                .to_str()
                .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| {
                    ConfigurationError::InvalidArguments(
                        "packet SPID must be an unsigned integer".into(),
                    )
                })?;
            Request::Packet(PacketSpid(spid))
        }
        _ => {
            return Err(ConfigurationError::InvalidArguments(
                "usage: mibl [--debug] [--details] parameter NAME | packet SPID | command NAME"
                    .into(),
            ));
        }
    };
    Ok(Configuration {
        directory: directory.into(),
        debug,
        details,
        request,
    })
}
fn setup_tracing(debug: bool) {
    tracing_subscriber::fmt()
        .with_max_level(if debug {
            tracing::level_filters::LevelFilter::DEBUG
        } else {
            tracing::level_filters::LevelFilter::OFF
        })
        .with_writer(io::stderr)
        .with_ansi(false)
        .without_time()
        .init();
}
enum Response {
    Parameter(Lookup<ParameterDescription>),
    Packet(Lookup<PacketDescription>),
    Command(Lookup<CommandDescription>),
    Search(Vec<Candidate>),
}
fn query(mib: &Mib, request: &Request) -> Response {
    match request {
        Request::Parameter(name) => Response::Parameter(mib.parameter(name)),
        Request::Command(name) => Response::Command(mib.command(name)),
        Request::Packet(spid) => Response::Packet(mib.packet(*spid)),
        _ => todo!("request is not exposed until its owning viewer slice"),
    }
}

fn render(
    response: &Response,
    details: bool,
    stdout: &mut dyn io::Write,
) -> Result<ExitCode, io::Error> {
    match response {
        Response::Command(Lookup::NotFound(_))
        | Response::Parameter(Lookup::NotFound(_))
        | Response::Packet(Lookup::NotFound(_)) => {
            return Ok(ExitCode::from(1));
        }
        Response::Command(Lookup::Found(description)) => {
            render::command(description, details, stdout)?
        }
        Response::Parameter(Lookup::Found(description)) => {
            render::parameter(description, details, stdout)?
        }
        Response::Packet(Lookup::Found(description)) => {
            render::packet(description, details, stdout)?
        }
        Response::Command(Lookup::Ambiguous(candidates))
        | Response::Parameter(Lookup::Ambiguous(candidates))
        | Response::Packet(Lookup::Ambiguous(candidates)) => {
            render::candidates(
                std::iter::once(candidates.first.as_ref())
                    .chain(std::iter::once(candidates.second.as_ref()))
                    .chain(candidates.rest.iter()),
                stdout,
            )?;
        }
        _ => todo!("response is not exposed until its owning viewer slice"),
    }
    Ok(ExitCode::SUCCESS)
}

fn report_error(error: &CliError, stderr: &mut dyn io::Write) -> ExitCode {
    let message = match error {
        CliError::Configuration(ConfigurationError::MissingMibDir) => "MIB_DIR is not set".into(),
        CliError::Configuration(ConfigurationError::EmptyMibDir) => "MIB_DIR is empty".into(),
        CliError::Configuration(ConfigurationError::InvalidArguments(message)) => message.clone(),
        CliError::Load(error) => error.to_string(),
        CliError::Output(error) => format!("cannot write output: {error}"),
    };
    let _ = writeln!(stderr, "mibl: {message}");
    ExitCode::from(2)
}
fn run() -> Result<ExitCode, CliError> {
    let configuration = configure(
        std::env::args_os().skip(1).collect(),
        std::env::var_os("MIB_DIR"),
    )
    .map_err(CliError::Configuration)?;
    setup_tracing(configuration.debug);
    let mib = Mib::load(&configuration.directory).map_err(CliError::Load)?;
    let response = query(&mib, &configuration.request);
    let mut stdout = io::stdout().lock();
    let status = render(&response, configuration.details, &mut stdout).map_err(CliError::Output)?;
    io::Write::flush(&mut stdout).map_err(CliError::Output)?;
    Ok(status)
}
fn main() -> ExitCode {
    match run() {
        Ok(status) => status,
        Err(error) => report_error(&error, &mut io::stderr().lock()),
    }
}
