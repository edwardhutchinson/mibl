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
