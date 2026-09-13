//! Application-owned configuration, tracing, rendering and exit status.
#![allow(dead_code)] // Other request/response variants belong to later viewer slices.
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
    let debug = args.first().is_some_and(|arg| arg == "--debug");
    if debug {
        args = &args[1..];
    }
    let request = match args {
        [verb, name] if verb == "parameter" && !name.is_empty() => {
            let name = name.to_str().ok_or_else(|| {
                ConfigurationError::InvalidArguments("parameter name must be UTF-8".into())
            })?;
            Request::Parameter(ParameterName(name.into()))
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
                "usage: mibl [--debug] parameter NAME | packet SPID".into(),
            ));
        }
    };
    Ok(Configuration {
        directory: directory.into(),
        debug,
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
        Request::Packet(spid) => Response::Packet(mib.packet(*spid)),
        _ => todo!("request is not exposed until its owning viewer slice"),
    }
}

fn render(response: &Response, stdout: &mut dyn io::Write) -> Result<ExitCode, io::Error> {
    match response {
        Response::Parameter(Lookup::NotFound(_)) | Response::Packet(Lookup::NotFound(_)) => {
            return Ok(ExitCode::from(1));
        }
        Response::Parameter(Lookup::Found(description)) => render_parameter(description, stdout)?,
        Response::Packet(Lookup::Found(description)) => render_packet(description, stdout)?,
        Response::Parameter(Lookup::Ambiguous(candidates))
        | Response::Packet(Lookup::Ambiguous(candidates)) => {
            render_candidates(
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

fn render_parameter(description: &ParameterDescription, out: &mut dyn io::Write) -> io::Result<()> {
    render_parameter_summary(&description.parameter, out)?;
    render_problems(&description.occurrences.problems, out)?;
    if let Some(packets) = &description.occurrences.value {
        for packet in packets {
            render_problems(&packet.packet.problems, out)?;
            if let Some(summary) = &packet.packet.value {
                render_packet_summary(summary, out)?;
            }
            for occurrence in &packet.occurrences {
                render_occurrence(occurrence, false, out)?;
            }
        }
    }
    Ok(())
}
fn render_parameter_summary(p: &ParameterSummary, out: &mut dyn io::Write) -> io::Result<()> {
    writeln!(out, "Parameter: {}", p.name.0.escape_debug())?;
    writeln!(out, "Description: {}", display_text(&p.description))?;
    let encoding = &p.encoding;
    let type_name = match encoding.ptc.value {
        Some(1) => "Boolean",
        Some(2) => "Enumerated",
        Some(3) => "Unsigned integer",
        Some(4) => "Signed integer",
        Some(5) => "Real",
        Some(6) => "Bit string",
        Some(7) => "Octet string",
        Some(8) => "Character string",
        Some(9) => "Absolute time",
        Some(10) => "Relative time",
        Some(11) => "Deduced",
        Some(13) => "Saved synthetic",
        _ => "Unsupported",
    };
    writeln!(
        out,
        "Type: {type_name}, PTC {}, PFC {}",
        display_number(&encoding.ptc),
        display_number(&encoding.pfc)
    )?;
    writeln!(
        out,
        "Encoded width: {}",
        encoding
            .encoded_bits
            .value
            .map_or_else(|| "unavailable".into(), |n| format!("{n} bits"))
    )?;
    writeln!(out, "Endian: {}", display_text(&encoding.endian))?;
    writeln!(out, "Units: {}", display_text(&p.units))?;
    for problems in [
        &encoding.ptc.problems,
        &encoding.pfc.problems,
        &encoding.encoded_bits.problems,
        &encoding.endian.problems,
        &p.units.problems,
    ] {
        render_problems(problems, out)?;
    }
    render_definition(&p.definition, out)?;
    render_problems(&p.calibrations.problems, out)
}
fn render_definition(definition: &Definition, out: &mut dyn io::Write) -> io::Result<()> {
    writeln!(
        out,
        "Source: {}:{}",
        definition.source.file.display(),
        definition.source.line
    )?;
    writeln!(out, "Column\tField\tRecorded presence\tInterpretation")?;
    for field in &definition.fields {
        let recorded = match &field.presence {
            Presence::Omitted => "omitted".into(),
            Presence::Empty => "empty".into(),
            Presence::Text(text) => format!("text {text:?}"),
        };
        for meaning in &field.meanings {
            let interpreted = meaning.interpretation.value.as_ref().map_or_else(
                || "unavailable".into(),
                |i| {
                    let value = display_scalar(&i.value);
                    match &i.origin {
                        InterpretationOrigin::Recorded => value,
                        InterpretationOrigin::DocumentedDefault { rule } => {
                            format!("{value} [default: {rule}]")
                        }
                    }
                },
            );
            writeln!(
                out,
                "{}\t{}\t{recorded}\t{interpreted}",
                field.column, meaning.schema_name
            )?;
            render_problems(&meaning.interpretation.problems, out)?;
        }
    }
    Ok(())
}
fn render_packet_summary(packet: &PacketSummary, out: &mut dyn io::Write) -> io::Result<()> {
    writeln!(out, "Packet: {}", packet.spid.0)?;
    writeln!(out, "Name: {}", display_text(&packet.name))?;
    writeln!(out, "Description: {}", display_text(&packet.description))?;
    render_problems(&packet.name.problems, out)?;
    render_definition(&packet.definition, out)?;
    render_problems(&packet.characteristics.problems, out)?;
    if let Some(definitions) = &packet.characteristics.value {
        for definition in definitions {
            render_definition(definition, out)?;
        }
    }
    Ok(())
}
fn render_packet(packet: &PacketDescription, out: &mut dyn io::Write) -> io::Result<()> {
    render_packet_summary(&packet.packet, out)?;
    let identification = &packet.identification;
    for definition in &identification.definitions {
        render_definition(definition, out)?;
    }
    writeln!(out, "APID: {}", display_number(&identification.apid))?;
    writeln!(
        out,
        "Service type: {}",
        display_number(&identification.service_type)
    )?;
    writeln!(
        out,
        "Service subtype: {}",
        display_number(&identification.service_subtype)
    )?;
    for problems in [
        &identification.apid.problems,
        &identification.service_type.problems,
        &identification.service_subtype.problems,
        &identification.criteria.problems,
    ] {
        render_problems(problems, out)?;
    }
    if let Some(criteria) = &identification.criteria.value {
        for criterion in criteria {
            writeln!(out, "Expected: {}", display_number(&criterion.expected))?;
            render_problems(&criterion.expected.problems, out)?;
            render_location(&criterion.extraction, out)?;
            for definition in &criterion.definitions {
                render_definition(definition, out)?;
            }
        }
    }
    render_problems(&packet.layout.problems, out)?;
    if let Some(layout) = &packet.layout.value {
        render_layout(layout, out)?;
    }
    Ok(())
}
fn render_layout(
    layout: &[Layout<ParameterOccurrence>],
    out: &mut dyn io::Write,
) -> io::Result<()> {
    for element in layout {
        match element {
            Layout::Element(occurrence) => render_occurrence(occurrence, true, out)?,
            Layout::Repeat {
                repetition,
                children,
                ..
            } => {
                render_repetition(repetition, out)?;
                render_layout(children, out)?;
            }
            Layout::Conditional {
                condition,
                children,
                ..
            } => {
                writeln!(out, "Condition: {}", condition.expression.escape_debug())?;
                render_layout(children, out)?;
            }
        }
    }
    Ok(())
}
fn render_occurrence(
    occurrence: &ParameterOccurrence,
    expand_parameter: bool,
    out: &mut dyn io::Write,
) -> io::Result<()> {
    writeln!(out, "Occurrence: {}", occurrence.reference.0.escape_debug())?;
    render_location(&occurrence.location, out)?;
    for enclosure in &occurrence.enclosing {
        match enclosure {
            Enclosure::Repetition(repetition) => render_repetition(repetition, out)?,
            Enclosure::Condition(condition) => {
                writeln!(out, "Condition: {}", condition.expression.escape_debug())?
            }
        }
    }
    render_definition(&occurrence.definition, out)?;
    render_problems(&occurrence.parameter.problems, out)?;
    if expand_parameter && let Some(parameter) = &occurrence.parameter.value {
        render_parameter_summary(parameter, out)?;
    }
    Ok(())
}
fn render_repetition(repetition: &Info<Repetition>, out: &mut dyn io::Write) -> io::Result<()> {
    render_problems(&repetition.problems, out)?;
    match &repetition.value {
        Some(Repetition::Fixed { count, stride_bits }) => {
            writeln!(
                out,
                "Repeated {count} times, stride {} bits",
                display_number(stride_bits)
            )?;
            render_problems(&stride_bits.problems, out)?;
        }
        Some(Repetition::Runtime(declaration)) => writeln!(
            out,
            "Runtime repetition: {}",
            declaration.expression.escape_debug()
        )?,
        None => writeln!(out, "Repetition unavailable")?,
    }
    Ok(())
}
fn render_location(location: &Location, out: &mut dyn io::Write) -> io::Result<()> {
    match &location.position.value {
        Some(Position::PacketAbsolute { byte, bit }) => {
            writeln!(out, "Location: byte {byte} bit {bit}")?
        }
        Some(position) => writeln!(out, "Location: {position:?}")?,
        None => writeln!(out, "Location: unavailable")?,
    }
    writeln!(
        out,
        "Encoded width: {} bits",
        display_number(&location.encoded_bits)
    )?;
    render_problems(&location.position.problems, out)?;
    render_problems(&location.encoded_bits.problems, out)
}
fn display_scalar(value: &Scalar) -> String {
    match value {
        Scalar::Text(s) | Scalar::Code(s) | Scalar::Decimal(s) => format!("{s:?}"),
        Scalar::Integer(n) => n.to_string(),
        Scalar::Unsigned(n) => n.to_string(),
        Scalar::Boolean(b) => b.to_string(),
    }
}
fn display_text(info: &Info<String>) -> String {
    info.value
        .as_ref()
        .map_or_else(|| "unavailable".into(), |s| s.escape_debug().to_string())
}
fn display_number<T: std::fmt::Display>(info: &Info<T>) -> String {
    info.value
        .as_ref()
        .map_or_else(|| "unavailable".into(), ToString::to_string)
}
fn render_problems(problems: &[Problem], out: &mut dyn io::Write) -> io::Result<()> {
    for problem in problems {
        writeln!(
            out,
            "Unavailable/problem: {}",
            problem.explanation.escape_debug()
        )?;
    }
    Ok(())
}
fn render_candidates<'a>(
    candidates: impl Iterator<Item = &'a Candidate>,
    out: &mut dyn io::Write,
) -> io::Result<()> {
    writeln!(out, "Kind\tIdentity\tName\tDescription\tSource")?;
    for candidate in candidates {
        let (kind, identity) = match &candidate.identity {
            Identity::Parameter(name) => ("parameter", name.0.escape_debug().to_string()),
            Identity::Packet(spid) => ("packet", spid.0.to_string()),
            Identity::Command(name) => ("command", name.0.escape_debug().to_string()),
        };
        writeln!(
            out,
            "{kind}\t{identity}\t{}\t{}\t{}:{}",
            display_text(&candidate.name),
            display_text(&candidate.description),
            candidate.source.file.display(),
            candidate.source.line
        )?;
    }
    Ok(())
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
    let status = render(&response, &mut stdout).map_err(CliError::Output)?;
    io::Write::flush(&mut stdout).map_err(CliError::Output)?;
    Ok(status)
}
fn main() -> ExitCode {
    match run() {
        Ok(status) => status,
        Err(error) => report_error(&error, &mut io::stderr().lock()),
    }
}
