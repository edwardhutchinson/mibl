mod common;
use common::Fixture;
use mibl::{Mib, model::*};

const PID: &str = "3\t25\t42\t7\t0\t89000\tSynthetic housekeeping\t\t-1\t10";
fn fixture() -> Fixture {
    let dir = Fixture::new();
    dir.write("pcf.dat", "ZUT00002\tMode\t\t\t3\t4\t99\t\t\tN\tR");
    dir.write("pid.dat", PID);
    dir.write("tpcf.dat", "89000\tZUY_HK_00001\t32");
    dir.write("pic.dat", "3\t25\t16\t8\t-1\t0\t42");
    dir.write("plf.dat", "ZUT00002\t89000\t19\t0");
    dir
}
#[test]
fn inspect_fixed_packet_and_follow_parameter_back_to_its_packet() {
    let dir = fixture();
    let mib = Mib::load(dir.path()).unwrap();
    let Lookup::Found(packet) = mib.packet(PacketSpid(89000)) else {
        panic!("expected packet")
    };
    assert_eq!(packet.packet.name.value.as_deref(), Some("ZUY_HK_00001"));
    assert_eq!(packet.identification.apid.value, Some(42));
    let criteria = packet.identification.criteria.value.unwrap();
    assert_eq!(criteria[0].expected.value, Some(7));
    assert_eq!(criteria[0].extraction.encoded_bits.value, Some(8));
    let layout = packet.layout.value.unwrap();
    let Layout::Element(occurrence) = &layout[0] else {
        panic!("expected occurrence")
    };
    assert!(matches!(
        occurrence.location.position.value,
        Some(Position::PacketAbsolute { byte: 19, bit: 0 })
    ));
    assert_eq!(occurrence.location.encoded_bits.value, Some(8));
    assert_eq!(
        occurrence
            .parameter
            .value
            .as_ref()
            .unwrap()
            .description
            .value
            .as_deref(),
        Some("Mode")
    );
    let Lookup::Found(parameter) = mib.parameter(&ParameterName("ZUT00002".into())) else {
        panic!("expected parameter")
    };
    let packets = parameter.occurrences.value.unwrap();
    assert_eq!(
        packets[0].packet.value.as_ref().unwrap().spid,
        PacketSpid(89000)
    );
    assert_eq!(packets[0].occurrences.len(), 1);
}

#[test]
fn repetitions_expand_with_bit_strides_and_keep_duplicate_positions() {
    let dir = fixture();
    dir.write(
        "plf.dat",
        "ZUT00002\t89000\t20\t4\nZUT00002\t89000\t19\t0\t3\t12",
    );
    let Lookup::Found(packet) = Mib::load(dir.path()).unwrap().packet(PacketSpid(89000)) else {
        panic!()
    };
    let layout = packet.layout.value.unwrap();
    let occurrences: Vec<_> = layout
        .iter()
        .map(|l| match l {
            Layout::Element(o) => o,
            _ => panic!(),
        })
        .collect();
    assert_eq!(occurrences.len(), 4);
    let positions: Vec<_> = occurrences
        .iter()
        .map(|o| match o.location.position.value {
            Some(Position::PacketAbsolute { byte, bit }) => (byte, bit),
            _ => panic!(),
        })
        .collect();
    assert_eq!(positions, [(19, 0), (20, 4), (20, 4), (22, 0)]);
    assert_eq!(occurrences[1].definition.source.line.get(), 1);
    for o in &occurrences[1..3] {
        assert!(
            o.location
                .position
                .problems
                .iter()
                .any(|p| matches!(p.kind, ProblemKind::InconsistentDefinition { .. }))
        );
    }
    assert!(matches!(
        occurrences[0].enclosing[0],
        Enclosure::Repetition(Info {
            value: Some(Repetition::Fixed { count: 3, .. }),
            ..
        })
    ));
}

#[test]
fn ambiguous_identification_preserves_expected_values_and_all_definitions() {
    let dir = fixture();
    dir.write("pic.dat", "3\t25\t16\t8\t-1\t0\t42\n3\t25\t16\t16\t-1\t0");
    let Lookup::Found(packet) = Mib::load(dir.path()).unwrap().packet(PacketSpid(89000)) else {
        panic!()
    };
    assert!(
        packet
            .identification
            .criteria
            .problems
            .iter()
            .any(|p| matches!(p.kind, ProblemKind::AmbiguousReference { .. }))
    );
    let criteria = packet.identification.criteria.value.unwrap();
    assert_eq!(criteria[0].expected.value, Some(7));
    assert_eq!(criteria[0].definitions.len(), 3);
    assert!(matches!(
        criteria[0].extraction.position.value,
        Some(Position::PacketAbsolute { byte: 16, bit: 0 })
    ));
    assert_eq!(criteria[0].extraction.encoded_bits.value, None);
    assert!(
        criteria[0]
            .extraction
            .encoded_bits
            .problems
            .iter()
            .any(|p| matches!(p.kind, ProblemKind::InconsistentDefinition { .. }))
    );
}

#[test]
fn tpcf_recorded_metadata_survives_duplicate_names() {
    let dir = fixture();
    dir.write("tpcf.dat", "89000\tFirst\t32\n89000\tSecond\t64");
    let Lookup::Found(packet) = Mib::load(dir.path()).unwrap().packet(PacketSpid(89000)) else {
        panic!()
    };
    assert!(packet.packet.name.value.is_none());
    let definitions = packet.packet.characteristics.value.unwrap();
    assert_eq!(definitions.len(), 2);
    assert!(matches!(&definitions[1].fields[2].presence, Presence::Text(s) if s == "64"));
}

#[test]
fn reverse_lookup_orders_packets_numerically_and_retains_cross_parameter_collisions() {
    let dir = fixture();
    dir.write(
        "pcf.dat",
        &format!(
            "ZUT00002\tMode\t\t\t3\t4\t99\t\t\tN\tR\n{}",
            common::PARAMETER
        ),
    );
    dir.write(
        "pid.dat",
        &format!("{PID}\n3\t25\t42\t0\t0\t9\tSmall\t\t-1\t10"),
    );
    dir.write(
        "plf.dat",
        "ZUT00002\t89000\t19\t0\nTEMP\t89000\t19\t0\nZUT00002\t9\t22\t0\nZUT00002\t9\t20\t0",
    );
    let Lookup::Found(parameter) = Mib::load(dir.path())
        .unwrap()
        .parameter(&ParameterName("ZUT00002".into()))
    else {
        panic!()
    };
    let packets = parameter.occurrences.value.unwrap();
    assert_eq!(
        packets
            .iter()
            .map(|p| p.packet.value.as_ref().unwrap().spid.0)
            .collect::<Vec<_>>(),
        [9, 89000]
    );
    assert!(matches!(
        packets[0].occurrences[0].location.position.value,
        Some(Position::PacketAbsolute { byte: 20, bit: 0 })
    ));
    assert!(
        !packets[1].occurrences[0]
            .location
            .position
            .problems
            .is_empty()
    );
}

#[test]
fn lookup_outcomes_and_link_failures_preserve_usable_local_information() {
    let dir = fixture();
    dir.write(
        "pcf.dat",
        &format!("{}\n{}", common::PARAMETER, common::PARAMETER),
    );
    dir.write(
        "plf.dat",
        "TEMP\t89000\t19\t0\nMISSING\t89000\t20\t0\nTEMP\t12\t1\t0",
    );
    dir.write("tpcf.dat", "");
    dir.write("pic.dat", "");
    let mib = Mib::load(dir.path()).unwrap();
    let Lookup::Found(packet) = mib.packet(PacketSpid(89000)) else {
        panic!()
    };
    assert!(matches!(
        packet.packet.name.problems[0].kind,
        ProblemKind::MissingReference { .. }
    ));
    let criteria = packet.identification.criteria.value.unwrap();
    assert_eq!(criteria[0].expected.value, Some(7));
    let layout = packet.layout.value.unwrap();
    let Layout::Element(first) = &layout[0] else {
        panic!()
    };
    assert!(matches!(
        first.parameter.problems[0].kind,
        ProblemKind::AmbiguousReference { .. }
    ));
    assert!(first.location.position.value.is_some());
    let Layout::Element(second) = &layout[1] else {
        panic!()
    };
    assert!(matches!(
        second.parameter.problems[0].kind,
        ProblemKind::MissingReference { .. }
    ));
    assert!(matches!(
        mib.packet(PacketSpid(9)),
        Lookup::NotFound(NotFoundReason::NoMatchingIdentity)
    ));
    dir.write("pid.dat", &format!("{PID}\n{PID}\n{PID}"));
    let mib = Mib::load(dir.path()).unwrap();
    let Lookup::Ambiguous(candidates) = mib.packet(PacketSpid(89000)) else {
        panic!()
    };
    assert_eq!(candidates.first.source.line.get(), 1);
    assert_eq!(candidates.second.source.line.get(), 2);
    assert_eq!(candidates.rest[0].source.line.get(), 3);
    dir.write("pid.dat", "");
    assert!(matches!(
        Mib::load(dir.path()).unwrap().packet(PacketSpid(89000)),
        Lookup::NotFound(NotFoundReason::DefinitionsUnavailable)
    ));
}

#[test]
fn new_tables_follow_partial_loading_and_recorded_presence_policy() {
    for (file, usable) in [
        ("pid.dat", PID),
        ("tpcf.dat", "89000"),
        ("pic.dat", "3\t25\t-1\t0\t-1\t0"),
        ("plf.dat", "TEMP\t89000\t1\t0"),
    ] {
        let dir = Fixture::new();
        dir.write(file, &format!("malformed\n{usable}\n"));
        assert!(
            Mib::load(dir.path()).is_ok(),
            "{file} supports loading without other tables"
        );
    }
    let dir = fixture();
    dir.write(
        "pid.dat",
        &format!("{PID}\tN\tnot-a-number\n{PID}\t\t\tY\t0\tN\t\textra"),
    );
    let mib = Mib::load(dir.path()).unwrap();
    let Lookup::Found(packet) = mib.packet(PacketSpid(89000)) else {
        panic!()
    };
    assert_eq!(packet.packet.definition.source.line.get(), 2);
    assert_eq!(packet.packet.definition.fields.len(), 16);
    assert!(matches!(
        packet.packet.definition.fields[10].presence,
        Presence::Empty
    ));
    assert!(matches!(
        packet.packet.characteristics.value.unwrap()[0].fields[2].presence,
        Presence::Text(_)
    ));
    std::fs::remove_dir_all(dir.path()).unwrap();
    // The returned snapshot and descriptions have no remaining file dependency.
    assert!(matches!(mib.packet(PacketSpid(89000)), Lookup::Found(_)));
}

#[test]
fn reverse_lookup_preserves_missing_and_ambiguous_packet_targets() {
    let dir = Fixture::new();
    dir.write("pcf.dat", common::PARAMETER);
    dir.write("pid.dat", &format!("{PID}\n{PID}"));
    dir.write("plf.dat", "TEMP\t89000\t19\t0\nTEMP\t9\t1\t0");
    let Lookup::Found(parameter) = Mib::load(dir.path())
        .unwrap()
        .parameter(&ParameterName("TEMP".into()))
    else {
        panic!()
    };
    let packets = parameter.occurrences.value.unwrap();
    assert!(matches!(
        packets[0].packet.problems[0].kind,
        ProblemKind::MissingReference {
            reference: Reference::Root(Identity::Packet(PacketSpid(9)))
        }
    ));
    assert!(matches!(
        packets[1].packet.problems[0].kind,
        ProblemKind::AmbiguousReference { .. }
    ));
    assert_eq!(
        packets[1].occurrences[0].location.encoded_bits.value,
        Some(8)
    );
}

#[test]
fn invalid_positions_and_repetition_fields_keep_parameter_and_width() {
    let dir = fixture();
    dir.write("plf.dat", "ZUT00002\t89000\t-1\t0\nZUT00002\t89000\t20\t8\nZUT00002\t89000\t21\t0\t0\nZUT00002\t89000\t22\t0\t2\t-1");
    let Lookup::Found(packet) = Mib::load(dir.path()).unwrap().packet(PacketSpid(89000)) else {
        panic!()
    };
    let layout = packet.layout.value.unwrap();
    assert_eq!(layout.len(), 5);
    for element in layout {
        let Layout::Element(occurrence) = element else {
            panic!()
        };
        assert!(occurrence.parameter.value.is_some());
        assert_eq!(occurrence.location.encoded_bits.value, Some(8));
        assert!(
            !occurrence.location.position.problems.is_empty() || !occurrence.enclosing.is_empty()
        );
    }
}

#[test]
fn missing_layout_table_is_distinct_from_readable_empty_layout() {
    let dir = Fixture::new();
    dir.write("pid.dat", PID);
    let Lookup::Found(packet) = Mib::load(dir.path()).unwrap().packet(PacketSpid(89000)) else {
        panic!()
    };
    assert!(packet.layout.value.is_none());
    assert!(matches!(
        packet.layout.problems[0].kind,
        ProblemKind::MissingReference { .. }
    ));
    dir.write("plf.dat", "");
    let Lookup::Found(packet) = Mib::load(dir.path()).unwrap().packet(PacketSpid(89000)) else {
        panic!()
    };
    assert!(packet.layout.value.unwrap().is_empty());
    assert!(packet.layout.problems.is_empty());
}

#[test]
fn disabled_identification_fields_retain_the_pic_definition() {
    let dir = fixture();
    dir.write("pic.dat", "3\t25\t-1\t0\t-1\t0");
    let Lookup::Found(packet) = Mib::load(dir.path()).unwrap().packet(PacketSpid(89000)) else {
        panic!()
    };
    assert!(packet.identification.criteria.value.unwrap().is_empty());
    assert_eq!(packet.identification.definitions.len(), 1);
    assert!(matches!(
        packet.identification.definitions[0].fields[6].presence,
        Presence::Omitted
    ));
}

#[test]
fn reverse_occurrences_flag_plf_linked_to_a_variable_packet() {
    let dir = fixture();
    dir.write("pid.dat", &PID.replace("\t-1\t10", "\t3\t10"));
    let mib = Mib::load(dir.path()).unwrap();
    let Lookup::Found(packet) = mib.packet(PacketSpid(89000)) else {
        panic!()
    };
    assert!(packet.layout.value.is_none());
    let Lookup::Found(parameter) = mib.parameter(&ParameterName("ZUT00002".into())) else {
        panic!()
    };
    assert!(parameter.occurrences.problems.iter().any(|p| matches!(
        p.kind,
        ProblemKind::MissingReference {
            reference: Reference::Supporting {
                table: Table::Vpd,
                ..
            }
        }
    )));
    let occurrences = parameter.occurrences.value.unwrap();
    assert!(
        occurrences[0]
            .packet
            .problems
            .iter()
            .any(|p| matches!(p.kind, ProblemKind::InconsistentDefinition { .. }))
    );
    assert_eq!(occurrences[0].occurrences.len(), 1);
}
