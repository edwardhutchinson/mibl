mod common;

use common::{Fixture, PARAMETER};
use mibl::{Mib, model::*};

const COMMAND: &str = "DEMO_TC\tDemonstration command\tLong description\t\t\tHEADER";

fn command(mib: &Mib) -> CommandDescription {
    match mib.command(&CommandName("DEMO_TC".into())) {
        Lookup::Found(command) => command,
        other => panic!("expected command, got {other:?}"),
    }
}

#[test]
fn exact_command_lookup_retains_definition_duplicates_and_both_absence_reasons() {
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    let mib = Mib::load(dir.path()).unwrap();
    assert!(matches!(
        mib.command(&CommandName("DEMO_TC".into())),
        Lookup::NotFound(NotFoundReason::DefinitionsUnavailable)
    ));
    dir.write("ccf.dat", COMMAND);
    let mib = Mib::load(dir.path()).unwrap();
    let result = command(&mib);
    assert_eq!(
        result.description.value.as_deref(),
        Some("Demonstration command")
    );
    assert_eq!(result.definition.source.line.get(), 1);
    assert_eq!(
        result.definition.fields[2].presence,
        Presence::Text("Long description".into())
    );
    assert!(matches!(
        mib.command(&CommandName("demo_tc".into())),
        Lookup::NotFound(NotFoundReason::NoMatchingIdentity)
    ));
    dir.write("ccf.dat", &format!("{COMMAND}\n{COMMAND}\n{COMMAND}"));
    let Lookup::Ambiguous(candidates) = Mib::load(dir.path())
        .unwrap()
        .command(&CommandName("DEMO_TC".into()))
    else {
        panic!("expected duplicates")
    };
    assert_eq!(candidates.first.source.line.get(), 1);
    assert_eq!(candidates.second.source.line.get(), 2);
    assert_eq!(candidates.rest[0].source.line.get(), 3);
    // Loaded snapshots and owned descriptions are unaffected by later writes.
    assert_eq!(command(&mib).definition, result.definition);
}

fn argument(element: &Layout<CommandElement>) -> &CommandArgument {
    match element {
        Layout::Element(CommandElement::Argument(a)) => a,
        other => panic!("expected argument: {other:?}"),
    }
}

#[test]
fn basic_layout_orders_arguments_and_fixed_areas_and_preserves_cpc_ambiguity() {
    let dir = Fixture::new();
    dir.write("ccf.dat", COMMAND);
    dir.write("cdf.dat", "DEMO_TC\tE\t\t8\t16\t0\tARG3\tR\t3\nDEMO_TC\tE\t\t8\t0\t0\tARG1\tD\nDEMO_TC\tF\t\t8\t8\t0\tARG2\tE\tON\nDEMO_TC\tA\tPadding\t4\t24\t0\t\t\tF");
    dir.write("cpc.dat", "ARG1\tFirst\t3\t4\t\t\tV\t\t\t\t\t\t17\t\t\tB\nARG2\tSecond\t3\t4\nARG3\tThird\t3\t4\t\t\t\t\t\t\t\t\t\t\t\tExtra description\tL");
    let result = command(&Mib::load(dir.path()).unwrap());
    let elements = result.arguments.value.unwrap();
    assert_eq!(elements.len(), 4);
    for (i, element) in elements[..3].iter().enumerate() {
        let a = argument(element);
        assert!(
            matches!(a.location.position.value, Some(Position::ApplicationDeclaredBit(n)) if n == i as u64 * 8)
        );
        assert_eq!(a.encoding.encoded_bits.value, Some(8));
        assert_eq!(a.location.encoded_bits.value, Some(8));
        assert!(a.encoding.endian.value.is_none());
        assert!(matches!(
            a.encoding.endian.problems[0].kind,
            ProblemKind::UnsupportedInterpretation { .. }
        ));
        let d = a.definition.value.as_ref().unwrap();
        assert_eq!(
            d.fields[0]
                .meanings
                .iter()
                .map(|m| m.schema_name.as_str())
                .collect::<Vec<_>>(),
            ["CPC_PNAME", "CPC_NAME"]
        );
        assert_eq!(
            d.fields[14]
                .meanings
                .iter()
                .map(|m| m.schema_name.as_str())
                .collect::<Vec<_>>(),
            ["CPC_OBTID", "CPC_OBTIP"]
        );
        assert_eq!(d.fields[15].meanings.len(), 2);
        assert!(
            d.fields[15]
                .meanings
                .iter()
                .all(|m| m.interpretation.value.is_none())
        );
    }
    let a = argument(&elements[0]);
    assert_eq!(a.units.value.as_deref(), Some("V"));
    assert!(
        matches!(&a.rules.default.value.as_ref().unwrap().source, ValueSource::Literal(Scalar::Text(s)) if s == "17")
    );
    assert!(
        matches!(&a.rules.element_value.value.as_ref().unwrap().source, ValueSource::Literal(Scalar::Text(s)) if s == "17")
    );
    let a = argument(&elements[1]);
    assert_eq!(
        a.rules
            .element_value
            .value
            .as_ref()
            .unwrap()
            .representation
            .value
            .as_deref(),
        Some("E")
    );
    assert!(matches!(
        &a.definition.value.as_ref().unwrap().fields[15].presence,
        Presence::Omitted
    ));
    let Layout::Element(CommandElement::Fixed(f)) = &elements[3] else {
        panic!("expected fixed area")
    };
    assert_eq!(f.location.encoded_bits.value, Some(4));
    assert!(
        matches!(&f.value.value.as_ref().unwrap().source, ValueSource::Literal(Scalar::Text(s)) if s == "F")
    );
}

#[test]
fn duplicate_positions_missing_arguments_and_width_conflicts_stay_local() {
    let dir = Fixture::new();
    dir.write("ccf.dat", COMMAND);
    dir.write(
        "cdf.dat",
        "DEMO_TC\tE\t\t16\t0\t0\tKNOWN\nDEMO_TC\tE\t\t8\t8\t0\tMISSING\nDEMO_TC\tE\t\t8\t8\t0\tDUP",
    );
    dir.write(
        "cpc.dat",
        "KNOWN\tKnown\t3\t4\nDUP\tFirst\t3\t4\nDUP\tSecond\t3\t12",
    );
    let result = command(&Mib::load(dir.path()).unwrap());
    let elements = result.arguments.value.unwrap();
    let a = argument(&elements[0]);
    assert_eq!(a.location.encoded_bits.value, Some(8));
    let ProblemKind::InconsistentDefinition {
        fields,
        values,
        available,
    } = &a.location.encoded_bits.problems[0].kind
    else {
        panic!("missing conflict")
    };
    assert!(fields.contains(&"CDF_ELLEN".into()) && fields.contains(&"CPC_PFC".into()));
    assert!(values.contains(&Scalar::Integer(16)) && values.contains(&Scalar::Unsigned(8)));
    assert_eq!(available.len(), 2);
    for (index, element) in elements[1..].iter().enumerate() {
        let a = argument(element);
        assert_eq!(a.element.source.line.get(), index + 2);
        assert!(matches!(
            a.location.position.problems[0].kind,
            ProblemKind::InconsistentDefinition { .. }
        ));
        assert!(a.definition.value.is_none());
        assert!(a.location.encoded_bits.value.is_none());
    }
    assert!(matches!(
        argument(&elements[1]).definition.problems[0].kind,
        ProblemKind::MissingReference { .. }
    ));
    let ProblemKind::AmbiguousReference { alternatives, .. } =
        &argument(&elements[2]).definition.problems[0].kind
    else {
        panic!("missing ambiguity")
    };
    assert_eq!(alternatives.first.definition.source.line.get(), 2);
    assert_eq!(alternatives.second.definition.source.line.get(), 3);
}

#[test]
fn telemetry_defaults_and_editable_inputs_keep_runtime_dependencies_explicit() {
    for copies in 0..=2 {
        let dir = Fixture::new();
        dir.write("ccf.dat", COMMAND);
        dir.write(
            "cdf.dat",
            "DEMO_TC\tE\t\t8\t0\t0\tARG\tT\t9\tTEMP\nDEMO_TC\tE\t\t8\t8\t0\tARG",
        );
        dir.write("cpc.dat", "ARG\tArgument\t3\t4");
        dir.write("pcf.dat", &vec![PARAMETER; copies].join("\n"));
        let result = command(&Mib::load(dir.path()).unwrap());
        let elements = result.arguments.value.unwrap();
        let a = argument(&elements[0]);
        let value = &a.rules.element_value;
        let ValueSource::Telemetry {
            parameter,
            declaration,
        } = &value.value.as_ref().unwrap().source
        else {
            panic!("expected telemetry")
        };
        assert_eq!(parameter.0, "TEMP");
        assert!(matches!(
            value.problems[0].kind,
            ProblemKind::RuntimeDependent { .. }
        ));
        let dependency = &declaration.dependencies[0];
        assert_eq!(
            dependency.reference,
            Reference::Root(Identity::Parameter(ParameterName("TEMP".into())))
        );
        match copies {
            0 => assert!(matches!(
                dependency.targets.problems[0].kind,
                ProblemKind::MissingReference { .. }
            )),
            1 => assert_eq!(
                dependency.targets.value.as_ref().unwrap()[0]
                    .definition
                    .source
                    .file
                    .to_str(),
                Some("pcf.dat")
            ),
            2 => assert!(matches!(
                dependency.targets.problems[0].kind,
                ProblemKind::AmbiguousReference { .. }
            )),
            _ => unreachable!(),
        }
        assert_eq!(a.element.fields[8].presence, Presence::Text("9".into()));
        assert!(matches!(
            argument(&elements[1])
                .rules
                .element_value
                .value
                .as_ref()
                .unwrap()
                .source,
            ValueSource::Runtime(_)
        ));
    }
}

#[test]
fn shared_loading_retains_suffix_uncertainty_and_drops_established_malformed_cells() {
    let dir = Fixture::new();
    dir.write("ccf.dat", &format!("bad\n{COMMAND}"));
    dir.write(
        "cdf.dat",
        "DEMO_TC\tE\t\t8\tno\t0\tARG\nDEMO_TC\tX\t\t8\t0\t0\tARG\nDEMO_TC\tE\t\t8\t0\t0\tARG",
    );
    dir.write("cpc.dat", "ARG\tBad numeric\tx\t4\nARG\tBad code\t3\t4\tX\nARG\tUsable\t3\t4\t\t\t\t\t\t\t\t\t\t\t\tNot an endian\tAlso not an endian\textra");
    let result = command(&Mib::load(dir.path()).unwrap());
    assert_eq!(result.definition.source.line.get(), 2);
    let elements = result.arguments.value.unwrap();
    assert_eq!(elements.len(), 1);
    let a = argument(&elements[0]);
    assert_eq!(a.element.source.line.get(), 3);
    let d = a.definition.value.as_ref().unwrap();
    assert_eq!(d.source.line.get(), 3);
    assert_eq!(d.fields.len(), 17);
    assert_eq!(
        d.fields[15].presence,
        Presence::Text("Not an endian".into())
    );
    assert_eq!(
        d.fields[16].presence,
        Presence::Text("Also not an endian".into())
    );
    assert_eq!(a.encoding.encoded_bits.value, Some(8));
    assert!(a.encoding.endian.value.is_none());
}

#[test]
fn missing_cdf_stays_unavailable_and_declared_groups_keep_their_members() {
    let dir = Fixture::new();
    dir.write("ccf.dat", COMMAND);
    let result = command(&Mib::load(dir.path()).unwrap());
    assert!(result.arguments.value.is_none());
    assert!(matches!(
        result.arguments.problems[0].kind,
        ProblemKind::MissingReference { .. }
    ));
    dir.write(
        "cdf.dat",
        "DEMO_TC\tE\t\t8\t0\t1\tCOUNT\tR\t2\nDEMO_TC\tE\t\t0\t8\t0\tDATA",
    );
    dir.write("cpc.dat", "COUNT\tCount\t3\t4\nDATA\tVariable bytes\t7\t0");
    let result = command(&Mib::load(dir.path()).unwrap());
    let layout = result.arguments.value.unwrap();
    let [_, group] = &layout[..] else {
        panic!("the repeater keeps its declared group: {layout:?}")
    };
    let Layout::Repeat { children, .. } = group else {
        panic!("expected a repeated group: {group:?}")
    };
    assert_eq!(children.len(), 1);
    let data = argument(&children[0]);
    assert!(data.location.encoded_bits.value.is_none());
    assert!(matches!(
        data.encoding.encoded_bits.problems[0].kind,
        ProblemKind::RuntimeDependent { .. }
    ));
}

#[test]
fn an_editable_argument_inheriting_an_absent_default_still_requires_runtime_input() {
    let dir = Fixture::new();
    dir.write("ccf.dat", COMMAND);
    dir.write("cdf.dat", "DEMO_TC\tE\t\t8\t0\t0\tARG\tD\tIGNORED");
    dir.write("cpc.dat", "ARG\tArgument\t3\t4");
    let result = command(&Mib::load(dir.path()).unwrap());
    let elements = result.arguments.value.unwrap();
    let a = argument(&elements[0]);
    assert!(a.rules.default.value.is_none());
    assert!(matches!(
        a.rules.element_value.value.as_ref().unwrap().source,
        ValueSource::Runtime(_)
    ));
    assert!(matches!(
        a.rules.element_value.problems[0].kind,
        ProblemKind::RuntimeDependent { .. }
    ));
    assert_eq!(
        a.element.fields[8].presence,
        Presence::Text("IGNORED".into())
    );
}

/// The synthetic equivalent of the surveyed S2KTC074: N1 repeats
/// {APID, N2, {Type, N3, {Subtype}}}, each counter a non-editable parameter with recorded value 1.
const NESTED: &str = "DEMO_TC\tF\tN1\t8\t0\t5\tN1\tR\t1\n\
DEMO_TC\tE\tAPID\t16\t8\t\tAPID\tR\n\
DEMO_TC\tF\tN2\t8\t24\t3\tN2\tR\t1\n\
DEMO_TC\tE\tType\t8\t32\t\tTYPE\tR\n\
DEMO_TC\tF\tN3\t8\t40\t1\tN3\tR\t1\n\
DEMO_TC\tE\tSubtype\t8\t48\t\tSUBTYPE\tR";

const NESTED_PARAMETERS: &str = "N1\tGroup count\t3\t4\nAPID\tApplication id\t3\t12\nN2\tType count\t3\t4\nTYPE\tType\t3\t4\nN3\tSubtype count\t3\t4\nSUBTYPE\tSubtype\t3\t4";

/// The declared elements of a layout in traversal order, whether or not a group encloses them.
fn declared_arguments(layout: &[Layout<CommandElement>]) -> Vec<&CommandArgument> {
    let mut arguments = Vec::new();
    for node in layout {
        match node {
            Layout::Element(CommandElement::Argument(a)) => arguments.push(a.as_ref()),
            Layout::Element(CommandElement::Fixed(_)) => {}
            Layout::Repeat { children, .. } | Layout::Conditional { children, .. } => {
                arguments.extend(declared_arguments(children))
            }
        }
    }
    arguments
}

fn repeat(
    node: &Layout<CommandElement>,
) -> (&Definition, &Info<Repetition>, &[Layout<CommandElement>]) {
    match node {
        Layout::Repeat {
            definition,
            repetition,
            children,
        } => (definition, repetition, children),
        other => panic!("expected a repeated group: {other:?}"),
    }
}

/// A runtime repetition names the element that supplies its count and stays unexpanded.
fn runtime_repetition(repetition: &Info<Repetition>) -> &RuntimeDeclaration {
    match &repetition.value {
        Some(Repetition::Runtime(declaration)) => declaration,
        other => panic!("expected a declared runtime repetition: {other:?}"),
    }
}

fn declared_bit(argument: &CommandArgument) -> u64 {
    match argument.location.position.value {
        Some(Position::ApplicationDeclaredBit(bit)) => bit,
        ref other => panic!("expected a declared application-data position: {other:?}"),
    }
}

#[test]
fn declared_groups_become_nested_element_trees_that_keep_declared_positions() {
    let dir = Fixture::new();
    dir.write("ccf.dat", COMMAND);
    dir.write("cdf.dat", NESTED);
    dir.write("cpc.dat", NESTED_PARAMETERS);
    let result = command(&Mib::load(dir.path()).unwrap());
    assert!(result.arguments.problems.is_empty());
    let layout = result.arguments.value.unwrap();
    // Every declared element stays in the tree at its declared unexpanded position.
    assert_eq!(
        declared_arguments(&layout)
            .iter()
            .map(|a| declared_bit(a))
            .collect::<Vec<_>>(),
        [0, 8, 24, 32, 40, 48]
    );
    assert_eq!(
        declared_arguments(&layout)
            .iter()
            .map(|a| a.element.source.line.get())
            .collect::<Vec<_>>(),
        [1, 2, 3, 4, 5, 6]
    );
    let [n1, outer] = &layout[..] else {
        panic!("the repeater stays beside its group: {layout:?}")
    };
    assert_eq!(argument(n1).element.source.line.get(), 1);
    let (definition, repetition, children) = repeat(outer);
    assert_eq!(definition.source.line.get(), 1);
    // The count is the repeater's value at command invocation, so it is never expanded.
    let declaration = runtime_repetition(repetition);
    assert!(
        declaration.expression.contains("CDF_GRPSIZE=5")
            && declaration
                .expression
                .contains("the recorded value 1 of N1")
            && declaration.expression.contains("assume one repetition"),
        "{}",
        declaration.expression
    );
    assert!(matches!(
        repetition.problems[0].kind,
        ProblemKind::RuntimeDependent { .. }
    ));
    assert_eq!(repetition.sources[0].file.to_str(), Some("cdf.dat"));
    let dependency = &declaration.dependencies[0];
    assert_eq!(
        dependency.reference,
        Reference::Supporting {
            table: Table::Cpc,
            key: "N1".into()
        }
    );
    assert_eq!(
        dependency.targets.value.as_ref().unwrap()[0]
            .definition
            .source
            .line
            .get(),
        1
    );
    assert_eq!(children.len(), 3);
    // A member carries the unexpanded-position rule of its group, not the group's own source.
    let apid = argument(&children[0]);
    assert_eq!(declared_bit(apid), 8);
    assert_eq!(apid.location.constraints.len(), 1);
    assert!(apid.location.constraints[0].dependencies.is_empty());
    assert!(
        apid.location.constraints[0]
            .expression
            .contains("unexpanded application-data position")
            && apid.location.constraints[0]
                .expression
                .contains("5 declared element(s)"),
        "{}",
        apid.location.constraints[0].expression
    );
    // Groups nest: N2 repeats {Type, N3, {Subtype}} inside N1's group.
    let (n2, inner) = (&children[1], &children[2]);
    assert_eq!(argument(n2).element.source.line.get(), 3);
    assert_eq!(argument(n2).location.constraints.len(), 1);
    let (definition, repetition, children) = repeat(inner);
    assert_eq!(definition.source.line.get(), 3);
    assert!(
        runtime_repetition(repetition)
            .expression
            .contains("CDF_GRPSIZE=3")
    );
    assert_eq!(children.len(), 3);
    let (n3, innermost) = (&children[1], &children[2]);
    assert_eq!(argument(n3).element.source.line.get(), 5);
    let (definition, repetition, children) = repeat(innermost);
    assert_eq!(definition.source.line.get(), 5);
    assert!(
        runtime_repetition(repetition)
            .expression
            .contains("CDF_GRPSIZE=1")
    );
    assert_eq!(children.len(), 1);
    let subtype = argument(&children[0]);
    assert_eq!(declared_bit(subtype), 48);
    // The deepest member carries the position rule of every enclosing group, outermost first.
    assert_eq!(
        subtype
            .location
            .constraints
            .iter()
            .map(|declaration| declaration.sources[0].line.get())
            .collect::<Vec<_>>(),
        [1, 3, 5]
    );
    assert_eq!(
        subtype
            .location
            .constraints
            .iter()
            .map(|declaration| declaration.dependencies.len())
            .collect::<Vec<_>>(),
        [0, 0, 0]
    );
}

#[test]
fn repetition_sources_follow_editable_and_telemetry_repeaters_and_keep_missing_targets_visible() {
    let dir = Fixture::new();
    dir.write("ccf.dat", COMMAND);
    // An editable counter has no recorded value, so the operator supplies the count.
    dir.write(
        "cdf.dat",
        "DEMO_TC\tE\t\t8\t0\t1\tCOUNT\tR\t\nDEMO_TC\tE\t\t8\t8\t0\tDATA\tR",
    );
    dir.write("cpc.dat", "COUNT\tCount\t3\t4\nDATA\tData\t3\t4");
    let result = command(&Mib::load(dir.path()).unwrap());
    let layout = result.arguments.value.unwrap();
    let (_, repetition, _) = repeat(&layout[1]);
    let declaration = runtime_repetition(repetition);
    assert!(
        declaration
            .expression
            .contains("the value an operator supplies for COUNT at command invocation"),
        "{}",
        declaration.expression
    );
    assert!(
        repetition
            .problems
            .iter()
            .all(|p| matches!(p.kind, ProblemKind::RuntimeDependent { .. }))
    );
    // A telemetry-sourced count depends on the monitoring parameter, whose missing definition
    // stays visible beside the usable declaration.
    dir.write(
        "cdf.dat",
        "DEMO_TC\tE\t\t8\t0\t1\tCOUNT\tT\t0\tTEMP\nDEMO_TC\tE\t\t8\t8\t0\tDATA\tR",
    );
    dir.write("cpc.dat", "COUNT\tCount\t3\t4\nDATA\tData\t3\t4");
    let result = command(&Mib::load(dir.path()).unwrap());
    let layout = result.arguments.value.unwrap();
    let (_, repetition, _) = repeat(&layout[1]);
    let declaration = runtime_repetition(repetition);
    assert!(
        declaration
            .expression
            .contains("the value of monitoring parameter TEMP"),
        "{}",
        declaration.expression
    );
    assert_eq!(
        declaration.dependencies[0].reference,
        Reference::Root(Identity::Parameter(ParameterName("TEMP".into())))
    );
    assert!(matches!(
        declaration.dependencies[0].targets.problems[0].kind,
        ProblemKind::MissingReference { .. }
    ));
    assert!(matches!(
        repetition.problems[0].kind,
        ProblemKind::MissingReference { .. }
    ));
    assert!(matches!(
        repetition.problems[1].kind,
        ProblemKind::RuntimeDependent { .. }
    ));
    dir.write("pcf.dat", PARAMETER);
    let result = command(&Mib::load(dir.path()).unwrap());
    let layout = result.arguments.value.unwrap();
    let (_, repetition, _) = repeat(&layout[1]);
    let declaration = runtime_repetition(repetition);
    assert_eq!(
        declaration.dependencies[0].targets.value.as_ref().unwrap()[0]
            .definition
            .source
            .file
            .to_str(),
        Some("pcf.dat")
    );
}

#[test]
fn fixed_areas_inside_groups_stay_distinguishable_and_cannot_repeat_themselves() {
    let dir = Fixture::new();
    dir.write("ccf.dat", COMMAND);
    // A fixed area has no value, so it cannot declare how often the following elements repeat.
    dir.write(
        "cdf.dat",
        "DEMO_TC\tA\tPadding\t4\t0\t2\t\t\tF\nDEMO_TC\tE\t\t8\t4\t0\tARG\tR\nDEMO_TC\tA\tTrailer\t4\t12\t0\t\t\t2",
    );
    dir.write("cpc.dat", "ARG\tArgument\t3\t4");
    let result = command(&Mib::load(dir.path()).unwrap());
    let layout = result.arguments.value.unwrap();
    let (_, repetition, children) = repeat(&layout[1]);
    assert!(repetition.value.is_none());
    let ProblemKind::InconsistentDefinition {
        fields,
        values,
        available,
    } = &repetition.problems[0].kind
    else {
        panic!(
            "expected a contradictory declaration: {:?}",
            repetition.problems
        )
    };
    assert_eq!(fields, &["CDF_GRPSIZE".to_string()]);
    assert_eq!(values, &[Scalar::Integer(2)]);
    assert_eq!(available.len(), 3);
    assert!(repetition.problems[0].explanation.contains("fixed area"));
    assert_eq!(
        repetition.problems[0].sources[0].line.get(),
        1,
        "the problem names the repeater's declaration"
    );
    // Every member stays usable inside the group, including fixed areas.
    assert_eq!(children.len(), 2);
    let argument_member = argument(&children[0]);
    assert_eq!(declared_bit(argument_member), 4);
    assert_eq!(argument_member.location.constraints.len(), 1);
    assert!(
        argument_member.location.constraints[0]
            .expression
            .contains("unexpanded application-data position")
    );
    let Layout::Element(CommandElement::Fixed(trailer)) = &children[1] else {
        panic!("expected a fixed area member: {:?}", children[1])
    };
    assert_eq!(trailer.location.encoded_bits.value, Some(4));
    assert!(matches!(
        &trailer.value.value.as_ref().unwrap().source,
        ValueSource::Literal(Scalar::Text(text)) if text == "2"
    ));
}

#[test]
fn incomplete_and_out_of_range_group_declarations_keep_every_declared_element() {
    let dir = Fixture::new();
    dir.write("ccf.dat", COMMAND);
    // GRPSIZE declares more following elements than the command retains.
    dir.write(
        "cdf.dat",
        "DEMO_TC\tE\t\t8\t0\t3\tCOUNT\tR\t2\nDEMO_TC\tE\t\t8\t8\t0\tDATA\tR",
    );
    dir.write("cpc.dat", "COUNT\tCount\t3\t4\nDATA\tData\t3\t4");
    let result = command(&Mib::load(dir.path()).unwrap());
    let layout = result.arguments.value.unwrap();
    let (_, repetition, children) = repeat(&layout[1]);
    let ProblemKind::InconsistentDefinition {
        values, available, ..
    } = &repetition.problems[0].kind
    else {
        panic!("expected an incomplete group: {:?}", repetition.problems)
    };
    assert_eq!(values, &[Scalar::Integer(3)]);
    assert_eq!(available.len(), 2);
    assert_eq!(children.len(), 1);
    assert_eq!(declared_bit(argument(&children[0])), 8);
    assert!(matches!(
        repetition.problems[1].kind,
        ProblemKind::RuntimeDependent { .. }
    ));
    // A group size that cannot declare a group stays an element with its own problem.
    dir.write(
        "cdf.dat",
        "DEMO_TC\tE\t\t8\t8\t-1\tCOUNT\tR\t2\nDEMO_TC\tE\t\t8\t0\t0\tDATA\tR",
    );
    let result = command(&Mib::load(dir.path()).unwrap());
    let layout = result.arguments.value.unwrap();
    assert_eq!(layout.len(), 2);
    let count = argument(&layout[1]);
    assert_eq!(declared_bit(count), 8);
    assert!(matches!(
        count.location.position.problems[0].kind,
        ProblemKind::InconsistentDefinition { .. }
    ));
    assert!(
        count.location.position.problems[0]
            .explanation
            .contains("fewer than one element")
    );
    // A size above the declared field range keeps the group it declares, clamped to the elements
    // that exist, with the range disagreement beside it.
    dir.write(
        "cdf.dat",
        "DEMO_TC\tE\t\t8\t0\t100\tCOUNT\tR\t2\nDEMO_TC\tE\t\t8\t8\t0\tDATA\tR",
    );
    let result = command(&Mib::load(dir.path()).unwrap());
    let layout = result.arguments.value.unwrap();
    assert_eq!(layout.len(), 2);
    let (definition, repetition, children) = repeat(&layout[1]);
    assert_eq!(definition.source.line.get(), 1);
    assert!(
        runtime_repetition(repetition)
            .expression
            .contains("CDF_GRPSIZE=100")
    );
    let ProblemKind::InconsistentDefinition {
        values, available, ..
    } = &repetition.problems[0].kind
    else {
        panic!("expected a range disagreement: {:?}", repetition.problems)
    };
    assert_eq!(values, &[Scalar::Integer(100)]);
    assert_eq!(available.len(), 2);
    assert!(repetition.problems[0].explanation.contains("1 to 99"));
    assert_eq!(children.len(), 1);
    assert_eq!(declared_bit(argument(&children[0])), 8);
}

#[test]
fn group_nesting_stops_at_the_supported_depth_and_keeps_every_declared_element() {
    let dir = Fixture::new();
    dir.write("ccf.dat", COMMAND);
    // Every element repeats all the following ones, so the declaration nests as deep as it can.
    let depth = 70;
    let rows: Vec<String> = (0..depth)
        .map(|index| {
            format!(
                "DEMO_TC\tE\t\t8\t{}\t{}\tARG{index}\tR",
                index * 8,
                depth - 1 - index
            )
        })
        .collect();
    dir.write("cdf.dat", &rows.join("\n"));
    let parameters: Vec<String> = (0..depth)
        .map(|index| format!("ARG{index}\tArgument {index}\t3\t4"))
        .collect();
    dir.write("cpc.dat", &parameters.join("\n"));
    let result = command(&Mib::load(dir.path()).unwrap());
    let layout = result.arguments.value.unwrap();
    assert_eq!(declared_arguments(&layout).len(), depth);
    let elements = declared_arguments(&layout);
    assert!(elements.iter().all(|a| a.location.position.value.is_some()));
    let problems: Vec<&Problem> = declared_arguments(&layout)
        .iter()
        .flat_map(|a| a.location.position.problems.iter())
        .collect();
    assert!(
        problems.iter().any(|p| p
            .explanation
            .contains("nests groups deeper than the supported depth")),
        "the cap reports itself: {problems:?}"
    );
    // Elements beyond the cap stay usable instead of disappearing from the tree.
    assert!(
        declared_arguments(&layout)
            .iter()
            .filter(|a| a.location.position.problems.is_empty())
            .count()
            > 0
    );
}

#[test]
fn duplicate_declared_positions_inside_a_group_keep_declared_order_with_evidence() {
    let dir = Fixture::new();
    dir.write("ccf.dat", COMMAND);
    dir.write(
        "cdf.dat",
        "DEMO_TC\tE\t\t8\t0\t2\tCOUNT\tR\t2\nDEMO_TC\tE\t\t8\t8\t0\tA\tR\nDEMO_TC\tE\t\t8\t8\t0\tB\tR",
    );
    dir.write(
        "cpc.dat",
        "COUNT\tCount\t3\t4\nA\tFirst\t3\t4\nB\tSecond\t3\t4",
    );
    let result = command(&Mib::load(dir.path()).unwrap());
    let layout = result.arguments.value.unwrap();
    let (_, _, children) = repeat(&layout[1]);
    assert_eq!(children.len(), 2);
    for (index, node) in children.iter().enumerate() {
        let a = argument(node);
        assert_eq!(a.element.source.line.get(), index + 2);
        assert_eq!(declared_bit(a), 8);
        let ProblemKind::InconsistentDefinition { available, .. } =
            &a.location.position.problems[0].kind
        else {
            panic!("expected duplicate positions: {a:?}")
        };
        assert_eq!(available.len(), 2);
        // The enclosing group's position rule survives beside the duplicate-position evidence.
        assert_eq!(a.location.constraints.len(), 1);
    }
}

#[test]
fn elements_following_a_declared_group_keep_the_unexpanded_position_rule() {
    let dir = Fixture::new();
    dir.write("ccf.dat", COMMAND);
    dir.write(
        "cdf.dat",
        "DEMO_TC\tF\t\t8\t0\t1\tCOUNT\tR\t2\nDEMO_TC\tE\t\t8\t8\t0\tMEMBER\tR\nDEMO_TC\tE\t\t8\t16\t0\tAFTER\tR",
    );
    dir.write(
        "cpc.dat",
        "COUNT\tCount\t3\t4\nMEMBER\tMember\t3\t4\nAFTER\tAfter the group\t3\t4",
    );
    let result = command(&Mib::load(dir.path()).unwrap());
    let layout = result.arguments.value.unwrap();
    let [_, group, after] = &layout[..] else {
        panic!("the declared group stays between its elements: {layout:?}")
    };
    let (_, _, children) = repeat(group);
    let member = argument(&children[0]);
    let after = argument(after);
    // The declared positions assume one repetition, and both elements keep the rule that says so.
    assert_eq!(declared_bit(member), 8);
    assert_eq!(declared_bit(after), 16);
    assert_eq!(member.location.constraints, after.location.constraints);
    assert!(
        after.location.constraints[0]
            .expression
            .contains("shifts with the 1 declared element(s) this group repeats"),
        "{}",
        after.location.constraints[0].expression
    );
    assert_eq!(after.location.constraints[0].sources[0].line.get(), 1);
}

#[test]
fn declared_layout_order_governs_group_membership_not_file_order() {
    let dir = Fixture::new();
    dir.write("ccf.dat", COMMAND);
    // The repeater is declared after its member in the file but before it in layout order.
    dir.write(
        "cdf.dat",
        "DEMO_TC\tE\t\t8\t16\t0\tAFTER\tR\nDEMO_TC\tF\t\t8\t0\t1\tCOUNT\tR\t2\nDEMO_TC\tE\t\t8\t8\t0\tMEMBER\tR",
    );
    dir.write(
        "cpc.dat",
        "COUNT\tCount\t3\t4\nAFTER\tAfter\t3\t4\nMEMBER\tMember\t3\t4",
    );
    let result = command(&Mib::load(dir.path()).unwrap());
    let layout = result.arguments.value.unwrap();
    let [count, group, after] = &layout[..] else {
        panic!("declared position order determines the group: {layout:?}")
    };
    assert_eq!(declared_bit(argument(count)), 0);
    let (definition, _, children) = repeat(group);
    assert_eq!(definition.source.line.get(), 2);
    assert_eq!(children.len(), 1);
    assert_eq!(declared_bit(argument(&children[0])), 8);
    assert_eq!(argument(&children[0]).element.source.line.get(), 3);
    assert_eq!(declared_bit(argument(after)), 16);
    assert_eq!(argument(after).element.source.line.get(), 1);
}
