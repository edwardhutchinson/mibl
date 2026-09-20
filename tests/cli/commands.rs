use crate::common::{Fixture, PARAMETER};
use crate::support::run;

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
