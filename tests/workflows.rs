//! Cross-feature acceptance scenarios for the complete viewer (issue #18).
//!
//! One synthetic snapshot carries every canonical supporting table at once, so
//! calibration, packet structures, command rules, search and incomplete-data
//! behaviour are exercised together rather than one slice at a time. It is a
//! publishable equivalent of the surveyed `ZUT00002`, packet `89000`,
//! `S2KTC001`, `S2KTC074` and `mode` search cases. No supplied reference file or
//! example MIB row is reproduced here.
mod common;
use common::Fixture;
use mibl::{LoadError, Mib, model::*};
use std::{
    collections::BTreeSet,
    io,
    num::NonZeroUsize,
    path::{Component, Path, PathBuf},
    process::{Command, Output},
};

/// A status-category parameter whose referenced key belongs to the textual family, so its
/// category and reference disagree, plus a parameter whose calibration key resolves nowhere.
const PCF: &str = "\
DEMO_MODE\tOperational mode\t\t\t3\t4\t\t\t\tN\tR\tDEMO_MODE_TXF
DEMO_TEMP\tTemperature\t\tK\t3\t12\t\t\t\tN\tR\tDEMO_TEMP_CAF
DEMO_STATE\tState\t\t\t3\t4\t\t\t\tS\tR\tDEMO_STATE_TXF
DEMO_COUNT\tCount\t\t\t3\t4\t\t\t\tN\tR
DEMO_ABSENT\tParameter whose calibration is undeclared\t\t\t3\t4\t\t\t\tN\tR\tDEMO_ABSENT_CAL
";
/// A fixed packet with a repeated occurrence beside a variable packet sharing its parameters.
const PID: &str = "\
3\t25\t42\t7\t0\t89000\tDemonstration housekeeping\t\t-1\t10
3\t26\t42\t0\t0\t89001\tDemonstration variable packet\t\t7\t10
";
const TPCF: &str = "\
89000\tDEMO_HK\t32
89001\tDEMO_VAR
";
const PIC: &str = "\
3\t25\t16\t8\t-1\t0\t42
3\t26\t16\t8\t-1\t0\t42
";
const PLF: &str = "\
DEMO_MODE\t89000\t19\t0\t1\t0\t0\t1
DEMO_TEMP\t89000\t20\t0\t2\t16\t0\t1
DEMO_STATE\t89000\t24\t0\t1\t0\t0\t1
";
const VPD: &str = "\
7\t1\tDEMO_COUNT\t2\t0\tN\tN\t\t0
7\t2\tDEMO_TEMP\t0\t0\tN\tN\t\t0
7\t3\tDEMO_MODE\t0\t0\tN\tN\t\t0
";
/// Two conditional alternatives for `DEMO_TEMP` beside its direct `PCF_CURTX` declaration.
const CUR: &str = "\
DEMO_TEMP\t1\tDEMO_MODE\t0\tDEMO_TEMP_MCF
DEMO_TEMP\t2\tDEMO_MODE\t1\tDEMO_TEMP_LGF
";
const CAF: &str = "DEMO_TEMP_CAF\tTemperature calibration\tR\tU\tH";
const CAP: &str = "\
DEMO_TEMP_CAF\tA\t1.5
DEMO_TEMP_CAF\tFF\t2.5
";
const MCF: &str = "DEMO_TEMP_MCF\tTemperature polynomial\t1.5";
const LGF: &str = "DEMO_TEMP_LGF\tTemperature logarithm\t2";
/// Two textual headers share one key, so its reference keeps both definitions.
const TXF: &str = "\
DEMO_STATE_TXF\tState calibration\tU\t2
DEMO_STATE_TXF\tState calibration duplicate\tU\t2
DEMO_MODE_TXF\tMode calibration\tU\t2
";
const TXP: &str = "\
DEMO_STATE_TXF\t0\t1\tIDLE
DEMO_STATE_TXF\t2\t3\tACTIVE
DEMO_MODE_TXF\t0\t0\tOFFLINE
DEMO_MODE_TXF\t1\t1\tONLINE
";
const CCF: &str = "\
DEMO_TC001\tDemonstration command one\tDistribute demonstration commands\t\tN\tDEMO_HDR01\t3\t25\t42\t3
DEMO_TC074\tDemonstration command two\tEnable forwarding of demonstration packets\t\tN\tDEMO_HDR02\t14\t1\t28\t6
DEMO_TC003\tThird demonstration command\tDeclared widths and an empty header\t\tN\tDEMO_HDR03\t3\t25\t42\t2
";
/// One malformed row is dropped while every neighbouring declaration stays usable.
const CDF: &str = "\
DEMO_TC001\tE\tFirst argument\t8\t0\t\tARG1\tR
DEMO_TC001\tE\tSecond argument\t8\t8\t\tARG2\tT\t0\tDEMO_COUNT
DEMO_TC001\tE\tThird argument\t8\t16\t\tARG3\tR
DEMO_TC074\tF\tGroup count\t8\t0\t6\tN1\tR\t1
DEMO_TC074\tE\tApplication id\t16\t8\t\tAPID\tR
DEMO_TC074\tA\tPadding\t4\t24\t\t\t\tF
DEMO_TC074\tF\tType count\t8\t28\t3\tN2\tR\t1
DEMO_TC074\tE\tType\t8\t36\t\tTYPE\tR
DEMO_TC074\tF\tSubtype count\t8\t44\t1\tN3\tR\t1
DEMO_TC074\tE\tSubtype\t8\t52\t\tSUBTYPE\tR
DEMO_TC003\tE\tAgreeing width\t8\t0\t\tARG4\tR
DEMO_TC003\tE\tDisagreeing width\t16\t8\t\tARG5\tR
DEMO_ORPHAN\tX\tUnsupported element type\t8\t0
";
const CPC: &str = "\
ARG1\tFirst argument\t3\t4\tR\tH\t\tN\tDEMO_RANGE_1\tDEMO_CONV_1\tDEMO_ALIAS_1
ARG2\tSecond argument\t3\t4\tR\tO\t\tN
ARG3\tThird argument\t3\t4\tR\tH\t\tN\tDEMO_ABSENT_RANGE
N1\tGroup count\t3\t4
APID\tApplication id\t3\t12
N2\tType count\t3\t4
TYPE\tType\t3\t4\tR\tH\t\tN\tDEMO_RANGE_2
N3\tSubtype count\t3\t4
SUBTYPE\tSubtype\t3\t4
ARG4\tAgreeing argument\t3\t4
ARG5\tDisagreeing argument\t3\t4
ORPHAN_ARG\tOrphan argument\tbroken\t4
";
const PRF: &str = "\
DEMO_RANGE_1\tFirst argument range\tE\tU\tH\t2\tmAmp
DEMO_RANGE_2\tType range\tR\tU\tD\t1
";
const PRV: &str = "\
DEMO_RANGE_1\t20\t7F
DEMO_RANGE_1\t0\t3
DEMO_RANGE_2\t0\t2
";
const CCA: &str = "DEMO_CONV_1\tFirst argument conversion\tR\tU\tH\tmAmp\t2";
const CCS: &str = "\
DEMO_CONV_1\t10\t1.5
DEMO_CONV_1\tFF\t3.5
";
const PAF: &str = "DEMO_ALIAS_1\tFirst argument alias\tR\t2";
const PAS: &str = "\
DEMO_ALIAS_1\tLOW\t0
DEMO_ALIAS_1\tHIGH\t2.5
";
/// One row carries unknown extra trailing columns the compatibility policy ignores.
const TCP: &str = "\
DEMO_HDR01\tDemonstration command header
DEMO_HDR02\tExtended demonstration header
DEMO_HDR03\tDemonstration header without elements\t\t\textra\tcolumns
";
const PCDF: &str = "\
DEMO_HDR01\tPkt Version Number\tF\t3\t0\t\t1\tD
DEMO_HDR01\tPkt Type\tF\t1\t3\t\t1\tD
DEMO_HDR01\tAPID\tA\t11\t5\tHP002\t2A\t
DEMO_HDR01\tSequence Count\tP\t14\t16\tHP004\t0\t
DEMO_HDR01\tAck Flags\tK\t4\t30\tHP001\t0\t
DEMO_HDR01\tService Type\tT\t8\t34\tHP005\t0\t
DEMO_HDR01\tService Subtype\tS\t8\t42\tHP006\t0\t
DEMO_HDR01\tPkt Length\tP\t16\t50\tHP003\t0\t
DEMO_HDR02\tPkt Version Number\tF\t3\t0\t\t0\tD
DEMO_HDR02\tAPID\tA\t11\t5\tHP002\t2A\t
DEMO_HDR02\tService Type\tT\t8\t16\tHP005\t0\t
DEMO_HDR02\tService Subtype\tS\t8\t24\tHP006\t0\t
";
const PCPC: &str = "\
HP001\tAcknowledgement flags\tU
HP002\tApplication process id\tU
HP003\tPacket length\tI
HP004\tSequence count\tI
HP005\tService type\tU
HP006\tService subtype\tU
";

/// Every canonical supporting table family the contract names, in load order.
const TABLES: [(&str, &str); 25] = [
    ("pcf.dat", PCF),
    ("pid.dat", PID),
    ("tpcf.dat", TPCF),
    ("pic.dat", PIC),
    ("plf.dat", PLF),
    ("vpd.dat", VPD),
    ("cur.dat", CUR),
    ("caf.dat", CAF),
    ("cap.dat", CAP),
    ("mcf.dat", MCF),
    ("lgf.dat", LGF),
    ("txf.dat", TXF),
    ("txp.dat", TXP),
    ("ccf.dat", CCF),
    ("cdf.dat", CDF),
    ("cpc.dat", CPC),
    ("prf.dat", PRF),
    ("prv.dat", PRV),
    ("cca.dat", CCA),
    ("ccs.dat", CCS),
    ("paf.dat", PAF),
    ("pas.dat", PAS),
    ("tcp.dat", TCP),
    ("pcdf.dat", PCDF),
    ("pcpc.dat", PCPC),
];

fn write_snapshot(dir: &Fixture) {
    for (file, text) in TABLES {
        dir.write(file, text);
    }
}

fn combined() -> Fixture {
    let dir = Fixture::new();
    write_snapshot(&dir);
    dir
}

fn loaded(dir: &Fixture) -> Mib {
    Mib::load(dir.path()).unwrap()
}

fn parameter(mib: &Mib, name: &str) -> ParameterDescription {
    let Lookup::Found(description) = mib.parameter(&ParameterName(name.into())) else {
        panic!("expected parameter {name}")
    };
    description
}

fn packet(mib: &Mib, spid: u64) -> PacketDescription {
    let Lookup::Found(description) = mib.packet(PacketSpid(spid)) else {
        panic!("expected packet {spid}")
    };
    description
}

fn command(mib: &Mib, name: &str) -> CommandDescription {
    let Lookup::Found(description) = mib.command(&CommandName(name.into())) else {
        panic!("expected command {name}")
    };
    description
}

fn source(file: &str, line: usize) -> Source {
    Source {
        file: PathBuf::from(file),
        line: NonZeroUsize::new(line).unwrap(),
    }
}

fn spids(packets: &[PacketOccurrences]) -> Vec<u64> {
    packets
        .iter()
        .map(|group| group.packet.value.as_ref().unwrap().spid.0)
        .collect()
}

/// Declared positions exactly as the contract describes them.
fn position_text(position: &Info<Position>) -> String {
    match &position.value {
        Some(Position::PacketAbsolute { byte, bit }) => format!("byte {byte} bit {bit}"),
        Some(Position::RelativeBits(bits)) => format!("relative {bits} bits"),
        Some(Position::ApplicationDeclaredBit(bit)) => format!("application-declared bit {bit}"),
        Some(Position::HeaderBit(bit)) => format!("header bit {bit}"),
        Some(Position::Runtime(_)) => "runtime".into(),
        None => "unavailable".into(),
    }
}

fn repetition_text(repetition: &Info<Repetition>) -> String {
    match &repetition.value {
        Some(Repetition::Fixed { count, stride_bits }) => {
            format!("fixed {count} stride {:?}", stride_bits.value)
        }
        Some(Repetition::Runtime(declaration)) => format!("runtime {}", declaration.expression),
        None => "unavailable".into(),
    }
}

fn presence_text(presence: &Presence) -> String {
    match presence {
        Presence::Omitted => "omitted".into(),
        Presence::Empty => "empty".into(),
        Presence::Text(text) => text.clone(),
    }
}

/// Every declared argument, whether or not a group encloses it.
fn declared_arguments(layout: &[Layout<CommandElement>]) -> Vec<&CommandArgument> {
    let mut arguments = Vec::new();
    for node in layout {
        match node {
            Layout::Element(CommandElement::Argument(a)) => arguments.push(a.as_ref()),
            Layout::Element(CommandElement::Fixed(_)) => {}
            Layout::Repeat { children, .. } | Layout::Conditional { children, .. } => {
                arguments.extend(declared_arguments(children));
            }
        }
    }
    arguments
}

fn argument_text(argument: &CommandArgument) -> String {
    format!(
        "{} at {} width {:?}",
        presence_text(&argument.element.fields[6].presence),
        position_text(&argument.location.position),
        argument.location.encoded_bits.value
    )
}

fn command_node_text(node: &Layout<CommandElement>) -> String {
    match node {
        Layout::Element(CommandElement::Argument(a)) => argument_text(a),
        Layout::Element(CommandElement::Fixed(f)) => format!(
            "Fixed area {} at {} width {:?}",
            presence_text(&f.definition.fields[2].presence),
            position_text(&f.location.position),
            f.location.encoded_bits.value
        ),
        Layout::Repeat {
            definition,
            repetition,
            children,
        } => format!(
            "repeat {}:{} {} [{}]",
            definition.source.file.display(),
            definition.source.line,
            repetition_text(repetition),
            children
                .iter()
                .map(command_node_text)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Layout::Conditional { children, .. } => format!(
            "conditional [{}]",
            children
                .iter()
                .map(command_node_text)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// The discrepancy evidence a comparison leaves beside usable information.
fn discrepancy(problems: &[Problem]) -> (&[String], &[Scalar]) {
    problems
        .iter()
        .find_map(|problem| match &problem.kind {
            ProblemKind::InconsistentDefinition { fields, values, .. } => {
                Some((fields.as_slice(), values.as_slice()))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected a discrepancy problem: {problems:?}"))
}

fn literal(value: &Info<ArgumentValue>) -> Option<Scalar> {
    match &value.value.as_ref()?.source {
        ValueSource::Literal(scalar) => Some(scalar.clone()),
        _ => None,
    }
}

/// Every query kind of the combined snapshot, formatted for comparison.
fn query_digest(mib: &Mib) -> String {
    format!(
        "parameter {:?}\npacket {:?}\ncommand {:?}\nsearch {:?}\npus {:?}",
        mib.parameter(&ParameterName("DEMO_TEMP".into())),
        mib.packet(PacketSpid(89001)),
        mib.command(&CommandName("DEMO_TC074".into())),
        mib.search("mode", SearchScope::All),
        mib.pus(3, Some(25)),
    )
}

fn run(dir: &Fixture, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mibl"))
        .env("MIB_DIR", dir.path())
        .args(args)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

/// Definition headers of `--details` output, which name every reachable source.
fn definition_sources(text: &str) -> Vec<&str> {
    text.lines()
        .filter_map(|line| line.strip_prefix("[D"))
        .filter_map(|line| line.split_once("] ").map(|(_, source)| source))
        .collect()
}

#[test]
fn parameter_scenario_combines_calibration_occurrences_and_recorded_fields() {
    let dir = combined();
    let mib = loaded(&dir);
    // A ZUT00002 equivalent: definition, encoding and a fixed occurrence beside a calibration
    // whose declared family cannot supply the referenced key.
    let mode = parameter(&mib, "DEMO_MODE");
    assert_eq!(mode.parameter.name.0, "DEMO_MODE");
    assert_eq!(
        mode.parameter.description.value.as_deref(),
        Some("Operational mode")
    );
    assert_eq!(mode.parameter.encoding.ptc.value, Some(3));
    assert_eq!(mode.parameter.encoding.pfc.value, Some(4));
    assert_eq!(mode.parameter.encoding.encoded_bits.value, Some(8));
    assert_eq!(mode.parameter.encoding.endian.value.as_deref(), Some("B"));
    assert_eq!(mode.parameter.units.value, None);
    assert_eq!(mode.parameter.definition.source, source("pcf.dat", 1));
    // Recorded presence, documented defaults and interpreted values stay separate.
    assert_eq!(mode.parameter.definition.fields.len(), 24);
    assert_eq!(
        mode.parameter.definition.fields[9].presence,
        Presence::Text("N".into())
    );
    assert_eq!(
        mode.parameter.definition.fields[12].presence,
        Presence::Omitted
    );
    assert!(matches!(
        mode.parameter.definition.fields[12].meanings[0]
            .interpretation
            .value
            .as_ref()
            .unwrap()
            .origin,
        InterpretationOrigin::DocumentedDefault { .. }
    ));
    // The surveyed category/reference disagreement keeps its definition and intervals usable.
    let alternatives = mode.parameter.calibrations.value.as_ref().unwrap();
    assert_eq!(alternatives.len(), 1);
    let calibration = alternatives[0].calibration.value.as_ref().unwrap();
    assert_eq!(calibration.definition.source, source("txf.dat", 3));
    let Some(CalibrationForm::Textual { intervals }) = &calibration.form.value else {
        panic!("expected the textual family")
    };
    let intervals = intervals.value.as_ref().unwrap();
    assert_eq!(intervals.len(), 2);
    assert_eq!(intervals[0].text.value.as_deref(), Some("OFFLINE"));
    assert_eq!(intervals[1].definition.source, source("txp.dat", 4));
    let (fields, values) = discrepancy(&alternatives[0].calibration.problems);
    assert_eq!(
        fields,
        &[
            "PCF_CATEG".to_string(),
            "PCF_CURTX".to_string(),
            "PCF_PTC".to_string(),
            "CUR_SELECT".to_string()
        ]
    );
    assert_eq!(
        values,
        &[
            Scalar::Code("N".into()),
            Scalar::Text("DEMO_MODE_TXF".into())
        ]
    );
    // The fixed declared slot and the runtime group of the variable packet stay separate.
    let packets = mode.occurrences.value.as_ref().unwrap();
    assert_eq!(spids(packets), vec![89000, 89001]);
    let fixed = &packets[0].occurrences[0];
    assert_eq!(position_text(&fixed.location.position), "byte 19 bit 0");
    assert_eq!(fixed.location.encoded_bits.value, Some(8));
    assert!(fixed.enclosing.is_empty() && fixed.enclosing_definitions.is_empty());
    assert_eq!(fixed.definition.source, source("plf.dat", 1));
    let variable = &packets[1].occurrences[0];
    assert_eq!(
        position_text(&variable.location.position),
        "relative 16 bits"
    );
    assert_eq!(variable.definition.source, source("vpd.dat", 3));
    let [Enclosure::Repetition(repetition)] = &variable.enclosing[..] else {
        panic!("expected the declared group")
    };
    let Some(Repetition::Runtime(declaration)) = &repetition.value else {
        panic!("expected a runtime repetition")
    };
    assert!(declaration.dependencies.iter().any(
        |dependency| matches!(&dependency.reference, Reference::Root(Identity::Parameter(name)) if name.0 == "DEMO_COUNT")
    ));
    assert!(
        repetition
            .problems
            .iter()
            .any(|problem| matches!(problem.kind, ProblemKind::RuntimeDependent { .. }))
    );
    assert_eq!(variable.enclosing_definitions.len(), 1);
    assert_eq!(
        variable.enclosing_definitions[0].source,
        source("vpd.dat", 1)
    );

    // Bounded fixed repetition expands, and conditional alternatives keep declared order.
    let temperature = parameter(&mib, "DEMO_TEMP");
    let packets = temperature.occurrences.value.as_ref().unwrap();
    assert_eq!(spids(packets), vec![89000, 89001]);
    assert_eq!(
        packets
            .iter()
            .map(|group| group.occurrences.len())
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
    for (index, occurrence) in packets[0].occurrences.iter().enumerate() {
        assert_eq!(
            position_text(&occurrence.location.position),
            format!("byte {} bit 0", 20 + 2 * index)
        );
        assert_eq!(occurrence.location.encoded_bits.value, Some(16));
        let [Enclosure::Repetition(repetition)] = &occurrence.enclosing[..] else {
            panic!("expected the fixed repetition")
        };
        assert_eq!(repetition_text(repetition), "fixed 2 stride Some(16)");
    }
    assert_eq!(
        position_text(&packets[1].occurrences[0].location.position),
        "relative 0 bits"
    );
    let alternatives = temperature.parameter.calibrations.value.as_ref().unwrap();
    assert_eq!(alternatives.len(), 3);
    assert_eq!(
        alternatives[0].selection.as_ref().unwrap().source,
        source("cur.dat", 1)
    );
    assert_eq!(
        alternatives[0].condition.as_ref().unwrap().expression,
        "raw(DEMO_MODE) = 0"
    );
    assert_eq!(
        alternatives[1].selection.as_ref().unwrap().source,
        source("cur.dat", 2)
    );
    assert_eq!(
        alternatives[1].condition.as_ref().unwrap().expression,
        "raw(DEMO_MODE) = 1"
    );
    assert!(alternatives[2].selection.is_none() && alternatives[2].condition.is_none());
    for (alternative, (table, key)) in alternatives.iter().zip([
        (Table::Mcf, "DEMO_TEMP_MCF"),
        (Table::Lgf, "DEMO_TEMP_LGF"),
        (Table::Caf, "DEMO_TEMP_CAF"),
    ]) {
        let calibration = alternative.calibration.value.as_ref().unwrap();
        assert!(matches!(
            &calibration.reference,
            Reference::Supporting { table: found, key: found_key } if *found == table && found_key == key
        ));
        // The simultaneous direct and conditional declarations stay visible with their fields.
        let (fields, values) = discrepancy(&alternative.calibration.problems);
        assert!(
            values.contains(&Scalar::Text("DEMO_TEMP_CAF".into())),
            "{values:?}"
        );
        assert_eq!(
            fields,
            &[
                "PCF_CATEG".to_string(),
                "PCF_CURTX".to_string(),
                "PCF_PTC".to_string(),
                "CUR_SELECT".to_string()
            ]
        );
    }
    let Some(CalibrationForm::Numerical {
        points,
        interpolation,
    }) = &alternatives[2]
        .calibration
        .value
        .as_ref()
        .unwrap()
        .form
        .value
    else {
        panic!("expected the numerical family")
    };
    let points = points.value.as_ref().unwrap();
    assert_eq!(points[0].raw.value, Some(Scalar::Unsigned(10)));
    assert_eq!(
        points[0].engineering.value,
        Some(Scalar::Decimal("1.5".into()))
    );
    assert_eq!(points[1].raw.value, Some(Scalar::Unsigned(255)));
    assert_eq!(points[1].definition.source, source("cap.dat", 2));
    assert_eq!(interpolation.value.as_deref(), Some("F"));

    // A duplicated linked definition keeps every candidate beside a usable interval list.
    let state = parameter(&mib, "DEMO_STATE");
    let alternatives = state.parameter.calibrations.value.as_ref().unwrap();
    assert_eq!(alternatives.len(), 2);
    for (alternative, line) in alternatives.iter().zip([1, 2]) {
        let calibration = alternative.calibration.value.as_ref().unwrap();
        assert_eq!(calibration.definition.source, source("txf.dat", line));
        let ProblemKind::AmbiguousReference {
            reference,
            alternatives: candidates,
        } = &alternative.calibration.problems[0].kind
        else {
            panic!("{:?}", alternative.calibration.problems)
        };
        assert!(matches!(
            reference,
            Reference::Supporting { table: Table::Txf, key } if key == "DEMO_STATE_TXF"
        ));
        assert_eq!(candidates.first.definition.source, source("txf.dat", 1));
        assert_eq!(candidates.second.definition.source, source("txf.dat", 2));
        let Some(CalibrationForm::Textual { intervals }) = &calibration.form.value else {
            panic!("expected the textual family")
        };
        assert_eq!(intervals.value.as_ref().unwrap().len(), 2);
    }
    // A key that resolves nowhere names every table the declared family would use.
    let absent = parameter(&mib, "DEMO_ABSENT");
    let alternative = &absent.parameter.calibrations.value.as_ref().unwrap()[0];
    assert!(alternative.calibration.value.is_none());
    for table in [Table::Caf, Table::Mcf, Table::Lgf] {
        assert!(
            alternative.calibration.problems.iter().any(|problem| matches!(
                &problem.kind,
                ProblemKind::MissingReference { reference: Reference::Supporting { table: found, key } }
                    if *found == table && key == "DEMO_ABSENT_CAL"
            )),
            "{table:?}: {:?}",
            alternative.calibration.problems
        );
    }
}

#[test]
fn packet_scenarios_combine_identification_fixed_expansion_and_runtime_groups() {
    let dir = combined();
    let mib = loaded(&dir);
    let housekeeping = packet(&mib, 89000);
    assert_eq!(housekeeping.packet.spid, PacketSpid(89000));
    assert_eq!(housekeeping.packet.name.value.as_deref(), Some("DEMO_HK"));
    assert_eq!(
        housekeeping.packet.description.value.as_deref(),
        Some("Demonstration housekeeping")
    );
    assert_eq!(housekeeping.identification.apid.value, Some(42));
    assert_eq!(housekeeping.identification.service_type.value, Some(3));
    assert_eq!(housekeeping.identification.service_subtype.value, Some(25));
    let criteria = housekeeping.identification.criteria.value.as_ref().unwrap();
    assert_eq!(criteria.len(), 1);
    assert_eq!(criteria[0].expected.value, Some(7));
    assert_eq!(
        position_text(&criteria[0].extraction.position),
        "byte 16 bit 0"
    );
    assert_eq!(criteria[0].extraction.encoded_bits.value, Some(8));
    // Declared order, bounded expansion of the two-place repetition and embedded calibrations.
    let layout = housekeeping.layout.value.as_ref().unwrap();
    let described: Vec<String> = layout
        .iter()
        .map(|node| {
            let Layout::Element(occurrence) = node else {
                panic!("expected a fixed occurrence: {node:?}")
            };
            format!(
                "{} at {} width {:?} enclosing {} calibrations {}",
                occurrence.reference.0,
                position_text(&occurrence.location.position),
                occurrence.location.encoded_bits.value,
                occurrence.enclosing.len(),
                occurrence
                    .parameter
                    .value
                    .as_ref()
                    .unwrap()
                    .calibrations
                    .value
                    .as_ref()
                    .map_or(0, Vec::len)
            )
        })
        .collect();
    assert_eq!(
        described,
        vec![
            "DEMO_MODE at byte 19 bit 0 width Some(8) enclosing 0 calibrations 1",
            "DEMO_TEMP at byte 20 bit 0 width Some(16) enclosing 1 calibrations 3",
            "DEMO_TEMP at byte 22 bit 0 width Some(16) enclosing 1 calibrations 3",
            "DEMO_STATE at byte 24 bit 0 width Some(8) enclosing 0 calibrations 2",
        ]
    );
    for node in &layout[1..3] {
        let Layout::Element(occurrence) = node else {
            panic!()
        };
        let [Enclosure::Repetition(repetition)] = &occurrence.enclosing[..] else {
            panic!()
        };
        assert_eq!(repetition_text(repetition), "fixed 2 stride Some(16)");
    }
    // The variable packet keeps its declared groups and runtime dependencies unexpanded.
    let variable = packet(&mib, 89001);
    assert_eq!(variable.packet.name.value.as_deref(), Some("DEMO_VAR"));
    let layout = variable.layout.value.as_ref().unwrap();
    assert_eq!(layout.len(), 2);
    let Layout::Element(count) = &layout[0] else {
        panic!()
    };
    assert_eq!(count.reference.0, "DEMO_COUNT");
    assert_eq!(position_text(&count.location.position), "byte 10 bit 0");
    let Layout::Repeat {
        definition,
        repetition,
        children,
    } = &layout[1]
    else {
        panic!("expected the declared group")
    };
    assert_eq!(definition.source, source("vpd.dat", 1));
    assert!(repetition_text(repetition).contains("value of DEMO_COUNT"));
    let Some(Repetition::Runtime(declaration)) = &repetition.value else {
        panic!()
    };
    let targets = declaration.dependencies[0].targets.value.as_ref().unwrap();
    assert_eq!(
        targets.len(),
        1,
        "dependency targets are recorded definitions only"
    );
    assert_eq!(targets[0].definition.source, source("pcf.dat", 4));
    let described: Vec<String> = children
        .iter()
        .map(|node| {
            let Layout::Element(occurrence) = node else {
                panic!()
            };
            format!(
                "{} at {} width {:?} enclosing {}",
                occurrence.reference.0,
                position_text(&occurrence.location.position),
                occurrence.location.encoded_bits.value,
                occurrence.enclosing.len()
            )
        })
        .collect();
    assert_eq!(
        described,
        vec![
            "DEMO_TEMP at relative 0 bits width Some(16) enclosing 1",
            "DEMO_MODE at relative 16 bits width Some(8) enclosing 1",
        ]
    );
    // Ordering is deterministic across independent loads of the same declarations.
    assert_eq!(query_digest(&loaded(&combined())), query_digest(&mib));
}

#[test]
fn command_scenarios_combine_argument_rules_nested_groups_and_expanded_headers() {
    let dir = combined();
    let mib = loaded(&dir);
    // S2KTC001 equivalent: three declared arguments, one of them sourced from telemetry.
    let one = command(&mib, "DEMO_TC001");
    assert_eq!(one.name.0, "DEMO_TC001");
    assert_eq!(
        one.description.value.as_deref(),
        Some("Demonstration command one")
    );
    assert_eq!(one.definition.source, source("ccf.dat", 1));
    let arguments = declared_arguments(one.arguments.value.as_ref().unwrap());
    assert_eq!(
        arguments
            .iter()
            .map(|argument| argument_text(argument))
            .collect::<Vec<_>>(),
        vec![
            "ARG1 at application-declared bit 0 width Some(8)",
            "ARG2 at application-declared bit 8 width Some(8)",
            "ARG3 at application-declared bit 16 width Some(8)",
        ]
    );
    let first = arguments[0];
    let ranges = first.rules.ranges.value.as_ref().unwrap();
    assert_eq!(ranges.len(), 2);
    assert_eq!(ranges[0].low.value, Some(Scalar::Unsigned(32)));
    assert_eq!(ranges[0].high.value, Some(Scalar::Unsigned(127)));
    assert_eq!(ranges[1].low.value, Some(Scalar::Unsigned(0)));
    assert_eq!(ranges[1].high.value, Some(Scalar::Unsigned(3)));
    assert_eq!(ranges[0].representation.value.as_deref(), Some("E"));
    assert_eq!(ranges[0].definition.source, source("prv.dat", 1));
    assert_eq!(
        first.rules.supporting_definitions[0].source,
        source("prf.dat", 1)
    );
    let aliases = first.rules.aliases.value.as_ref().unwrap();
    assert_eq!(aliases.len(), 2);
    assert_eq!(aliases[0].raw.value, Some(Scalar::Decimal("0".into())));
    assert_eq!(aliases[0].text.value.as_deref(), Some("LOW"));
    assert_eq!(aliases[1].raw.value, Some(Scalar::Decimal("2.5".into())));
    assert_eq!(aliases[1].definition.source, source("pas.dat", 2));
    let conversions = first.rules.calibrations.value.as_ref().unwrap();
    assert_eq!(conversions.len(), 1);
    let calibration = conversions[0].calibration.value.as_ref().unwrap();
    assert!(matches!(
        &calibration.reference,
        Reference::Supporting { table: Table::Cca, key } if key == "DEMO_CONV_1"
    ));
    let Some(CalibrationForm::CommandConversion {
        points,
        interpolation,
    }) = &calibration.form.value
    else {
        panic!("expected a command conversion")
    };
    let points = points.value.as_ref().unwrap();
    assert_eq!(points.len(), 2);
    assert_eq!(points[0].raw.value, Some(Scalar::Unsigned(16)));
    assert_eq!(
        points[0].engineering.value,
        Some(Scalar::Decimal("1.5".into()))
    );
    assert_eq!(points[1].raw.value, Some(Scalar::Unsigned(255)));
    assert_eq!(
        points[1].engineering.value,
        Some(Scalar::Decimal("3.5".into()))
    );
    assert_eq!(points[1].definition.source, source("ccs.dat", 2));
    assert!(interpolation.value.is_none() && interpolation.problems.is_empty());
    // Editable input and telemetry sourcing stay declared rather than evaluated.
    let Some(ValueSource::Runtime(declaration)) = &first
        .rules
        .element_value
        .value
        .as_ref()
        .map(|value| &value.source)
    else {
        panic!("expected editable input")
    };
    assert!(declaration.expression.contains("editable"));
    let Some(ValueSource::Telemetry {
        parameter,
        declaration,
    }) = &arguments[1]
        .rules
        .element_value
        .value
        .as_ref()
        .map(|value| &value.source)
    else {
        panic!("expected a telemetry source")
    };
    assert_eq!(parameter.0, "DEMO_COUNT");
    let targets = declaration.dependencies[0].targets.value.as_ref().unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].definition.source, source("pcf.dat", 4));
    // A declared reference without a usable target stays local to the affected rule family.
    let third = arguments[2];
    assert!(third.rules.ranges.value.is_none());
    assert!(third.rules.ranges.problems.iter().any(|problem| matches!(
        &problem.kind,
        ProblemKind::MissingReference { reference: Reference::Supporting { table: Table::Prf, key } }
            if key == "DEMO_ABSENT_RANGE"
    )));
    assert_eq!(third.rules.aliases.value.as_ref().unwrap().len(), 0);
    assert_eq!(
        position_text(&third.location.position),
        "application-declared bit 16"
    );
    // The expanded header stays separate from the application data and keeps every kind.
    let header = one.header.value.as_ref().unwrap();
    assert_eq!(header.definition.source, source("tcp.dat", 1));
    let fields = header.fields.value.as_ref().unwrap();
    assert_eq!(
        fields
            .iter()
            .map(|field| presence_text(&field.definition.fields[1].presence))
            .collect::<Vec<_>>(),
        vec![
            "Pkt Version Number",
            "Pkt Type",
            "APID",
            "Sequence Count",
            "Ack Flags",
            "Service Type",
            "Service Subtype",
            "Pkt Length"
        ]
    );
    assert_eq!(
        fields
            .iter()
            .map(|field| field.field_kind.value.clone().unwrap())
            .collect::<Vec<_>>(),
        vec!["F", "F", "A", "P", "K", "T", "S", "P"]
    );
    assert_eq!(
        fields
            .iter()
            .map(|field| position_text(&field.location.position))
            .collect::<Vec<_>>(),
        vec![
            "header bit 0",
            "header bit 3",
            "header bit 5",
            "header bit 16",
            "header bit 30",
            "header bit 34",
            "header bit 42",
            "header bit 50"
        ]
    );
    assert_eq!(
        fields
            .iter()
            .map(|field| field.location.encoded_bits.value)
            .collect::<Vec<_>>(),
        vec![
            Some(3),
            Some(1),
            Some(11),
            Some(14),
            Some(4),
            Some(8),
            Some(8),
            Some(16)
        ]
    );
    assert_eq!(
        fields
            .iter()
            .map(|field| literal(&field.value))
            .collect::<Vec<_>>(),
        vec![
            Some(Scalar::Unsigned(1)),
            Some(Scalar::Unsigned(1)),
            Some(Scalar::Unsigned(42)),
            Some(Scalar::Integer(0)),
            Some(Scalar::Unsigned(0)),
            Some(Scalar::Unsigned(0)),
            Some(Scalar::Unsigned(0)),
            Some(Scalar::Integer(0)),
        ]
    );
    assert_eq!(
        fields
            .iter()
            .map(|field| field
                .value
                .value
                .as_ref()
                .unwrap()
                .representation
                .value
                .clone()
                .unwrap())
            .collect::<Vec<_>>(),
        vec!["H", "H", "H", "D", "H", "H", "H", "D"]
    );
    assert_eq!(
        fields
            .iter()
            .map(|field| field
                .parameter
                .value
                .as_ref()
                .map(|target| match &target.reference {
                    Reference::Supporting { table, key } => format!("{table:?} {key}"),
                    other => format!("{other:?}"),
                }))
            .collect::<Vec<_>>(),
        vec![
            None,
            None,
            Some("Pcpc HP002".into()),
            Some("Pcpc HP004".into()),
            Some("Pcpc HP001".into()),
            Some("Pcpc HP005".into()),
            Some("Pcpc HP006".into()),
            Some("Pcpc HP003".into()),
        ]
    );
    assert_eq!(
        fields[2]
            .parameter
            .value
            .as_ref()
            .unwrap()
            .definition
            .source,
        source("pcpc.dat", 2)
    );
    // A fixed area reports the radix that governs it while keeping its recorded column.
    assert_eq!(
        fields[0].definition.fields[7].presence,
        Presence::Text("D".into())
    );
    assert_eq!(
        fields[0]
            .value
            .value
            .as_ref()
            .unwrap()
            .representation
            .value
            .as_deref(),
        Some("H")
    );

    // S2KTC074 equivalent: nested repetitions, a fixed area, argument rules and its own header.
    let two = command(&mib, "DEMO_TC074");
    let layout = two.arguments.value.as_ref().unwrap();
    assert_eq!(layout.len(), 2);
    assert_eq!(
        layout.iter().map(command_node_text).collect::<Vec<_>>(),
        vec![
            "N1 at application-declared bit 0 width Some(8)".to_string(),
            format!(
                "repeat cdf.dat:4 {} [{}]",
                "runtime CDF_GRPSIZE=6 repeats the following 6 declared element(s) \
using the recorded value 1 of N1; declared CDF_BIT positions assume one repetition",
                [
                    "APID at application-declared bit 8 width Some(16)".to_string(),
                    "Fixed area Padding at application-declared bit 24 width Some(4)".to_string(),
                    "N2 at application-declared bit 28 width Some(8)".to_string(),
                    format!(
                        "repeat cdf.dat:7 {} [{}]",
                        "runtime CDF_GRPSIZE=3 repeats the following 3 declared element(s) \
using the recorded value 1 of N2; declared CDF_BIT positions assume one repetition",
                        [
                            "TYPE at application-declared bit 36 width Some(8)".to_string(),
                            "N3 at application-declared bit 44 width Some(8)".to_string(),
                            format!(
                                "repeat cdf.dat:9 {} [{}]",
                                "runtime CDF_GRPSIZE=1 repeats the following 1 declared element(s) \
using the recorded value 1 of N3; declared CDF_BIT positions assume one repetition",
                                "SUBTYPE at application-declared bit 52 width Some(8)"
                            ),
                        ]
                        .join(", ")
                    ),
                ]
                .join(", ")
            ),
        ]
    );
    let Layout::Repeat { repetition, .. } = &layout[1] else {
        panic!()
    };
    let Some(Repetition::Runtime(declaration)) = &repetition.value else {
        panic!()
    };
    assert!(declaration.dependencies.iter().any(|dependency| matches!(
        &dependency.reference,
        Reference::Supporting { table: Table::Cpc, key } if key == "N1"
    )));
    let nested = declared_arguments(layout);
    assert_eq!(nested.len(), 6, "runtime counts stay unexpanded");
    assert_eq!(
        nested
            .iter()
            .map(|argument| argument_text(argument))
            .collect::<Vec<_>>(),
        vec![
            "N1 at application-declared bit 0 width Some(8)",
            "APID at application-declared bit 8 width Some(16)",
            "N2 at application-declared bit 28 width Some(8)",
            "TYPE at application-declared bit 36 width Some(8)",
            "N3 at application-declared bit 44 width Some(8)",
            "SUBTYPE at application-declared bit 52 width Some(8)",
        ]
    );
    // A group member can still declare its own rules, and repeaters keep recorded values.
    let nested_type = nested[3];
    let ranges = nested_type.rules.ranges.value.as_ref().unwrap();
    assert_eq!(ranges.len(), 1);
    assert_eq!(ranges[0].low.value, Some(Scalar::Unsigned(0)));
    assert_eq!(ranges[0].high.value, Some(Scalar::Unsigned(2)));
    assert_eq!(
        literal(&nested[0].rules.element_value),
        Some(Scalar::Text("1".into()))
    );
    // The fixed area keeps its declared width and content and repeats nothing.
    let Layout::Repeat { children, .. } = &layout[1] else {
        panic!()
    };
    let Layout::Element(CommandElement::Fixed(padding)) = &children[1] else {
        panic!("expected a fixed area: {:?}", children[1])
    };
    assert_eq!(padding.location.encoded_bits.value, Some(4));
    assert_eq!(literal(&padding.value), Some(Scalar::Text("F".into())));
    assert_eq!(
        two.header.value.as_ref().unwrap().definition.source,
        source("tcp.dat", 2)
    );

    // The declared length and the declared width are reconciled on the affected argument only.
    let three = command(&mib, "DEMO_TC003");
    let arguments = declared_arguments(three.arguments.value.as_ref().unwrap());
    assert_eq!(
        arguments
            .iter()
            .map(|argument| argument_text(argument))
            .collect::<Vec<_>>(),
        vec![
            "ARG4 at application-declared bit 0 width Some(8)",
            "ARG5 at application-declared bit 8 width Some(8)",
        ]
    );
    assert!(arguments[0].location.position.problems.is_empty());
    assert!(arguments[0].location.encoded_bits.problems.is_empty());
    let ProblemKind::InconsistentDefinition { fields, values, .. } =
        &arguments[1].location.encoded_bits.problems[0].kind
    else {
        panic!("{:?}", arguments[1].location.encoded_bits.problems)
    };
    assert_eq!(
        arguments[1].location.encoded_bits.value,
        Some(8),
        "the CPC encoding still supplies the declared width"
    );
    assert!(fields.contains(&"CDF_ELLEN".to_string()), "{fields:?}");
    assert!(values.contains(&Scalar::Integer(16)), "{values:?}");
    // A resolved header without retained elements stays distinct from an unavailable one.
    let header = three.header.value.as_ref().unwrap();
    assert_eq!(header.definition.source, source("tcp.dat", 3));
    assert_eq!(header.fields.value.as_ref().unwrap().len(), 0);
    assert!(header.fields.problems.is_empty());
}

#[test]
fn duplicate_roots_and_linked_definitions_stay_separate_and_ordered() {
    let dir = combined();
    dir.write(
        "pid.dat",
        "3\t25\t42\t7\t0\t89000\tDemonstration housekeeping\t\t-1\t10\n\
3\t26\t42\t0\t0\t89001\tDemonstration variable packet\t\t7\t10\n\
3\t25\t42\t7\t0\t89000\tDemonstration housekeeping duplicate\t\t-1\t10\n",
    );
    dir.write(
        "ccf.dat",
        "DEMO_TC001\tDemonstration command one\tDistribute demonstration commands\t\tN\tDEMO_HDR01\t3\t25\t42\t3\n\
DEMO_TC074\tDemonstration command two\tEnable forwarding of demonstration packets\t\tN\tDEMO_HDR02\t14\t1\t28\t6\n\
DEMO_TC003\tThird demonstration command\tDeclared widths and an empty header\t\tN\tDEMO_HDR03\t3\t25\t42\t2\n\
DEMO_TC001\tDemonstration command one duplicate\tDistribute demonstration commands\t\tN\tDEMO_HDR01\t3\t25\t42\t3\n",
    );
    dir.write(
        "pcf.dat",
        &format!("{PCF}DEMO_STATE\tDuplicate state\t\t\t3\t4\t\t\t\tS\tR\tDEMO_STATE_TXF\n"),
    );
    let mib = loaded(&dir);
    // Duplicate roots keep both definitions in source order and never select one.
    let Lookup::Ambiguous(candidates) = mib.packet(PacketSpid(89000)) else {
        panic!("expected candidates")
    };
    assert_eq!(candidates.first.source, source("pid.dat", 1));
    assert_eq!(candidates.second.source, source("pid.dat", 3));
    assert_eq!(candidates.first.name.value.as_deref(), Some("DEMO_HK"));
    assert_eq!(
        candidates.second.description.value.as_deref(),
        Some("Demonstration housekeeping duplicate")
    );
    assert_eq!(candidates.rest.len(), 0);
    let Lookup::Ambiguous(candidates) = mib.command(&CommandName("DEMO_TC001".into())) else {
        panic!("expected candidates")
    };
    assert_eq!(candidates.first.source, source("ccf.dat", 1));
    assert_eq!(candidates.second.source, source("ccf.dat", 4));
    let Lookup::Ambiguous(candidates) = mib.parameter(&ParameterName("DEMO_STATE".into())) else {
        panic!("expected candidates")
    };
    assert_eq!(candidates.first.source, source("pcf.dat", 3));
    assert_eq!(candidates.second.source, source("pcf.dat", 6));
    // An unrelated identity stays uniquely found, and its occurrence names both competing roots.
    let mode = parameter(&mib, "DEMO_MODE");
    let packets = mode.occurrences.value.as_ref().unwrap();
    assert_eq!(packets.len(), 2);
    assert!(packets[0].packet.value.is_none());
    assert_eq!(
        packets[1].packet.value.as_ref().unwrap().spid,
        PacketSpid(89001)
    );
    assert_eq!(packets[0].occurrences.len(), 1);
    assert_eq!(
        position_text(&packets[0].occurrences[0].location.position),
        "byte 19 bit 0",
        "the declared occurrence survives duplicate packet roots"
    );
    let ProblemKind::AmbiguousReference {
        reference,
        alternatives,
    } = &packets[0].packet.problems[0].kind
    else {
        panic!("{:?}", packets[0].packet.problems)
    };
    assert!(matches!(
        reference,
        Reference::Root(Identity::Packet(PacketSpid(89000)))
    ));
    assert_eq!(alternatives.first.definition.source, source("pid.dat", 1));
    assert_eq!(alternatives.second.definition.source, source("pid.dat", 3));
    let Lookup::Found(variable) = mib.packet(PacketSpid(89001)) else {
        panic!("duplicate roots are per identity")
    };
    assert_eq!(variable.packet.name.value.as_deref(), Some("DEMO_VAR"));
    assert_eq!(
        mode.parameter.definition.source,
        source("pcf.dat", 1),
        "an identity index never overwrites a duplicate root"
    );
}

#[test]
fn partial_snapshots_keep_usable_information_beside_missing_tables() {
    let dir = combined();
    for file in ["plf.dat", "prf.dat", "txp.dat", "pcdf.dat"] {
        std::fs::remove_file(dir.path().join(file)).unwrap();
    }
    let mib = loaded(&dir);
    // Packet identification survives without its layout rows.
    let housekeeping = packet(&mib, 89000);
    assert_eq!(housekeeping.identification.apid.value, Some(42));
    assert_eq!(
        housekeeping
            .identification
            .criteria
            .value
            .as_ref()
            .unwrap()
            .len(),
        1
    );
    assert!(housekeeping.layout.value.is_none());
    assert!(housekeeping.layout.problems.iter().any(|problem| matches!(
        &problem.kind,
        ProblemKind::MissingReference {
            reference: Reference::Supporting {
                table: Table::Plf,
                ..
            }
        }
    )));
    // The parameter loses only the fixed occurrence and names the missing table.
    let mode = parameter(&mib, "DEMO_MODE");
    assert_eq!(mode.parameter.encoding.encoded_bits.value, Some(8));
    let packets = mode.occurrences.value.as_ref().unwrap();
    assert_eq!(spids(packets), vec![89001]);
    assert!(mode.occurrences.problems.iter().any(|problem| matches!(
        &problem.kind,
        ProblemKind::MissingReference { reference: Reference::Supporting { table: Table::Plf, key } }
            if key == "DEMO_MODE"
    )));
    // A textual calibration keeps its header while the interval table is absent.
    let state = parameter(&mib, "DEMO_STATE");
    for alternative in state.parameter.calibrations.value.iter().flatten() {
        let calibration = alternative.calibration.value.as_ref().unwrap();
        let Some(CalibrationForm::Textual { intervals }) = &calibration.form.value else {
            panic!("expected the textual family")
        };
        assert!(intervals.value.is_none());
        assert!(intervals.problems.iter().any(|problem| matches!(
            &problem.kind,
            ProblemKind::MissingReference { reference: Reference::Supporting { table: Table::Txp, key } }
                if key == "DEMO_STATE_TXF"
        )));
    }
    // Command arguments, aliases and conversions stay usable beside the missing rule and header rows.
    let one = command(&mib, "DEMO_TC001");
    let arguments = declared_arguments(one.arguments.value.as_ref().unwrap());
    assert_eq!(arguments.len(), 3);
    assert!(arguments[0].rules.ranges.value.is_none());
    assert_eq!(arguments[0].rules.aliases.value.as_ref().unwrap().len(), 2);
    assert_eq!(
        arguments[0]
            .rules
            .calibrations
            .value
            .as_ref()
            .unwrap()
            .len(),
        1
    );
    let header = one.header.value.as_ref().unwrap();
    assert_eq!(header.definition.source, source("tcp.dat", 1));
    assert!(header.fields.value.is_none());
    assert!(header.fields.problems.iter().any(|problem| matches!(
        &problem.kind,
        ProblemKind::MissingReference {
            reference: Reference::Supporting {
                table: Table::Pcdf,
                ..
            }
        }
    )));
    // A snapshot with usable supporting rows and no roots still loads and answers every kind.
    let supporting = Fixture::new();
    for (file, text) in [
        ("caf.dat", CAF),
        ("cap.dat", CAP),
        ("pcpc.dat", PCPC),
        ("prv.dat", PRV),
        ("vpd.dat", VPD),
    ] {
        supporting.write(file, text);
    }
    let mib = loaded(&supporting);
    assert!(matches!(
        mib.parameter(&ParameterName("DEMO_TEMP".into())),
        Lookup::NotFound(NotFoundReason::DefinitionsUnavailable)
    ));
    assert!(matches!(
        mib.packet(PacketSpid(89000)),
        Lookup::NotFound(NotFoundReason::DefinitionsUnavailable)
    ));
    assert!(matches!(
        mib.command(&CommandName("DEMO_TC001".into())),
        Lookup::NotFound(NotFoundReason::DefinitionsUnavailable)
    ));
    assert!(mib.search("mode", SearchScope::All).is_empty());
    assert!(mib.pus(3, None).is_empty());
    // Unusable and absent directories keep their visible typed outcomes.
    let malformed = Fixture::new();
    malformed.write("pcf.dat", "not a parameter row\n");
    assert!(matches!(
        Mib::load(malformed.path()),
        Err(LoadError::NoUsableSupportedRows { directory }) if directory == malformed.path()
    ));
    assert!(matches!(
        Mib::load(&dir.path().join("gone")),
        Err(LoadError::InaccessibleDirectory { cause, .. })
            if cause.kind() == io::ErrorKind::NotFound
    ));
}

#[test]
fn snapshot_independence_and_owned_results_survive_source_loss() {
    let dir = combined();
    let mib = loaded(&dir);
    let before = query_digest(&mib);
    let Lookup::Found(owned) = mib.parameter(&ParameterName("DEMO_MODE".into())) else {
        panic!()
    };
    // Changing and then removing every source leaves the loaded snapshot unchanged.
    std::fs::write(
        dir.path().join("pcf.dat"),
        "OTHER\tOther\t\t\t3\t4\t\t\t\tN\tR\n",
    )
    .unwrap();
    std::fs::remove_file(dir.path().join("plf.dat")).unwrap();
    assert_eq!(query_digest(&mib), before);
    std::fs::remove_dir_all(dir.path()).unwrap();
    assert_eq!(query_digest(&mib), before);
    // Reloading the vanished directory is a visible, typed error.
    assert!(matches!(
        Mib::load(dir.path()),
        Err(LoadError::InaccessibleDirectory { .. })
    ));
    // Owned results outlive the snapshot and keep relative, one-based provenance.
    drop(mib);
    assert_eq!(
        owned.parameter.description.value.as_deref(),
        Some("Operational mode")
    );
    assert_eq!(
        spids(owned.occurrences.value.as_ref().unwrap()),
        vec![89000, 89001]
    );
    assert_eq!(owned.parameter.definition.source.file, Path::new("pcf.dat"));
    assert_eq!(owned.parameter.definition.source.line.get(), 1);
    assert!(
        !owned
            .parameter
            .definition
            .source
            .file
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    );
    // An independent load of the same synthetic content is byte-identical.
    assert_eq!(query_digest(&loaded(&combined())), before);
}

#[test]
fn cli_scenarios_render_every_combined_workflow() {
    let dir = combined();
    let for_expected = |args: &[&str], expected: &[&str]| {
        let output = run(&dir, args);
        assert!(output.status.success(), "{args:?}: {}", stderr(&output));
        assert!(output.stderr.is_empty(), "{args:?}");
        let text = stdout(&output);
        for expected in expected {
            assert!(
                text.contains(expected),
                "missing {expected} in {args:?}: {text}"
            );
        }
    };
    for_expected(
        &["parameter", "DEMO_MODE"],
        &[
            "Parameter DEMO_MODE",
            "Description: Operational mode",
            "Encoding: unsigned integer, 8 bits",
            "Source: pcf.dat:1",
            "89000  DEMO_HK   byte 19 bit 0     8 bits  once",
            "89001  DEMO_VAR  relative 16 bits  8 bits",
            "TXF DEMO_MODE_TXF at txf.dat:3",
            "0..0 -> OFFLINE",
            "1..1 -> ONLINE",
            "inconsistent definition",
        ],
    );
    for_expected(
        &["parameter", "DEMO_TEMP"],
        &[
            "89000  DEMO_HK   byte 20 bit 0    16 bits  1/2, stride 16 bits",
            "89000  DEMO_HK   byte 22 bit 0    16 bits  2/2, stride 16 bits",
            "MCF DEMO_TEMP_MCF at mcf.dat:1",
            "LGF DEMO_TEMP_LGF at lgf.dat:1",
            "CAF DEMO_TEMP_CAF at caf.dat:1",
            "10 -> \"1.5\" [cap.dat:1]",
            "255 -> \"2.5\" [cap.dat:2]",
            "runtime dependent",
        ],
    );
    for_expected(
        &["packet", "89000"],
        &[
            "Packet 89000  DEMO_HK",
            "Description: Demonstration housekeeping",
            "APID: 42",
            "PI1 expected: 7; location: byte 16 bit 0; width: 8 bits",
            "DEMO_MODE   byte 19 bit 0  8 bits   once",
            "DEMO_TEMP   byte 20 bit 0  16 bits  1/2, stride 16 bits",
            "DEMO_TEMP   byte 22 bit 0  16 bits  2/2, stride 16 bits",
            "DEMO_STATE  byte 24 bit 0  8 bits   once",
        ],
    );
    for_expected(
        &["packet", "89001"],
        &[
            "Packet 89001  DEMO_VAR",
            "DEMO_COUNT  byte 10 bit 0  8 bits  once",
            "Repeat group  vpd.dat:1    runtime Repeat group using value of DEMO_COUNT",
            "  DEMO_TEMP  relative 0 bits  16 bits",
            "  DEMO_MODE  relative 16 bits  8 bits",
            "End repeat",
        ],
    );
    for_expected(
        &["command", "DEMO_TC001"],
        &[
            "Command DEMO_TC001",
            "Description: Demonstration command one",
            "ARG1     First argument   application-declared bit 0",
            "ARG2     Second argument  application-declared bit 8",
            "ARG3     Third argument   application-declared bit 16",
            "telemetry DEMO_COUNT",
            "32 .. 127 [E] [prv.dat:1]",
            "0 .. 3 [E] [prv.dat:2]",
            "\"0\" -> LOW [pas.dat:1]",
            "\"2.5\" -> HIGH [pas.dat:2]",
            "CCA DEMO_CONV_1 at cca.dat:1",
            "16 -> \"1.5\" [ccs.dat:1]",
            "255 -> \"3.5\" [ccs.dat:2]",
            "ARG3 at application-declared bit 16",
            "no matching PRF definition",
            "TCP DEMO_HDR01  Demonstration command header",
            "Pkt Version Number",
            "F fixed",
            "APID                HP002      header bit 5   11 bits  A APID from CCF_APID",
            "42 [H] (declared default)",
            "Sequence Count      HP004      header bit 16  14 bits  P set automatically at invocation",
            "0 [D] (declared default)",
            "Ack Flags           HP001      header bit 30  4 bits   K acknowledgement flags from CCF_ACK",
            "Service Type        HP005      header bit 34  8 bits   T service type from CCF_TYPE",
            "Service Subtype     HP006      header bit 42  8 bits   S service subtype from CCF_STYPE",
        ],
    );
    for_expected(
        &["command", "DEMO_TC074"],
        &[
            "Command DEMO_TC074",
            "Description: Demonstration command two",
            "N1  Group count  application-declared bit 0",
            "Repeat group  cdf.dat:4  runtime CDF_GRPSIZE=6",
            "  APID  Application id  application-declared bit 8",
            "  Fixed area  \"Padding\"  application-declared bit 24  4 bits",
            "  Repeat group  cdf.dat:7  runtime CDF_GRPSIZE=3",
            "    Repeat group  cdf.dat:9  runtime CDF_GRPSIZE=1",
            "      SUBTYPE  Subtype  application-declared bit 52",
            "TYPE at application-declared bit 36",
            "0 .. 2 [R] [prv.dat:3]",
            "TCP DEMO_HDR02  Extended demonstration header",
        ],
    );
    for_expected(
        &["command", "DEMO_TC003"],
        &[
            "Command DEMO_TC003",
            "ARG5     Disagreeing argument  application-declared bit 8",
            "inconsistent definition; CDF_ELLEN disagrees with the CPC encoded width",
            "TCP DEMO_HDR03  Demonstration header without elements",
            "No header elements declared",
        ],
    );
    // Search always returns the plain candidate table, and its identities feed exact lookup.
    let output = run(&dir, &["search", "mode"]);
    assert!(output.status.success() && output.stderr.is_empty());
    let text = stdout(&output);
    assert!(text.starts_with("Kind") && text.contains("Source"));
    assert!(!text.contains('\t'));
    let rows: Vec<&str> = text.lines().skip(1).collect();
    assert!(rows.len() >= 2, "{text}");
    // A name match precedes a description match, whichever kind holds it.
    assert!(
        rows[0].starts_with("parameter  DEMO_MODE"),
        "a name match ranks first: {text}"
    );
    assert!(
        rows.iter().any(|row| row.starts_with("command")),
        "description matches of other kinds stay included: {text}"
    );
    assert!(text.contains("pcf.dat:1") && text.contains("ccf.dat:1"));
    let scoped = run(&dir, &["search", "mode", "--scope", "parameters"]);
    assert!(scoped.status.success());
    let scoped = stdout(&scoped);
    assert!(!scoped.contains("DEMO_TC001") && !scoped.contains("89000"));
    assert!(
        scoped
            .lines()
            .any(|row| row.starts_with("parameter  DEMO_MODE")),
        "{scoped}"
    );
    assert!(run(&dir, &["parameter", "DEMO_MODE"]).status.success());
    // Equal ranks keep the contract's kind order before the identity and source order.
    let output = run(&dir, &["search", "DEMO"]);
    assert!(output.status.success() && output.stderr.is_empty());
    let text = stdout(&output);
    let kinds: Vec<&str> = text
        .lines()
        .skip(1)
        .map(|row| row.split_whitespace().next().unwrap())
        .collect();
    assert_eq!(
        kinds,
        vec![
            "parameter",
            "parameter",
            "parameter",
            "parameter",
            "parameter",
            "command",
            "command",
            "command",
            "packet",
            "packet",
        ],
        "{text}"
    );
    let identities: Vec<&str> = text
        .lines()
        .skip(1)
        .map(|row| row.split_whitespace().nth(1).unwrap())
        .collect();
    assert_eq!(
        &identities[..5],
        &[
            "DEMO_ABSENT",
            "DEMO_COUNT",
            "DEMO_MODE",
            "DEMO_STATE",
            "DEMO_TEMP"
        ]
    );
    assert_eq!(
        &identities[5..8],
        &["DEMO_TC001", "DEMO_TC003", "DEMO_TC074"]
    );
    // Separate processes render the same declarations identically.
    assert_eq!(
        run(&dir, &["--details", "command", "DEMO_TC074"]).stdout,
        run(&dir, &["--details", "command", "DEMO_TC074"]).stdout
    );
    // A PUS coordinate lists the packet and command definitions that share it.
    let output = run(&dir, &["pus", "3,25"]);
    assert!(output.status.success() && output.stderr.is_empty());
    let text = stdout(&output);
    assert!(text.contains("TM(3,25)") && text.contains("89000"));
    assert!(text.contains("TC(3,25)") && text.contains("DEMO_TC001"));
    // Losing the whole directory between runs is a clear loading error, never a silent miss.
    std::fs::remove_dir_all(dir.path()).unwrap();
    let output = run(&dir, &["parameter", "DEMO_MODE"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        stderr(&output).contains("cannot read MIB directory"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn cli_details_reach_every_canonical_supporting_table_family() {
    let dir = combined();
    let mut files = BTreeSet::new();
    for (verb, identity) in [
        ("parameter", "DEMO_MODE"),
        ("parameter", "DEMO_TEMP"),
        ("parameter", "DEMO_STATE"),
        ("parameter", "DEMO_ABSENT"),
        ("packet", "89000"),
        ("packet", "89001"),
        ("command", "DEMO_TC001"),
        ("command", "DEMO_TC074"),
    ] {
        let output = run(&dir, &["--details", verb, identity]);
        assert!(
            output.status.success(),
            "{verb} {identity}: {}",
            stderr(&output)
        );
        assert!(output.stderr.is_empty());
        let text = stdout(&output);
        let sources = definition_sources(&text);
        assert!(
            !sources.is_empty(),
            "{verb} {identity} reached no definition: {text}"
        );
        for source in sources {
            files.insert(source.split(':').next().unwrap().to_string());
        }
    }
    let expected: BTreeSet<String> = TABLES.iter().map(|(file, _)| file.to_string()).collect();
    assert_eq!(files, expected);
}

#[test]
fn cli_debug_diagnostics_and_silent_misses_keep_their_outcomes() {
    let dir = combined();
    for file in ["plf.dat", "pcdf.dat"] {
        std::fs::remove_file(dir.path().join(file)).unwrap();
    }
    let debug = run(&dir, &["--debug", "command", "DEMO_TC074"]);
    assert!(debug.status.success());
    let text = stderr(&debug);
    for expected in [
        "skipped file",
        "plf.dat",
        "pcdf.dat",
        "No such file or directory",
        "dropped row",
        "cdf.dat",
        "line=13",
        "CDF_ELTYPE: invalid code \"X\"",
        "original=DEMO_ORPHAN",
        "cpc.dat",
        "CPC_PTC: invalid integer \"broken\"",
        "original=ORPHAN_ARG",
        "ignored extra columns",
        "tcp.dat",
        "line=3",
        "extra=4",
        "exact command lookup",
        "command=DEMO_TC074",
        "matches=1",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    // Diagnostics never change stdout, which stays identical without --debug.
    let quiet = run(&dir, &["command", "DEMO_TC074"]);
    assert!(quiet.status.success() && quiet.stderr.is_empty());
    assert_eq!(quiet.stdout, debug.stdout);
    assert!(stdout(&quiet).contains("End repeat"));
    // Exact misses are silent with status 1, and explain themselves only in debug mode.
    for (args, kind) in [
        (vec!["parameter", "demo_mode"], "parameter"),
        (vec!["packet", "89999"], "packet"),
        (vec!["command", "DEMO_TC999"], "command"),
    ] {
        let output = run(&dir, &args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(
            output.stdout.is_empty() && output.stderr.is_empty(),
            "{args:?}: {} {}",
            stdout(&output),
            stderr(&output)
        );
        let mut debug_args = vec!["--debug"];
        debug_args.extend(args.iter().copied());
        let output = run(&dir, &debug_args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty());
        let text = stderr(&output);
        for expected in [
            format!("exact {kind} lookup"),
            format!("{kind} not found"),
            "NoMatchingIdentity".to_string(),
        ] {
            assert!(text.contains(&expected), "missing {expected}: {text}");
        }
    }
    // A snapshot without roots of a kind keeps its other absence reason just as silent.
    let supporting = Fixture::new();
    for (file, text) in [
        ("caf.dat", CAF),
        ("cap.dat", CAP),
        ("vpd.dat", VPD),
        ("pcpc.dat", PCPC),
    ] {
        supporting.write(file, text);
    }
    for (args, kind) in [
        (vec!["parameter", "DEMO_MODE"], "parameter"),
        (vec!["packet", "89000"], "packet"),
        (vec!["command", "DEMO_TC001"], "command"),
    ] {
        let output = run(&supporting, &args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(
            output.stdout.is_empty() && output.stderr.is_empty(),
            "{args:?}: {} {}",
            stdout(&output),
            stderr(&output)
        );
        let mut debug_args = vec!["--debug"];
        debug_args.extend(args.iter().copied());
        let output = run(&supporting, &debug_args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty());
        let text = stderr(&output);
        for expected in [
            format!("{kind} not found"),
            "DefinitionsUnavailable".to_string(),
        ] {
            assert!(text.contains(&expected), "missing {expected}: {text}");
        }
    }
}
