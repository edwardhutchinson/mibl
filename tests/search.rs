mod common;
use common::{Fixture, PARAMETER};
use mibl::{Mib, model::*};

fn fixture() -> Fixture {
    let dir = Fixture::new();
    dir.write("pcf.dat", &PARAMETER.replace("Temperature", "Mode"));
    dir.write("pid.dat", "3\t25\t42\t7\t0\t89000\tMode\t\t-1\t10");
    dir.write("tpcf.dat", "89000\tHOUSEKEEPING\t32");
    dir.write("ccf.dat", "SET_STATE\tMode\t\t\t\tHEADER");
    dir
}

fn identities(mib: &Mib, query: &str, scope: SearchScope) -> Vec<Identity> {
    mib.search(query, scope)
        .into_iter()
        .map(|c| c.identity)
        .collect()
}

#[test]
fn mode_walkthrough_scopes_and_reuses_identities_in_exact_lookup() {
    let dir = fixture();
    let mib = Mib::load(dir.path()).unwrap();
    let parameter = Identity::Parameter(ParameterName("TEMP".into()));
    let packet = Identity::Packet(PacketSpid(89000));
    let command = Identity::Command(CommandName("SET_STATE".into()));
    assert_eq!(
        identities(&mib, "MoDe", SearchScope::Parameters),
        vec![parameter.clone()]
    );
    assert_eq!(
        identities(&mib, "MoDe", SearchScope::Packets),
        vec![packet.clone()]
    );
    assert_eq!(
        identities(&mib, "MoDe", SearchScope::Commands),
        vec![command.clone()]
    );
    assert_eq!(
        identities(&mib, "MoDe", SearchScope::All),
        vec![parameter, packet, command]
    );
    for candidate in mib.search("mode", SearchScope::All) {
        match candidate.identity {
            Identity::Parameter(name) => assert!(matches!(mib.parameter(&name), Lookup::Found(_))),
            Identity::Packet(spid) => assert!(matches!(mib.packet(spid), Lookup::Found(_))),
            Identity::Command(name) => assert!(matches!(mib.command(&name), Lookup::Found(_))),
        }
    }
    for (query, scope) in [
        ("tmp", SearchScope::Parameters),
        ("hskpg", SearchScope::Packets),
        ("89000", SearchScope::Packets),
        ("890", SearchScope::Packets),
        ("stst", SearchScope::Commands),
    ] {
        assert_eq!(mib.search(query, scope).len(), 1, "{query}");
    }
}

#[test]
fn ranking_puts_exact_then_prefix_then_fuzzy_names_then_descriptions() {
    let dir = Fixture::new();
    let rows = [
        ("OTHER", "mode"),
        ("XMODE", ""),
        ("MODE_LONG", ""),
        ("MODE", ""),
    ]
    .map(|(name, description)| {
        PARAMETER
            .replace("TEMP", name)
            .replace("Temperature", description)
    });
    dir.write("pcf.dat", &rows.join("\n"));
    let mib = Mib::load(dir.path()).unwrap();
    let expected = ["MODE", "MODE_LONG", "XMODE", "OTHER"]
        .map(|name| Identity::Parameter(ParameterName(name.into())));
    assert_eq!(identities(&mib, "mode", SearchScope::All), expected);
}

#[test]
fn ties_preserve_duplicates_and_sort_kind_numeric_identity_and_source() {
    let dir = fixture();
    dir.write(
        "pcf.dat",
        &format!("{0}\n{0}", PARAMETER.replace("Temperature", "Mode")),
    );
    dir.write("pid.dat", "3\t25\t42\t7\t0\t100\tMode\t\t-1\t10\n3\t25\t42\t7\t0\t20\tMode\t\t-1\t10\n3\t25\t42\t7\t0\t20\tMode\t\t-1\t10");
    dir.write("ccf.dat", "ZZ\tMode\t\t\t\tHEADER\nAA\tMode\t\t\t\tHEADER");
    let mib = Mib::load(dir.path()).unwrap();
    let candidates = mib.search("mode", SearchScope::All);
    assert_eq!(
        candidates
            .iter()
            .map(|c| c.identity.clone())
            .collect::<Vec<_>>(),
        vec![
            Identity::Parameter(ParameterName("TEMP".into())),
            Identity::Parameter(ParameterName("TEMP".into())),
            Identity::Packet(PacketSpid(20)),
            Identity::Packet(PacketSpid(20)),
            Identity::Packet(PacketSpid(100)),
            Identity::Command(CommandName("AA".into())),
            Identity::Command(CommandName("ZZ".into())),
        ]
    );
    assert_eq!(
        candidates
            .iter()
            .map(|c| c.source.line.get())
            .collect::<Vec<_>>(),
        [1, 2, 2, 3, 1, 2, 1]
    );
    assert!(matches!(mib.packet(PacketSpid(20)), Lookup::Ambiguous(_)));
}

#[test]
fn uncapped_missing_fields_blank_queries_and_owned_snapshot_candidates() {
    let dir = Fixture::new();
    dir.write(
        "pcf.dat",
        &vec![PARAMETER.replace("Temperature", ""); 1200].join("\n"),
    );
    dir.write("pid.dat", "3\t25\t42\t7\t0\t99\t\t\t-1\t10");
    let mib = Mib::load(dir.path()).unwrap();
    for query in ["", "  \t\n", "zzzzzzzz"] {
        assert!(mib.search(query, SearchScope::All).is_empty());
    }
    assert_eq!(mib.search("temp", SearchScope::All).len(), 1200);
    let packet = mib.search("99", SearchScope::All);
    assert_eq!(packet.len(), 1);
    assert!(packet[0].name.value.is_none());
    assert!(packet[0].description.value.is_none());
    drop(dir);
    let candidates = mib.search("temp", SearchScope::Parameters);
    drop(mib);
    assert_eq!(candidates.len(), 1200);
    assert_eq!(candidates[1199].source.line.get(), 1200);
    assert!(candidates[0].description.value.is_none());
    assert_eq!(candidates[0].name.value.as_deref(), Some("TEMP"));
}

#[test]
fn supporting_only_snapshot_has_no_candidates() {
    let dir = Fixture::new();
    dir.write("caf.dat", "CURVE\tCurve\tR\tU");
    assert!(
        Mib::load(dir.path())
            .unwrap()
            .search("curve", SearchScope::All)
            .is_empty()
    );
}

#[test]
fn numeric_identity_classes_outrank_packet_names_and_descriptions() {
    let dir = Fixture::new();
    dir.write("pid.dat", "3\t25\t42\t7\t0\t120\t12\t\t-1\t10\n3\t25\t42\t7\t0\t12\t\t\t-1\t10\n3\t25\t42\t7\t0\t9\t12\t\t-1\t10\n3\t25\t42\t7\t0\t8\t\t\t-1\t10");
    dir.write("tpcf.dat", "8\t12\t32");
    let mib = Mib::load(dir.path()).unwrap();
    assert_eq!(
        identities(&mib, "12", SearchScope::All),
        [12, 120, 8, 9].map(|spid| Identity::Packet(PacketSpid(spid)))
    );
}

/// One packet root whose TPCF reference is ambiguous records both names, and both stay
/// searchable case-insensitively while the displayed name stays unavailable.
#[test]
fn every_recorded_tpcf_name_matches_and_the_name_stays_ambiguous() {
    let dir = Fixture::new();
    dir.write("pid.dat", "3\t25\t42\t0\t0\t100\tPacket\t\t-1\t10");
    dir.write("tpcf.dat", "100\tUNIQUE_ALPHA\n100\tUNIQUE_BETA");
    let mib = Mib::load(dir.path()).unwrap();
    for (query, scope) in [
        ("UNIQUE", SearchScope::Packets),
        ("unique_alpha", SearchScope::Packets),
        ("UNIQUE_BETA", SearchScope::Packets),
        ("unique", SearchScope::All),
        ("UnIqUe_BeTa", SearchScope::All),
    ] {
        let candidates = mib.search(query, scope);
        assert_eq!(candidates.len(), 1, "{query} {scope:?}");
        let candidate = &candidates[0];
        assert_eq!(candidate.identity, Identity::Packet(PacketSpid(100)));
        // No linked definition is chosen to display a hit: the name stays unavailable and
        // keeps the evidence naming both recorded candidates.
        assert!(candidate.name.value.is_none(), "{query}");
        let [
            Problem {
                kind:
                    ProblemKind::AmbiguousReference {
                        alternatives,
                        reference,
                    },
                ..
            },
        ] = candidate.name.problems.as_slice()
        else {
            panic!("{query}: {:?}", candidate.name.problems)
        };
        assert_eq!(
            reference,
            &Reference::Supporting {
                table: Table::Tpcf,
                key: "100".into()
            }
        );
        assert_eq!(alternatives.first.definition.source.line.get(), 1);
        assert_eq!(alternatives.second.definition.source.line.get(), 2);
        assert!(alternatives.rest.is_empty());
        // The returned identity is reusable: exact lookup finds the one PID root and keeps
        // the ambiguous name beside every recorded definition.
        let Lookup::Found(description) = mib.packet(PacketSpid(100)) else {
            panic!("{query}: exact packet lookup is no longer Found")
        };
        assert!(description.packet.name.value.is_none());
        assert_eq!(
            description
                .packet
                .characteristics
                .value
                .unwrap()
                .iter()
                .map(|d| d.source.line.get())
                .collect::<Vec<_>>(),
            [1, 2]
        );
    }
    // Folding applies to every retained name, including non-ASCII text.
    let dir = Fixture::new();
    dir.write("pid.dat", "3\t25\t42\t0\t0\t100\tPacket\t\t-1\t10");
    dir.write("tpcf.dat", "100\tÜBER_ALPHA\n100\tÜBER_BETA");
    let mib = Mib::load(dir.path()).unwrap();
    assert_eq!(mib.search("über_beta", SearchScope::All).len(), 1);
    assert_eq!(mib.search("ÜBER", SearchScope::Packets).len(), 1);
}

/// Several matching names still give one candidate per PID root, and duplicate PID roots
/// each keep their own candidate.
#[test]
fn one_candidate_per_pid_root_however_many_recorded_names_match() {
    let dir = Fixture::new();
    dir.write(
        "pid.dat",
        "3\t25\t42\t0\t0\t100\tPacket A\t\t-1\t10\n3\t25\t42\t0\t0\t200\tPacket B\t\t-1\t10",
    );
    dir.write(
        "tpcf.dat",
        "100\tALPHA_FIRST\n100\tALPHA_SECOND\n200\tALPHA_THIRD",
    );
    let mib = Mib::load(dir.path()).unwrap();
    let spids = |query| {
        mib.search(query, SearchScope::Packets)
            .into_iter()
            .map(|c| c.identity)
            .collect::<Vec<_>>()
    };
    // Both names of the first root match one query and stay one candidate, ordered by SPID.
    assert_eq!(
        spids("ALPHA"),
        [100, 200].map(|spid| Identity::Packet(PacketSpid(spid)))
    );
    assert_eq!(
        spids("alpha_second"),
        vec![Identity::Packet(PacketSpid(100))]
    );
    assert_eq!(
        spids("ALPHA_THIRD"),
        vec![Identity::Packet(PacketSpid(200))]
    );
}

/// Duplicate identical TPCF names stay one candidate per PID root, and duplicate PID roots
/// stay separate candidates in source order.
#[test]
fn duplicate_names_and_duplicate_pid_roots_stay_separate_candidates() {
    let dir = Fixture::new();
    dir.write("tpcf.dat", "100\tDOUBLE\n100\tDOUBLE\n200\tDOUBLE");
    dir.write(
        "pid.dat",
        "3\t25\t42\t0\t0\t100\tPacket A\t\t-1\t10\n3\t25\t42\t0\t0\t100\tPacket A\t\t-1\t10\n3\t25\t42\t0\t0\t200\tPacket B\t\t-1\t10",
    );
    let mib = Mib::load(dir.path()).unwrap();
    let candidates = mib.search("double", SearchScope::Packets);
    assert_eq!(
        candidates
            .iter()
            .map(|c| c.source.line.get())
            .collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert_eq!(
        candidates
            .iter()
            .map(|c| c.identity.clone())
            .collect::<Vec<_>>(),
        [100, 100, 200].map(|spid| Identity::Packet(PacketSpid(spid)))
    );
    // Only the root recording two TPCF rows has an ambiguous name; the single-row root
    // still shows its one recorded name.
    assert_eq!(
        candidates
            .iter()
            .map(|c| c.name.value.as_deref())
            .collect::<Vec<_>>(),
        [None, None, Some("DOUBLE")]
    );
}

/// Exact SPIDs outrank identity prefixes, which outrank fuzzy recorded names, which
/// outrank description-only matches.
#[test]
fn recorded_names_rank_below_identities_and_above_descriptions() {
    let dir = Fixture::new();
    dir.write(
        "pid.dat",
        "3\t25\t42\t0\t0\t12\tPlain\t\t-1\t10\n3\t25\t42\t0\t0\t120\tPlain\t\t-1\t10\n3\t25\t42\t0\t0\t5\tPlain\t\t-1\t10\n3\t25\t42\t0\t0\t4\t12\t\t-1\t10",
    );
    dir.write(
        "tpcf.dat",
        "12\tTWE\n120\tTWE_LONG\n5\t12_ALPHA\n5\t12_BETA\n4\tTWE_OTHER",
    );
    let mib = Mib::load(dir.path()).unwrap();
    // SPID 12 matches exactly, 120 by identity prefix, 5 through its recorded names and
    // 4 only through its description. The description-only root has the lowest SPID, so
    // its position proves names rank ahead of descriptions rather than numeric order.
    assert_eq!(
        identities(&mib, "12", SearchScope::Packets),
        [12, 120, 5, 4].map(|spid| Identity::Packet(PacketSpid(spid)))
    );
}

#[test]
fn unicode_case_and_literal_search_punctuation_are_supported() {
    let dir = Fixture::new();
    dir.write("pcf.dat", &PARAMETER.replace("Temperature", "Über mode!"));
    let mib = Mib::load(dir.path()).unwrap();
    assert_eq!(mib.search("ÜBER", SearchScope::All).len(), 1);
    assert_eq!(mib.search("mode!", SearchScope::All).len(), 1);
    assert!(mib.search("^TEMP", SearchScope::All).is_empty());
}
