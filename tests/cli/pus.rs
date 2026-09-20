use crate::common::{Fixture, PARAMETER};
use crate::support::run;

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
