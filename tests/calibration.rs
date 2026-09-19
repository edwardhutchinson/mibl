mod common;
use common::Fixture;
use mibl::{Mib, model::*};

fn parameter(dir: &Fixture) -> ParameterSummary {
    let Lookup::Found(p) = Mib::load(dir.path())
        .unwrap()
        .parameter(&ParameterName("TEMP".into()))
    else {
        panic!("parameter must remain available");
    };
    p.parameter
}
fn root(dir: &Fixture, category: &str, key: &str) {
    dir.write(
        "pcf.dat",
        &format!("TEMP\tTemperature\t\tK\t3\t4\t\t\t\t{category}\tR\t{key}"),
    );
}
#[test]
fn numerical_curve_preserves_points_formats_and_recorded_defaults() {
    let dir = Fixture::new();
    root(&dir, "N", "CURVE");
    dir.write("caf.dat", "CURVE\tCurve\tR\tU\tH");
    dir.write("cap.dat", "CURVE\tA\t1.5\nCURVE\tFF\t2.5");
    let p = parameter(&dir);
    let alts = p.calibrations.value.unwrap();
    let c = alts[0].calibration.value.as_ref().unwrap();
    let CalibrationForm::Numerical {
        points,
        interpolation,
    } = c.form.value.as_ref().unwrap()
    else {
        panic!()
    };
    let points = points.value.as_ref().unwrap();
    assert_eq!(points.len(), 2);
    assert_eq!(points[0].raw.value, Some(Scalar::Unsigned(10)));
    assert_eq!(
        points[0].engineering.value,
        Some(Scalar::Decimal("1.5".into()))
    );
    assert_eq!(points[1].definition.source.line.get(), 2);
    assert_eq!(interpolation.value.as_deref(), Some("F"));
    assert_eq!(c.definition.fields[7].presence, Presence::Omitted);
}

#[test]
fn polynomial_logarithmic_and_textual_definitions_are_interpreted() {
    let dir = Fixture::new();
    for (table, category, data) in [
        ("mcf", "N", "CAL\tPolynomial\t1.5"),
        ("lgf", "N", "CAL\tLogarithmic\t2"),
        ("txf", "S", "CAL\tStatus\tI"),
    ] {
        root(&dir, category, "CAL");
        dir.write(&format!("{table}.dat"), data);
        dir.write("txp.dat", "CAL\t-1\t2\tIdle");
        let p = parameter(&dir);
        let a = p.calibrations.value.unwrap();
        let c = a.last().unwrap().calibration.value.as_ref().unwrap();
        match c.form.value.as_ref().unwrap() {
            CalibrationForm::Polynomial { coefficients }
            | CalibrationForm::Logarithmic { coefficients } => {
                assert_eq!(coefficients.len(), 5);
                assert_eq!(coefficients[4].value, Some(Scalar::Decimal("0".into())));
            }
            CalibrationForm::Textual { intervals } => {
                let i = &intervals.value.as_ref().unwrap()[0];
                assert_eq!(i.low.value, Some(Scalar::Integer(-1)));
                assert_eq!(i.text.value.as_deref(), Some("Idle"));
            }
            _ => panic!("wrong family"),
        }
        std::fs::remove_file(dir.path().join(format!("{table}.dat"))).unwrap();
    }
}

#[test]
fn conditional_order_dependencies_and_duplicate_positions_remain_visible() {
    let dir = Fixture::new();
    root(&dir, "N", "");
    dir.write(
        "pcf.dat",
        &format!(
            "TEMP\tTemperature\t\tK\t3\t4\t\t\t\tN\tR\n{}",
            common::PARAMETER.replace("TEMP", "MODE")
        ),
    );
    dir.write(
        "cur.dat",
        "TEMP\t2\tMISSING\t3\tB\nTEMP\t1\tMODE\t0\tA\nTEMP\t2\tMODE\t1\tA",
    );
    dir.write("mcf.dat", "A\tFirst\t1\nB\tSecond\t2");
    let p = parameter(&dir);
    let a = p.calibrations.value.unwrap();
    assert_eq!(a.len(), 3);
    assert_eq!(a[0].selection.as_ref().unwrap().source.line.get(), 2);
    let condition = a[0].condition.as_ref().unwrap();
    assert!(condition.expression.contains("MODE") && condition.expression.contains("0"));
    assert_eq!(
        condition.dependencies[0]
            .targets
            .value
            .as_ref()
            .unwrap()
            .len(),
        1
    );
    assert!(matches!(
        a[1].condition.as_ref().unwrap().dependencies[0]
            .targets
            .problems[0]
            .kind,
        ProblemKind::MissingReference { .. }
    ));
    assert!(
        p.calibrations
            .problems
            .iter()
            .any(|p| matches!(p.kind, ProblemKind::InconsistentDefinition { .. }))
    );
    assert!(
        a[0].calibration
            .problems
            .iter()
            .any(|p| matches!(p.kind, ProblemKind::RuntimeDependent { .. }))
    );
}

#[test]
fn conflicting_categories_and_ambiguous_targets_keep_usable_alternatives() {
    let dir = Fixture::new();
    // Synthetic equivalent of the surveyed category/reference disagreement.
    root(&dir, "N", "STATUS");
    dir.write("txf.dat", "STATUS\tMode\tU\nSTATUS\tOther mode\tU");
    dir.write("txp.dat", "STATUS\t0\t0\tOff");
    let p = parameter(&dir);
    let a = p.calibrations.value.unwrap();
    assert_eq!(a.len(), 2);
    for a in a {
        assert!(matches!(
            a.calibration.value.unwrap().form.value,
            Some(CalibrationForm::Textual { .. })
        ));
        assert!(
            a.calibration
                .problems
                .iter()
                .any(|p| matches!(p.kind, ProblemKind::InconsistentDefinition { .. }))
        );
        assert!(
            a.calibration
                .problems
                .iter()
                .any(|p| matches!(p.kind, ProblemKind::AmbiguousReference { .. }))
        );
    }
    root(&dir, "S", "MISSING");
    let p = parameter(&dir);
    let a = p.calibrations.value.unwrap();
    assert!(
        matches!(&a[0].calibration.problems[0].kind, ProblemKind::MissingReference { reference: Reference::Supporting { table: Table::Txf, key } } if key == "MISSING")
    );
}

#[test]
fn cli_details_render_all_families_interpretations_and_conflicts() {
    let dir = Fixture::new();
    // PCF_CATEG names the family: the numerical families for N and the textual one for S.
    root(&dir, "N", "A");
    dir.write("caf.dat", "A\tCurve\tR\tU\tH");
    dir.write("cap.dat", "A\tFF\t2.5");
    dir.write("mcf.dat", "A\tPolynomial\t1.5");
    dir.write("lgf.dat", "A\tLogarithmic\t2");
    dir.write("txf.dat", "A\tStatus\tU");
    dir.write("txp.dat", "A\t0\t1\tIdle");
    dir.write(
        "cur.dat",
        "TEMP\t2\tABSENT\t1\tMISSING\nTEMP\t1\tTEMP\t0\tA",
    );
    for expected in [
        "Numerical",
        "Polynomial",
        "Logarithmic",
        "255",
        "2.5",
        // Interpreted values derived from documented defaults keep their marker.
        "Numerical curve; extrapolation: F [default]",
        "\"1.5\", \"0\" [default]",
        "CUR_SELECT",
        "CUR_POS",
        "raw(TEMP) = 0",
        "runtime dependent",
        "ambiguous reference; 3 CAF, MCF, LGF candidates",
        "inconsistent definition",
        "MISSING",
        "caf.dat:1",
        "cap.dat:1",
        "mcf.dat:1",
        "lgf.dat:1",
    ] {
        assert!(text(&dir, "TEMP").contains(expected), "missing {expected}");
    }
    root(&dir, "S", "A");
    for expected in ["Textual", "Idle", "txf.dat:1", "txp.dat:1"] {
        assert!(text(&dir, "TEMP").contains(expected), "missing {expected}");
    }
}

fn text(dir: &Fixture, name: &str) -> String {
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_mibl"))
        .env("MIB_DIR", dir.path())
        .args(["--details", "parameter", name])
        .output()
        .unwrap();
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn partial_loading_keeps_supporting_rows_and_independent_point_fields() {
    let dir = Fixture::new();
    for (file, row) in [
        ("cur.dat", "TEMP\t1\tMODE\t0\tA"),
        ("cap.dat", "A\t0\t1"),
        ("mcf.dat", "A\tPolynomial\t1"),
        ("lgf.dat", "A\tLogarithmic\t1"),
        ("txf.dat", "A\tStatus\tU"),
        ("txp.dat", "A\t0\t1\tIdle"),
    ] {
        dir.write(file, row);
        assert!(matches!(
            Mib::load(dir.path())
                .unwrap()
                .parameter(&ParameterName("TEMP".into())),
            Lookup::NotFound(NotFoundReason::DefinitionsUnavailable)
        ));
        std::fs::remove_file(dir.path().join(file)).unwrap();
    }
    root(&dir, "N", "A");
    dir.write("caf.dat", "BROKEN\tCurve\tX\tU\nA\tCurve\tR\tI\tH");
    dir.write(
        "cap.dat",
        "A\t10\tbad-number\textra\nA\t-2\t3.5\nA\tmissing-column",
    );
    let mib = Mib::load(dir.path()).unwrap();
    dir.write("cap.dat", "A\t99\t99");
    let Lookup::Found(p) = mib.parameter(&ParameterName("TEMP".into())) else {
        panic!()
    };
    let a = p.parameter.calibrations.value.unwrap();
    let c = a[0].calibration.value.as_ref().unwrap();
    assert_eq!(c.definition.source.line.get(), 2);
    let CalibrationForm::Numerical { points, .. } = c.form.value.as_ref().unwrap() else {
        panic!()
    };
    let points = points.value.as_ref().unwrap();
    assert_eq!(points.len(), 2);
    assert_eq!(points[0].raw.value, Some(Scalar::Integer(10))); // Radix applies only to unsigned values.
    assert!(points[0].engineering.value.is_none());
    assert!(matches!(
        points[0].engineering.problems[0].kind,
        ProblemKind::UnsupportedInterpretation { .. }
    ));
    assert_eq!(
        points[1].engineering.value,
        Some(Scalar::Decimal("3.5".into()))
    );
}

#[test]
fn ambiguous_runtime_dependencies_and_missing_point_tables_preserve_definitions() {
    let dir = Fixture::new();
    root(&dir, "N", "");
    let root = std::fs::read_to_string(dir.path().join("pcf.dat")).unwrap();
    let mode = common::PARAMETER.replace("TEMP", "MODE");
    dir.write("pcf.dat", &format!("{root}\n{mode}\n{mode}"));
    dir.write("cur.dat", "TEMP\t1\tMODE\t0\tA");
    dir.write("caf.dat", "A\tCurve\tR\tU");
    let p = parameter(&dir);
    let a = p.calibrations.value.unwrap();
    let targets = &a[0].condition.as_ref().unwrap().dependencies[0].targets;
    assert_eq!(targets.value.as_ref().unwrap().len(), 2);
    assert!(
        matches!(&targets.problems[0].kind, ProblemKind::AmbiguousReference { alternatives, .. } if alternatives.rest.is_empty())
    );
    let c = a[0].calibration.value.as_ref().unwrap();
    assert_eq!(c.definition.source.file.to_str(), Some("caf.dat"));
    let CalibrationForm::Numerical { points, .. } = c.form.value.as_ref().unwrap() else {
        panic!()
    };
    assert!(points.value.is_none());
    assert!(matches!(
        points.problems[0].kind,
        ProblemKind::MissingReference { .. }
    ));
}

#[test]
fn simultaneous_direct_and_conditional_declarations_keep_the_declared_family() {
    let dir = Fixture::new();
    // Both a PCF_CURTX reference and CUR rows name calibrations, so both sets stay
    // available and the disagreement is attached. PCF_CATEG still names the family, so
    // the TXF definition sharing the key is not this numerical parameter's calibration.
    root(&dir, "N", "A");
    dir.write("caf.dat", "A\tCurve\tR\tU\tH");
    dir.write("cap.dat", "A\t0\t1");
    dir.write("txf.dat", "A\tStatus\tU");
    dir.write("txp.dat", "A\t0\t0\tIdle");
    dir.write("cur.dat", "TEMP\t1\tTEMP\t0\tA");
    let a = parameter(&dir).calibrations.value.unwrap();
    assert_eq!(a.len(), 2); // The conditional row and the direct reference.
    assert!(a[0].condition.is_some() && a[1].selection.is_none());
    for a in a {
        assert!(matches!(
            a.calibration.value.unwrap().form.value,
            Some(CalibrationForm::Numerical { .. })
        ));
        assert!(
            a.calibration
                .problems
                .iter()
                .any(|p| matches!(p.kind, ProblemKind::InconsistentDefinition { .. }))
        );
    }
}

#[test]
fn status_definitions_fall_back_to_numerical_families_with_a_problem() {
    let dir = Fixture::new();
    // A status parameter may not silently keep a numerical calibration: the disagreement stays visible.
    root(&dir, "S", "A");
    dir.write("mcf.dat", "A\tPolynomial\t1.5");
    let a = parameter(&dir).calibrations.value.unwrap();
    let c = a[0].calibration.value.as_ref().unwrap();
    assert!(matches!(
        c.form.value,
        Some(CalibrationForm::Polynomial { .. })
    ));
    assert!(
        a[0].calibration
            .problems
            .iter()
            .any(|p| matches!(p.kind, ProblemKind::InconsistentDefinition { .. }))
    );
}

#[test]
fn every_competing_row_shares_one_duplicate_position_problem() {
    let dir = Fixture::new();
    root(&dir, "N", "");
    dir.write(
        "cur.dat",
        "TEMP\t1\tMISSING\t0\tA\nTEMP\t1\tMISSING\t1\tB\nTEMP\t1\tMISSING\t2\tC",
    );
    dir.write("mcf.dat", "A\tFirst\t1\nB\tSecond\t2\nC\tThird\t3");
    let p = parameter(&dir);
    let a = p.calibrations.value.unwrap();
    assert_eq!(a.len(), 3);
    let [problem] = &p.calibrations.problems[..] else {
        panic!("one problem covers the shared position");
    };
    let ProblemKind::InconsistentDefinition {
        values, available, ..
    } = &problem.kind
    else {
        panic!()
    };
    assert_eq!(values, &[Scalar::Integer(1)]);
    assert_eq!(available.len(), 3);
    assert_eq!(problem.sources.len(), 3);
}

#[test]
fn textual_and_numerical_namespaces_can_share_a_key() {
    let dir = Fixture::new();
    dir.write("caf.dat", "A\tCurve\tR\tU\tD");
    dir.write("cap.dat", "A\t0\t1");
    dir.write("txf.dat", "A\tStatus\tU");
    dir.write("txp.dat", "A\t0\t0\tIdle");
    for category in ["N", "S"] {
        for conditional in [false, true] {
            root(&dir, category, if conditional { "" } else { "A" });
            dir.write(
                "cur.dat",
                if conditional {
                    "TEMP\t1\tTEMP\t0\tA"
                } else {
                    ""
                },
            );
            let a = parameter(&dir).calibrations.value.unwrap();
            assert_eq!(a.len(), 1);
            assert!(!a[0].calibration.problems.iter().any(|p| matches!(
                p.kind,
                ProblemKind::AmbiguousReference { .. } | ProblemKind::InconsistentDefinition { .. }
            )));
            assert_eq!(
                matches!(
                    a[0].calibration.value.as_ref().unwrap().form.value,
                    Some(CalibrationForm::Textual { .. })
                ),
                category == "S"
            );
        }
    }
}
