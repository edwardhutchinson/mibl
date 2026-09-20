//! Parameter acceptance scenarios over the combined snapshot: recorded
//! definition fields, calibrations of every family, fixed and variable
//! occurrences, and the evidence a broken reference leaves behind.
//!
//! Each test loads the combined snapshot itself, so no test depends on another
//! test's assertions or on the order they run in.

use crate::{
    fixture::combined,
    support::{loaded, parameter, position_text, repetition_text, source, spids},
};
use mibl::model::{
    CalibrationForm, Enclosure, Identity, InterpretationOrigin, Presence, Problem, ProblemKind,
    Reference, Repetition, Scalar, Table,
};

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

#[test]
fn parameter_definition_keeps_recorded_fields_and_documented_defaults() {
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
}

#[test]
fn parameter_calibration_keeps_intervals_beside_a_disagreeing_category() {
    let dir = combined();
    let mib = loaded(&dir);
    let mode = parameter(&mib, "DEMO_MODE");
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
}

#[test]
fn parameter_occurrences_separate_declared_slots_from_runtime_groups() {
    let dir = combined();
    let mib = loaded(&dir);
    let mode = parameter(&mib, "DEMO_MODE");
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
}

#[test]
fn parameter_occurrences_expand_bounded_repetition_with_their_declared_stride() {
    let dir = combined();
    let mib = loaded(&dir);
    // Bounded fixed repetition expands with its declared positions and stride.
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
}

#[test]
fn parameter_calibration_keeps_direct_and_conditional_alternatives_in_order() {
    let dir = combined();
    let mib = loaded(&dir);
    let temperature = parameter(&mib, "DEMO_TEMP");
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
}

#[test]
fn parameter_calibration_keeps_every_duplicate_linked_definition() {
    let dir = combined();
    let mib = loaded(&dir);
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
}

#[test]
fn parameter_calibration_names_the_declared_family_when_a_key_resolves_nowhere() {
    let dir = combined();
    let mib = loaded(&dir);
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
