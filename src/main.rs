//! Application-owned configuration, tracing, rendering and exit status.
#![allow(dead_code)] // Other request/response variants belong to later viewer slices.
mod render;
use clap::{Arg, ArgAction, Command};
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
    Arguments(clap::Error),
}
enum CliError {
    Configuration(ConfigurationError),
    Load(LoadError),
    Output(io::Error),
}
fn cli() -> Command {
    Command::new("mibl")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Inspect SCOS MIB definitions")
        .subcommand_required(true)
        .args_override_self(true)
        .arg(
            Arg::new("debug")
                .long("debug")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Show loading and query diagnostics"),
        )
        .arg(
            Arg::new("details")
                .long("details")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Show recorded fields and problem evidence"),
        )
        .subcommand(
            Command::new("parameter")
                .about("Look up a monitoring parameter")
                .arg(
                    Arg::new("NAME")
                        .required(true)
                        .value_parser(clap::builder::NonEmptyStringValueParser::new()),
                ),
        )
        .subcommand(
            Command::new("packet")
                .about("Look up a telemetry packet definition")
                .arg(Arg::new("SPID").required(true).value_parser(parse_spid)),
        )
        .subcommand(
            Command::new("command")
                .about("Look up a telecommand definition")
                .arg(
                    Arg::new("NAME")
                        .required(true)
                        .value_parser(clap::builder::NonEmptyStringValueParser::new()),
                ),
        )
}

fn parse_spid(value: &str) -> Result<u64, String> {
    if !value.is_empty()
        && value.bytes().all(|b| b.is_ascii_digit())
        && let Ok(spid) = value.parse()
    {
        return Ok(spid);
    }
    Err("packet SPID must be an unsigned integer".into())
}

/// Preserve non-Unicode directory paths, resolving them only for lookup requests.
fn configure(
    arguments: Vec<OsString>,
    mib_dir: Option<OsString>,
) -> Result<Configuration, ConfigurationError> {
    let matches = cli()
        .try_get_matches_from(std::iter::once(OsString::from("mibl")).chain(arguments))
        .map_err(ConfigurationError::Arguments)?;
    let directory = mib_dir.ok_or(ConfigurationError::MissingMibDir)?;
    if directory.is_empty() {
        return Err(ConfigurationError::EmptyMibDir);
    }
    let request = match matches.subcommand() {
        Some(("parameter", args)) => Request::Parameter(ParameterName(
            args.get_one::<String>("NAME")
                .expect("required NAME")
                .clone(),
        )),
        Some(("command", args)) => Request::Command(CommandName(
            args.get_one::<String>("NAME")
                .expect("required NAME")
                .clone(),
        )),
        Some(("packet", args)) => Request::Packet(PacketSpid(
            *args.get_one::<u64>("SPID").expect("required SPID"),
        )),
        _ => unreachable!("clap requires a declared subcommand"),
    };
    Ok(Configuration {
        directory: directory.into(),
        debug: matches.get_flag("debug"),
        details: matches.get_flag("details"),
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
            return Ok(ExitCode::from(3));
        }
        _ => todo!("response is not exposed until its owning viewer slice"),
    }
    Ok(ExitCode::SUCCESS)
}

fn report_error(error: &CliError, stderr: &mut dyn io::Write) -> ExitCode {
    let message = match error {
        CliError::Configuration(ConfigurationError::MissingMibDir) => "MIB_DIR is not set".into(),
        CliError::Configuration(ConfigurationError::EmptyMibDir) => "MIB_DIR is empty".into(),
        CliError::Configuration(ConfigurationError::Arguments(error)) => {
            let result = if error.use_stderr() {
                write!(stderr, "{error}")
            } else {
                error.print()
            };
            return match result {
                Ok(()) => ExitCode::from(error.exit_code() as u8),
                Err(error) => report_error(&CliError::Output(error), stderr),
            };
        }
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
