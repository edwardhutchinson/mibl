//! Snapshot-level acceptance scenarios over the combined data: duplicate
//! roots, partial snapshots and independence from the sources a load read.
//!
//! These scenarios mutate the directory they loaded, so each one keeps the
//! order its assertions depend on.

use crate::{
    common::Fixture,
    fixture::{CAF, CAP, PCF, PCPC, PRV, VPD, combined},
    support::{
        command, declared_arguments, loaded, packet, parameter, position_text, query_digest,
        source, spids,
    },
};
use mibl::{
    LoadError, Mib,
    model::{
        CalibrationForm, CommandName, Identity, Lookup, NotFoundReason, PacketSpid, ParameterName,
        ProblemKind, Reference, SearchScope, Table,
    },
};
use std::{
    io,
    path::{Component, Path},
};

#[test]
fn duplicate_roots_and_linked_definitions_stay_separate_and_ordered() {
    let dir = combined();
    dir.write(
        "pid.dat",
        "3\t25\t42\t7\t0\t89000\tDemonstration housekeeping\t\t-1\t10\n\
3\t26\t42\t0\t0\t89001\tDemonstration variable packet\t\t7\t10\n\
3\t25\t42\t7\t0\t89000\tDemonstration housekeeping duplicate\t\t-1\t10\n",
    );
    dir.write(
        "ccf.dat",
        "DEMO_TC001\tDemonstration command one\tDistribute demonstration commands\t\tN\tDEMO_HDR01\t3\t25\t42\t3\n\
DEMO_TC074\tDemonstration command two\tEnable forwarding of demonstration packets\t\tN\tDEMO_HDR02\t14\t1\t28\t6\n\
DEMO_TC003\tThird demonstration command\tDeclared widths and an empty header\t\tN\tDEMO_HDR03\t3\t25\t42\t2\n\
DEMO_TC001\tDemonstration command one duplicate\tDistribute demonstration commands\t\tN\tDEMO_HDR01\t3\t25\t42\t3\n",
    );
    dir.write(
        "pcf.dat",
        &format!("{PCF}DEMO_STATE\tDuplicate state\t\t\t3\t4\t\t\t\tS\tR\tDEMO_STATE_TXF\n"),
    );
    let mib = loaded(&dir);
    // Duplicate roots keep both definitions in source order and never select one.
    let Lookup::Ambiguous(candidates) = mib.packet(PacketSpid(89000)) else {
        panic!("expected candidates")
    };
    assert_eq!(candidates.first.source, source("pid.dat", 1));
    assert_eq!(candidates.second.source, source("pid.dat", 3));
    assert_eq!(candidates.first.name.value.as_deref(), Some("DEMO_HK"));
    assert_eq!(
        candidates.second.description.value.as_deref(),
        Some("Demonstration housekeeping duplicate")
    );
    assert_eq!(candidates.rest.len(), 0);
    let Lookup::Ambiguous(candidates) = mib.command(&CommandName("DEMO_TC001".into())) else {
        panic!("expected candidates")
    };
    assert_eq!(candidates.first.source, source("ccf.dat", 1));
    assert_eq!(candidates.second.source, source("ccf.dat", 4));
    let Lookup::Ambiguous(candidates) = mib.parameter(&ParameterName("DEMO_STATE".into())) else {
        panic!("expected candidates")
    };
    assert_eq!(candidates.first.source, source("pcf.dat", 3));
    assert_eq!(candidates.second.source, source("pcf.dat", 6));
    // An unrelated identity stays uniquely found, and its occurrence names both competing roots.
    let mode = parameter(&mib, "DEMO_MODE");
    let packets = mode.occurrences.value.as_ref().unwrap();
    assert_eq!(packets.len(), 2);
    assert!(packets[0].packet.value.is_none());
    assert_eq!(
        packets[1].packet.value.as_ref().unwrap().spid,
        PacketSpid(89001)
    );
    assert_eq!(packets[0].occurrences.len(), 1);
    assert_eq!(
        position_text(&packets[0].occurrences[0].location.position),
        "byte 19 bit 0",
        "the declared occurrence survives duplicate packet roots"
    );
    let ProblemKind::AmbiguousReference {
        reference,
        alternatives,
    } = &packets[0].packet.problems[0].kind
    else {
        panic!("{:?}", packets[0].packet.problems)
    };
    assert!(matches!(
        reference,
        Reference::Root(Identity::Packet(PacketSpid(89000)))
    ));
    assert_eq!(alternatives.first.definition.source, source("pid.dat", 1));
    assert_eq!(alternatives.second.definition.source, source("pid.dat", 3));
    let Lookup::Found(variable) = mib.packet(PacketSpid(89001)) else {
        panic!("duplicate roots are per identity")
    };
    assert_eq!(variable.packet.name.value.as_deref(), Some("DEMO_VAR"));
    assert_eq!(
        mode.parameter.definition.source,
        source("pcf.dat", 1),
        "an identity index never overwrites a duplicate root"
    );
}

#[test]
fn partial_snapshots_keep_usable_information_beside_missing_tables() {
    let dir = combined();
    for file in ["plf.dat", "prf.dat", "txp.dat", "pcdf.dat"] {
        std::fs::remove_file(dir.path().join(file)).unwrap();
    }
    let mib = loaded(&dir);
    // Packet identification survives without its layout rows.
    let housekeeping = packet(&mib, 89000);
    assert_eq!(housekeeping.identification.apid.value, Some(42));
    assert_eq!(
        housekeeping
            .identification
            .criteria
            .value
            .as_ref()
            .unwrap()
            .len(),
        1
    );
    assert!(housekeeping.layout.value.is_none());
    assert!(housekeeping.layout.problems.iter().any(|problem| matches!(
        &problem.kind,
        ProblemKind::MissingReference {
            reference: Reference::Supporting {
                table: Table::Plf,
                ..
            }
        }
    )));
    // The parameter loses only the fixed occurrence and names the missing table.
    let mode = parameter(&mib, "DEMO_MODE");
    assert_eq!(mode.parameter.encoding.encoded_bits.value, Some(8));
    let packets = mode.occurrences.value.as_ref().unwrap();
    assert_eq!(spids(packets), vec![89001]);
    assert!(mode.occurrences.problems.iter().any(|problem| matches!(
        &problem.kind,
        ProblemKind::MissingReference { reference: Reference::Supporting { table: Table::Plf, key } }
            if key == "DEMO_MODE"
    )));
    // A textual calibration keeps its header while the interval table is absent.
    let state = parameter(&mib, "DEMO_STATE");
    for alternative in state.parameter.calibrations.value.iter().flatten() {
        let calibration = alternative.calibration.value.as_ref().unwrap();
        let Some(CalibrationForm::Textual { intervals }) = &calibration.form.value else {
            panic!("expected the textual family")
        };
        assert!(intervals.value.is_none());
        assert!(intervals.problems.iter().any(|problem| matches!(
            &problem.kind,
            ProblemKind::MissingReference { reference: Reference::Supporting { table: Table::Txp, key } }
                if key == "DEMO_STATE_TXF"
        )));
    }
    // Command arguments, aliases and conversions stay usable beside the missing rule and header rows.
    let one = command(&mib, "DEMO_TC001");
    let arguments = declared_arguments(one.arguments.value.as_ref().unwrap());
    assert_eq!(arguments.len(), 3);
    assert!(arguments[0].rules.ranges.value.is_none());
    assert_eq!(arguments[0].rules.aliases.value.as_ref().unwrap().len(), 2);
    assert_eq!(
        arguments[0]
            .rules
            .calibrations
            .value
            .as_ref()
            .unwrap()
            .len(),
        1
    );
    let header = one.header.value.as_ref().unwrap();
    assert_eq!(header.definition.source, source("tcp.dat", 1));
    assert!(header.fields.value.is_none());
    assert!(header.fields.problems.iter().any(|problem| matches!(
        &problem.kind,
        ProblemKind::MissingReference {
            reference: Reference::Supporting {
                table: Table::Pcdf,
                ..
            }
        }
    )));
    // A snapshot with usable supporting rows and no roots still loads and answers every kind.
    let supporting = Fixture::new();
    for (file, text) in [
        ("caf.dat", CAF),
        ("cap.dat", CAP),
        ("pcpc.dat", PCPC),
        ("prv.dat", PRV),
        ("vpd.dat", VPD),
    ] {
        supporting.write(file, text);
    }
    let mib = loaded(&supporting);
    assert!(matches!(
        mib.parameter(&ParameterName("DEMO_TEMP".into())),
        Lookup::NotFound(NotFoundReason::DefinitionsUnavailable)
    ));
    assert!(matches!(
        mib.packet(PacketSpid(89000)),
        Lookup::NotFound(NotFoundReason::DefinitionsUnavailable)
    ));
    assert!(matches!(
        mib.command(&CommandName("DEMO_TC001".into())),
        Lookup::NotFound(NotFoundReason::DefinitionsUnavailable)
    ));
    assert!(mib.search("mode", SearchScope::All).is_empty());
    assert!(mib.pus(3, None).is_empty());
    // Unusable and absent directories keep their visible typed outcomes.
    let malformed = Fixture::new();
    malformed.write("pcf.dat", "not a parameter row\n");
    assert!(matches!(
        Mib::load(malformed.path()),
        Err(LoadError::NoUsableSupportedRows { directory }) if directory == malformed.path()
    ));
    assert!(matches!(
        Mib::load(&dir.path().join("gone")),
        Err(LoadError::InaccessibleDirectory { cause, .. })
            if cause.kind() == io::ErrorKind::NotFound
    ));
}

#[test]
fn snapshot_independence_and_owned_results_survive_source_loss() {
    let dir = combined();
    let mib = loaded(&dir);
    let before = query_digest(&mib);
    let Lookup::Found(owned) = mib.parameter(&ParameterName("DEMO_MODE".into())) else {
        panic!()
    };
    // Changing and then removing every source leaves the loaded snapshot unchanged.
    std::fs::write(
        dir.path().join("pcf.dat"),
        "OTHER\tOther\t\t\t3\t4\t\t\t\tN\tR\n",
    )
    .unwrap();
    std::fs::remove_file(dir.path().join("plf.dat")).unwrap();
    assert_eq!(query_digest(&mib), before);
    std::fs::remove_dir_all(dir.path()).unwrap();
    assert_eq!(query_digest(&mib), before);
    // Reloading the vanished directory is a visible, typed error.
    assert!(matches!(
        Mib::load(dir.path()),
        Err(LoadError::InaccessibleDirectory { .. })
    ));
    // Owned results outlive the snapshot and keep relative, one-based provenance.
    drop(mib);
    assert_eq!(
        owned.parameter.description.value.as_deref(),
        Some("Operational mode")
    );
    assert_eq!(
        spids(owned.occurrences.value.as_ref().unwrap()),
        vec![89000, 89001]
    );
    assert_eq!(owned.parameter.definition.source.file, Path::new("pcf.dat"));
    assert_eq!(owned.parameter.definition.source.line.get(), 1);
    assert!(
        !owned
            .parameter
            .definition
            .source
            .file
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    );
    // An independent load of the same synthetic content is byte-identical.
    assert_eq!(query_digest(&loaded(&combined())), before);
}
