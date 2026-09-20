mod common;

use common::Fixture;
use mibl::{Mib, model::*};

fn fixture(vpd: &str) -> Fixture {
    let dir = Fixture::new();
    dir.write(
        "pcf.dat",
        "COUNT\tCounter\t\t\t3\t4\t8\t\t\tN\tR\nVALUE\tValue\t\t\t3\t4\t16\t\t\tN\tR",
    );
    dir.write("pid.dat", "3\t25\t42\t0\t0\t100\tVariable\t\t7\t10");
    dir.write("vpd.dat", vpd);
    dir
}

fn packet(mib: &Mib) -> PacketDescription {
    let Lookup::Found(p) = mib.packet(PacketSpid(100)) else {
        panic!("expected packet")
    };
    p
}

fn element(l: &Layout<ParameterOccurrence>) -> &ParameterOccurrence {
    let Layout::Element(o) = l else {
        panic!("expected occurrence")
    };
    o
}

#[test]
fn ordered_variable_fields_use_encoding_padding_and_signed_offsets() {
    let dir = fixture("7\t2\tCOUNT\t\t\t\t\t\t99\t\t\t\t\t-4\n7\t1\tVALUE\t\t\t\t\t\t1");
    let mib = Mib::load(dir.path()).unwrap();
    let layout = packet(&mib).layout.value.unwrap();
    assert_eq!(layout.len(), 2);
    let first = element(&layout[0]);
    assert_eq!(first.reference.0, "VALUE");
    assert_eq!(first.location.encoded_bits.value, Some(8));
    assert!(matches!(
        first.location.position.value,
        Some(Position::PacketAbsolute { byte: 11, bit: 0 })
    ));
    assert!(matches!(
        element(&layout[1]).location.position.value,
        Some(Position::PacketAbsolute { byte: 11, bit: 4 })
    ));
    assert!(matches!(
        first.definition.fields[13].presence,
        Presence::Omitted
    ));
    assert_eq!(first.definition.source.line.get(), 2);
}

#[test]
fn nested_repetitions_stay_finite_and_keep_counter_dependencies() {
    let dir = fixture(
        "7\t1\tCOUNT\t3\t0\tN\tN\t\t0\n7\t2\tVALUE\t1\t2\tN\tN\t\t0\n7\t3\tVALUE\t0\t0\tN\tN\t\t1\n7\t4\tCOUNT\t0\t0\tN\tN\t\t1\n7\t5\tVALUE\t0\t0\tN\tN\t\t1",
    );
    let layout = packet(&Mib::load(dir.path()).unwrap())
        .layout
        .value
        .unwrap();
    assert_eq!(element(&layout[0]).reference.0, "COUNT");
    let Layout::Repeat {
        repetition,
        children,
        ..
    } = &layout[1]
    else {
        panic!("runtime group")
    };
    let Some(Repetition::Runtime(declaration)) = &repetition.value else {
        panic!()
    };
    assert!(
        matches!(&declaration.dependencies[0].reference, Reference::Root(Identity::Parameter(n)) if n.0 == "COUNT")
    );
    let Layout::Repeat {
        repetition,
        children: inner,
        ..
    } = &children[0]
    else {
        panic!("fixed group without dummy occurrence")
    };
    assert!(matches!(
        repetition.value,
        Some(Repetition::Fixed { count: 2, .. })
    ));
    assert_eq!(inner.len(), 1);
    assert_eq!(element(&inner[0]).enclosing.len(), 2);
    assert_eq!(element(&children[1]).enclosing.len(), 1);
    assert_eq!(layout.len(), 3);
    let after = element(&layout[2]);
    assert_eq!(after.location.encoded_bits.value, Some(8));
    assert!(
        after
            .location
            .position
            .problems
            .iter()
            .any(|p| matches!(p.kind, ProblemKind::RuntimeDependent { .. }))
    );
    assert!(!after.location.constraints.is_empty());
}

#[test]
fn containing_packets_include_variable_occurrences_without_plf_and_merge_fixed_packets() {
    let dir = fixture("7\t1\tCOUNT\t1\t2\tN\tN\t\t0\n7\t2\tVALUE\t0\t0\tN\tN\t\t1");
    for fixed in [false, true] {
        if fixed {
            dir.write(
                "pid.dat",
                "3\t25\t42\t0\t0\t100\tVariable\t\t7\t10\n3\t25\t42\t0\t0\t9\tFixed\t\t-1\t10",
            );
            dir.write("plf.dat", "VALUE\t9\t20\t0");
        }
        let mib = Mib::load(dir.path()).unwrap();
        let Lookup::Found(p) = mib.parameter(&ParameterName("VALUE".into())) else {
            panic!()
        };
        let packets = p.occurrences.value.unwrap();
        assert_eq!(packets.len(), if fixed { 2 } else { 1 });
        let variable = packets.last().unwrap();
        assert_eq!(variable.packet.value.as_ref().unwrap().spid.0, 100);
        assert_eq!(variable.occurrences.len(), 1);
        assert!(matches!(
            variable.occurrences[0].enclosing[0],
            Enclosure::Repetition(_)
        ));
        if fixed {
            assert_eq!(packets[0].packet.value.as_ref().unwrap().spid.0, 9);
        }
    }
    // A supporting-only VPD snapshot is usable, but has no packet root definitions.
    let only = Fixture::new();
    only.write("vpd.dat", "7\t1\tVALUE\t\t\t\t\t\t1");
    assert!(matches!(
        Mib::load(only.path()).unwrap().packet(PacketSpid(100)),
        Lookup::NotFound(NotFoundReason::DefinitionsUnavailable)
    ));
}

#[test]
fn duplicate_positions_and_broken_links_preserve_definitions_and_known_widths() {
    let dir = fixture(
        "7\t1\tCOUNT\t0\t0\tN\tN\t\t1\n7\t1\tVALUE\t0\t0\tN\tN\t\t1\n7\t2\tMISSING\t0\t0\tN\tN\t\t1\n7\t3\tCOUNT\t0\t0\tN\tN\t\t1",
    );
    let layout = packet(&Mib::load(dir.path()).unwrap())
        .layout
        .value
        .unwrap();
    assert_eq!(layout.len(), 4);
    for (index, node) in layout[..2].iter().enumerate() {
        let o = element(node);
        assert_eq!(o.definition.source.line.get(), index + 1);
        assert_eq!(o.location.encoded_bits.value, Some(8));
        assert!(
            o.location
                .position
                .problems
                .iter()
                .any(|p| matches!(p.kind, ProblemKind::InconsistentDefinition { .. }))
        );
    }
    assert!(matches!(
        element(&layout[2]).parameter.problems[0].kind,
        ProblemKind::MissingReference { .. }
    ));
    assert_eq!(element(&layout[3]).location.encoded_bits.value, Some(8));
    assert!(element(&layout[3]).location.position.value.is_none());
}

#[test]
fn choice_and_deduced_fields_retain_runtime_sources_without_expanding_targets() {
    let dir = fixture(
        "7\t1\tCOUNT\t0\t0\tY\tY\t\t1\n7\t2\tDEDUCED\t0\t0\tN\tN\t\t1\n7\t3\tVALUE\t0\t0\tN\tN\t\t1",
    );
    dir.write("pcf.dat", "COUNT\tCounter\t\t\t3\t4\t8\t\t\tN\tR\nDEDUCED\tDeduced value\t\t\t11\t0\t16\t\tCOUNT\tN\tR\nVALUE\tValue\t\t\t3\t4\t8\t\t\tN\tR");
    let layout = packet(&Mib::load(dir.path()).unwrap())
        .layout
        .value
        .unwrap();
    assert_eq!(layout.len(), 4);
    assert!(
        element(&layout[0])
            .location
            .constraints
            .iter()
            .any(|d| d.expression.contains("PCF_PID"))
    );
    let Layout::Conditional {
        condition,
        children,
        ..
    } = &layout[1]
    else {
        panic!()
    };
    assert!(condition.expression.contains("TPSD"));
    assert!(children.is_empty());
    let deduced = element(&layout[2]);
    assert!(deduced.location.encoded_bits.value.is_none());
    let runtime = deduced
        .location
        .encoded_bits
        .problems
        .iter()
        .find_map(|p| match &p.kind {
            ProblemKind::RuntimeDependent { declaration } => Some(declaration),
            _ => None,
        })
        .unwrap();
    assert!(runtime.dependencies.iter().any(
        |d| matches!(&d.reference, Reference::Root(Identity::Parameter(n)) if n.0 == "COUNT")
    ));
    assert_eq!(element(&layout[3]).location.encoded_bits.value, Some(8));
    assert!(element(&layout[3]).location.position.value.is_none());
}

#[test]
fn inconsistent_padding_and_ambiguous_parameters_keep_independent_information() {
    let dir = fixture("7\t1\tVALUE\t0\t0\tN\tN\t\t1\n7\t2\tCOUNT\t0\t0\tN\tN\t\t1");
    dir.write("pcf.dat", "VALUE\tValue\t\t\t3\t4\t4\t\t\tN\tR\nCOUNT\tCounter\t\t\t3\t4\t8\t\t\tN\tR\nCOUNT\tDuplicate\t\t\t3\t4\t8\t\t\tN\tR");
    let layout = packet(&Mib::load(dir.path()).unwrap())
        .layout
        .value
        .unwrap();
    let first = element(&layout[0]);
    assert_eq!(first.location.encoded_bits.value, Some(8));
    assert!(
        first
            .location
            .position
            .problems
            .iter()
            .any(|p| matches!(p.kind, ProblemKind::InconsistentDefinition { .. }))
    );
    assert!(first.location.position.value.is_none());
    let second = element(&layout[1]);
    assert!(matches!(
        second.parameter.problems[0].kind,
        ProblemKind::AmbiguousReference { .. }
    ));
    assert_eq!(second.location.encoded_bits.value, Some(8));
    assert!(matches!(
        second.location.position.value,
        Some(Position::PacketAbsolute { byte: 10, bit: 4 })
    ));
}

#[test]
fn vpd_loading_retains_presence_defaults_and_usable_rows_in_an_owned_snapshot() {
    let dir =
        fixture("7\t1\tCOUNT\tbroken\t0\tN\tN\t\t1\n7\t2\tVALUE\t\t\t\t\t\t1\t\t\t\t\t\textra");
    // Keep a second independent root to exercise a mixed supporting-table load.
    dir.write(
        "pcf.dat",
        &format!("{}\nVALUE\tValue\t\t\t3\t4\t8\t\t\tN\tR", common::PARAMETER),
    );
    let mib = Mib::load(dir.path()).unwrap();
    std::fs::remove_dir_all(dir.path()).unwrap();
    let layout = packet(&mib).layout.value.unwrap();
    assert_eq!(layout.len(), 1);
    let definition = &element(&layout[0]).definition;
    assert_eq!(definition.source.line.get(), 2);
    assert_eq!(definition.fields.len(), 14);
    assert!(matches!(definition.fields[3].presence, Presence::Empty));
    assert!(matches!(
        definition.fields[3].meanings[0]
            .interpretation
            .value
            .as_ref()
            .unwrap()
            .origin,
        InterpretationOrigin::DocumentedDefault { .. }
    ));
}

#[test]
fn broken_nested_groups_and_future_repeat_extensions_remain_inspectable() {
    for count in ["2", "-1"] {
        let dir = fixture(&format!(
            "7\t1\tCOUNT\t8\t{count}\tN\tN\t\t0\n7\t2\tVALUE\t0\t0\tN\tN\t\t1"
        ));
        let layout = packet(&Mib::load(dir.path()).unwrap())
            .layout
            .value
            .unwrap();
        let Layout::Repeat {
            repetition,
            children,
            ..
        } = &layout[0]
        else {
            panic!()
        };
        assert!(
            repetition
                .problems
                .iter()
                .any(|p| matches!(p.kind, ProblemKind::InconsistentDefinition { .. }))
        );
        assert_eq!(element(&children[0]).location.encoded_bits.value, Some(8));
        if count == "-1" {
            assert!(
                repetition
                    .problems
                    .iter()
                    .any(|p| matches!(p.kind, ProblemKind::UnsupportedInterpretation { .. }))
            );
        }
    }
}

#[test]
fn orphan_variable_structure_retains_occurrences_and_missing_packet_reference() {
    let dir = fixture("7\t1\tVALUE\t0\t0\tN\tN\t\t1");
    dir.write("pid.dat", "");
    let Lookup::Found(parameter) = Mib::load(dir.path())
        .unwrap()
        .parameter(&ParameterName("VALUE".into()))
    else {
        panic!()
    };
    let packets = parameter.occurrences.value.unwrap();
    assert_eq!(packets.len(), 1);
    assert!(matches!(
        packets[0].packet.problems[0].kind,
        ProblemKind::MissingReference { .. }
    ));
    assert_eq!(
        packets[0].occurrences[0].location.encoded_bits.value,
        Some(8)
    );
    assert!(packets[0].occurrences[0].location.position.value.is_none());
}

#[test]
fn variable_length_field_keeps_known_start_when_no_padding_is_declared() {
    let dir = fixture("7\t1\tVALUE\t0\t0\tN\tN\t\t1\n7\t2\tCOUNT\t0\t0\tN\tN\t\t1");
    dir.write(
        "pcf.dat",
        "VALUE\tVariable string\t\t\t7\t0\t\t\t\tN\tR\nCOUNT\tCounter\t\t\t3\t4\t8\t\t\tN\tR",
    );
    let layout = packet(&Mib::load(dir.path()).unwrap())
        .layout
        .value
        .unwrap();
    assert!(matches!(
        element(&layout[0]).location.position.value,
        Some(Position::PacketAbsolute { byte: 10, bit: 0 })
    ));
    assert!(element(&layout[0]).location.encoded_bits.value.is_none());
    assert!(element(&layout[1]).location.position.value.is_none());
}

#[test]
fn duplicate_packet_roots_keep_separate_locations_and_candidate_context() {
    let dir = fixture("7\t1\tVALUE\t0\t0\tN\tN\t\t1");
    dir.write(
        "pid.dat",
        "3\t25\t42\t0\t0\t100\tFirst\t\t7\t10\n3\t25\t42\t0\t0\t100\tSecond\t\t7\t20",
    );
    let Lookup::Found(parameter) = Mib::load(dir.path())
        .unwrap()
        .parameter(&ParameterName("VALUE".into()))
    else {
        panic!()
    };
    let packets = parameter.occurrences.value.unwrap();
    assert_eq!(packets.len(), 2);
    for (i, byte) in [11, 21].into_iter().enumerate() {
        assert_eq!(
            packets[i]
                .packet
                .value
                .as_ref()
                .unwrap()
                .definition
                .source
                .line
                .get(),
            i + 1
        );
        assert!(matches!(
            packets[i].packet.problems[0].kind,
            ProblemKind::AmbiguousReference { .. }
        ));
        assert!(
            matches!(packets[i].occurrences[0].location.position.value, Some(Position::PacketAbsolute { byte: b, bit: 0 }) if b == byte)
        );
    }
}

#[test]
fn fixed_outer_group_preserves_nested_runtime_evidence_after_the_group() {
    let dir = fixture(
        "7\t1\tVALUE\t2\t2\tN\tN\t\t0\n7\t2\tCOUNT\t1\t0\tN\tN\t\t0\n7\t3\tVALUE\t0\t0\tN\tN\t\t1\n7\t4\tVALUE\t0\t0\tN\tN\t\t1",
    );
    let mib = Mib::load(dir.path()).unwrap();
    let layout = packet(&mib).layout.value.unwrap();
    let after = element(&layout[1]);
    assert!(after.location.constraints.iter().any(|d| d.expression.contains("using value of COUNT") && d.dependencies.iter().any(|dep| matches!(&dep.reference, Reference::Root(Identity::Parameter(n)) if n.0 == "COUNT"))));
    assert!(
        after
            .location
            .position
            .sources
            .iter()
            .any(|s| s.file == std::path::Path::new("pcf.dat") && s.line.get() == 1)
    );
    let Lookup::Found(parameter) = mib.parameter(&ParameterName("VALUE".into())) else {
        panic!()
    };
    let packets = parameter.occurrences.value.unwrap();
    let inner = &packets[0].occurrences[0];
    assert_eq!(inner.enclosing_definitions.len(), 2);
    assert_eq!(inner.enclosing_definitions[0].source.line.get(), 1);
    assert_eq!(inner.enclosing_definitions[1].source.line.get(), 2);
    assert!(
        matches!(&inner.enclosing_definitions[0].fields[4].presence, Presence::Text(s) if s == "2")
    );
}
