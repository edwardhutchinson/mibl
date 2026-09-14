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
fn missing_layout_and_deferred_repetition_never_claim_a_complete_flat_layout() {
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
    assert!(matches!(
        result.arguments.problems[0].kind,
        ProblemKind::UnsupportedInterpretation { .. }
    ));
    let elements = result.arguments.value.unwrap();
    assert_eq!(elements.len(), 2);
    assert!(argument(&elements[1]).location.encoded_bits.value.is_none());
    assert!(matches!(
        argument(&elements[1]).encoding.encoded_bits.problems[0].kind,
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
