use crate::common::{Fixture, PARAMETER};
use crate::support::run;

#[test]
fn search_mode_walkthrough_always_uses_candidate_table_and_reuses_identities() {
    let dir = Fixture::new();
    dir.write("pcf.dat", &PARAMETER.replace("Temperature", "Mode"));
    dir.write("pid.dat", "3\t25\t42\t7\t0\t89000\tMode\t\t-1\t10");
    dir.write("ccf.dat", "SET_STATE\tMode\t\t\t\tHEADER");
    let all = run(&dir, &["search", "MoDe", "--scope", "all"]);
    assert!(all.status.success(), "{:?}", all);
    assert!(all.stderr.is_empty());
    let text = String::from_utf8(all.stdout.clone()).unwrap();
    assert!(text.starts_with("Kind"));
    assert_eq!(text.lines().count(), 4);
    for (scope, kind, identity) in [
        ("parameters", "parameter", "TEMP"),
        ("packets", "packet", "89000"),
        ("commands", "command", "SET_STATE"),
    ] {
        let output = run(&dir, &["search", "mode", "--scope", scope]);
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert!(text.contains(kind) && text.contains(identity));
        assert!(run(&dir, &[kind, identity]).status.success());
    }
    assert_eq!(all.stdout, run(&dir, &["search", "mode"]).stdout);
    assert_eq!(
        all.stdout,
        run(&dir, &["--details", "search", "mode"]).stdout
    );
    let debug = run(&dir, &["search", "mode", "--debug"]);
    assert_eq!(all.stdout, debug.stdout);
    assert!(!debug.stderr.is_empty());
    for query in ["", " \t ", "zzzzzzzz"] {
        let empty = run(&dir, &["search", query]);
        assert!(empty.status.success());
        assert!(empty.stderr.is_empty());
        assert_eq!(String::from_utf8(empty.stdout).unwrap().lines().count(), 1);
    }
    dir.write("pcf.dat", &format!("{PARAMETER}\n{PARAMETER}"));
    assert_eq!(
        run(&dir, &["search", "temp"]).stdout,
        run(&dir, &["parameter", "TEMP"]).stdout
    );
    for args in [vec!["search"], vec!["search", "mode", "--scope", "invalid"]] {
        assert_eq!(run(&dir, &args).status.code(), Some(2));
    }
}

#[test]
fn search_finds_a_packet_by_every_recorded_tpcf_name() {
    let dir = Fixture::new();
    dir.write("pid.dat", "3\t25\t42\t0\t0\t100\tPacket\t\t-1\t10");
    dir.write("tpcf.dat", "100\tUNIQUE_ALPHA\n100\tUNIQUE_BETA");
    for query in ["UNIQUE", "unique_alpha", "Unique_Beta"] {
        let output = run(&dir, &["search", query, "--scope", "packets"]);
        assert!(output.status.success(), "{query}");
        assert!(output.stderr.is_empty(), "{query}");
        assert_eq!(output.stdout, run(&dir, &["search", query]).stdout);
        let text = String::from_utf8(output.stdout.clone()).unwrap();
        assert_eq!(text.lines().count(), 2, "{query}: {text}");
        assert!(
            text.contains("packet") && text.contains("100"),
            "{query}: {text}"
        );
        // Every recorded name is searchable but none is chosen for the table.
        assert!(text.contains("unavailable"), "{query}: {text}");
        assert!(!text.contains("UNIQUE_"), "{query}: {text}");
    }
    // The returned identity stays usable: exact lookup succeeds and keeps the ambiguity
    // and both recorded definitions visible.
    let exact = run(&dir, &["packet", "100"]);
    assert!(exact.status.success());
    let details = run(&dir, &["--details", "packet", "100"]);
    assert!(details.status.success());
    let text = String::from_utf8(details.stdout).unwrap();
    assert!(text.contains("Packet 100  unavailable"), "{text}");
    assert!(
        text.contains("ambiguous reference; 2 TPCF candidates"),
        "{text}"
    );
    assert!(
        text.contains("UNIQUE_ALPHA") && text.contains("UNIQUE_BETA"),
        "{text}"
    );
    // Two identical TPCF names are still one candidate row for the one PID root.
    dir.write("tpcf.dat", "100\tDOUBLE\n100\tDOUBLE");
    for query in ["double", "DOUBLE"] {
        let output = run(&dir, &["search", query, "--scope", "packets"]);
        assert!(output.status.success(), "{query}");
        let text = String::from_utf8(output.stdout).unwrap();
        assert_eq!(text.lines().count(), 2, "{query}: {text}");
        assert!(text.contains("unavailable"), "{query}: {text}");
        assert!(!text.contains("DOUBLE"), "{query}: {text}");
    }
}

#[test]
fn search_and_ambiguous_lookups_share_pus_coordinates() {
    let dir = Fixture::new();
    dir.write("pcf.dat", &format!("{PARAMETER}\n{PARAMETER}"));
    let packet = "3\t25\t42\t0\t0\t100\tStatus\t\t-1\t10";
    dir.write("pid.dat", &format!("{packet}\n{packet}"));
    let command = "REPORT\tStatus\t\t\t\tHEADER\t3";
    dir.write("ccf.dat", &format!("{command}\n{command}"));
    for (kind, identity, pus) in [
        ("parameter", "TEMP", "-"),
        ("packet", "100", "TM(3,25)"),
        ("command", "REPORT", "TC(3,-)"),
    ] {
        let ambiguous = run(&dir, &[kind, identity]);
        assert_eq!(ambiguous.status.code(), Some(3));
        let search = run(&dir, &["search", identity]);
        assert!(search.status.success());
        assert_eq!(ambiguous.stdout, search.stdout);
        let text = String::from_utf8(search.stdout).unwrap();
        assert_eq!(
            text.lines().next().unwrap().split_whitespace().nth(2),
            Some("PUS")
        );
        for row in text.lines().skip(1) {
            assert_eq!(row.split_whitespace().nth(2), Some(pus));
        }
    }
    let visual_only = run(&dir, &["search", "TM(3,25)"]);
    assert_eq!(
        String::from_utf8(visual_only.stdout)
            .unwrap()
            .lines()
            .count(),
        1
    );
    dir.write("ccf.dat", "NO_SERVICE\tStatus\t\t\t\tHEADER\t\t25");
    let output = run(&dir, &["search", "NO_SERVICE"]);
    let text = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        text.lines().nth(1).unwrap().split_whitespace().nth(2),
        Some("-")
    );
}
