use crate::common::{Fixture, PARAMETER};
use crate::support::{accepted_fixture, run};
use std::process::Command;

#[test]
fn both_absence_reasons_are_silent_and_ambiguity_shows_all_sources() {
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    let missing = run(&dir, &["parameter", "temp"]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(missing.stdout.is_empty() && missing.stderr.is_empty());
    dir.write("pcf.dat", "bad row");
    dir.write("caf.dat", "CURVE\tCurve\tR\tU");
    let unavailable = run(&dir, &["parameter", "TEMP"]);
    assert_eq!(unavailable.status.code(), Some(1));
    assert!(unavailable.stdout.is_empty() && unavailable.stderr.is_empty());
    dir.write("pcf.dat", &format!("{PARAMETER}\n{PARAMETER}"));
    let ambiguous = run(&dir, &["parameter", "TEMP"]);
    assert_eq!(ambiguous.status.code(), Some(3));
    assert!(ambiguous.stderr.is_empty());
    let text = String::from_utf8(ambiguous.stdout).unwrap();
    assert!(text.starts_with("Kind") && text.lines().next().unwrap().contains("Source"));
    assert!(!text.contains('\t'));
    assert!(text.contains("pcf.dat:1") && text.contains("pcf.dat:2"));
    assert_eq!(text.lines().count(), 3);
}

#[test]
fn debug_explains_dropped_rows_skipped_files_extras_and_query_decisions() {
    let dir = Fixture::new();
    let full = format!("{PARAMETER}{}", "\t".repeat(14));
    dir.write("pcf.dat", &format!("bad row\n{full}\n"));
    let output = run(&dir, &["--debug", "parameter", "missing"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let text = String::from_utf8(output.stderr).unwrap();
    for expected in [
        "dropped row",
        "pcf.dat",
        "line=1",
        "PCF_PTC is required",
        "original=bad row",
        "skipped file",
        "caf.dat",
        "ignored extra columns",
        "exact parameter lookup",
        "NoMatchingIdentity",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
}

#[test]
fn accepted_normal_overviews_are_compact_and_aligned() {
    let dir = accepted_fixture();
    let output = run(&dir, &["parameter", "DEMO_TEMP"]);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        concat!(
            "Parameter DEMO_TEMP\nDescription: Demonstration temperature\n",
            "Encoding: unsigned integer, 16 bits, big endian (PTC 3 / PFC 12)\nUnits: K\nSource: pcf.dat:1\n\n",
            "Packets\nSPID   Name     Location       Width    Repeat\n",
            "42001  DEMO_HK  byte 16 bit 0  16 bits  once\n\nCalibrations\nnone declared\n\nProblems\nnone\n",
            "Use --details for recorded fields and problem evidence.\n"
        )
    );
    let output = run(&dir, &["packet", "42001"]);
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        concat!(
            "Packet 42001  DEMO_HK\nDescription: Demonstration housekeeping\nSource: pid.dat:1\n\n",
            "Identification\nAPID: 42\nService: type 3, subtype 25\nAdditional criteria: none declared\n\n",
            "Layout\nParameter  Location       Width    Repeat\nDEMO_TEMP  byte 16 bit 0  16 bits  once\n\n",
            "Problems\nnone\n",
            "Use --details for recorded fields and problem evidence.\n"
        )
    );
}

#[test]
fn conflicting_packet_evidence_retains_all_candidates_and_reuses_definitions() {
    let dir = accepted_fixture();
    let overview = run(&dir, &["packet", "42002"]);
    let text = String::from_utf8(overview.stdout.clone()).unwrap();
    assert!(
        text.contains("PI1 expected: 7; location: byte 10 bit 0; width: unavailable [P2]"),
        "{text}"
    );
    assert!(text.contains("1/2, stride 16 bits") && text.contains("2/2, stride 16 bits"));
    assert!(
        text.contains("[P6] Identification criteria: ambiguous reference; 2 PIC candidates."),
        "{text}"
    );
    let output = run(&dir, &["--details", "packet", "42002"]);
    assert!(output.status.success() && output.stderr.is_empty());
    let details = String::from_utf8(output.stdout).unwrap();
    assert!(details.starts_with(text.split("Use --details").next().unwrap()));
    let definitions = details.split("\nDefinitions\n").nth(1).unwrap();
    for (id, src) in [
        (1, "pid.dat:2"),
        (2, "tpcf.dat:2"),
        (3, "tpcf.dat:3"),
        (4, "pic.dat:2"),
        (5, "pic.dat:3"),
        (6, "pcf.dat:2"),
        (7, "plf.dat:2"),
        (8, "plf.dat:3"),
        (9, "plf.dat:4"),
        (10, "pcf.dat:3"),
        (11, "pcf.dat:4"),
    ] {
        assert_eq!(
            definitions.matches(&format!("[D{id}] {src}\n")).count(),
            1,
            "{definitions}"
        );
    }
    assert_eq!(
        definitions.lines().filter(|l| l.starts_with("[D")).count(),
        11
    );
    for expected in [
        "DEMO_HK_A",
        "DEMO_HK_B",
        "First duplicate",
        "Second duplicate",
        "PIC_PI1_WID",
        "Values: 8, 16",
        "PCF_DESCR2",
        "PLF_TIME",
        "Interpreted: 0, documented default",
        "Rule: PLF_TIME defaults to",
        "Recorded: empty",
        "Recorded: omitted",
        "Description: Demonstration mode",
        "Units: unavailable",
    ] {
        assert!(details.contains(expected), "missing {expected}: {details}");
    }
    assert!(
        definitions.contains("Used by: DEMO_MODE at byte 20 bit 0")
            && definitions.contains("Used by: DEMO_MODE at byte 22 bit 0")
    );
    let output = run(&dir, &["--details", "parameter", "DEMO_MODE"]);
    let details = String::from_utf8(output.stdout).unwrap();
    assert!(details.contains("DEMO_HK_A") && details.contains("DEMO_HK_B"));
    assert!(
        !details.contains("DEMO_UNKNOWN")
            && !details.contains("DEMO_DUP")
            && !details.contains("pic.dat")
    );
}

#[test]
fn long_unicode_text_and_controls_survive_both_modes_and_redirection() {
    use std::{fs::File, process::Stdio};
    let dir = Fixture::new();
    let name = format!("界{}e\u{301}", "長".repeat(105));
    let description = format!("{}é 界\u{1b}[31m\rX\u{7}\\\"", "Description ".repeat(24));
    dir.write(
        "pcf.dat",
        &format!("{name}\t{description}\t\tK\t3\t4\t99\t\t\tN\tR"),
    );
    for prefix in [vec![], vec!["--details"]] {
        let mut args = prefix;
        args.extend(["parameter", &name]);
        let output = run(&dir, &args);
        assert!(output.status.success() && output.stderr.is_empty());
        let text = String::from_utf8(output.stdout.clone()).unwrap();
        assert!(text.contains(&name) && text.contains(&"Description ".repeat(24)));
        assert!(text.contains("é 界\\u{1b}[31m\\rX\\u{7}"), "{text}");
        assert!(!text.chars().any(|c| c.is_control() && c != '\n'));
        let path = dir.path().join("redirected.txt");
        let status = Command::new(env!("CARGO_BIN_EXE_mibl"))
            .env("MIB_DIR", dir.path())
            .args(&args)
            .stdout(Stdio::from(File::create(&path).unwrap()))
            .status()
            .unwrap();
        assert!(status.success());
        assert_eq!(std::fs::read(path).unwrap(), output.stdout);
    }
    // Different display widths, including a combining mark, must align the location column.
    dir.write(
        "pcf.dat",
        &format!("{name}\tLong\t\tK\t3\t4\t8\t\t\tN\tR\nAe\u{301}\tShort\t\tK\t3\t4\t8\t\t\tN\tR"),
    );
    dir.write("pid.dat", "3\t25\t42\t0\t0\t42\tPacket\t\t-1\t10");
    dir.write(
        "plf.dat",
        &format!("{name}\t42\t16\t0\t1\nAe\u{301}\t42\t17\t0\t1"),
    );
    let output = run(&dir, &["packet", "42"]);
    let text = String::from_utf8(output.stdout).unwrap();
    let long = text.lines().find(|l| l.starts_with(&name)).unwrap();
    let short = text.lines().find(|l| l.starts_with("Ae\u{301}")).unwrap();
    assert!(long.starts_with(&format!("{name}  byte")));
    assert!(short.starts_with(&format!("Ae\u{301}{}byte", " ".repeat(213))));
    // Root candidates share the same table formatter and keep full descriptions in either mode.
    let row = format!("{name}\t{description}\t\tK\t3\t4\t8\t\t\tN\tR");
    dir.write("pcf.dat", &format!("{row}\n{row}\n{row}"));
    let normal = run(&dir, &["parameter", &name]);
    let detailed = run(&dir, &["--details", "parameter", &name]);
    assert_eq!(normal.stdout, detailed.stdout);
    let text = String::from_utf8(normal.stdout).unwrap();
    assert_eq!(text.lines().count(), 4);
    assert_eq!(text.matches(&name).count(), 6);
    assert_eq!(text.matches(&"Description ".repeat(24)).count(), 3);
    assert!(text.contains("pcf.dat:3") && !text.contains('\t'));
}

#[test]
fn details_cover_every_column_of_every_reachable_definition_in_both_directions() {
    let dir = accepted_fixture();
    for (verb, identity, expected_sources) in [
        (
            "packet",
            "42001",
            vec![
                "pid.dat:1",
                "tpcf.dat:1",
                "pic.dat:1",
                "pcf.dat:1",
                "plf.dat:1",
            ],
        ),
        (
            "parameter",
            "DEMO_TEMP",
            vec!["pcf.dat:1", "pid.dat:1", "tpcf.dat:1", "plf.dat:1"],
        ),
        (
            "packet",
            "42002",
            vec![
                "pid.dat:2",
                "tpcf.dat:2",
                "tpcf.dat:3",
                "pic.dat:2",
                "pic.dat:3",
                "pcf.dat:2",
                "plf.dat:2",
                "plf.dat:3",
                "plf.dat:4",
                "pcf.dat:3",
                "pcf.dat:4",
            ],
        ),
        (
            "parameter",
            "DEMO_MODE",
            vec![
                "pcf.dat:2",
                "pid.dat:2",
                "tpcf.dat:2",
                "tpcf.dat:3",
                "plf.dat:2",
            ],
        ),
    ] {
        let output = run(&dir, &["--details", verb, identity]);
        let text = String::from_utf8(output.stdout).unwrap();
        let definitions = text.split("\nDefinitions\n").nth(1).unwrap();
        let blocks: Vec<_> = definitions.split("[D").skip(1).collect();
        assert_eq!(blocks.len(), expected_sources.len(), "{text}");
        for (block, expected_source) in blocks.iter().zip(expected_sources) {
            assert!(
                block.lines().next().unwrap().ends_with(expected_source),
                "{block}"
            );
            let count = match expected_source.split('.').next().unwrap() {
                "pcf" => 24,
                "pid" => 16,
                "pic" => 7,
                "plf" => 8,
                "tpcf" => 3,
                _ => unreachable!(),
            };
            let columns: Vec<_> = block
                .lines()
                .filter(|l| l.starts_with("  Column "))
                .collect();
            assert_eq!(columns.len(), count);
            for (index, column) in columns.iter().enumerate() {
                assert!(column.starts_with(&format!("  Column {}  ", index + 1)));
            }
            assert_eq!(block.matches("    Recorded: ").count(), count);
            assert_eq!(block.matches("    Interpreted: ").count(), count);
        }
    }
}
