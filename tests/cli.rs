mod common;
use common::{Fixture, PARAMETER};
use std::process::{Command, Output};

fn run(dir: &Fixture, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mibl"))
        .env("MIB_DIR", dir.path())
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn parameter_details_show_fields_type_units_presence_defaults_and_source() {
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    let output = run(&dir, &["--details", "parameter", "TEMP"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "TEMP",
        "Temperature",
        "unsigned integer",
        "PTC 3",
        "PFC 4",
        "8 bits",
        "Units: K",
        "pcf.dat:1",
        "PCF_NAME",
        "PCF_DESCR2",
        "omitted",
        "empty",
        "default",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    assert!(text.find("PCF_NAME").unwrap() < text.find("PCF_DESCR2").unwrap());
}

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
    assert!(ambiguous.status.success());
    assert!(ambiguous.stderr.is_empty());
    let text = String::from_utf8(ambiguous.stdout).unwrap();
    assert!(text.starts_with("Kind") && text.lines().next().unwrap().contains("Source"));
    assert!(!text.contains('\t'));
    assert!(text.contains("pcf.dat:1") && text.contains("pcf.dat:2"));
    assert_eq!(text.lines().count(), 3);
}

#[test]
fn configuration_loading_and_argument_errors_are_visible_without_debug() {
    for value in [None, Some("")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_mibl"));
        command.env_remove("MIB_DIR").args(["parameter", "TEMP"]);
        if let Some(value) = value {
            command.env("MIB_DIR", value);
        }
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .contains("MIB_DIR")
        );
    }
    let dir = Fixture::new();
    let empty = run(&dir, &["parameter", "TEMP"]);
    assert_eq!(empty.status.code(), Some(2));
    assert!(
        String::from_utf8(empty.stderr)
            .unwrap()
            .contains("no usable supported rows")
    );
    let invalid = run(&dir, &["parameter"]);
    assert_eq!(invalid.status.code(), Some(2));
    assert!(
        String::from_utf8(invalid.stderr)
            .unwrap()
            .contains("usage:")
    );
    std::fs::remove_dir_all(dir.path()).unwrap();
    let inaccessible = run(&dir, &["parameter", "TEMP"]);
    assert_eq!(inaccessible.status.code(), Some(2));
    assert!(
        String::from_utf8(inaccessible.stderr)
            .unwrap()
            .contains("cannot read MIB directory")
    );
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

#[cfg(unix)]
#[test]
fn configuration_accepts_non_unicode_directory_paths() {
    use std::os::unix::ffi::OsStringExt;
    let dir = Fixture::new();
    let path = dir
        .path()
        .join(std::ffi::OsString::from_vec(vec![b'm', 0xff]));
    std::fs::create_dir(&path).unwrap();
    std::fs::write(path.join("pcf.dat"), PARAMETER).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mibl"))
        .env("MIB_DIR", path)
        .args(["parameter", "TEMP"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
}

#[test]
fn packet_details_and_parameter_containment_render_fixed_layout() {
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    dir.write("pid.dat", "3\t25\t42\t7\t0\t89000\tHousekeeping\t\t-1\t10");
    dir.write("tpcf.dat", "89000\tZUY_HK_00001\t32");
    dir.write("pic.dat", "3\t25\t16\t8\t-1\t0");
    dir.write("plf.dat", "TEMP\t89000\t19\t0\t2\t8");
    let output = run(&dir, &["--details", "packet", "089000"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "Packet 89000",
        "ZUY_HK_00001",
        "APID: 42",
        "PI1 expected: 7",
        "byte 16 bit 0",
        "byte 19 bit 0",
        "byte 20 bit 0",
        "8 bits",
        "Temperature",
        "TPCF_SIZE",
        "PIC_PI1_WID",
        "PLF_NBOCC",
        "PCF_PTC",
        "2/2, stride 8 bits",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    let output = run(&dir, &["parameter", "TEMP"]);
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("89000") && text.contains("byte 20 bit 0"));
    let output = run(&dir, &["packet", "99"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
}

#[test]
fn packet_absence_ambiguity_and_malformed_rows_follow_cli_policy() {
    let dir = Fixture::new();
    dir.write("tpcf.dat", "89000\tSynthetic");
    let output = run(&dir, &["packet", "89000"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    let row = "3\t25\t42\t0\t0\t89000\tHousekeeping\t\t-1\t10";
    dir.write("pid.dat", &format!("bad\n{row}\n{row}"));
    let output = run(&dir, &["packet", "89000"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.starts_with("Kind") && text.lines().next().unwrap().contains("Source"));
    assert!(text.contains("pid.dat:2") && text.contains("pid.dat:3"));
    let output = run(&dir, &["--debug", "packet", "99"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let text = String::from_utf8(output.stderr).unwrap();
    assert!(text.contains("dropped row") && text.contains("pid.dat") && text.contains("bad"));
    for invalid in ["-1", "+1", "no", "18446744073709551616"] {
        let output = run(&dir, &["packet", invalid]);
        assert_eq!(output.status.code(), Some(2));
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn packet_without_additional_criteria_still_renders_pic_fields() {
    let dir = Fixture::new();
    dir.write("pid.dat", "3\t25\t42\t0\t0\t89000\tHousekeeping\t\t-1\t10");
    dir.write("pic.dat", "3\t25\t-1\t0\t-1\t0");
    let output = run(&dir, &["--details", "packet", "89000"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("PIC_PI1_OFF") && text.contains("PIC_APID") && text.contains("pic.dat:1")
    );
    assert!(!text.contains("Expected:"));
}

#[test]
fn overview_is_default_and_details_and_debug_are_independent_prefix_flags() {
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    let overview = run(&dir, &["parameter", "TEMP"]);
    let text = String::from_utf8(overview.stdout.clone()).unwrap();
    assert!(
        text.starts_with(
            "Parameter TEMP\nDescription: Temperature\nEncoding: unsigned integer, 8 bits"
        ),
        "{text}"
    );
    assert!(!text.contains("PCF_NAME"));
    assert!(text.contains("Use --details for recorded fields and problem evidence."));
    let details = run(&dir, &["--details", "parameter", "TEMP"]);
    assert!(details.status.success());
    let text = String::from_utf8(details.stdout.clone()).unwrap();
    assert!(text.contains("\nProblem evidence\n") && text.contains("\nDefinitions\n"));
    assert!(text.contains("PCF_NAME"));
    assert!(!text.contains("Use --details"));
    for flags in [["--debug", "--details"], ["--details", "--debug"]] {
        let output = run(&dir, &[flags[0], flags[1], "parameter", "TEMP"]);
        assert!(output.status.success());
        assert_eq!(output.stdout, details.stdout);
        assert!(!output.stderr.is_empty());
    }
    for args in [
        vec!["--details", "--details", "parameter", "TEMP"],
        vec!["--debug", "--debug", "parameter", "TEMP"],
        vec!["--overview", "parameter", "TEMP"],
        vec!["-d", "parameter", "TEMP"],
        vec!["parameter", "TEMP", "--details"],
        vec!["parameter", "--details"],
    ] {
        let output = run(&dir, &args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

fn accepted_fixture() -> Fixture {
    let dir = Fixture::new();
    dir.write("pcf.dat", "DEMO_TEMP\tDemonstration temperature\t\tK\t3\t12\t16\t\t\tN\tR\t\t\t\t\t\t\t\t\t\t\t\tB\nDEMO_MODE\tDemonstration mode\t\t\t2\t8\t8\t\t\tS\tR\t\t\t\t\t\t\t\t\t\t\t\tB\nDEMO_DUP\tFirst duplicate\t\t\t3\t4\t8\t\t\tN\tR\nDEMO_DUP\tSecond duplicate\t\t\t3\t4\t8\t\t\tN\tR");
    dir.write("pid.dat", "3\t25\t42\t0\t0\t42001\tDemonstration housekeeping\t\t-1\t10\n3\t26\t42\t7\t0\t42002\tDemonstration packet with partial definitions\t\t-1\t10");
    dir.write(
        "tpcf.dat",
        "42001\tDEMO_HK\n42002\tDEMO_HK_A\n42002\tDEMO_HK_B",
    );
    dir.write(
        "pic.dat",
        "3\t25\t-1\t0\t-1\t0\n3\t26\t10\t8\t-1\t0\n3\t26\t10\t16\t-1\t0",
    );
    dir.write("plf.dat", "DEMO_TEMP\t42001\t16\t0\t1\nDEMO_MODE\t42002\t20\t0\t2\t16\nDEMO_UNKNOWN\t42002\t24\t0\t1\nDEMO_DUP\t42002\t25\t0\t1");
    dir
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
            "42001  DEMO_HK  byte 16 bit 0  16 bits  once\n\nProblems\n",
            "[P1] Calibration: unsupported interpretation; expansion is not implemented.\n",
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
            "Problems\n[P1] DEMO_TEMP calibration: unsupported interpretation; expansion is not implemented.\n",
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
fn pic_ambiguity_preserves_known_pi2_width_and_default_expected_value() {
    let dir = accepted_fixture();
    dir.write("pid.dat", "3\t25\t42\t\t\t42001\tHousekeeping\t\t-1\t10");
    dir.write("pic.dat", "3\t25\t-1\t0\t10\t8\n3\t25\t-1\t0\t10\t8");
    dir.write("tpcf.dat", "42001\tKnown name\t32");
    for details in [false, true] {
        let args = if details {
            vec!["--details", "packet", "42001"]
        } else {
            vec!["packet", "42001"]
        };
        let output = run(&dir, &args);
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("Packet 42001  Known name"));
        assert!(
            text.contains("PI2 expected: 0 [default]; location: byte 10 bit 0; width: 8 bits"),
            "{text}"
        );
        assert!(!text.contains("PI1 expected") && !text.contains("inconsistent definition"));
        assert!(text.contains("ambiguous reference; 2 PIC candidates"));
        if details {
            assert!(text.contains("pic.dat:1") && text.contains("pic.dat:2"));
        }
    }
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

#[test]
fn duplicate_positions_are_not_collapsed_and_unavailable_layout_is_not_empty() {
    let dir = accepted_fixture();
    dir.write(
        "plf.dat",
        "DEMO_TEMP\t42001\t16\t0\t2\t0\nDEMO_TEMP\t42001\t16\t0\t1",
    );
    for verb in ["packet", "parameter"] {
        let identity = if verb == "packet" {
            "42001"
        } else {
            "DEMO_TEMP"
        };
        let output = run(&dir, &[verb, identity]);
        let text = String::from_utf8(output.stdout).unwrap();
        let occurrences = text.split("\nProblems\n").next().unwrap();
        assert_eq!(occurrences.matches("byte 16 bit 0").count(), 3);
        assert!(text.contains("1/2, stride 0 bits") && text.contains("2/2, stride 0 bits"));
        assert!(text.contains("inconsistent definition"));
    }
    dir.write("plf.dat", "");
    let output = run(&dir, &["packet", "42001"]);
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("\nLayout\nnone\n")
    );
    std::fs::remove_file(dir.path().join("plf.dat")).unwrap();
    let output = run(&dir, &["packet", "42001"]);
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("\nLayout\nunavailable\n")
    );
}
