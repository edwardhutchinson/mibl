use crate::common::{Fixture, PARAMETER};
use crate::support::run;

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
fn parameter_packets_say_none_only_when_every_occurrence_source_is_known() {
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    for prefix in [vec![], vec!["--details"]] {
        let mut args = prefix.clone();
        args.extend(["parameter", "TEMP"]);
        let output = run(&dir, &args);
        assert!(
            output.status.success(),
            "a usable root definition stays Found with status 0"
        );
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("\nPackets\nunavailable\n"), "{text}");
        assert!(!text.contains("\nPackets\nnone\n"), "{text}");
        assert!(text.contains("no matching PLF definition"), "{text}");
        assert!(
            text.contains("Units: K"),
            "the definition stays usable: {text}"
        );
    }
    // A readable PLF table without a matching row is a known empty result.
    dir.write("plf.dat", "");
    for prefix in [vec![], vec!["--details"]] {
        let mut args = prefix.clone();
        args.extend(["parameter", "TEMP"]);
        let output = run(&dir, &args);
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("\nPackets\nnone\n"), "{text}");
        assert!(!text.contains("\nPackets\nunavailable\n"), "{text}");
        assert!(!text.contains("no matching PLF definition"), "{text}");
    }
    // A declared variable packet structure without VPD data stays unavailable.
    dir.write(
        "pid.dat",
        "3\t25\t42\t0\t0\t89001\tDemonstration variable packet\t\t7\t10",
    );
    for prefix in [vec![], vec!["--details"]] {
        let mut args = prefix.clone();
        args.extend(["parameter", "TEMP"]);
        let output = run(&dir, &args);
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("\nPackets\nunavailable\n"), "{text}");
        assert!(text.contains("no matching VPD definition"), "{text}");
    }
}
