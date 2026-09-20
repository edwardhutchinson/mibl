//! Telecommand acceptance scenarios over the combined snapshot: declared
//! arguments with their rules, conversions and value sources, nested runtime
//! groups, a fixed area and expanded headers.
//!
//! The layout text helpers below serve these scenarios only, so they stay here
//! rather than in the shared support module.

use crate::{
    fixture::combined,
    support::{command, declared_arguments, loaded, position_text, repetition_text, source},
};
use mibl::model::{
    ArgumentValue, CalibrationForm, CommandArgument, CommandElement, Info, Layout, Presence,
    ProblemKind, Reference, Repetition, Scalar, Table, ValueSource,
};

fn presence_text(presence: &Presence) -> String {
    match presence {
        Presence::Omitted => "omitted".into(),
        Presence::Empty => "empty".into(),
        Presence::Text(text) => text.clone(),
    }
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

fn literal(value: &Info<ArgumentValue>) -> Option<Scalar> {
    match &value.value.as_ref()?.source {
        ValueSource::Literal(scalar) => Some(scalar.clone()),
        _ => None,
    }
}

#[test]
fn command_arguments_keep_declared_positions_ranges_and_aliases() {
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
}

#[test]
fn command_arguments_keep_their_declared_conversions() {
    let dir = combined();
    let mib = loaded(&dir);
    let one = command(&mib, "DEMO_TC001");
    let arguments = declared_arguments(one.arguments.value.as_ref().unwrap());
    let first = arguments[0];
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
}

#[test]
fn command_arguments_keep_editable_and_telemetry_value_sources() {
    let dir = combined();
    let mib = loaded(&dir);
    let one = command(&mib, "DEMO_TC001");
    let arguments = declared_arguments(one.arguments.value.as_ref().unwrap());
    let first = arguments[0];
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
}

#[test]
fn command_argument_without_a_usable_range_stays_local_to_its_rule() {
    let dir = combined();
    let mib = loaded(&dir);
    let one = command(&mib, "DEMO_TC001");
    let arguments = declared_arguments(one.arguments.value.as_ref().unwrap());
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
}

#[test]
fn command_header_expands_every_declared_element_kind() {
    let dir = combined();
    let mib = loaded(&dir);
    let one = command(&mib, "DEMO_TC001");
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
}

#[test]
fn command_groups_keep_nested_runtime_repetitions_unexpanded() {
    let dir = combined();
    let mib = loaded(&dir);
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
}

#[test]
fn command_declared_width_disagreement_stays_local_to_the_affected_argument() {
    let dir = combined();
    let mib = loaded(&dir);
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
