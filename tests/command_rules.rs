mod common;

use common::Fixture;
use mibl::{Mib, model::*};

const COMMAND: &str = "DEMO_TC\tDemonstration command\t\t\t\tHEADER";

const ELEMENT: &str = "DEMO_TC\tE\t\t8\t0\t0\tARG\tR";

fn command(dir: &Fixture) -> CommandDescription {
    let Lookup::Found(command) = Mib::load(dir.path())
        .unwrap()
        .command(&CommandName("DEMO_TC".into()))
    else {
        panic!("command must remain available");
    };
    command
}

fn elements_of(dir: &Fixture) -> Vec<CommandElement> {
    command(dir)
        .arguments
        .value
        .expect("declared argument layout")
        .into_iter()
        .map(|node| match node {
            Layout::Element(element) => element,
            other => panic!("expected a basic element: {other:?}"),
        })
        .collect()
}

/// CCF root, one application-data element and its CPC row, with the PRF, CCA and PAF references.
fn fixture(dir: &Fixture, prfref: &str, ccaref: &str, pafref: &str) {
    dir.write("ccf.dat", COMMAND);
    dir.write("cdf.dat", ELEMENT);
    dir.write(
        "cpc.dat",
        &format!("ARG\tArgument\t3\t4\tR\tH\t\tN\t{prfref}\t{ccaref}\t{pafref}"),
    );
}

fn argument(elements: &[CommandElement], index: usize) -> &CommandArgument {
    match &elements[index] {
        CommandElement::Argument(a) => a,
        other => panic!("expected an argument element: {other:?}"),
    }
}

/// A declared reference whose target rows never resolved names its table and key.
fn assert_missing<T: std::fmt::Debug>(info: &Info<T>, table: Table, key: &str) {
    assert!(info.value.is_none(), "{info:?}");
    assert!(
        matches!(&info.problems[0].kind, ProblemKind::MissingReference { reference: Reference::Supporting { table: found, key: found_key } } if *found == table && found_key == key),
        "{info:?}"
    );
}

fn range(value: &AllowedRange) -> (Option<Scalar>, Option<Scalar>) {
    (value.low.value.clone(), value.high.value.clone())
}

#[test]
fn range_sets_interpret_bounds_with_the_declared_format_radix_and_representation() {
    let dir = Fixture::new();
    fixture(&dir, "6", "", "");
    dir.write("prf.dat", "6\tRT Address\tE\tU\tH\t2\tmAmp");
    dir.write("prv.dat", "6\t20\t7F\n6\t0\t3");
    let elements = elements_of(&dir);
    let a = argument(&elements, 0);
    let ranges = a.rules.ranges.value.as_ref().unwrap();
    assert_eq!(ranges.len(), 2);
    assert_eq!(ranges[0].low.value, Some(Scalar::Unsigned(32)));
    assert_eq!(ranges[0].high.value, Some(Scalar::Unsigned(127)));
    assert_eq!(ranges[1].low.value, Some(Scalar::Unsigned(0)));
    assert_eq!(ranges[1].high.value, Some(Scalar::Unsigned(3)));
    assert_eq!(ranges[0].definition.source.line.get(), 1);
    assert_eq!(ranges[1].definition.source.line.get(), 2);
    assert!(
        ranges
            .iter()
            .all(|r| r.definition.source.file.to_str() == Some("prv.dat"))
    );
    // PRF_INTER names the representation of the declared bounds; the format and radix stay recorded.
    assert!(
        ranges
            .iter()
            .all(|r| r.representation.value.as_deref() == Some("E"))
    );
    assert!(a.rules.ranges.problems.is_empty());
    let [prf] = &a.rules.supporting_definitions[..] else {
        panic!("the range set definition is supporting evidence")
    };
    assert_eq!(prf.source.file.to_str(), Some("prf.dat"));
    assert_eq!(prf.fields[5].presence, Presence::Text("2".into()));
}

#[test]
fn range_bounds_follow_each_declared_input_format_and_documented_default() {
    for (prf, prv, low, high) in [
        // An omitted PRF_INTER, DSPFMT and RADIX take their documented defaults.
        (
            "1\tHK SID\t\tA\t\t1",
            "1\tHK_SID_1\tHK_SID_5",
            Some(Scalar::Text("HK_SID_1".into())),
            Some(Scalar::Text("HK_SID_5".into())),
        ),
        (
            "3\tCurrent\tR\t\t\t1\tmAmp",
            "3\t-6.1\t6.1",
            Some(Scalar::Decimal("-6.1".into())),
            Some(Scalar::Decimal("6.1".into())),
        ),
        (
            "4\tExtent\tE\tI\t\t1",
            "4\t-5\t5",
            Some(Scalar::Integer(-5)),
            Some(Scalar::Integer(5)),
        ),
        (
            "7\tSubaddr\tR\tU\tD\t1",
            "7\t0\t3",
            Some(Scalar::Unsigned(0)),
            Some(Scalar::Unsigned(3)),
        ),
        // Time-valued bounds have no numeric reduction, so their declared text is retained.
        (
            "9999\tTime Range\tR\tT\tD\t1",
            "9999\t1995.001.00.00.01\t2020.364.23.59.00",
            Some(Scalar::Text("1995.001.00.00.01".into())),
            Some(Scalar::Text("2020.364.23.59.00".into())),
        ),
    ] {
        let dir = Fixture::new();
        let key = prf.split('\t').next().unwrap();
        fixture(&dir, key, "", "");
        dir.write("prf.dat", prf);
        dir.write("prv.dat", prv);
        let elements = elements_of(&dir);
        let a = argument(&elements, 0);
        let ranges = a.rules.ranges.value.as_ref().unwrap();
        assert_eq!(range(&ranges[0]), (low, high), "for {prf}");
        assert!(a.rules.ranges.problems.is_empty(), "for {prf}");
        assert_eq!(
            a.rules.supporting_definitions[0].source.file.to_str(),
            Some("prf.dat")
        );
    }
}

#[test]
fn range_set_defaults_are_interpreted_as_documented_defaults() {
    let dir = Fixture::new();
    fixture(&dir, "6", "", "");
    dir.write("prf.dat", "6\tHK SID\t\t\t\t1");
    dir.write("prv.dat", "6\t1.5\t2.5");
    let elements = elements_of(&dir);
    let a = argument(&elements, 0);
    // The omitted PRF_INTER, PRF_DSPFMT and PRF_RADIX select raw real decimal input.
    let ranges = a.rules.ranges.value.as_ref().unwrap();
    assert_eq!(ranges[0].representation.value.as_deref(), Some("R"));
    assert_eq!(
        range(&ranges[0]),
        (
            Some(Scalar::Decimal("1.5".into())),
            Some(Scalar::Decimal("2.5".into()))
        )
    );
    let prf = &a.rules.supporting_definitions[0];
    for column in [2, 3, 4] {
        assert!(
            matches!(
                prf.fields[column].meanings[0]
                    .interpretation
                    .value
                    .as_ref()
                    .unwrap()
                    .origin,
                InterpretationOrigin::DocumentedDefault { .. }
            ),
            "column {}",
            column + 1
        );
        assert_eq!(prf.fields[column].presence, Presence::Empty);
    }
}

#[test]
fn alias_sets_interpret_every_mapping_beside_its_declared_text() {
    let dir = Fixture::new();
    fixture(&dir, "", "", "108");
    dir.write("paf.dat", "108\tREAL INPUT CALIB\tR\t2");
    dir.write("pas.dat", "108\tLOW\t0\n108\tHIGH\t2.5");
    let elements = elements_of(&dir);
    let a = argument(&elements, 0);
    let aliases = a.rules.aliases.value.as_ref().unwrap();
    assert_eq!(aliases.len(), 2);
    assert_eq!(aliases[0].raw.value, Some(Scalar::Decimal("0".into())));
    assert_eq!(aliases[0].text.value.as_deref(), Some("LOW"));
    assert_eq!(aliases[1].raw.value, Some(Scalar::Decimal("2.5".into())));
    assert_eq!(aliases[1].definition.source.line.get(), 2);
    assert!(a.rules.aliases.problems.is_empty());
    assert_eq!(
        a.rules.supporting_definitions[0].source.file.to_str(),
        Some("paf.dat")
    );
    // A signed raw format keeps negative alias values.
    dir.write("paf.dat", "108\tREAL INPUT CALIB\tI\t2");
    dir.write("pas.dat", "108\tBACKWARD\t-1\n108\tFORWARD\t1");
    let elements = elements_of(&dir);
    let aliases = argument(&elements, 0).rules.aliases.value.clone().unwrap();
    assert_eq!(aliases[0].raw.value, Some(Scalar::Integer(-1)));
    assert_eq!(aliases[0].text.value.as_deref(), Some("BACKWARD"));
    // PAF declares no radix column, so an omitted RAWFMT keeps decimal unsigned values.
    dir.write("paf.dat", "108\tREAL INPUT CALIB");
    dir.write("pas.dat", "108\tLOW\t0\n108\tHIGH\t3");
    let elements = elements_of(&dir);
    let a = argument(&elements, 0);
    let aliases = a.rules.aliases.value.as_ref().unwrap();
    assert_eq!(aliases[0].raw.value, Some(Scalar::Unsigned(0)));
    assert_eq!(aliases[1].raw.value, Some(Scalar::Unsigned(3)));
    assert!(matches!(
        a.rules.supporting_definitions[0].fields[2].meanings[0]
            .interpretation
            .value
            .as_ref()
            .unwrap()
            .origin,
        InterpretationOrigin::DocumentedDefault { .. }
    ));
}

#[test]
fn command_conversions_keep_declared_formats_and_independent_points() {
    let dir = Fixture::new();
    fixture(&dir, "", "1", "");
    dir.write("cca.dat", "1\tTC Num Curve 1\tR\tU\tH\tmAmp\t3");
    dir.write("ccs.dat", "1\t10\t1\n1\t0\t0\n1\tFF\t2.5");
    let elements = elements_of(&dir);
    let a = argument(&elements, 0);
    let alternatives = a.rules.calibrations.value.as_ref().unwrap();
    let [alternative] = &alternatives[..] else {
        panic!("one declared conversion")
    };
    assert!(alternative.condition.is_none() && alternative.selection.is_none());
    let calibration = alternative.calibration.value.as_ref().unwrap();
    assert_eq!(
        calibration.reference,
        Reference::Supporting {
            table: Table::Cca,
            key: "1".into()
        }
    );
    assert_eq!(calibration.definition.source.file.to_str(), Some("cca.dat"));
    let Some(CalibrationForm::CommandConversion {
        points,
        interpolation,
    }) = calibration.form.value.as_ref()
    else {
        panic!("expected a command conversion")
    };
    // CCA declares no interpolation, and CPC_INTER names a raw or engineering input, not extrapolation.
    assert!(interpolation.value.is_none() && interpolation.problems.is_empty());
    let points = points.value.as_ref().unwrap();
    assert_eq!(points.len(), 3);
    assert_eq!(points[0].raw.value, Some(Scalar::Unsigned(16)));
    assert_eq!(
        points[0].engineering.value,
        Some(Scalar::Decimal("1".into()))
    );
    assert_eq!(points[2].raw.value, Some(Scalar::Unsigned(255)));
    assert_eq!(
        points[2].engineering.value,
        Some(Scalar::Decimal("2.5".into()))
    );
    assert_eq!(points[1].definition.source.line.get(), 2);
    assert!(alternative.calibration.problems.is_empty());
}

#[test]
fn combined_argument_rules_stay_separate_and_undeclared_families_stay_empty() {
    let dir = Fixture::new();
    dir.write("ccf.dat", COMMAND);
    dir.write(
        "cdf.dat",
        "DEMO_TC\tE\t\t8\t0\t0\tARG\tR\nDEMO_TC\tE\t\t8\t8\t0\tPLAIN\tR",
    );
    dir.write(
        "cpc.dat",
        "ARG\tArgument\t3\t4\tR\tH\t\tN\t6\t1\t108\nPLAIN\tPlain\t3\t4",
    );
    dir.write("prf.dat", "6\tRT Address\tE\tU\tH\t1\tmAmp");
    dir.write("prv.dat", "6\t20\t7F");
    dir.write("cca.dat", "1\tTC Num Curve 1\tR\tU\tH\tmAmp\t1");
    dir.write("ccs.dat", "1\tFF\t2.5");
    dir.write("paf.dat", "108\tREAL INPUT CALIB\tR\t1");
    dir.write("pas.dat", "108\tLOW\t0");
    let elements = elements_of(&dir);
    let a = argument(&elements, 0);
    assert_eq!(a.rules.ranges.value.as_ref().unwrap().len(), 1);
    assert_eq!(a.rules.aliases.value.as_ref().unwrap().len(), 1);
    assert_eq!(a.rules.calibrations.value.as_ref().unwrap().len(), 1);
    assert_eq!(a.rules.supporting_definitions.len(), 2);
    assert!(a.rules.ranges.problems.is_empty());
    assert!(a.rules.aliases.problems.is_empty());
    assert!(a.rules.calibrations.problems.is_empty());
    let plain = argument(&elements, 1);
    // Undeclared families are known empty rather than unavailable or unsupported.
    assert!(plain.rules.ranges.value.as_ref().is_some_and(Vec::is_empty));
    assert!(
        plain
            .rules
            .aliases
            .value
            .as_ref()
            .is_some_and(Vec::is_empty)
    );
    assert!(
        plain
            .rules
            .calibrations
            .value
            .as_ref()
            .is_some_and(Vec::is_empty)
    );
    assert!(plain.rules.supporting_definitions.is_empty());
}

#[test]
fn missing_headers_missing_value_rows_and_ambiguous_references_stay_local() {
    let dir = Fixture::new();
    fixture(&dir, "6", "1", "108");
    // Declared references whose tables are absent name each table without removing the argument.
    let elements = elements_of(&dir);
    let a = argument(&elements, 0);
    assert_missing(&a.rules.ranges, Table::Prf, "6");
    assert_missing(&a.rules.calibrations, Table::Cca, "1");
    assert_missing(&a.rules.aliases, Table::Paf, "108");
    // A header without its value rows is a missing reference to the value table.
    dir.write("prf.dat", "6\tRT Address\tE\tU\tH\t1");
    dir.write("cca.dat", "1\tCurve\tR\tU\tH");
    dir.write("paf.dat", "108\tAliases\tR");
    let elements = elements_of(&dir);
    let a = argument(&elements, 0);
    assert_missing(&a.rules.ranges, Table::Prv, "6");
    assert_missing(&a.rules.aliases, Table::Pas, "108");
    let alternative = &a.rules.calibrations.value.as_ref().unwrap()[0];
    let Some(CalibrationForm::CommandConversion { points, .. }) = alternative
        .calibration
        .value
        .as_ref()
        .unwrap()
        .form
        .value
        .as_ref()
    else {
        panic!("expected a command conversion")
    };
    assert!(points.value.is_none());
    assert!(matches!(
        points.problems[0].kind,
        ProblemKind::MissingReference {
            reference: Reference::Supporting {
                table: Table::Ccs,
                ..
            }
        }
    ));
}

#[test]
fn duplicate_headers_keep_every_set_with_one_ambiguity_problem() {
    let dir = Fixture::new();
    fixture(&dir, "6", "", "");
    dir.write("prf.dat", "6\tFirst\tE\tU\tH\t1\n6\tSecond\tR\tU\tD\t1");
    dir.write("prv.dat", "6\t20\t7F");
    let elements = elements_of(&dir);
    let a = argument(&elements, 0);
    // Both sets stay available, each interpreted by its own declarations.
    let ranges = a.rules.ranges.value.as_ref().unwrap();
    assert_eq!(ranges.len(), 2);
    assert_eq!(ranges[0].representation.value.as_deref(), Some("E"));
    assert_eq!(ranges[1].representation.value.as_deref(), Some("R"));
    let [problem] = &a.rules.ranges.problems[..] else {
        panic!("one problem covers the ambiguous reference")
    };
    let ProblemKind::AmbiguousReference {
        reference: Reference::Supporting {
            table: Table::Prf, ..
        },
        alternatives,
    } = &problem.kind
    else {
        panic!("expected an ambiguous range set")
    };
    assert_eq!(alternatives.rest.len(), 0);
    assert_eq!(alternatives.first.definition.source.line.get(), 1);
    assert_eq!(problem.sources.len(), 3);
    // A third matching header keeps its own definition in the remainder.
    dir.write(
        "prf.dat",
        "6\tFirst\tE\tU\tH\t1\n6\tSecond\tR\tU\tD\t1\n6\tThird\tR\tU\tO\t1",
    );
    let elements = elements_of(&dir);
    let alternatives = match &argument(&elements, 0).rules.ranges.problems[0].kind {
        ProblemKind::AmbiguousReference { alternatives, .. } => alternatives.clone(),
        other => panic!("expected an ambiguous range set: {other:?}"),
    };
    assert_eq!(alternatives.rest.len(), 1);
    assert_eq!(alternatives.rest[0].definition.source.line.get(), 3);
}

#[test]
fn ambiguous_aliases_and_conversions_retain_every_definition() {
    let dir = Fixture::new();
    fixture(&dir, "", "1", "108");
    dir.write("paf.dat", "108\tFirst\tR\t1\n108\tSecond\tI\t1");
    dir.write("pas.dat", "108\tLOW\t0");
    dir.write("cca.dat", "1\tFirst\tR\tU\tH\t\t1\n1\tSecond\tI\tU\tD\t\t1");
    dir.write("ccs.dat", "1\t10\t1");
    let elements = elements_of(&dir);
    let a = argument(&elements, 0);
    // Every retained set keeps its own copy of the shared value rows, interpreted by its own format.
    let aliases = a.rules.aliases.value.as_ref().unwrap();
    assert_eq!(aliases.len(), 2);
    assert_eq!(aliases[0].raw.value, Some(Scalar::Decimal("0".into())));
    assert_eq!(aliases[1].raw.value, Some(Scalar::Integer(0)));
    assert!(matches!(
        &a.rules.aliases.problems[0].kind,
        ProblemKind::AmbiguousReference { alternatives, .. } if alternatives.rest.is_empty()
    ));
    let alternatives = a.rules.calibrations.value.as_ref().unwrap();
    assert_eq!(alternatives.len(), 2);
    for (index, alternative) in alternatives.iter().enumerate() {
        let problems = &alternative.calibration.problems;
        assert!(
            matches!(
                &problems[0].kind,
                ProblemKind::AmbiguousReference { alternatives, .. } if alternatives.rest.is_empty()
            ),
            "alternative {index}"
        );
    }
    // Each conversion interprets the shared points with its own format.
    let forms: Vec<_> = alternatives
        .iter()
        .map(|a| {
            let Some(CalibrationForm::CommandConversion { points, .. }) =
                a.calibration.value.as_ref().unwrap().form.value.as_ref()
            else {
                panic!("expected a command conversion")
            };
            points.value.as_ref().unwrap()[0].raw.value.clone()
        })
        .collect();
    assert_eq!(
        forms,
        [Some(Scalar::Unsigned(16)), Some(Scalar::Unsigned(10))]
    );
}

#[test]
fn malformed_neighboring_rows_drop_only_themselves_and_keep_usable_evidence() {
    let dir = Fixture::new();
    fixture(&dir, "6", "1", "108");
    dir.write(
        "prf.dat",
        "6\tGood\tE\tU\tH\t2\tmAmp\n6\tBad code\tX\tU\tH\t1\n6\tUnparsable count\tE\tU\tH\tmany",
    );
    dir.write("prv.dat", "\t20\t7F\n6\t20\t7F\n6\t0\t3");
    dir.write("cca.dat", "1\tCurve\tR\tU\tH\n1\tBad code\tR\tU\tX");
    dir.write("ccs.dat", "1\t10\t1\n1\t10\n1\tFF\t2.5");
    dir.write("paf.dat", "108\tAliases\tU\n108\tBad code\tU\tX");
    dir.write("pas.dat", "108\tLOW\t0\n108\t\t\n108\tHIGH\t2");
    let elements = elements_of(&dir);
    let a = argument(&elements, 0);
    let ranges = a.rules.ranges.value.as_ref().unwrap();
    assert_eq!(ranges.len(), 2);
    assert_eq!(
        range(&ranges[0]),
        (Some(Scalar::Unsigned(32)), Some(Scalar::Unsigned(127)))
    );
    assert!(ranges[0].definition.source.line.get() == 2);
    let aliases = a.rules.aliases.value.as_ref().unwrap();
    assert_eq!(aliases.len(), 2);
    assert_eq!(aliases[1].raw.value, Some(Scalar::Unsigned(2)));
    assert_eq!(aliases[1].definition.source.line.get(), 3);
    let alternatives = a.rules.calibrations.value.as_ref().unwrap();
    let Some(CalibrationForm::CommandConversion { points, .. }) = alternatives[0]
        .calibration
        .value
        .as_ref()
        .unwrap()
        .form
        .value
        .as_ref()
    else {
        panic!("expected a command conversion")
    };
    let points = points.value.as_ref().unwrap();
    assert_eq!(points.len(), 2);
    assert_eq!(points[1].definition.source.line.get(), 3);
}

#[test]
fn unavailable_bound_interpretations_keep_the_bound_and_a_structured_problem() {
    let dir = Fixture::new();
    fixture(&dir, "6", "", "");
    dir.write("prf.dat", "6\tRT Address\tE\tU\tH\t1\tmAmp");
    dir.write("prv.dat", "6\tnot-hex\t\n6\t7F\t20");
    let elements = elements_of(&dir);
    let a = argument(&elements, 0);
    let ranges = a.rules.ranges.value.as_ref().unwrap();
    // A declared upper bound stays unavailable, and an omitted one is not an empty range.
    assert!(ranges[0].low.value.is_none());
    assert!(ranges[0].high.value.is_none());
    assert!(matches!(
        ranges[0].low.problems[0].kind,
        ProblemKind::UnsupportedInterpretation { .. }
    ));
    assert_eq!(
        ranges[0].definition.fields[1].presence,
        Presence::Text("not-hex".into())
    );
    assert!(
        ranges[0].high.problems.is_empty() && ranges[1].high.problems.is_empty(),
        "an omitted optional bound is missing evidence, not a problem"
    );
    assert_eq!(ranges[1].low.value, Some(Scalar::Unsigned(127)));
}

#[test]
fn every_command_supporting_table_loads_alone_and_keeps_usable_rows() {
    let dir = Fixture::new();
    for (file, row) in [
        ("cca.dat", "1\tCurve\tR\tU\tH"),
        ("ccs.dat", "1\t10\t1"),
        ("paf.dat", "108\tAliases\tU"),
        ("pas.dat", "108\tLOW\t0"),
        ("prf.dat", "6\tRT Address\tE\tU\tH"),
        ("prv.dat", "6\t20\t7F"),
    ] {
        dir.write(file, row);
        let mib = Mib::load(dir.path()).unwrap();
        assert!(matches!(
            mib.command(&CommandName("DEMO_TC".into())),
            Lookup::NotFound(NotFoundReason::DefinitionsUnavailable)
        ));
        std::fs::remove_file(dir.path().join(file)).unwrap();
    }
    // A snapshot keeps the rows it read even when the files change afterwards.
    fixture(&dir, "6", "1", "108");
    dir.write("prf.dat", "6\tRT Address\tE\tU\tH\t1\tmAmp");
    dir.write("prv.dat", "6\t20\t7F");
    dir.write("cca.dat", "1\tCurve\tR\tU\tH");
    dir.write("ccs.dat", "1\t10\t1");
    dir.write("paf.dat", "108\tAliases\tU");
    dir.write("pas.dat", "108\tLOW\t0");
    let mib = Mib::load(dir.path()).unwrap();
    dir.write("prv.dat", "6\tFF\tFF");
    let Lookup::Found(command) = mib.command(&CommandName("DEMO_TC".into())) else {
        panic!("command must remain available")
    };
    let Some(Layout::Element(CommandElement::Argument(a))) =
        command.arguments.value.unwrap().into_iter().next()
    else {
        panic!("expected an argument")
    };
    assert_eq!(
        range(&a.rules.ranges.value.unwrap()[0]),
        (Some(Scalar::Unsigned(32)), Some(Scalar::Unsigned(127)))
    );
}

#[test]
fn declared_counts_that_disagree_with_retained_rows_stay_visible() {
    let dir = Fixture::new();
    fixture(&dir, "6", "1", "108");
    dir.write("prf.dat", "6\tRT Address\tE\tU\tH\t3\tmAmp");
    dir.write("prv.dat", "6\t20\t7F\n6\t0\t3");
    dir.write("cca.dat", "1\tCurve\tR\tU\tH\tmAmp\t2");
    dir.write("ccs.dat", "1\t10\t1");
    dir.write("paf.dat", "108\tAliases\tU\t4");
    dir.write("pas.dat", "108\tLOW\t0");
    let elements = elements_of(&dir);
    let a = argument(&elements, 0);
    // Every retained bound, mapping and point survives beside the disagreement.
    assert_eq!(a.rules.ranges.value.as_ref().unwrap().len(), 2);
    assert_eq!(a.rules.aliases.value.as_ref().unwrap().len(), 1);
    let alternatives = a.rules.calibrations.value.as_ref().unwrap();
    assert_eq!(alternatives.len(), 1);
    for (problems, declared, retained, count, rows) in [
        (&a.rules.ranges.problems, "PRF_NRANGE", "PRV_NUMBR", 3, 2),
        (&a.rules.aliases.problems, "PAF_NALIAS", "PAS_NUMBR", 4, 1),
    ] {
        let [problem] = &problems[..] else {
            panic!("one count disagreement expected: {problems:?}")
        };
        let ProblemKind::InconsistentDefinition {
            fields,
            values,
            available,
        } = &problem.kind
        else {
            panic!("expected a count disagreement: {problem:?}")
        };
        assert_eq!(fields[0], declared);
        assert_eq!(fields[1], retained);
        assert_eq!(values[0], Scalar::Integer(count));
        assert_eq!(values[1], Scalar::Unsigned(rows));
        assert_eq!(available.len(), rows as usize + 1);
        assert_eq!(problem.sources.len(), rows as usize + 1);
    }
    let Some(CalibrationForm::CommandConversion { points, .. }) = alternatives[0]
        .calibration
        .value
        .as_ref()
        .unwrap()
        .form
        .value
        .as_ref()
    else {
        panic!("expected a command conversion")
    };
    assert_eq!(points.value.as_ref().unwrap().len(), 1);
    let ProblemKind::InconsistentDefinition { fields, values, .. } = &points.problems[0].kind
    else {
        panic!("expected a count disagreement")
    };
    assert_eq!(fields[0], "CCA_NCURVE");
    assert_eq!(fields[1], "CCS_NUMBR");
    assert_eq!(values[0], Scalar::Integer(2));
    assert_eq!(values[1], Scalar::Unsigned(1));
    // A declared count that matches the retained rows adds nothing.
    dir.write("prf.dat", "6\tRT Address\tE\tU\tH\t2\tmAmp");
    dir.write("cca.dat", "1\tCurve\tR\tU\tH\tmAmp\t1");
    dir.write("paf.dat", "108\tAliases\tU\t1");
    let elements = elements_of(&dir);
    let a = argument(&elements, 0);
    assert!(a.rules.ranges.problems.is_empty());
    assert!(a.rules.aliases.problems.is_empty());
    assert!(
        a.rules.calibrations.value.as_ref().unwrap()[0]
            .calibration
            .problems
            .is_empty()
    );
}

#[test]
fn unavailable_argument_layout_is_unavailable_rather_than_empty() {
    let dir = Fixture::new();
    dir.write("ccf.dat", COMMAND);
    let text = text(&dir, &[]);
    // A missing CDF leaves the rules unknown, which is not a command that declares none.
    assert!(text.contains("\nApplication data\nunavailable\n"), "{text}");
    assert!(text.contains("\nArgument rules\nunavailable\n"), "{text}");
}

#[test]
fn cli_renders_every_rule_family_interpreted_values_and_conflicts() {
    let dir = Fixture::new();
    dir.write("ccf.dat", COMMAND);
    dir.write(
        "cdf.dat",
        "DEMO_TC\tE\t\t8\t0\t0\tARG\tR\nDEMO_TC\tE\t\t8\t8\t0\tMISSING\tR",
    );
    dir.write(
        "cpc.dat",
        "ARG\tArgument\t3\t4\tR\tH\t\tN\t6\t1\t108\nMISSING\tMissing\t3\t4\t\t\t\tN\t404",
    );
    dir.write("prf.dat", "6\tRT Address\tE\tU\tH\t1\tmAmp");
    dir.write("prv.dat", "6\t20\t7F");
    dir.write("cca.dat", "1\tTC Num Curve 1\tR\tU\tH\tmAmp\t1");
    dir.write("ccs.dat", "1\t10\t1");
    dir.write("paf.dat", "108\tREAL INPUT CALIB\tI\t2");
    dir.write("pas.dat", "108\tBACKWARD\t-1");
    for expected in [
        "Argument rules",
        "Ranges",
        "32 .. 127 [E] [prv.dat:1]",
        "Aliases",
        "-1 -> BACKWARD [pas.dat:1]",
        "Calibrations",
        "CCA 1 at cca.dat:1",
        "Numerical curve; extrapolation: unavailable",
        "16 -> \"1\" [ccs.dat:1]",
        "missing reference; no matching PRF definition",
        "MISSING at application-declared bit 8",
        "inconsistent definition; PAF_NALIAS disagrees with the number of retained PAS_NUMBR rows",
        "Use --details",
    ] {
        let text = text(&dir, &[]);
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    let details = text(&dir, &["--details"]);
    for expected in [
        "PRF_NUMBR",
        "PRF_RADIX",
        "PRV_MINVAL",
        "CCA_ENGFMT",
        "CCS_XVALS",
        "PAF_RAWFMT",
        "PAS_ALVAL",
        "Recorded: text \"20\"",
        "Interpreted: \"20\", recorded",
        "prf.dat:1",
        "prv.dat:1",
        "cca.dat:1",
        "ccs.dat:1",
        "paf.dat:1",
        "pas.dat:1",
    ] {
        assert!(details.contains(expected), "missing {expected}");
    }
}

#[test]
fn cli_says_none_declared_without_argument_rules() {
    let dir = Fixture::new();
    fixture(&dir, "", "", "");
    let text = text(&dir, &[]);
    assert!(text.contains("\nArgument rules\nnone declared\n"), "{text}");
}

fn text(dir: &Fixture, flags: &[&str]) -> String {
    let mut args = flags.to_vec();
    args.extend(["command", "DEMO_TC"]);
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_mibl"))
        .env("MIB_DIR", dir.path())
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap()
}
