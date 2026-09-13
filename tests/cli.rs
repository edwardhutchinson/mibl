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
    let output = run(&dir, &["parameter", "TEMP"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "TEMP",
        "Temperature",
        "Unsigned integer",
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
    assert!(text.starts_with("Kind\tIdentity\tName\tDescription\tSource\n"));
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
    let output = run(&dir, &["packet", "089000"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "Packet: 89000",
        "ZUY_HK_00001",
        "APID: 42",
        "Expected: 7",
        "byte 16 bit 0",
        "byte 19 bit 0",
        "byte 20 bit 0",
        "8 bits",
        "Temperature",
        "TPCF_SIZE",
        "PIC_PI1_WID",
        "PLF_NBOCC",
        "PCF_PTC",
        "Repeated 2 times",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    let output = run(&dir, &["parameter", "TEMP"]);
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Packet: 89000") && text.contains("byte 20 bit 0"));
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
    assert!(text.starts_with("Kind\tIdentity\tName\tDescription\tSource"));
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
    let output = run(&dir, &["packet", "89000"]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("PIC_PI1_OFF") && text.contains("PIC_APID") && text.contains("pic.dat:1")
    );
    assert!(!text.contains("Expected:"));
}
