use crate::common::{Fixture, PARAMETER};
use crate::support::run;
use std::process::Command;

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
