use crate::common::{Fixture, PARAMETER};
use crate::tui;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use mibl::Mib;
use ratatui::{Terminal, backend::TestBackend};

fn screen(app: &mut tui::App<'_>) -> String {
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal.draw(|frame| app.draw(frame)).unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|c| c.symbol())
        .collect()
}

fn key(app: &mut tui::App<'_>, code: KeyCode) {
    assert!(!app.handle(KeyEvent::new(code, KeyModifiers::NONE)).unwrap());
}

#[test]
fn browse_and_open_complete_definitions_with_keyboard() {
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    dir.write("pid.dat", "3\t25\t42\t7\t0\t89000\tMode\t\t-1\t10");
    dir.write("ccf.dat", "SET_STATE\tMode\t\t\t\tHEADER");
    let mib = Mib::load(dir.path()).unwrap();
    let mut app = tui::App::new(&mib);
    let initial = screen(&mut app);
    assert!(initial.contains("89000") && initial.contains("? help"));
    assert!(!initial.contains("Enter open") && !initial.contains("Ctrl-C quit"));
    key(&mut app, KeyCode::Char('2'));
    assert!(screen(&mut app).contains("TEMP"));
    key(&mut app, KeyCode::Enter);
    assert!(screen(&mut app).contains("Parameter TEMP"));
    key(&mut app, KeyCode::End);
    assert!(screen(&mut app).contains("PCF_"));
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::Char('3'));
    assert!(screen(&mut app).contains("SET_STATE"));
    key(&mut app, KeyCode::Enter);
    assert!(screen(&mut app).contains("Command SET_STATE"));
    assert!(
        app.handle(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE))
            .unwrap()
    );
}

fn type_text(app: &mut tui::App<'_>, value: &str) {
    for c in value.chars() {
        key(app, KeyCode::Char(c));
    }
    key(app, KeyCode::Enter);
}

#[test]
fn scoped_search_preserves_ranking_and_distinguishes_no_matches_from_unavailable_roots() {
    let dir = Fixture::new();
    dir.write(
        "pcf.dat",
        &format!(
            "{}\n{}\n{}",
            PARAMETER
                .replace("TEMP", "OTHER")
                .replace("Temperature", "mode"),
            PARAMETER.replace("TEMP", "MODE_LONG"),
            PARAMETER.replace("TEMP", "MODE")
        ),
    );
    dir.write("ccf.dat", "SET_STATE\tmode\t\t\t\tHEADER");
    let mib = Mib::load(dir.path()).unwrap();
    let mut app = tui::App::new(&mib);
    assert!(screen(&mut app).contains("Definitions unavailable"));
    key(&mut app, KeyCode::Char('2'));
    key(&mut app, KeyCode::Char('/'));
    type_text(&mut app, "MoDe");
    let text = screen(&mut app);
    assert!(
        text.contains("Search Parameters") && text.contains("3 definitions"),
        "{text}"
    );
    assert!(text.find("MODE ").unwrap() < text.find("MODE_LONG").unwrap());
    key(&mut app, KeyCode::Enter);
    assert!(screen(&mut app).contains("Parameter MODE"));
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::Tab);
    assert!(screen(&mut app).contains("Search Commands") && screen(&mut app).contains("SET_STATE"));
    key(&mut app, KeyCode::Tab);
    let mixed = screen(&mut app);
    assert!(mixed.contains("Search All") && mixed.contains("4 definitions"));
    assert!(mixed.contains("Kind") && mixed.contains("parameter") && mixed.contains("command"));
    key(&mut app, KeyCode::Char('/'));
    type_text(&mut app, "zzzzqqqq");
    assert!(screen(&mut app).contains("No matching definitions"));
    key(&mut app, KeyCode::Char('/'));
    key(&mut app, KeyCode::Enter);
    assert!(screen(&mut app).contains("No matching definitions"));
    key(&mut app, KeyCode::Char('1'));
    assert!(screen(&mut app).contains("Definitions unavailable"));
}

#[test]
fn pus_filter_validation_and_table_reports_are_keyboard_accessible() {
    let dir = Fixture::new();
    dir.write(
        "pid.dat",
        "3\t25\t42\t7\t0\t89000\tMode\t\t-1\t10\n3\t26\t42\t7\t0\t89001\tOther\t\t-1\t10",
    );
    dir.write("pcf.dat", "");
    let mib = Mib::load(dir.path()).unwrap();
    let mut app = tui::App::new(&mib);
    key(&mut app, KeyCode::Char('f'));
    type_text(&mut app, "3,25");
    let text = screen(&mut app);
    assert!(
        text.contains("PUS 3,25") && text.contains("89000") && !text.contains("89001"),
        "{text}"
    );
    key(&mut app, KeyCode::Char('f'));
    type_text(&mut app, "-1");
    assert!(screen(&mut app).contains("unsigned integers"));
    key(&mut app, KeyCode::Esc);
    assert!(screen(&mut app).contains("PUS 3,25"));
    key(&mut app, KeyCode::Char('f'));
    type_text(&mut app, "3");
    assert!(screen(&mut app).contains("89001"));
    key(&mut app, KeyCode::Char('t'));
    let text = screen(&mut app);
    assert!(text.contains("pcf.dat") && text.contains("missing") && text.contains("Code"));
    key(&mut app, KeyCode::End);
    assert!(screen(&mut app).contains("vpd.dat"));
    key(&mut app, KeyCode::Esc);
    assert!(screen(&mut app).contains("PUS 3"));
}

fn read_document(app: &mut tui::App<'_>) -> String {
    let mut collected = String::new();
    let mut previous = String::new();
    for _ in 0..1000 {
        let text = screen(app);
        if text == previous {
            return collected;
        }
        collected.push_str(&text);
        previous = text;
        key(app, KeyCode::PageDown);
    }
    panic!("document did not reach its end");
}

#[test]
fn all_supported_definitions_and_problem_evidence_are_reachable_by_scrolling() {
    let dir = crate::workflow_fixture::combined();
    let mib = Mib::load(dir.path()).unwrap();
    // Every later interaction must use the snapshot even after the files disappear.
    drop(dir);
    let mut app = tui::App::new(&mib);
    let mut all = String::new();
    for (view, identity, expected) in [
        (
            '2',
            "DEMO_MODE",
            vec![
                "Parameter DEMO_MODE",
                "89000",
                "89001",
                "OFFLINE",
                "inconsistent definition",
                "PCF_NAME",
            ],
        ),
        (
            '2',
            "DEMO_TEMP",
            vec![
                "Parameter DEMO_TEMP",
                "MCF DEMO_TEMP_MCF",
                "LGF DEMO_TEMP_LGF",
                "CAF DEMO_TEMP_CAF",
                "runtime dependent",
            ],
        ),
        (
            '1',
            "89000",
            vec!["Packet 89000", "byte 19 bit 0", "PI1 expected", "PID_SPID"],
        ),
        (
            '1',
            "89001",
            vec!["Packet 89001", "Repeat group", "End repeat", "VPD_POS"],
        ),
        (
            '3',
            "DEMO_TC001",
            vec![
                "Command DEMO_TC001",
                "ARG3",
                "no matching PRF definition",
                "CCA DEMO_CONV_1",
                "Service Type",
                "PCDF_",
            ],
        ),
        (
            '3',
            "DEMO_TC074",
            vec![
                "Command DEMO_TC074",
                "Fixed area",
                "Repeat group",
                "CDF_GRPSIZE",
                "TCP DEMO_HDR02",
            ],
        ),
    ] {
        key(&mut app, KeyCode::Char(view));
        key(&mut app, KeyCode::Char('/'));
        type_text(&mut app, identity);
        key(&mut app, KeyCode::Enter);
        let text = read_document(&mut app);
        for expected in expected {
            assert!(text.contains(expected), "{identity} missing {expected}");
        }
        all.push_str(&text);
    }
    for (table, _) in crate::workflow_fixture::TABLES {
        assert!(all.contains(table), "missing recorded source {table}");
    }
}

#[test]
fn duplicate_selection_never_chooses_one_root_and_long_lists_keep_selection_visible() {
    let dir = Fixture::new();
    let mut rows = (0..100)
        .map(|i| PARAMETER.replace("TEMP", &format!("PARAM_{i:03}")))
        .collect::<Vec<_>>();
    rows.push(PARAMETER.into());
    rows.push(PARAMETER.into());
    dir.write("pcf.dat", &rows.join("\n"));
    let mib = Mib::load(dir.path()).unwrap();
    let mut app = tui::App::new(&mib);
    key(&mut app, KeyCode::Char('2'));
    screen(&mut app);
    key(&mut app, KeyCode::End);
    let listed = screen(&mut app);
    for header in ["Identity", "Name", "Description", "Source"] {
        assert!(listed.contains(header), "missing sticky header {header}");
    }
    assert!(!listed.contains("parameter TEMP"));
    assert!(listed.contains("> TEMP"));
    key(&mut app, KeyCode::Enter);
    let text = screen(&mut app);
    assert!(
        text.contains("Ambiguous identity")
            && text.contains("pcf.dat:101")
            && text.contains("pcf.dat:102")
    );
    assert!(!text.contains("Parameter TEMP"));
    key(&mut app, KeyCode::Esc);
    assert!(screen(&mut app).contains("> TEMP"));
    key(&mut app, KeyCode::Home);
    key(&mut app, KeyCode::PageDown);
    key(&mut app, KeyCode::Enter);
    assert!(screen(&mut app).contains("Parameter PARAM_021"));
}

#[test]
fn cancelled_search_keeps_scope_and_selection_and_input_accepts_quit_letter() {
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    let mib = Mib::load(dir.path()).unwrap();
    let mut app = tui::App::new(&mib);
    key(&mut app, KeyCode::Char('2'));
    let before = screen(&mut app);
    key(&mut app, KeyCode::Char('/'));
    key(&mut app, KeyCode::Tab);
    key(&mut app, KeyCode::Char('q'));
    key(&mut app, KeyCode::Esc);
    assert_eq!(screen(&mut app), before);
    let mut release = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
    release.kind = crossterm::event::KeyEventKind::Release;
    assert!(!app.handle(release).unwrap());
}

#[test]
fn filters_distinguish_unavailable_roots_from_no_matching_definitions() {
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    let mib = Mib::load(dir.path()).unwrap();
    let mut app = tui::App::new(&mib);
    key(&mut app, KeyCode::Char('/'));
    type_text(&mut app, "missing");
    assert!(screen(&mut app).contains("Definitions unavailable"));
    key(&mut app, KeyCode::Char('f'));
    type_text(&mut app, "3");
    assert!(screen(&mut app).contains("Definitions unavailable"));
    key(&mut app, KeyCode::Char('2'));
    key(&mut app, KeyCode::Char('/'));
    type_text(&mut app, "missing");
    assert!(screen(&mut app).contains("No matching definitions"));
}

#[test]
fn narrow_screens_offer_scrollable_help_and_wide_definition_text() {
    let dir = Fixture::new();
    dir.write(
        "pcf.dat",
        &PARAMETER.replace(
            "Temperature",
            &format!("{} far-right-evidence", "界".repeat(80)),
        ),
    );
    let mib = Mib::load(dir.path()).unwrap();
    let mut app = tui::App::new(&mib);
    let small_screen = |app: &mut tui::App<'_>| {
        let mut terminal = Terminal::new(TestBackend::new(40, 12)).unwrap();
        terminal.draw(|frame| app.draw(frame)).unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>()
    };
    assert!(small_screen(&mut app).contains("? help"));
    key(&mut app, KeyCode::Char('?'));
    assert!(small_screen(&mut app).contains("Keyboard controls"));
    key(&mut app, KeyCode::End);
    assert!(small_screen(&mut app).contains("Ctrl-C"));
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::Char('2'));
    key(&mut app, KeyCode::Enter);
    for _ in 0..11 {
        key(&mut app, KeyCode::Right);
    }
    assert!(small_screen(&mut app).contains("far-right-evidence"));
    key(&mut app, KeyCode::Home);
    assert!(small_screen(&mut app).contains("Parameter TEMP"));
    // A resized terminal, including one with no usable content area, must still draw.
    for (width, height) in [(1, 1), (0, 0), (80, 24)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| app.draw(frame)).unwrap();
    }
}

#[test]
fn inventory_escapes_recorded_control_characters() {
    let dir = Fixture::new();
    dir.write(
        "pcf.dat",
        &PARAMETER.replace("Temperature", "Mode\u{1b}[31m"),
    );
    let mib = Mib::load(dir.path()).unwrap();
    let mut app = tui::App::new(&mib);
    key(&mut app, KeyCode::Char('2'));
    assert!(screen(&mut app).contains("Mode\\u{1b}[31m"));
}

#[test]
fn top_tabs_identify_lists_definitions_and_table_reports() {
    let dir = crate::workflow_fixture::combined();
    let mib = Mib::load(dir.path()).unwrap();
    let mut app = tui::App::new(&mib);
    let active_tab = |app: &mut tui::App<'_>| {
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal.draw(|frame| app.draw(frame)).unwrap();
        let header = &terminal.backend().buffer().content[..480];
        let text: String = header.iter().map(|cell| cell.symbol()).collect();
        assert!(text.contains("╭─ mibl"), "{text}");
        let row = &terminal.backend().buffer().content[120..240];
        let labels: String = row.iter().map(|cell| cell.symbol()).collect();
        for label in ["Packets", "Parameters", "Commands", "Tables"] {
            assert!(labels.contains(label), "missing {label} in {labels}");
        }
        row.iter()
            .filter(|cell| cell.bg == ratatui::style::Color::Cyan)
            .map(|cell| cell.symbol())
            .collect::<String>()
            .trim()
            .to_owned()
    };
    assert_eq!(active_tab(&mut app), "P Packets");
    for expected in [
        "p Parameters",
        "c Commands",
        "u PUS",
        "t Tables",
        "P Packets",
    ] {
        key(&mut app, KeyCode::Tab);
        assert_eq!(active_tab(&mut app), expected);
    }
    key(&mut app, KeyCode::Char('p'));
    assert_eq!(active_tab(&mut app), "p Parameters");
    key(&mut app, KeyCode::Enter);
    assert_eq!(active_tab(&mut app), "p Parameters");
    key(&mut app, KeyCode::Char('c'));
    assert_eq!(active_tab(&mut app), "c Commands");
    key(&mut app, KeyCode::Char('P'));
    assert_eq!(active_tab(&mut app), "P Packets");
    key(&mut app, KeyCode::Char('C'));
    assert_eq!(active_tab(&mut app), "c Commands");
    key(&mut app, KeyCode::Char('t'));
    assert_eq!(active_tab(&mut app), "t Tables");
    assert!(screen(&mut app).contains("Supported-table load reports"));
    key(&mut app, KeyCode::Esc);
    assert_eq!(active_tab(&mut app), "c Commands");
    key(&mut app, KeyCode::Char('/'));
    key(&mut app, KeyCode::Tab); // Search all kinds.
    type_text(&mut app, "DEMO_MODE");
    assert_eq!(active_tab(&mut app), "");
    key(&mut app, KeyCode::Enter);
    assert_eq!(active_tab(&mut app), "p Parameters");
}

#[test]
fn definition_inspection_has_boxed_sections_and_section_navigation() {
    let dir = crate::workflow_fixture::combined();
    let mib = Mib::load(dir.path()).unwrap();
    let mut app = tui::App::new(&mib);
    key(&mut app, KeyCode::Enter);
    let first = screen(&mut app);
    assert!(first.contains("╭─ Summary"), "{first}");
    assert!(first.contains("╭─ Identification"));
    key(&mut app, KeyCode::Char(']'));
    let next = screen(&mut app);
    assert!(next.contains("╭─ Identification"));
    assert!(!next.contains("Packet 89000  DEMO_HK"));
    key(&mut app, KeyCode::Char('['));
    assert!(screen(&mut app).contains("Packet 89000  DEMO_HK"));
    for _ in 0..3 {
        key(&mut app, KeyCode::Char(']'));
    }
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal.draw(|frame| app.draw(frame)).unwrap();
    assert!(
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .any(|c| c.fg == ratatui::style::Color::Yellow)
    );
}

#[test]
fn pus_browser_orders_coordinates_and_opens_definitions_with_return_navigation() {
    let dir = Fixture::new();
    dir.write("pid.dat", "17\t1\t42\t0\t0\t90000\tLast\t\t-1\t10\n3\t26\t42\t0\t0\t89001\tSecond\t\t-1\t10\n3\t25\t42\t0\t0\t89000\tFirst\t\t-1\t10");
    dir.write("ccf.dat", "DUP\tOne\t\t\tN\tHEADER\t3\t25\nDUP\tTwo\t\t\tN\tHEADER\t3\t26\nUNKNOWN\tNo coordinates\t\t\tN\tHEADER\nNO_SUBTYPE\tNo subtype\t\t\tN\tHEADER\t3");
    let mib = Mib::load(dir.path()).unwrap();
    let mut app = tui::App::new(&mib);
    key(&mut app, KeyCode::Char('u'));
    let groups = screen(&mut app);
    assert!(
        groups.contains("Service") && groups.contains("Subtype") && groups.contains("unavailable")
    );
    key(&mut app, KeyCode::Enter);
    let first = screen(&mut app);
    assert!(first.contains("PUS 3,25") && first.contains("89000") && first.contains("DUP"));
    key(&mut app, KeyCode::Enter);
    assert!(screen(&mut app).contains("Packet 89000"));
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Enter);
    assert!(screen(&mut app).contains("Ambiguous identity"));
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::Esc);
    assert_eq!(screen(&mut app), groups);
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Enter);
    assert!(screen(&mut app).contains("PUS 3,26"));
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Enter);
    assert!(
        screen(&mut app).contains("PUS 3,unavailable") && screen(&mut app).contains("NO_SUBTYPE")
    );
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Enter);
    assert!(screen(&mut app).contains("PUS 17,1"));
    key(&mut app, KeyCode::Esc);
    key(&mut app, KeyCode::End);
    key(&mut app, KeyCode::Enter);
    assert!(
        screen(&mut app).contains("PUS unavailable,unavailable")
            && screen(&mut app).contains("UNKNOWN")
    );
    key(&mut app, KeyCode::Char('f'));
    type_text(&mut app, "3,25");
    assert!(screen(&mut app).contains("89000"));
}

#[test]
fn tables_are_selectable_and_unavailable_files_report_errors_in_place() {
    let dir = Fixture::new();
    dir.write("pcf.dat", PARAMETER);
    std::fs::create_dir(dir.path().join("cap.dat")).unwrap();
    let mib = Mib::load(dir.path()).unwrap();
    let mut app = tui::App::new(&mib);
    key(&mut app, KeyCode::Char('t'));
    assert!(screen(&mut app).contains("> CAF"));
    key(&mut app, KeyCode::Down);
    assert!(screen(&mut app).contains("> CAP"));
    key(&mut app, KeyCode::Enter);
    assert!(screen(&mut app).contains("cap.dat is unreadable"));
    key(&mut app, KeyCode::End);
    assert!(screen(&mut app).contains("> VPD"));
    key(&mut app, KeyCode::Enter);
    let text = screen(&mut app);
    assert!(
        text.contains("vpd.dat is missing") && text.contains("> VPD"),
        "{text}"
    );
    key(&mut app, KeyCode::Home);
    assert!(screen(&mut app).contains("> CAF"));
}
