use crate::common::{Fixture, PARAMETER};
use crate::support::{accepted_fixture, run};

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
fn fixed_packet_layout_keeps_the_width_of_ambiguous_parameter_candidates() {
    let dir = Fixture::new();
    // Both DEMO candidates declare PTC 3 / PFC 4 and so establish 8 bits, while their
    // PCF_WIDTH padding declarations disagree.
    dir.write(
        "pcf.dat",
        "DEMO\tFirst\t\t\t3\t4\t4\t\t\tN\tR\nDEMO\tSecond\t\t\t3\t4\t99\t\t\tN\tR",
    );
    dir.write("pid.dat", "3\t25\t1\t0\t0\t42\tSynthetic\t\t-1\t0");
    dir.write("plf.dat", "DEMO\t42\t0\t0");
    dir.write("tpcf.dat", "42\tPACKET");
    dir.write("pic.dat", "3\t25\t-1\t0\t-1\t0");
    for details in [false, true] {
        let args = if details {
            vec!["--details", "packet", "42"]
        } else {
            vec!["packet", "42"]
        };
        let output = run(&dir, &args);
        assert!(output.status.success() && output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("DEMO       byte 0 bit 0  8 bits"), "{text}");
        assert!(
            text.contains("ambiguous reference; 2 PCF candidates"),
            "{text}"
        );
        if details {
            assert!(
                text.contains("Candidate: PCF NAME DEMO: pcf.dat:1")
                    && text.contains("Candidate: PCF NAME DEMO: pcf.dat:2"),
                "{text}"
            );
        }
    }
    // The duplicate exact parameter lookup keeps reporting both candidates.
    let ambiguous = run(&dir, &["parameter", "DEMO"]);
    assert_eq!(ambiguous.status.code(), Some(3));
    let text = String::from_utf8(ambiguous.stdout).unwrap();
    assert!(text.contains("pcf.dat:1") && text.contains("pcf.dat:2"));
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
