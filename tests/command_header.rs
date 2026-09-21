mod common;

use common::Fixture;
use mibl::{Mib, model::*};

const COMMAND: &str = "DEMO_TC\tDemonstration command\t\t\t\tHDR";

/// The command's application-data element and its CPC row, present so header checks never
/// depend on the argument slice.
const ELEMENT: &str = "DEMO_TC\tE\t\t8\t0\t0\tARG\tR";

fn base(dir: &Fixture) {
    dir.write("ccf.dat", COMMAND);
    dir.write("cdf.dat", ELEMENT);
    dir.write("cpc.dat", "ARG\tArgument\t3\t4");
}

fn command(dir: &Fixture) -> CommandDescription {
    let Lookup::Found(command) = Mib::load(dir.path())
        .unwrap()
        .command(&CommandName("DEMO_TC".into()))
    else {
        panic!("command must remain available");
    };
    command
}

fn header(dir: &Fixture) -> Info<CommandHeader> {
    command(dir).header
}

fn fields(dir: &Fixture) -> Vec<HeaderField> {
    header(dir)
        .value
        .expect("expanded header")
        .fields
        .value
        .expect("declared header elements")
}

/// A declared reference whose target rows never resolved names its table and key.
fn assert_missing<T: std::fmt::Debug>(info: &Info<T>, table: Table, key: &str) {
    assert!(info.value.is_none(), "{info:?}");
    assert!(
        matches!(&info.problems[0].kind, ProblemKind::MissingReference { reference: Reference::Supporting { table: found, key: found_key } } if *found == table && found_key == key),
        "{info:?}"
    );
}

fn literal(value: &Info<ArgumentValue>) -> Option<&Scalar> {
    match &value.value.as_ref()?.source {
        ValueSource::Literal(scalar) => Some(scalar),
        other => panic!("expected a recorded header value: {other:?}"),
    }
}

#[test]
fn expanded_headers_resolve_ordered_fields_with_fixed_and_parameter_content() {
    let dir = Fixture::new();
    base(&dir);
    dir.write("tcp.dat", "OTHER\tOther header\nHDR\tDemonstration header");
    // Declared order is the bit offset, not the file order.
    dir.write(
        "pcdf.dat",
        &[
            "HDR\tTrailer\tF\t8\t16\t\t0A\tD",
            "HDR\tVersion Number\tF\t3\t0\t\t5\tH",
            "HDR\tLength\tP\t16\t32\tP009\t17\tO",
            "HDR\tCount\tP\t8\t8\tP007\t17\t",
            "HDR\tFlags\tK\t4\t24\tP001\t1F\t",
        ]
        .join("\n"),
    );
    dir.write(
        "pcpc.dat",
        "P001\tAck Flags\tU\nP007\tInteger sequence count\tI\nP009\tUnsigned count\tU",
    );
    let info = header(&dir);
    assert!(info.problems.is_empty(), "{info:?}");
    let header = info.value.expect("expanded header");
    // The TCP row named by CCF_PKTID carries the header definition and its provenance.
    assert_eq!(header.definition.source.file.to_str(), Some("tcp.dat"));
    assert_eq!(header.definition.source.line.get(), 2);
    assert_eq!(
        header.definition.fields[0].presence,
        Presence::Text("HDR".into())
    );
    assert!(header.fields.problems.is_empty(), "{:?}", header.fields);
    let fields = header.fields.value.as_deref().unwrap();
    assert_eq!(fields.len(), 5);
    let offsets: Vec<_> = fields
        .iter()
        .map(|field| field.location.position.value.clone().unwrap())
        .collect();
    assert!(matches!(
        offsets.as_slice(),
        [
            Position::HeaderBit(0),
            Position::HeaderBit(8),
            Position::HeaderBit(16),
            Position::HeaderBit(24),
            Position::HeaderBit(32)
        ]
    ));
    let widths: Vec<_> = fields
        .iter()
        .map(|field| field.location.encoded_bits.value)
        .collect();
    assert_eq!(widths, [Some(3), Some(8), Some(8), Some(4), Some(16)]);
    // Declared element types stay recorded, and every element keeps its own PCDF row.
    let kinds: Vec<_> = fields
        .iter()
        .map(|field| field.field_kind.value.as_deref())
        .collect();
    assert_eq!(
        kinds,
        [Some("F"), Some("P"), Some("F"), Some("K"), Some("P")]
    );
    let lines: Vec<_> = fields
        .iter()
        .map(|field| field.definition.source.line.get())
        .collect();
    assert_eq!(lines, [2, 4, 1, 5, 3]);
    assert!(
        fields
            .iter()
            .all(|field| field.definition.source.file.to_str() == Some("pcdf.dat"))
    );
    // A fixed area's content is the recorded value expressed in hex.
    assert_eq!(literal(&fields[0].value), Some(&Scalar::Unsigned(5)));
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
    assert_eq!(literal(&fields[2].value), Some(&Scalar::Unsigned(10)));
    // A signed integer parameter's default is decimal, whatever its PCDF_RADIX says.
    assert_eq!(literal(&fields[1].value), Some(&Scalar::Integer(17)));
    assert_eq!(
        fields[1]
            .value
            .value
            .as_ref()
            .unwrap()
            .representation
            .value
            .as_deref(),
        Some("D")
    );
    // An unsigned parameter uses its recorded radix, or the documented one when omitted.
    assert_eq!(literal(&fields[4].value), Some(&Scalar::Unsigned(15)));
    assert_eq!(
        fields[4]
            .value
            .value
            .as_ref()
            .unwrap()
            .representation
            .value
            .as_deref(),
        Some("O")
    );
    assert_eq!(literal(&fields[3].value), Some(&Scalar::Unsigned(31)));
    assert_eq!(
        fields[3]
            .value
            .value
            .as_ref()
            .unwrap()
            .representation
            .value
            .as_deref(),
        Some("H")
    );
    assert!(matches!(
        fields[3].definition.fields[7].meanings[0]
            .interpretation
            .value
            .as_ref()
            .unwrap()
            .origin,
        InterpretationOrigin::DocumentedDefault { .. }
    ));
    // Each parameter element links the PCPC definition that describes it.
    let parameter = fields[1]
        .parameter
        .value
        .as_ref()
        .expect("linked parameter");
    assert_eq!(
        parameter.reference,
        Reference::Supporting {
            table: Table::Pcpc,
            key: "P007".into()
        }
    );
    assert_eq!(parameter.definition.source.file.to_str(), Some("pcpc.dat"));
    assert_eq!(parameter.definition.source.line.get(), 2);
    assert!(fields[0].parameter.value.is_none() && fields[0].parameter.problems.is_empty());
    // Fixed areas keep the header's application-data slice, which is resolved separately.
    assert!(command(&dir).arguments.value.is_some());
}

#[test]
fn absent_header_tables_leave_the_command_found_with_reasoned_references() {
    let dir = Fixture::new();
    base(&dir);
    // No TCP, PCDF or PCPC rows at all.
    let info = header(&dir);
    assert_missing(&info, Table::Tcp, "HDR");
    // A resolved header whose field table is absent reports that table.
    dir.write("tcp.dat", "HDR\tDemonstration header");
    let info = header(&dir);
    assert!(info.problems.is_empty(), "{info:?}");
    let expanded = info.value.expect("expanded header");
    assert_missing(&expanded.fields, Table::Pcdf, "HDR");
    // A readable field table holding no record for this header declares no elements.
    dir.write("pcdf.dat", "OTHER\tElsewhere\tF\t1\t0\t\t0\tH");
    let expanded = header(&dir).value.expect("expanded header");
    assert_eq!(expanded.fields.value.as_ref().map(Vec::len), Some(0));
    // A parameter element whose PCPC rows are absent keeps everything else it declares.
    dir.write(
        "pcdf.dat",
        "HDR\tVersion Number\tF\t3\t0\t\t5\tH\nHDR\tAPID\tA\t11\t5\tP002\t0\tH",
    );
    let declared = fields(&dir);
    assert_eq!(declared.len(), 2);
    assert!(declared[0].parameter.value.is_none() && declared[0].parameter.problems.is_empty());
    assert_eq!(literal(&declared[0].value), Some(&Scalar::Unsigned(5)));
    assert_missing(&declared[1].parameter, Table::Pcpc, "P002");
    assert_eq!(declared[1].field_kind.value.as_deref(), Some("A"));
    assert!(declared[1].value.value.is_none());
    // PCPC rows that arrive later resolve the link and its declared format.
    dir.write("pcpc.dat", "P002\tAPID\tU");
    let linked = fields(&dir);
    assert_eq!(linked.len(), 2);
    let parameter = linked[1]
        .parameter
        .value
        .as_ref()
        .expect("linked parameter");
    assert_eq!(
        parameter.reference,
        Reference::Supporting {
            table: Table::Pcpc,
            key: "P002".into()
        }
    );
    assert_eq!(parameter.definition.source.file.to_str(), Some("pcpc.dat"));
    assert_eq!(literal(&linked[1].value), Some(&Scalar::Unsigned(0)));
}

#[test]
fn supporting_only_header_tables_load_and_report_unavailable_definitions() {
    let dir = Fixture::new();
    dir.write("tcp.dat", "HDR\tDemonstration header");
    dir.write("pcdf.dat", "HDR\tVersion Number\tF\t3\t0\t\t5\tH");
    dir.write("pcpc.dat", "P001\tAck Flags\tU");
    let mib = Mib::load(dir.path()).expect("supporting rows still load");
    assert!(matches!(
        mib.command(&CommandName("DEMO_TC".into())),
        Lookup::NotFound(NotFoundReason::DefinitionsUnavailable)
    ));
}

#[test]
fn duplicate_and_missing_linked_parameters_keep_every_declaration() {
    let dir = Fixture::new();
    base(&dir);
    dir.write("tcp.dat", "HDR\tDemonstration header");
    dir.write(
        "pcdf.dat",
        "HDR\tFirst\tA\t8\t0\tP002\t1\tH\nHDR\tSecond\tK\t8\t8\tP404\t1\tH",
    );
    dir.write(
        "pcpc.dat",
        "P002\tFirst meaning\tU\nP002\tSecond meaning\tU",
    );
    let fields = fields(&dir);
    let problem = &fields[0].parameter.problems[0];
    let ProblemKind::AmbiguousReference {
        reference,
        alternatives,
    } = &problem.kind
    else {
        panic!("duplicate parameters stay ambiguous: {problem:?}");
    };
    assert_eq!(
        reference,
        &Reference::Supporting {
            table: Table::Pcpc,
            key: "P002".into()
        }
    );
    assert_eq!(alternatives.first.definition.source.line.get(), 1);
    assert_eq!(alternatives.second.definition.source.line.get(), 2);
    assert!(fields[0].parameter.value.is_none());
    assert!(fields[0].value.value.is_none());
    assert_missing(&fields[1].parameter, Table::Pcpc, "P404");
    // Every element keeps its own declaration beside the unresolved link.
    assert_eq!(fields[0].field_kind.value.as_deref(), Some("A"));
    assert_eq!(fields[1].definition.source.line.get(), 2);
    assert!(matches!(
        fields[1].location.position.value,
        Some(Position::HeaderBit(8))
    ));
}

#[test]
fn duplicate_offsets_and_disagreeing_lengths_keep_every_declaration() {
    let dir = Fixture::new();
    base(&dir);
    dir.write("tcp.dat", "HDR\tDemonstration header");
    dir.write(
        "pcdf.dat",
        &[
            "HDR\tFirst at zero\tA\t8\t0\tP002\t1\tH",
            "HDR\tSecond at zero\tK\t16\t0\tP002\t1\tH",
            // ICD 7.0 requires one length for every record declaring the same element name,
            // within one packet header or across several.
            "OTHER\tElsewhere\tA\t16\t0\tP002\t1\tH",
        ]
        .join("\n"),
    );
    dir.write("pcpc.dat", "P002\tAPID\tU");
    let fields = fields(&dir);
    assert_eq!(fields.len(), 2);
    let ProblemKind::InconsistentDefinition {
        fields: names,
        available,
        ..
    } = &fields[0].location.position.problems[0].kind
    else {
        panic!("duplicate offsets keep every declaration: {fields:?}");
    };
    assert_eq!(names, &["PCDF_BIT"]);
    assert_eq!(available.len(), 2);
    assert_eq!(fields[1].definition.source.line.get(), 2);
    let ProblemKind::InconsistentDefinition {
        fields: names,
        values,
        available,
    } = &fields[0].location.encoded_bits.problems[0].kind
    else {
        panic!("declared lengths disagree: {fields:?}");
    };
    assert_eq!(names, &["PCDF_PNAME", "PCDF_LEN"]);
    assert_eq!(values, &[Scalar::Integer(8), Scalar::Integer(16)]);
    assert_eq!(available.len(), 3);
}

#[test]
fn contradictory_unsupported_and_malformed_header_declarations_keep_usable_fields() {
    let dir = Fixture::new();
    base(&dir);
    dir.write("tcp.dat", "HDR\tDemonstration header");
    dir.write(
        "pcdf.dat",
        &[
            "HDR\tNamed fixed\tF\t3\t0\tP001\t5\tH",
            "HDR\tUnnamed parameter\tA\t8\t3\t\t1\tH",
            "HDR\tUnreadable fixed\tF\t3\t8\t\tZZ\tH",
            "HDR\tFixed with unknown name\tF\t3\t11\tP404\t3\tH",
            "HDR\tUndeclared type\tC\t3\t14\t\t1\tH",
        ]
        .join("\n"),
    );
    dir.write("pcpc.dat", "P001\tAck Flags\tU");
    let fields = fields(&dir);
    // A malformed element type drops its own row; every other declaration stays.
    assert_eq!(fields.len(), 4);
    let ProblemKind::InconsistentDefinition {
        fields: names,
        values,
        available,
    } = &fields[0].parameter.problems[0].kind
    else {
        panic!("a fixed area keeps its contradictory name: {fields:?}");
    };
    assert_eq!(names, &["PCDF_TYPE", "PCDF_PNAME"]);
    assert_eq!(
        values,
        &[Scalar::Code("F".into()), Scalar::Text("P001".into())]
    );
    assert_eq!(available.len(), 2);
    assert_eq!(
        fields[0].parameter.value.as_ref().unwrap().reference,
        Reference::Supporting {
            table: Table::Pcpc,
            key: "P001".into()
        }
    );
    assert_eq!(literal(&fields[0].value), Some(&Scalar::Unsigned(5)));
    // A parameter element without a name keeps its kind and reports the incomplete declaration.
    let ProblemKind::InconsistentDefinition { fields: names, .. } =
        &fields[1].parameter.problems[0].kind
    else {
        panic!("an unnamed parameter element is reported: {fields:?}");
    };
    assert_eq!(names, &["PCDF_TYPE", "PCDF_PNAME"]);
    assert_eq!(fields[1].field_kind.value.as_deref(), Some("A"));
    assert!(fields[1].value.value.is_none());
    // An uninterpretable value keeps its recorded text beside the unsupported interpretation.
    assert!(fields[2].value.value.is_none());
    assert!(matches!(
        fields[2].value.problems[0].kind,
        ProblemKind::UnsupportedInterpretation { .. }
    ));
    assert_eq!(
        fields[2].definition.fields[6].presence,
        Presence::Text("ZZ".into())
    );
    // A fixed area whose declared name resolves nowhere keeps both declarations and its content.
    assert!(fields[3].parameter.value.is_none());
    assert!(
        fields[3]
            .parameter
            .problems
            .iter()
            .any(|problem| matches!(problem.kind, ProblemKind::MissingReference { .. })),
        "{:?}",
        fields[3].parameter
    );
    assert!(
        fields[3].parameter.problems.iter().any(|problem| matches!(
            &problem.kind,
            ProblemKind::InconsistentDefinition { fields, .. }
                if fields == &["PCDF_TYPE", "PCDF_PNAME"]
        )),
        "{:?}",
        fields[3].parameter
    );
    assert_eq!(literal(&fields[3].value), Some(&Scalar::Unsigned(3)));
}

#[test]
fn ambiguous_packet_headers_keep_every_declaration_and_the_command_found() {
    let dir = Fixture::new();
    base(&dir);
    dir.write(
        "tcp.dat",
        "OTHER\tOther header\nHDR\tFirst header\nHDR\tSecond header",
    );
    dir.write("pcdf.dat", "HDR\tVersion Number\tF\t3\t0\t\t5\tH");
    let info = header(&dir);
    let ProblemKind::AmbiguousReference {
        reference,
        alternatives,
    } = &info.problems[0].kind
    else {
        panic!("duplicate packet headers stay ambiguous: {info:?}");
    };
    assert_eq!(
        reference,
        &Reference::Supporting {
            table: Table::Tcp,
            key: "HDR".into()
        }
    );
    assert_eq!(alternatives.first.definition.source.line.get(), 2);
    assert_eq!(alternatives.second.definition.source.line.get(), 3);
    let header = info.value.expect("expanded header");
    // The first retained declaration in source order describes the expanded header.
    assert_eq!(header.definition.source.line.get(), 2);
    assert_eq!(fields(&dir).len(), 1);
}
