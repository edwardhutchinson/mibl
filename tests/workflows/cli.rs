//! CLI process scenarios over the combined snapshot: every lookup kind, the
//! candidate table and its scope, PUS coordinates, `--details` reaching a
//! definition from every table family, and `--debug` diagnostics.
//!
//! These helpers run the built binary, so they stay beside the scenarios that
//! spawn it rather than in the shared support module.

use crate::{
    common::Fixture,
    fixture::{CAF, CAP, PCPC, TABLES, VPD, combined},
};
use std::{
    collections::BTreeSet,
    process::{Command, Output},
};

fn run(dir: &Fixture, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mibl"))
        .env("MIB_DIR", dir.path())
        .args(args)
        .output()
        .unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

/// Definition headers of `--details` output, which name every reachable source.
fn definition_sources(text: &str) -> Vec<&str> {
    text.lines()
        .filter_map(|line| line.strip_prefix("[D"))
        .filter_map(|line| line.split_once("] ").map(|(_, source)| source))
        .collect()
}

#[test]
fn cli_scenarios_render_every_combined_workflow() {
    let dir = combined();
    let for_expected = |args: &[&str], expected: &[&str]| {
        let output = run(&dir, args);
        assert!(output.status.success(), "{args:?}: {}", stderr(&output));
        assert!(output.stderr.is_empty(), "{args:?}");
        let text = stdout(&output);
        for expected in expected {
            assert!(
                text.contains(expected),
                "missing {expected} in {args:?}: {text}"
            );
        }
    };
    for_expected(
        &["parameter", "DEMO_MODE"],
        &[
            "Parameter DEMO_MODE",
            "Description: Operational mode",
            "Encoding: unsigned integer, 8 bits",
            "Source: pcf.dat:1",
            "89000  DEMO_HK   byte 19 bit 0     8 bits  once",
            "89001  DEMO_VAR  relative 16 bits  8 bits",
            "TXF DEMO_MODE_TXF at txf.dat:3",
            "0..0 -> OFFLINE",
            "1..1 -> ONLINE",
            "inconsistent definition",
        ],
    );
    for_expected(
        &["parameter", "DEMO_TEMP"],
        &[
            "89000  DEMO_HK   byte 20 bit 0    16 bits  1/2, stride 16 bits",
            "89000  DEMO_HK   byte 22 bit 0    16 bits  2/2, stride 16 bits",
            "MCF DEMO_TEMP_MCF at mcf.dat:1",
            "LGF DEMO_TEMP_LGF at lgf.dat:1",
            "CAF DEMO_TEMP_CAF at caf.dat:1",
            "10 -> \"1.5\" [cap.dat:1]",
            "255 -> \"2.5\" [cap.dat:2]",
            "runtime dependent",
        ],
    );
    for_expected(
        &["packet", "89000"],
        &[
            "Packet 89000  DEMO_HK",
            "Description: Demonstration housekeeping",
            "APID: 42",
            "PI1 expected: 7; location: byte 16 bit 0; width: 8 bits",
            "DEMO_MODE   byte 19 bit 0  8 bits   once",
            "DEMO_TEMP   byte 20 bit 0  16 bits  1/2, stride 16 bits",
            "DEMO_TEMP   byte 22 bit 0  16 bits  2/2, stride 16 bits",
            "DEMO_STATE  byte 24 bit 0  8 bits   once",
        ],
    );
    for_expected(
        &["packet", "89001"],
        &[
            "Packet 89001  DEMO_VAR",
            "DEMO_COUNT  byte 10 bit 0  8 bits  once",
            "Repeat group  vpd.dat:1    runtime Repeat group using value of DEMO_COUNT",
            "  DEMO_TEMP  relative 0 bits  16 bits",
            "  DEMO_MODE  relative 16 bits  8 bits",
            "End repeat",
        ],
    );
    for_expected(
        &["command", "DEMO_TC001"],
        &[
            "Command DEMO_TC001",
            "Description: Demonstration command one",
            "ARG1     First argument   application-declared bit 0",
            "ARG2     Second argument  application-declared bit 8",
            "ARG3     Third argument   application-declared bit 16",
            "telemetry DEMO_COUNT",
            "32 .. 127 [E] [prv.dat:1]",
            "0 .. 3 [E] [prv.dat:2]",
            "\"0\" -> LOW [pas.dat:1]",
            "\"2.5\" -> HIGH [pas.dat:2]",
            "CCA DEMO_CONV_1 at cca.dat:1",
            "16 -> \"1.5\" [ccs.dat:1]",
            "255 -> \"3.5\" [ccs.dat:2]",
            "ARG3 at application-declared bit 16",
            "no matching PRF definition",
            "TCP DEMO_HDR01  Demonstration command header",
            "Pkt Version Number",
            "F fixed",
            "APID                HP002      header bit 5   11 bits  A APID from CCF_APID",
            "42 [H] (declared default)",
            "Sequence Count      HP004      header bit 16  14 bits  P set automatically at invocation",
            "0 [D] (declared default)",
            "Ack Flags           HP001      header bit 30  4 bits   K acknowledgement flags from CCF_ACK",
            "Service Type        HP005      header bit 34  8 bits   T service type from CCF_TYPE",
            "Service Subtype     HP006      header bit 42  8 bits   S service subtype from CCF_STYPE",
        ],
    );
    for_expected(
        &["command", "DEMO_TC074"],
        &[
            "Command DEMO_TC074",
            "Description: Demonstration command two",
            "N1  Group count  application-declared bit 0",
            "Repeat group  cdf.dat:4  runtime CDF_GRPSIZE=6",
            "  APID  Application id  application-declared bit 8",
            "  Fixed area  \"Padding\"  application-declared bit 24  4 bits",
            "  Repeat group  cdf.dat:7  runtime CDF_GRPSIZE=3",
            "    Repeat group  cdf.dat:9  runtime CDF_GRPSIZE=1",
            "      SUBTYPE  Subtype  application-declared bit 52",
            "TYPE at application-declared bit 36",
            "0 .. 2 [R] [prv.dat:3]",
            "TCP DEMO_HDR02  Extended demonstration header",
        ],
    );
    for_expected(
        &["command", "DEMO_TC003"],
        &[
            "Command DEMO_TC003",
            "ARG5     Disagreeing argument  application-declared bit 8",
            "inconsistent definition; CDF_ELLEN disagrees with the CPC encoded width",
            "TCP DEMO_HDR03  Demonstration header without elements",
            "No header elements declared",
        ],
    );
    // Search always returns the plain candidate table, and its identities feed exact lookup.
    let output = run(&dir, &["search", "mode"]);
    assert!(output.status.success() && output.stderr.is_empty());
    let text = stdout(&output);
    assert!(text.starts_with("Kind") && text.contains("Source"));
    assert!(!text.contains('\t'));
    let rows: Vec<&str> = text.lines().skip(1).collect();
    assert!(rows.len() >= 2, "{text}");
    // A name match precedes a description match, whichever kind holds it.
    assert!(
        rows[0].starts_with("parameter  DEMO_MODE"),
        "a name match ranks first: {text}"
    );
    assert!(
        rows.iter().any(|row| row.starts_with("command")),
        "description matches of other kinds stay included: {text}"
    );
    assert!(text.contains("pcf.dat:1") && text.contains("ccf.dat:1"));
    let scoped = run(&dir, &["search", "mode", "--scope", "parameters"]);
    assert!(scoped.status.success());
    let scoped = stdout(&scoped);
    assert!(!scoped.contains("DEMO_TC001") && !scoped.contains("89000"));
    assert!(
        scoped
            .lines()
            .any(|row| row.starts_with("parameter  DEMO_MODE")),
        "{scoped}"
    );
    assert!(run(&dir, &["parameter", "DEMO_MODE"]).status.success());
    // Equal ranks keep the contract's kind order before the identity and source order.
    let output = run(&dir, &["search", "DEMO"]);
    assert!(output.status.success() && output.stderr.is_empty());
    let text = stdout(&output);
    let kinds: Vec<&str> = text
        .lines()
        .skip(1)
        .map(|row| row.split_whitespace().next().unwrap())
        .collect();
    assert_eq!(
        kinds,
        vec![
            "parameter",
            "parameter",
            "parameter",
            "parameter",
            "parameter",
            "command",
            "command",
            "command",
            "packet",
            "packet",
        ],
        "{text}"
    );
    let identities: Vec<&str> = text
        .lines()
        .skip(1)
        .map(|row| row.split_whitespace().nth(1).unwrap())
        .collect();
    assert_eq!(
        &identities[..5],
        &[
            "DEMO_ABSENT",
            "DEMO_COUNT",
            "DEMO_MODE",
            "DEMO_STATE",
            "DEMO_TEMP"
        ]
    );
    assert_eq!(
        &identities[5..8],
        &["DEMO_TC001", "DEMO_TC003", "DEMO_TC074"]
    );
    // Separate processes render the same declarations identically.
    assert_eq!(
        run(&dir, &["--details", "command", "DEMO_TC074"]).stdout,
        run(&dir, &["--details", "command", "DEMO_TC074"]).stdout
    );
    // A PUS coordinate lists the packet and command definitions that share it.
    let output = run(&dir, &["pus", "3,25"]);
    assert!(output.status.success() && output.stderr.is_empty());
    let text = stdout(&output);
    assert!(text.contains("TM(3,25)") && text.contains("89000"));
    assert!(text.contains("TC(3,25)") && text.contains("DEMO_TC001"));
    // Losing the whole directory between runs is a clear loading error, never a silent miss.
    std::fs::remove_dir_all(dir.path()).unwrap();
    let output = run(&dir, &["parameter", "DEMO_MODE"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        stderr(&output).contains("cannot read MIB directory"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn cli_details_reach_every_canonical_supporting_table_family() {
    let dir = combined();
    let mut files = BTreeSet::new();
    for (verb, identity) in [
        ("parameter", "DEMO_MODE"),
        ("parameter", "DEMO_TEMP"),
        ("parameter", "DEMO_STATE"),
        ("parameter", "DEMO_ABSENT"),
        ("packet", "89000"),
        ("packet", "89001"),
        ("command", "DEMO_TC001"),
        ("command", "DEMO_TC074"),
    ] {
        let output = run(&dir, &["--details", verb, identity]);
        assert!(
            output.status.success(),
            "{verb} {identity}: {}",
            stderr(&output)
        );
        assert!(output.stderr.is_empty());
        let text = stdout(&output);
        let sources = definition_sources(&text);
        assert!(
            !sources.is_empty(),
            "{verb} {identity} reached no definition: {text}"
        );
        for source in sources {
            files.insert(source.split(':').next().unwrap().to_string());
        }
    }
    let expected: BTreeSet<String> = TABLES.iter().map(|(file, _)| file.to_string()).collect();
    assert_eq!(files, expected);
}

#[test]
fn cli_debug_diagnostics_and_silent_misses_keep_their_outcomes() {
    let dir = combined();
    for file in ["plf.dat", "pcdf.dat"] {
        std::fs::remove_file(dir.path().join(file)).unwrap();
    }
    let debug = run(&dir, &["--debug", "command", "DEMO_TC074"]);
    assert!(debug.status.success());
    let text = stderr(&debug);
    for expected in [
        "skipped file",
        "plf.dat",
        "pcdf.dat",
        "No such file or directory",
        "dropped row",
        "cdf.dat",
        "line=13",
        "CDF_ELTYPE: invalid code \"X\"",
        "original=DEMO_ORPHAN",
        "cpc.dat",
        "CPC_PTC: invalid integer \"broken\"",
        "original=ORPHAN_ARG",
        "ignored extra columns",
        "tcp.dat",
        "line=3",
        "extra=4",
        "exact command lookup",
        "command=DEMO_TC074",
        "matches=1",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    // Diagnostics never change stdout, which stays identical without --debug.
    let quiet = run(&dir, &["command", "DEMO_TC074"]);
    assert!(quiet.status.success() && quiet.stderr.is_empty());
    assert_eq!(quiet.stdout, debug.stdout);
    assert!(stdout(&quiet).contains("End repeat"));
    // Exact misses are silent with status 1, and explain themselves only in debug mode.
    for (args, kind) in [
        (vec!["parameter", "demo_mode"], "parameter"),
        (vec!["packet", "89999"], "packet"),
        (vec!["command", "DEMO_TC999"], "command"),
    ] {
        let output = run(&dir, &args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(
            output.stdout.is_empty() && output.stderr.is_empty(),
            "{args:?}: {} {}",
            stdout(&output),
            stderr(&output)
        );
        let mut debug_args = vec!["--debug"];
        debug_args.extend(args.iter().copied());
        let output = run(&dir, &debug_args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty());
        let text = stderr(&output);
        for expected in [
            format!("exact {kind} lookup"),
            format!("{kind} not found"),
            "NoMatchingIdentity".to_string(),
        ] {
            assert!(text.contains(&expected), "missing {expected}: {text}");
        }
    }
    // A snapshot without roots of a kind keeps its other absence reason just as silent.
    let supporting = Fixture::new();
    for (file, text) in [
        ("caf.dat", CAF),
        ("cap.dat", CAP),
        ("vpd.dat", VPD),
        ("pcpc.dat", PCPC),
    ] {
        supporting.write(file, text);
    }
    for (args, kind) in [
        (vec!["parameter", "DEMO_MODE"], "parameter"),
        (vec!["packet", "89000"], "packet"),
        (vec!["command", "DEMO_TC001"], "command"),
    ] {
        let output = run(&supporting, &args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(
            output.stdout.is_empty() && output.stderr.is_empty(),
            "{args:?}: {} {}",
            stdout(&output),
            stderr(&output)
        );
        let mut debug_args = vec!["--debug"];
        debug_args.extend(args.iter().copied());
        let output = run(&supporting, &debug_args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty());
        let text = stderr(&output);
        for expected in [
            format!("{kind} not found"),
            "DefinitionsUnavailable".to_string(),
        ] {
            assert!(text.contains(&expected), "missing {expected}: {text}");
        }
    }
}
