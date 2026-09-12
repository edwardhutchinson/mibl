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
