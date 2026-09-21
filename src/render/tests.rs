use mibl::model::*;

fn info<T>(value: T) -> Info<T> {
    Info {
        value: Some(value),
        problems: vec![],
        sources: vec![],
    }
}

fn definition(file: &str, line: usize) -> Definition {
    Definition {
        source: Source {
            file: file.into(),
            line: line.try_into().unwrap(),
        },
        fields: vec![],
    }
}

fn parameter_result() -> ParameterDescription {
    ParameterDescription {
        parameter: ParameterSummary {
            name: ParameterName("TEST".into()),
            definition: definition("pcf.dat", 1),
            description: info("Readable 界\tline\nnext\r\u{1b}\\\"".into()),
            encoding: Encoding {
                ptc: info(3),
                pfc: info(4),
                endian: info("B".into()),
                encoded_bits: info(8),
            },
            units: info("K".into()),
            calibrations: info(vec![]),
        },
        occurrences: info(vec![]),
    }
}

fn missing(key: &str) -> Problem {
    Problem {
        kind: ProblemKind::MissingReference {
            reference: Reference::Supporting {
                table: Table::Caf,
                key: key.into(),
            },
        },
        sources: vec![],
        explanation: format!("Missing {key}"),
    }
}

#[test]
fn nested_summary_problems_keep_known_values_and_all_occurrence_contexts() {
    let mut result = parameter_result();
    let mut parameter = result.parameter.clone();
    parameter.units.problems.push(missing("units evidence"));
    let problem = missing("field evidence");
    parameter.definition.fields.push(RecordedField {
        column: 4.try_into().unwrap(),
        presence: Presence::Text("K".into()),
        meanings: vec![FieldMeaning {
            schema_name: "PCF_UNIT".into(),
            interpretation: Info {
                value: Some(Interpretation {
                    value: Scalar::Text("K".into()),
                    origin: InterpretationOrigin::Recorded,
                }),
                problems: vec![problem],
                sources: vec![],
            },
        }],
    });
    result.parameter = parameter.clone();
    // The occurrence carries an additional result-local problem, independent of its record.
    parameter
        .units
        .problems
        .push(missing("occurrence-only evidence"));
    result.occurrences.value = Some(vec![PacketOccurrences {
        packet: info(PacketSummary {
            spid: PacketSpid(42),
            name: info("HK".into()),
            description: info("Housekeeping".into()),
            definition: definition("pid.dat", 1),
            characteristics: info(vec![]),
        }),
        occurrences: [10, 11]
            .into_iter()
            .map(|byte| ParameterOccurrence {
                reference: ParameterName("TEST".into()),
                parameter: info(parameter.clone()),
                definition: definition("plf.dat", byte as usize),
                location: Location {
                    position: info(Position::PacketAbsolute { byte, bit: 0 }),
                    encoded_bits: info(8),
                    constraints: vec![],
                },
                enclosing: vec![],
                enclosing_definitions: vec![],
            })
            .collect(),
    }]);
    let mut out = Vec::new();
    super::parameter(&result, true, &mut out).unwrap();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("Units: K"));
    assert!(text.contains("Missing occurrence-only evidence"), "{text}");
    let overview = text.split("\nProblem evidence\n").next().unwrap();
    let field_problem = overview.lines().find(|l| l.contains("PCF_UNIT")).unwrap();
    assert!(
        field_problem.contains("byte 10 bit 0") && field_problem.contains("byte 11 bit 0"),
        "{field_problem}"
    );
    assert_eq!(text.matches("[D1] pcf.dat:1\n").count(), 1);
    assert!(text.contains("Readable 界\\tline\\nnext\\r\\u{1b}\\\""));
}

#[test]
fn runtime_dependencies_and_ambiguous_recorded_meanings_keep_full_evidence() {
    let mut result = parameter_result();
    let mut target = definition("pic.dat", 7);
    target.fields.push(RecordedField {
        column: 3.try_into().unwrap(),
        presence: Presence::Text("raw\t\n\r\u{1b}\\\"界".into()),
        meanings: vec![
            FieldMeaning {
                schema_name: "FIRST_MEANING".into(),
                interpretation: info(Interpretation {
                    value: Scalar::Integer(7),
                    origin: InterpretationOrigin::Recorded,
                }),
            },
            FieldMeaning {
                schema_name: "OTHER_MEANING".into(),
                interpretation: Info {
                    value: None,
                    problems: vec![missing("nested field target")],
                    sources: vec![],
                },
            },
        ],
    });
    let reference = Reference::Supporting {
        table: Table::Pic,
        key: "3/25".into(),
    };
    let runtime = RuntimeDeclaration {
        expression: "COUNT\t+ 1\n界".into(),
        sources: vec![],
        dependencies: vec![Dependency {
            reference: reference.clone(),
            targets: info(vec![Target {
                reference: reference.clone(),
                definition: target.clone(),
            }]),
        }],
    };
    result
        .parameter
        .encoding
        .encoded_bits
        .problems
        .push(Problem {
            kind: ProblemKind::RuntimeDependent {
                declaration: runtime,
            },
            sources: vec![],
            explanation: "Runtime constraint".into(),
        });
    result.parameter.units.problems.push(Problem {
        kind: ProblemKind::AmbiguousReference {
            reference: reference.clone(),
            alternatives: AtLeastTwo {
                first: Box::new(Target {
                    reference: reference.clone(),
                    definition: target.clone(),
                }),
                second: Box::new(Target {
                    reference,
                    definition: definition("pic.dat", 8),
                }),
                rest: vec![],
            },
        },
        sources: vec![],
        explanation: "Both candidates retained".into(),
    });
    for details in [false, true] {
        let mut out = Vec::new();
        super::parameter(&result, details, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("Encoding: unsigned integer, 8 bits"));
        assert!(text.contains("OTHER_MEANING") && text.contains("runtime dependent"));
        assert!(!text.chars().any(|c| c.is_control() && c != '\n'));
        if details {
            for expected in [
                "COUNT\\t+ 1\\n界",
                "dependency PIC 3/25",
                "pic.dat:7 [D2]",
                "pic.dat:8 [D3]",
                "Meaning: FIRST_MEANING",
                "Meaning: OTHER_MEANING",
                "Interpreted: unavailable",
                "Missing nested field target",
                "Recorded: text \"raw\\t\\n\\r\\u{1b}\\\\\\\"界\"",
            ] {
                assert!(text.contains(expected), "missing {expected}: {text}");
            }
            assert_eq!(text.matches("[D2] pic.dat:7\n").count(), 1);
            assert!(
                text.contains("Used by: [P1] Parameter width runtime, dependency PIC 3/25")
                    && text.contains("candidate")
            );
        }
    }
}

#[test]
fn shared_problem_targets_keep_each_parent_context() {
    let mut result = parameter_result();
    let mut d = definition("pic.dat", 1);
    d.fields.push(RecordedField {
        column: 1.try_into().unwrap(),
        presence: Presence::Empty,
        meanings: vec![FieldMeaning {
            schema_name: "TARGET_FIELD".into(),
            interpretation: Info {
                value: None,
                problems: vec![missing("field")],
                sources: vec![],
            },
        }],
    });
    let target = Target {
        reference: Reference::Supporting {
            table: Table::Pic,
            key: "shared".into(),
        },
        definition: d,
    };
    let problem = Problem {
        kind: ProblemKind::AmbiguousReference {
            reference: target.reference.clone(),
            alternatives: AtLeastTwo {
                first: Box::new(target.clone()),
                second: Box::new(Target {
                    definition: definition("pic.dat", 2),
                    ..target
                }),
                rest: vec![],
            },
        },
        explanation: "Shared candidates".into(),
        sources: vec![],
    };
    result.parameter.description.problems.push(problem.clone());
    result.parameter.units.problems.push(problem);
    let mut out = Vec::new();
    super::parameter(&result, true, &mut out).unwrap();
    let text = String::from_utf8(out).unwrap();
    let overview = text.split("\nProblem evidence\n").next().unwrap();
    let nested = overview
        .lines()
        .find(|l| l.contains("TARGET_FIELD"))
        .unwrap();
    assert!(
        nested.contains("Parameter description candidate")
            && nested.contains("Parameter units candidate"),
        "{nested}"
    );
    assert_eq!(text.matches("[D2] pic.dat:1\n").count(), 1);
}
