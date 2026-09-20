mod common;
use common::{Fixture, PARAMETER};
use mibl::{Mib, model::*};
use std::process::{Command, Output};

fn run(dir: &Fixture, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mibl"))
        .env("MIB_DIR", dir.path())
        .args(args)
        .output()
        .unwrap()
}

fn report(mib: &Mib, table: Table) -> TableRows {
    mib.tables()
        .into_iter()
        .find(|r| r.table == table)
        .expect("every supported table is reported")
        .rows
}

/// One listing serves both ways of naming a table: codes are unique, and the file name is
/// the lowercase code, so code order and file order are the same order.
#[test]
fn catalogue_codes_files_and_meanings_are_unique_and_ordered() {
    let codes: Vec<_> = Table::ALL.iter().map(|table| table.code()).collect();
    assert_eq!(Table::ALL.len(), 25);
    for (index, table) in Table::ALL.iter().enumerate() {
        assert!(!table.meaning().is_empty(), "{}", codes[index]);
        assert_eq!(
            *table.file(),
            format!("{}.dat", codes[index].to_lowercase())
        );
    }
    let mut unique = codes.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(unique.len(), codes.len());
    let mut ordered = Table::ALL.to_vec();
    ordered.sort_by_key(|table| table.code());
    assert_eq!(ordered, Table::ALL.to_vec());
}

/// The loader reads every catalogued file, so a listing entry can never name a file the
/// loader ignores, and a supported table can never be absent from the listing.
#[test]
fn every_catalogued_table_is_read_by_the_loader() {
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    for table in Table::ALL {
        if table != Table::Pcf {
            dir.write(table.file(), "");
        }
    }
    let mib = Mib::load(dir.path()).unwrap();
    let reports = mib.tables();
    let listed: Vec<_> = reports.iter().map(|report| report.table).collect();
    assert_eq!(listed, Table::ALL.to_vec());
    assert_eq!(
        reports
            .iter()
            .filter(|report| report.rows != TableRows::Read { rows: 0 })
            .map(|report| report.table)
            .collect::<Vec<_>>(),
        vec![Table::Pcf]
    );
}

/// A readable table with no retained rows is not an absent table, and an unusable file is
/// not a readable one: all three states reach the listing separately.
#[test]
fn absent_readable_and_unreadable_tables_are_distinguished() {
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    dir.write("vpd.dat", "");
    std::fs::create_dir(dir.path().join("plf.dat")).unwrap();
    let mib = Mib::load(dir.path()).unwrap();
    assert_eq!(report(&mib, Table::Pcf), TableRows::Read { rows: 1 });
    assert_eq!(report(&mib, Table::Vpd), TableRows::Read { rows: 0 });
    assert_eq!(report(&mib, Table::Plf), TableRows::Unreadable);
    assert_eq!(report(&mib, Table::Caf), TableRows::Missing);
}

/// The loaded directory always holds three usable roots, one readable table without
/// retained rows, one unusable file and the rest absent.
fn listing_fixture() -> Fixture {
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    dir.write("pid.dat", "3\t25\t42\t7\t0\t89000\tDemonstration\t\t-1\t10");
    dir.write("ccf.dat", "DEMO_TC\tDemonstration command\t\t\t\tHDR");
    dir.write("vpd.dat", "");
    std::fs::create_dir(dir.path().join("plf.dat")).unwrap();
    dir
}

/// Every listing cell comes from the table it names, and every state is reported as the
/// source left it: a count, `missing`, or `unreadable`.
fn assert_listing(text: &str) {
    let rows: Vec<Vec<&str>> = text
        .lines()
        .skip(1)
        .map(|line| line.split_whitespace().collect())
        .collect();
    assert_eq!(rows.len(), Table::ALL.len());
    for (cells, table) in rows.iter().zip(Table::ALL) {
        assert_eq!(cells[0], table.code(), "{cells:?}");
        assert_eq!(cells[1], table.file(), "{cells:?}");
        assert_eq!(
            cells[2..cells.len() - 2].join(" "),
            table.meaning(),
            "{cells:?}"
        );
        assert_eq!(
            cells[cells.len() - 2],
            match table.root() {
                Some(TableRoot::Parameter) => "parameter",
                Some(TableRoot::Packet) => "packet",
                Some(TableRoot::Command) => "command",
                None => "-",
            },
            "{cells:?}"
        );
        assert_eq!(
            cells[cells.len() - 1],
            match (table, table.file()) {
                (Table::Pcf | Table::Pid | Table::Ccf, _) => "1",
                (_, "vpd.dat") => "0",
                (_, "plf.dat") => "unreadable",
                _ => "missing",
            },
            "{cells:?}"
        );
    }
}

#[test]
fn tables_lists_every_table_with_its_file_meaning_lookup_and_state() {
    let dir = listing_fixture();
    let output = run(&dir, &["tables"]);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let text = String::from_utf8(output.stdout.clone()).unwrap();
    assert!(!text.contains('\t'));
    let header = text.lines().next().unwrap();
    assert!(
        header.starts_with("Code  File") && header.ends_with("Lookup     Rows"),
        "{header}"
    );
    assert_listing(&text);
    // The listing is the same with --details, which adds nothing here, and --debug
    // changes only stderr.
    let details = run(&dir, &["--details", "tables"]);
    assert_eq!(details.stdout, output.stdout);
    let debug = run(&dir, &["--debug", "tables"]);
    assert_eq!(debug.stdout, output.stdout);
    let events = String::from_utf8(debug.stderr).unwrap();
    assert!(
        events.contains("loaded table") && events.contains("vpd.dat"),
        "{events}"
    );
}

#[test]
fn tables_requires_a_usable_directory_like_every_other_verb() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mibl"));
    command.env_remove("MIB_DIR").args(["tables"]);
    let output = command.output().unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("MIB_DIR")
    );
    let empty = run(&Fixture::new(), &["tables"]);
    assert_eq!(empty.status.code(), Some(2));
    assert!(empty.stdout.is_empty() && !empty.stderr.is_empty());
}
