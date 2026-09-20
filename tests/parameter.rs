mod common;

use common::{Fixture, PARAMETER};
use mibl::{Mib, model::*};

#[test]
fn lookup_preserves_recorded_fields_and_interprets_defaults() {
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    let mib = Mib::load(dir.path()).unwrap();
    let Lookup::Found(found) = mib.parameter(&ParameterName("TEMP".into())) else {
        panic!("expected parameter")
    };
    let p = found.parameter;
    assert_eq!(p.name.0, "TEMP");
    assert_eq!(p.description.value.as_deref(), Some("Temperature"));
    assert_eq!(p.units.value.as_deref(), Some("K"));
    assert_eq!(p.encoding.ptc.value, Some(3));
    assert_eq!(p.encoding.pfc.value, Some(4));
    assert_eq!(p.encoding.encoded_bits.value, Some(8));
    assert_eq!(p.encoding.endian.value.as_deref(), Some("B"));
    assert_eq!(p.definition.source.file.to_str(), Some("pcf.dat"));
    assert_eq!(p.definition.source.line.get(), 1);
    assert_eq!(p.definition.fields.len(), 24);
    assert!(matches!(p.definition.fields[2].presence, Presence::Empty));
    assert!(matches!(
        p.definition.fields[12].presence,
        Presence::Omitted
    ));
    let endian = &p.definition.fields[22];
    assert_eq!(endian.column.get(), 23);
    assert_eq!(endian.meanings[0].schema_name, "PCF_ENDIAN");
    assert!(matches!(
        endian.meanings[0]
            .interpretation
            .value
            .as_ref()
            .unwrap()
            .origin,
        InterpretationOrigin::DocumentedDefault { .. }
    ));
}

#[test]
fn exact_lookup_is_case_sensitive_and_retains_every_duplicate() {
    let dir = Fixture::new();
    dir.write(
        "pcf.dat",
        &format!("{PARAMETER}\n{PARAMETER}\n{PARAMETER}\n"),
    );
    let mib = Mib::load(dir.path()).unwrap();
    assert!(matches!(
        mib.parameter(&ParameterName("temp".into())),
        Lookup::NotFound(NotFoundReason::NoMatchingIdentity)
    ));
    let Lookup::Ambiguous(candidates) = mib.parameter(&ParameterName("TEMP".into())) else {
        panic!("expected duplicates")
    };
    assert_eq!(candidates.first.source.line.get(), 1);
    assert_eq!(candidates.second.source.line.get(), 2);
    assert_eq!(candidates.rest.len(), 1);
    assert_eq!(candidates.rest[0].source.line.get(), 3);
    assert_eq!(
        candidates.first.identity,
        Identity::Parameter(ParameterName("TEMP".into()))
    );
}

#[test]
fn supporting_only_snapshot_distinguishes_unavailable_roots_from_an_empty_mib() {
    let dir = Fixture::new();
    dir.write("caf.dat", "CURVE\tSynthetic curve\tR\tU");
    let mib = Mib::load(dir.path()).unwrap();
    assert!(matches!(
        mib.parameter(&ParameterName("TEMP".into())),
        Lookup::NotFound(NotFoundReason::DefinitionsUnavailable)
    ));
    dir.write("caf.dat", "CURVE\tMissing required format");
    assert!(
        matches!(Mib::load(dir.path()), Err(mibl::LoadError::NoUsableSupportedRows { directory }) if directory == dir.path())
    );
}

#[test]
fn malformed_neighbors_and_broken_references_do_not_hide_a_usable_definition() {
    let dir = Fixture::new();
    let mut cells: Vec<_> = PARAMETER.split('\t').collect();
    cells[7] = "MISSING";
    cells[8] = "NO_LINK";
    cells.extend([
        "NO_CURVE",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "More detail",
        "ignored",
        "also ignored",
    ]);
    dir.write(
        "pcf.dat",
        &format!(
            "bad row\n{}\n{}\n{}\n{}\r\n",
            PARAMETER.replace("\t3\t", "\tbad\t"),
            PARAMETER.replace("\t99\t", "\tbad\t"),
            PARAMETER.replace("\tN\tR", "\tX\tR"),
            cells.join("\t")
        ),
    );
    std::fs::create_dir(dir.path().join("caf.dat")).unwrap();
    dir.write("unrelated.dat", "not a supported row");
    let mib = Mib::load(dir.path()).unwrap();
    let Lookup::Found(found) = mib.parameter(&ParameterName("TEMP".into())) else {
        panic!("expected surviving row")
    };
    let fields = &found.parameter.definition.fields;
    assert_eq!(found.parameter.definition.source.line.get(), 5);
    assert_eq!(fields.len(), 24);
    assert!(matches!(&fields[7].presence, Presence::Text(s) if s == "MISSING"));
    assert!(matches!(&fields[11].presence, Presence::Text(s) if s == "NO_CURVE"));
    assert!(matches!(&fields[23].presence, Presence::Text(s) if s == "More detail"));
    assert!(matches!(fields[22].presence, Presence::Empty));
    assert_eq!(found.parameter.encoding.endian.value.as_deref(), Some("B"));
    assert!(matches!(
        &fields[17].meanings[0]
            .interpretation
            .value
            .as_ref()
            .unwrap()
            .value,
        Scalar::Integer(1)
    ));
}

#[test]
fn snapshot_and_owned_results_survive_source_changes_and_removal() {
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    let mib = Mib::load(dir.path()).unwrap();
    dir.write("pcf.dat", &PARAMETER.replace("Temperature", "Changed"));
    let Lookup::Found(mut first) = mib.parameter(&ParameterName("TEMP".into())) else {
        panic!("expected parameter")
    };
    assert_eq!(
        first.parameter.description.value.as_deref(),
        Some("Temperature")
    );
    first.parameter.description.value = Some("Caller mutation".into());
    std::fs::remove_dir_all(dir.path()).unwrap();
    let Lookup::Found(second) = mib.parameter(&ParameterName("TEMP".into())) else {
        panic!("snapshot must not reread files")
    };
    drop(mib);
    assert_eq!(
        second.parameter.description.value.as_deref(),
        Some("Temperature")
    );
    assert_eq!(second.parameter.definition.fields.len(), 24);
    assert_eq!(
        first.parameter.description.value.as_deref(),
        Some("Caller mutation")
    );
}

#[test]
fn load_errors_preserve_paths_and_io_causes() {
    use std::error::Error;
    let dir = Fixture::new();
    let missing = dir.path().join("absent");
    let Err(error) = Mib::load(&missing) else {
        panic!("expected load error")
    };
    assert!(error.source().is_some());
    assert!(error.to_string().contains("absent"));
    assert!(
        matches!(error, mibl::LoadError::InaccessibleDirectory { directory, cause } if directory == missing && cause.kind() == std::io::ErrorKind::NotFound)
    );
    dir.write("file", "not a directory");
    assert!(matches!(
        Mib::load(&dir.path().join("file")),
        Err(mibl::LoadError::InaccessibleDirectory { .. })
    ));
    dir.write("unknown.dat", "ignored");
    assert!(matches!(
        Mib::load(dir.path()),
        Err(mibl::LoadError::NoUsableSupportedRows { .. })
    ));
}

#[test]
fn unsupported_encoding_keeps_known_fields_and_original_codes() {
    let dir = Fixture::new();
    dir.write("pcf.dat", &PARAMETER.replace("\t3\t4\t", "\t99\t999\t"));
    let mib = Mib::load(dir.path()).unwrap();
    let Lookup::Found(found) = mib.parameter(&ParameterName("TEMP".into())) else {
        panic!("unsupported interpretation must retain row")
    };
    assert_eq!(found.parameter.units.value.as_deref(), Some("K"));
    assert_eq!(found.parameter.encoding.ptc.value, Some(99));
    assert!(found.parameter.encoding.encoded_bits.value.is_none());
    assert!(matches!(
        found.parameter.encoding.encoded_bits.problems[0].kind,
        ProblemKind::UnsupportedInterpretation { .. }
    ));
}

#[test]
fn encoded_width_uses_type_and_format_including_time_formats() {
    // Expected lengths from ICD 7.0 Appendix A, independent of PCF_WIDTH=99.
    for (ptc, pfc, bits) in [
        (1, 0, 1),
        (2, 12, 12),
        (4, 13, 24),
        (5, 2, 64),
        (7, 3, 24),
        (9, 1, 48),
        (9, 2, 64),
        (9, 17, 48),
        (9, 30, 64),
        (10, 10, 40),
    ] {
        let dir = Fixture::new();
        dir.write(
            "pcf.dat",
            &PARAMETER.replace("\t3\t4\t", &format!("\t{ptc}\t{pfc}\t")),
        );
        let mib = Mib::load(dir.path()).unwrap();
        let Lookup::Found(found) = mib.parameter(&ParameterName("TEMP".into())) else {
            panic!("expected parameter")
        };
        assert_eq!(
            found.parameter.encoding.encoded_bits.value,
            Some(bits),
            "PTC {ptc}, PFC {pfc}"
        );
    }
}

fn missing_table(problems: &[Problem], table: Table, key: &str) -> bool {
    problems.iter().any(|p| {
        matches!(
            &p.kind,
            ProblemKind::MissingReference {
                reference: Reference::Supporting {
                    table: found,
                    key: found_key,
                }
            } if *found == table && found_key == key
        )
    })
}

#[test]
fn occurrence_availability_distinguishes_missing_unreadable_and_readable_plf_sources() {
    // No PLF table can name the parameter, so the collection stays unavailable rather than
    // asserting a known empty, and the root definition remains usable and Found.
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    let Lookup::Found(found) = Mib::load(dir.path())
        .unwrap()
        .parameter(&ParameterName("TEMP".into()))
    else {
        panic!("a usable root definition stays Found")
    };
    assert!(found.occurrences.value.is_none());
    assert!(missing_table(
        &found.occurrences.problems,
        Table::Plf,
        "TEMP"
    ));
    assert_eq!(found.parameter.units.value.as_deref(), Some("K"));
    assert_eq!(found.parameter.definition.fields.len(), 24);

    // A readable PLF table without a matching row is a known empty result.
    dir.write("plf.dat", "");
    let Lookup::Found(found) = Mib::load(dir.path())
        .unwrap()
        .parameter(&ParameterName("TEMP".into()))
    else {
        panic!("expected parameter")
    };
    assert!(found.occurrences.value.as_ref().is_some_and(Vec::is_empty));
    assert!(found.occurrences.problems.is_empty());

    // An unreadable PLF table is unavailable, not a known empty result.
    std::fs::remove_file(dir.path().join("plf.dat")).unwrap();
    std::fs::create_dir(dir.path().join("plf.dat")).unwrap();
    let Lookup::Found(found) = Mib::load(dir.path())
        .unwrap()
        .parameter(&ParameterName("TEMP".into()))
    else {
        panic!("expected parameter")
    };
    assert!(found.occurrences.value.is_none());
    assert!(missing_table(
        &found.occurrences.problems,
        Table::Plf,
        "TEMP"
    ));

    // A readable PLF row keeps the fixed occurrence and needs no problem.
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    dir.write(
        "pid.dat",
        "3\t25\t42\t0\t0\t89000\tDemonstration housekeeping\t\t-1\t10",
    );
    dir.write("tpcf.dat", "89000\tDEMO_HK");
    dir.write("plf.dat", "TEMP\t89000\t16\t0");
    let Lookup::Found(found) = Mib::load(dir.path())
        .unwrap()
        .parameter(&ParameterName("TEMP".into()))
    else {
        panic!("expected parameter")
    };
    let packets = found.occurrences.value.as_ref().unwrap();
    assert_eq!(packets.len(), 1);
    assert_eq!(
        packets[0]
            .packet
            .value
            .as_ref()
            .unwrap()
            .name
            .value
            .as_deref(),
        Some("DEMO_HK")
    );
    assert_eq!(packets[0].occurrences.len(), 1);
    assert!(found.occurrences.problems.is_empty());
}

#[test]
fn occurrence_availability_follows_declared_variable_packet_structures() {
    // A declared variable structure whose VPD data is unavailable keeps the collection
    // unavailable, whether the table is missing, empty, or simply lacks a row for that TPSD.
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    dir.write("plf.dat", "");
    dir.write(
        "pid.dat",
        "3\t25\t42\t0\t0\t89001\tDemonstration variable packet\t\t7\t10",
    );
    for vpd in [None, Some(""), Some("8\t1\tOTHER\t\t\t\t\t\t8")] {
        if let Some(vpd) = vpd {
            dir.write("vpd.dat", vpd);
        }
        let Lookup::Found(found) = Mib::load(dir.path())
            .unwrap()
            .parameter(&ParameterName("TEMP".into()))
        else {
            panic!("expected parameter")
        };
        assert!(found.occurrences.value.is_none(), "vpd {vpd:?}");
        assert!(missing_table(&found.occurrences.problems, Table::Vpd, "7"));
    }

    // A readable VPD row for the declared TPSD that does not name the parameter is a known empty.
    dir.write("vpd.dat", "7\t1\tOTHER\t\t\t\t\t\t8");
    let Lookup::Found(found) = Mib::load(dir.path())
        .unwrap()
        .parameter(&ParameterName("TEMP".into()))
    else {
        panic!("expected parameter")
    };
    assert!(found.occurrences.value.as_ref().is_some_and(Vec::is_empty));
    assert!(found.occurrences.problems.is_empty());

    // A readable VPD row naming the parameter resolves the declared structure and keeps the
    // variable occurrence in its containing packet.
    dir.write("vpd.dat", "7\t1\tTEMP\t\t\t\t\t\t8");
    let Lookup::Found(found) = Mib::load(dir.path())
        .unwrap()
        .parameter(&ParameterName("TEMP".into()))
    else {
        panic!("expected parameter")
    };
    let packets = found.occurrences.value.as_ref().unwrap();
    assert_eq!(packets.len(), 1);
    assert_eq!(
        packets[0].packet.value.as_ref().unwrap().spid,
        PacketSpid(89001)
    );
    assert_eq!(packets[0].occurrences.len(), 1);
    assert!(found.occurrences.problems.is_empty());

    // Two PID roots sharing one TPSD describe one structure, so they report one reference.
    dir.write(
        "pid.dat",
        "3\t25\t42\t0\t0\t89001\tFirst variable packet\t\t7\t10\n3\t26\t42\t0\t0\t89002\tSecond variable packet\t\t7\t10",
    );
    std::fs::remove_file(dir.path().join("vpd.dat")).unwrap();
    let Lookup::Found(found) = Mib::load(dir.path())
        .unwrap()
        .parameter(&ParameterName("TEMP".into()))
    else {
        panic!("expected parameter")
    };
    assert!(found.occurrences.value.is_none());
    assert_eq!(
        found
            .occurrences
            .problems
            .iter()
            .filter(|p| matches!(
                p.kind,
                ProblemKind::MissingReference {
                    reference: Reference::Supporting {
                        table: Table::Vpd,
                        ..
                    }
                }
            ))
            .count(),
        1
    );
}

#[test]
fn known_occurrences_survive_beside_unavailable_sources() {
    // A variable occurrence stays visible beside a missing PLF table.
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    dir.write(
        "pid.dat",
        "3\t25\t42\t0\t0\t89001\tDemonstration variable packet\t\t7\t10",
    );
    dir.write("vpd.dat", "7\t1\tTEMP\t\t\t\t\t\t8");
    let Lookup::Found(found) = Mib::load(dir.path())
        .unwrap()
        .parameter(&ParameterName("TEMP".into()))
    else {
        panic!("expected parameter")
    };
    let packets = found.occurrences.value.as_ref().unwrap();
    assert_eq!(packets.len(), 1);
    assert_eq!(packets[0].occurrences.len(), 1);
    assert!(missing_table(
        &found.occurrences.problems,
        Table::Plf,
        "TEMP"
    ));

    // A fixed occurrence stays visible beside an unavailable variable structure.
    dir.write(
        "pid.dat",
        "3\t25\t42\t0\t0\t89000\tDemonstration housekeeping\t\t-1\t10\n3\t26\t42\t0\t0\t89001\tDemonstration variable packet\t\t7\t10",
    );
    dir.write("plf.dat", "TEMP\t89000\t16\t0");
    std::fs::remove_file(dir.path().join("vpd.dat")).unwrap();
    let Lookup::Found(found) = Mib::load(dir.path())
        .unwrap()
        .parameter(&ParameterName("TEMP".into()))
    else {
        panic!("expected parameter")
    };
    let packets = found.occurrences.value.as_ref().unwrap();
    assert_eq!(packets.len(), 1);
    assert_eq!(
        packets[0].packet.value.as_ref().unwrap().spid,
        PacketSpid(89000)
    );
    assert!(missing_table(&found.occurrences.problems, Table::Vpd, "7"));
}
