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
    assert_eq!(ambiguous.status.code(), Some(3));
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
            .contains("Usage:")
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
    assert_eq!(output.status.code(), Some(3));
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
fn overview_is_default_and_details_and_debug_are_independent_global_flags() {
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
        vec!["parameter", "TEMP", "--details"],
        vec!["parameter", "--details", "TEMP"],
        vec!["--details", "parameter", "TEMP", "--details"],
    ] {
        let output = run(&dir, &args);
        assert!(output.status.success(), "{args:?}");
        assert_eq!(output.stdout, details.stdout);
        assert!(output.stderr.is_empty());
    }
    for args in [
        vec!["--debug", "--debug", "parameter", "TEMP"],
        vec!["parameter", "--debug", "TEMP", "--debug"],
        vec!["--debug", "parameter", "TEMP", "--debug"],
    ] {
        let output = run(&dir, &args);
        assert!(output.status.success(), "{args:?}");
        assert_eq!(output.stdout, overview.stdout);
        assert!(!output.stderr.is_empty());
    }
    for args in [
        vec!["--overview", "parameter", "TEMP"],
        vec!["-d", "parameter", "TEMP"],
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

#[test]
fn command_overview_and_details_show_arguments_fixed_areas_and_runtime_evidence() {
    let dir = Fixture::new();
    dir.write(
        "ccf.dat",
        "DEMO_TC\tDemonstration command\tLong command description\t\t\tHEADER",
    );
    dir.write(
        "cdf.dat",
        "DEMO_TC\tE\t\t16\t0\t0\tARG\tT\t9\tTEMP\nDEMO_TC\tA\tPadding\t4\t8\t0\t\t\tF",
    );
    dir.write(
        "cpc.dat",
        "ARG\tArgument description\t3\t4\t\t\tV\t\t\t\t\t\t17\t\t\tB",
    );
    dir.write("pcf.dat", PARAMETER);
    for prefix in [vec![], vec!["--details"]] {
        let mut args = prefix.clone();
        args.extend(["command", "DEMO_TC"]);
        let output = run(&dir, &args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).unwrap();
        for expected in [
            "Command DEMO_TC",
            "Demonstration command",
            "ccf.dat:1",
            "ARG",
            "Argument description",
            "application-declared bit 0",
            "8 bits",
            "unsigned integer",
            "\"17\" [R [default]]",
            "telemetry TEMP",
            "Fixed area",
            "Padding",
            "4 bits",
            "inconsistent definition",
            "runtime dependent",
        ] {
            assert!(text.contains(expected), "missing {expected}: {text}");
        }
        if prefix.is_empty() {
            assert!(!text.contains("CCF_CNAME"));
            assert!(text.contains("Use --details"));
        } else {
            for expected in [
                "CCF_DESCR2",
                "Long command description",
                "CDF_VALUE",
                "CPC_PNAME",
                "CPC_NAME",
                "CPC_OBTID",
                "CPC_OBTIP",
                "CPC_ENDIAN",
                "CPC_DESCR2",
                "pcf.dat:1",
                "PCF_NAME",
                "Recorded: omitted",
                "Recorded: empty",
                "documented default",
            ] {
                assert!(text.contains(expected), "missing {expected}: {text}");
            }
            assert!(!text.contains("Use --details"));
        }
    }
}

#[test]
fn command_misses_are_silent_duplicates_are_candidates_and_debug_explains_loading() {
    let dir = Fixture::new();
    dir.write("cpc.dat", "ARG\tArgument\t3\t4");
    let unavailable = run(&dir, &["command", "DEMO_TC"]);
    assert_eq!(unavailable.status.code(), Some(1));
    assert!(unavailable.stdout.is_empty() && unavailable.stderr.is_empty());
    let row = "DEMO_TC\tCommand\t\t\t\tHEADER";
    dir.write("ccf.dat", &format!("bad\n{row}\n{row}"));
    let missing = run(&dir, &["command", "demo_tc"]);
    assert_eq!(missing.status.code(), Some(1));
    assert!(missing.stdout.is_empty() && missing.stderr.is_empty());
    let output = run(&dir, &["command", "DEMO_TC"]);
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.starts_with("Kind") && text.contains("ccf.dat:2") && text.contains("ccf.dat:3"));
    let output = run(&dir, &["--debug", "command", "missing"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let text = String::from_utf8(output.stderr).unwrap();
    for expected in [
        "dropped row",
        "ccf.dat",
        "exact command lookup",
        "NoMatchingIdentity",
    ] {
        assert!(text.contains(expected), "{text}");
    }
}

#[test]
fn command_groups_render_nested_blocks_beside_declared_repetition_sources() {
    let dir = Fixture::new();
    dir.write("ccf.dat", "DEMO_TC\tTC(14,1)\t\t\t\tHEADER");
    dir.write(
        "cdf.dat",
        "DEMO_TC\tF\tN1\t8\t0\t3\tN1\tR\t1\nDEMO_TC\tE\tAPID\t8\t8\t\tAPID\tR\nDEMO_TC\tF\tN2\t8\t16\t1\tN2\tR\t1\nDEMO_TC\tE\tType\t8\t24\t\tTYPE\tR",
    );
    dir.write(
        "cpc.dat",
        "N1\tGroup count\t3\t4\nAPID\tApplication id\t3\t4\nN2\tType count\t3\t4\nTYPE\tType\t3\t4",
    );
    for prefix in [vec![], vec!["--details"]] {
        let mut args = prefix.clone();
        args.extend(["command", "DEMO_TC"]);
        let output = run(&dir, &args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).unwrap();
        for expected in [
            "Application data",
            "Repeat group",
            "End repeat",
            "runtime CDF_GRPSIZE=3 repeats the following 3 declared element(s) using the recorded value 1 of N1",
            "runtime CDF_GRPSIZE=1 repeats the following 1 declared element(s) using the recorded value 1 of N2",
            "cdf.dat:1",
            "application-declared bit 8",
            "runtime dependent",
        ] {
            assert!(text.contains(expected), "missing {expected}: {text}");
        }
        // Group membership stays visible by nesting, and a nested layout is not a flat table.
        assert!(
            text.lines().any(|line| line.starts_with("Repeat group")),
            "{text}"
        );
        assert!(
            text.lines().any(|line| line.starts_with("  APID")),
            "{text}"
        );
        assert!(
            text.lines().any(|line| line.starts_with("  Repeat group")),
            "{text}"
        );
        assert!(
            text.lines().any(|line| line.starts_with("    TYPE")),
            "{text}"
        );
        assert!(
            text.lines().any(|line| line.starts_with("  End repeat")),
            "{text}"
        );
        assert!(!text.contains("Element value"), "{text}");
        if prefix.is_empty() {
            assert!(!text.contains("Declared CDF_BIT"), "{text}");
            assert!(text.contains("Use --details"), "{text}");
        } else {
            assert!(
                text.contains(
                    "Declared CDF_BIT is the unexpanded application-data position and shifts with the 3 declared element(s) this group repeats"
                ),
                "{text}"
            );
            assert!(text.contains("Recorded: text \"3\""), "{text}");
        }
    }
    // A command without declared groups keeps the aligned table.
    dir.write("cdf.dat", "DEMO_TC\tF\tARG\t8\t0\t0\tN1\tR\t1");
    let text = String::from_utf8(run(&dir, &["command", "DEMO_TC"]).stdout).unwrap();
    assert!(
        text.contains("Element value") && !text.contains("Repeat group"),
        "{text}"
    );
}

#[test]
fn command_group_problems_and_fixed_areas_render_beside_usable_elements() {
    let dir = Fixture::new();
    dir.write("ccf.dat", "DEMO_TC\tDemonstration command\t\t\t\tHEADER");
    // The repeater is a fixed area, so its group declares no count source and outlives its members.
    dir.write(
        "cdf.dat",
        "DEMO_TC\tA\tPadding\t4\t0\t3\t\t\tF\nDEMO_TC\tE\t\t8\t4\t0\tARG\tR\nDEMO_TC\tA\tTrailer\t4\t12\t0\t\t\t2",
    );
    dir.write("cpc.dat", "ARG\tArgument\t3\t4");
    let output = run(&dir, &["command", "DEMO_TC"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "Repeat group",
        "End repeat",
        "inconsistent definition",
        "cannot supply a repetition count",
        "more elements than its group retains",
        "cdf.dat:1",
        "application-declared bit 4",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    // A group with a contradictory declaration reports no usable repetition.
    assert!(
        text.lines()
            .any(|line| line.starts_with("Repeat group") && line.contains("unavailable")),
        "{text}"
    );
    // Fixed areas and arguments stay usable inside the incomplete group.
    assert!(text.lines().any(|line| line.starts_with("  ARG")), "{text}");
    assert!(
        text.lines().any(|line| line.starts_with("  Fixed area")),
        "{text}"
    );
}

#[test]
fn help_and_version_work_without_a_mib_directory() {
    for args in [
        vec!["--help"],
        vec!["parameter", "--help"],
        vec!["packet", "--help"],
        vec!["command", "--help"],
        vec!["--version"],
    ] {
        for directory in [None, Some("")] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_mibl"));
            command.env_remove("MIB_DIR").args(&args);
            if let Some(directory) = directory {
                command.env("MIB_DIR", directory);
            }
            let output = command.output().unwrap();
            assert!(output.status.success(), "{args:?}");
            assert!(output.stderr.is_empty());
            let text = String::from_utf8(output.stdout).unwrap();
            if args == ["--version"] {
                assert_eq!(text, format!("mibl {}\n", env!("CARGO_PKG_VERSION")));
            } else {
                assert!(
                    text.contains("Usage:")
                        && text.contains("--debug")
                        && text.contains("--details"),
                    "{text}"
                );
            }
        }
    }
}

#[test]
fn invalid_arguments_are_errors_and_subcommand_typos_have_suggestions() {
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    for args in [
        vec![],
        vec!["parameter", ""],
        vec!["command", ""],
        vec!["parameter", "TEMP", "extra"],
        vec!["packet", ""],
        vec!["packet", "１"],
        vec!["packet", "1.0"],
        vec!["packet", " 1"],
        vec!["packet", "1 "],
        vec!["packet", "--", "-1"],
        vec!["packet", "+1"],
        vec!["packet", "18446744073709551616"],
    ] {
        let output = run(&dir, &args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty() && !output.stderr.is_empty());
    }
    for spid in ["0", "000", "18446744073709551615"] {
        let output = run(&dir, &["packet", spid]);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty() && output.stderr.is_empty());
    }
    let output = run(&dir, &["paramete", "TEMP"]);
    assert_eq!(output.status.code(), Some(2));
    let text = String::from_utf8(output.stderr).unwrap();
    assert!(
        text.contains("similar subcommand") && text.contains("parameter"),
        "{text}"
    );
}

#[cfg(unix)]
#[test]
fn non_unicode_lookup_arguments_are_rejected() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    for verb in ["parameter", "packet", "command"] {
        let output = Command::new(env!("CARGO_BIN_EXE_mibl"))
            .env("MIB_DIR", dir.path())
            .arg(verb)
            .arg(OsString::from_vec(vec![0xff]))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty() && !output.stderr.is_empty());
    }
}

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
fn variable_packet_cli_shows_group_boundaries_and_runtime_dependencies() {
    let dir = Fixture::new();
    dir.write(
        "pcf.dat",
        "COUNT\tCounter\t\t\t3\t4\t8\t\t\tN\tR\nVALUE\tValue\t\t\t3\t4\t8\t\t\tN\tR",
    );
    dir.write("pid.dat", "3\t25\t42\t0\t0\t100\tVariable\t\t7\t10");
    dir.write(
        "vpd.dat",
        "7\t1\tCOUNT\t2\t0\tN\tN\t\t0\n7\t2\tCOUNT\t1\t2\tN\tN\t\t0\n7\t3\tVALUE\t0\t0\tN\tN\t\t1",
    );
    for args in [
        vec!["--details", "packet", "100"],
        vec!["--details", "parameter", "VALUE"],
    ] {
        let output = run(&dir, &args);
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).unwrap();
        for expected in [
            "VALUE",
            "8 bits",
            "runtime",
            "fixed count 2",
            "VPD_GRPSIZE",
            "vpd.dat:3",
            "COUNT",
        ] {
            assert!(text.contains(expected), "missing {expected}: {text}");
        }
        if args[1] == "packet" {
            assert!(text.contains("Repeat group"));
            assert!(text.contains("End repeat"));
            assert!(text.lines().any(|line| line.starts_with("  Repeat group")));
            assert!(text.lines().any(|line| line.starts_with("    VALUE")));
        }
    }
}

#[test]
fn pus_service_lists_packet_and_command_definitions() {
    let dir = Fixture::new();
    dir.write("pid.dat", "3\t25\t42\t0\t0\t100\tStatus\t\t-1\t10");
    dir.write("ccf.dat", "REPORT\tStatus\t\t\t\tHEADER\t3\t25");
    let output = run(&dir, &["pus", "3"]);
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        text.lines()
            .next()
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>(),
        ["Kind", "Identity", "PUS", "Name", "Description", "Source"]
    );
    assert!(
        text.contains("TM(3,25)") && text.contains("TC(3,25)"),
        "{text}"
    );
    assert_eq!(text.lines().count(), 3);
}

#[test]
fn pus_filters_subtypes_and_sorts_missing_subtypes_last() {
    let dir = Fixture::new();
    dir.write("pid.dat", "3\t25\t42\t0\t0\t100\tStatus\t\t-1\t10\n3\t25\t42\t0\t0\t9\tStatus\t\t-1\t10\n3\t1\t42\t0\t0\t200\tStatus\t\t-1\t10\n4\t25\t42\t0\t0\t300\tOther\t\t-1\t10");
    dir.write("ccf.dat", "Z_CMD\tStatus\t\t\t\tHEADER\t3\t25\nA_CMD\tStatus\t\t\t\tHEADER\t3\t25\nMISSING\tStatus\t\t\t\tHEADER\t3\nEARLY\tStatus\t\t\t\tHEADER\t3\t1\nNO_SERVICE\tStatus\t\t\t\tHEADER");
    for (query, expected) in [
        (
            "3",
            vec!["200", "EARLY", "9", "100", "A_CMD", "Z_CMD", "MISSING"],
        ),
        ("3,25", vec!["9", "100", "A_CMD", "Z_CMD"]),
        ("4", vec!["300"]),
        ("99", vec![]),
        ("3,99", vec![]),
    ] {
        let output = run(&dir, &["pus", query]);
        assert!(output.status.success(), "{output:?}");
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout.clone()).unwrap();
        let identities: Vec<_> = text
            .lines()
            .skip(1)
            .map(|line| line.split_whitespace().nth(1).unwrap())
            .collect();
        assert_eq!(identities, expected, "{text}");
        assert_eq!(
            text.lines()
                .next()
                .unwrap()
                .split_whitespace()
                .collect::<Vec<_>>(),
            ["Kind", "Identity", "PUS", "Name", "Description", "Source"]
        );
        for args in [
            vec!["--details", "--debug", "pus", query],
            vec!["pus", "--details", "--debug", query],
            vec!["pus", query, "--details", "--debug"],
        ] {
            let flagged = run(&dir, &args);
            assert!(flagged.status.success());
            assert_eq!(output.stdout, flagged.stdout);
        }
    }
    assert!(
        String::from_utf8(run(&dir, &["pus", "3"]).stdout)
            .unwrap()
            .contains("TC(3,-)")
    );
}

#[test]
fn pus_rejects_malformed_unsigned_coordinates() {
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    for value in [
        "", "-3", "3,-25", "+3", "3,+25", "3,abc", "3,25,1", "3,", ",25", "3 25", "3:25", " 3",
        "3, 25", "65536", "3,65536", "３",
    ] {
        let output = run(&dir, &["pus", value]);
        assert_eq!(output.status.code(), Some(2), "{value}: {output:?}");
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "mibl: PUS argument must be <SERVICE> or <SERVICE,SUBTYPE> as unsigned integers\n",
            "{value}"
        );
    }
    for value in ["0", "0,0", "65535", "65535,65535"] {
        assert!(run(&dir, &["pus", value]).status.success(), "{value}");
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

#[test]
fn pus_packet_candidates_show_unavailable_unsigned_coordinates() {
    let dir = Fixture::new();
    dir.write(
        "pid.dat",
        "3\t-1\t42\t0\t0\t100\tStatus\t\t-1\t10\n-1\t25\t42\t0\t0\t200\tStatus\t\t-1\t10",
    );
    let output = run(&dir, &["pus", "3"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert_eq!(text.lines().count(), 2);
    assert!(text.contains("TM(3,-)"), "{text}");
    let exact_subtype = run(&dir, &["pus", "3,25"]);
    assert!(exact_subtype.status.success());
    assert_eq!(
        String::from_utf8(exact_subtype.stdout)
            .unwrap()
            .lines()
            .count(),
        1
    );
    let search = run(&dir, &["search", "200"]);
    let text = String::from_utf8(search.stdout).unwrap();
    assert_eq!(
        text.lines().nth(1).unwrap().split_whitespace().nth(2),
        Some("-")
    );
}

#[test]
fn command_header_section_expands_the_packet_header_separately_from_application_data() {
    let dir = Fixture::new();
    dir.write("ccf.dat", "DEMO_TC\tDemonstration command\t\t\t\tHDR");
    dir.write("cdf.dat", "DEMO_TC\tE\t\t8\t0\t0\tARG\tR");
    dir.write("cpc.dat", "ARG\tArgument\t3\t4");
    dir.write("tcp.dat", "HDR\tDemonstration header");
    dir.write(
        "pcdf.dat",
        &[
            "HDR\tTrailer\tF\t8\t16\t\t0A\tD",
            "HDR\tVersion Number\tF\t3\t0\t\t5\tH",
            "HDR\tCount\tP\t8\t8\tP007\t17\t",
        ]
        .join("\n"),
    );
    dir.write("pcpc.dat", "P007\tInteger sequence count\tI");
    for prefix in [vec![], vec!["--details"]] {
        let mut args = prefix.clone();
        args.extend(["command", "DEMO_TC"]);
        let output = run(&dir, &args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).unwrap();
        for expected in [
            "Header\nTCP HDR  Demonstration header\nSource: tcp.dat:1",
            "Field",
            "Parameter",
            "Kind",
            "Value",
            "Version Number",
            "header bit 0",
            "3 bits",
            "F fixed",
            "0 [H]",
            "Trailer",
            "header bit 16",
            "10 [H]",
            "Count",
            "P007",
            "header bit 8",
            "P set automatically at invocation",
            "17 [D] (declared default)",
        ] {
            assert!(text.contains(expected), "missing {expected}: {text}");
        }
        // Declared header offsets order the elements, and the section follows the arguments.
        let header = text.find("\nHeader\n").unwrap();
        assert!(text.find("\nApplication data\n").unwrap() < header);
        assert!(text.find("\nArgument rules\n").unwrap() < header);
        let offset = |name: &str| text.find(name).unwrap();
        assert!(offset("Version Number") < offset("Count") && offset("Count") < offset("Trailer"));
        if prefix.is_empty() {
            assert!(!text.contains("PCDF_TYPE"), "{text}");
            assert!(!text.contains("PCPC_CODE"), "{text}");
            assert!(text.contains("Use --details"), "{text}");
        } else {
            for expected in [
                "TCP_ID",
                "TCP_DESC",
                "PCDF_TCNAME",
                "PCDF_TYPE",
                "PCDF_LEN",
                "PCDF_BIT",
                "PCDF_PNAME",
                "PCDF_VALUE",
                "PCDF_RADIX",
                "PCPC_PNAME",
                "PCPC_CODE",
                "Used by: Command header",
                "Used by: Header element Version Number at header bit 0",
            ] {
                assert!(text.contains(expected), "missing {expected}: {text}");
            }
        }
    }
}

#[test]
fn command_header_problems_render_beside_the_usable_arguments() {
    let dir = Fixture::new();
    dir.write("ccf.dat", "DEMO_TC\tDemonstration command\t\t\t\tHDR");
    dir.write("cdf.dat", "DEMO_TC\tE\t\t8\t0\t0\tARG\tR");
    dir.write("cpc.dat", "ARG\tArgument\t3\t4");
    // The commanded packet header is declared nowhere.
    let output = run(&dir, &["command", "DEMO_TC"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Application data"), "{text}");
    assert!(
        text.contains("Header\nunavailable") && text.contains("missing reference"),
        "{text}"
    );
    assert!(text.contains("no matching TCP definition"), "{text}");
    // A declared header keeps every element and reports the unresolved links.
    dir.write("tcp.dat", "HDR\tDemonstration header");
    dir.write(
        "pcdf.dat",
        "HDR\tAPID\tA\t11\t5\tP002\t1\tH\nHDR\tGhost\tA\t8\t16\tP404\t1\tH",
    );
    dir.write("pcpc.dat", "P002\tFirst APID\tU\nP002\tSecond APID\tU");
    let output = run(&dir, &["command", "DEMO_TC"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "Header",
        "Ghost",
        "P002",
        "P404",
        "A APID from CCF_APID",
        "ambiguous reference; 2 PCPC candidates.",
        "missing reference; no matching PCPC definition.",
        "unavailable",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    // Fixed areas below a duplicate offset keep their content beside the disagreement.
    dir.write(
        "pcdf.dat",
        "HDR\tFirst at zero\tF\t3\t0\t\t5\tH\nHDR\tSecond at zero\tF\t3\t0\t\t1\tH",
    );
    let output = run(&dir, &["command", "DEMO_TC"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("inconsistent definition"), "{text}");
    assert!(
        text.contains("Duplicate declared packet header element offsets"),
        "{text}"
    );
    assert!(text.contains("Second at zero"), "{text}");
}

#[test]
fn command_header_recorded_text_keeps_controls_escaped_and_columns_aligned() {
    let dir = Fixture::new();
    dir.write("ccf.dat", "DEMO_TC\tDemonstration command\t\t\t\tHDR");
    dir.write("cdf.dat", "DEMO_TC\tE\t\t8\t0\t0\tARG\tR");
    dir.write("cpc.dat", "ARG\tArgument\t3\t4");
    dir.write("tcp.dat", "HDR\tNominal\u{7} header");
    dir.write("pcdf.dat", "HDR\tPkt Version\u{b}Number\tF\t3\t0\t\t5\tH");
    dir.write("pcpc.dat", "P001\tAck Flags\tU");
    let output = run(&dir, &["--details", "command", "DEMO_TC"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    for expected in ["Nominal\\u{7} header", "Pkt Version\\u{b}Number"] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    assert!(!text.contains('\u{7}') && !text.contains('\u{b}'), "{text}");
    assert!(!text.contains('\t'), "{text}");
}
