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

#[test]
fn unicode_case_and_literal_search_punctuation_are_supported() {
    let dir = Fixture::new();
    dir.write("pcf.dat", &PARAMETER.replace("Temperature", "Über mode!"));
    let mib = Mib::load(dir.path()).unwrap();
    assert_eq!(mib.search("ÜBER", SearchScope::All).len(), 1);
    assert_eq!(mib.search("mode!", SearchScope::All).len(), 1);
    assert!(mib.search("^TEMP", SearchScope::All).is_empty());
}
