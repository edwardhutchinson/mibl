use super::*;

pub(crate) fn command(
    c: &CommandDescription,
    details: bool,
    out: &mut dyn Write,
) -> io::Result<()> {
    let mut view = View::default();
    view.definition(&c.definition, "Command");
    view.info(&c.description, "Command description");
    writeln!(
        out,
        "Command {}\nDescription: {}\nSource: {}",
        text(&c.name.0),
        string(&c.description),
        source(&c.definition.source)
    )?;
    view.info(&c.arguments, "Application data");
    writeln!(out, "\nApplication data")?;
    let mut rules = Vec::new();
    if let Some(layout) = &c.arguments.value {
        let mut rows = vec![vec![
            "Element".into(),
            "Description".into(),
            "Location".into(),
            "Width".into(),
            "Type".into(),
            "Units".into(),
            "CPC default".into(),
            "Element value".into(),
        ]];
        command_layout(layout, &mut view, &mut rows, &mut rules);
        if rows.len() == 1 {
            writeln!(out, "No elements declared")?;
        } else {
            table(&rows, out)?;
        }
    } else {
        writeln!(out, "unavailable")?;
    }
    writeln!(out, "\nArgument rules")?;
    if c.arguments.value.is_none() {
        // An unavailable layout is not a command that declares no rules.
        writeln!(out, "unavailable")?;
    } else if rules.is_empty() {
        writeln!(out, "none declared")?;
    } else {
        for line in &rules {
            writeln!(out, "{line}")?;
        }
    }
    let ids = view.info(&c.header, "Header");
    writeln!(out, "\nHeader: unavailable{}", markers(&ids))?;
    view.finish(details, out)
}

fn command_layout(
    layout: &[Layout<CommandElement>],
    view: &mut View,
    rows: &mut Vec<Vec<String>>,
    rules: &mut Vec<String>,
) {
    for item in layout {
        match item {
            Layout::Element(CommandElement::Argument(a)) => {
                let name = match &a.reference {
                    Reference::Supporting { key, .. } => text(key),
                    other => reference(other),
                };
                let context = format!("{name} at {}", position(&a.location.position));
                view.definition(&a.element, &context);
                view.info(&a.definition, &context);
                if let Some(d) = &a.definition.value {
                    view.definition(d, &context);
                }
                view.info(&a.description, &format!("{context} description"));
                view.info(&a.units, &format!("{context} units"));
                view.info(&a.encoding.ptc, &format!("{context} PTC"));
                view.info(&a.encoding.pfc, &format!("{context} PFC"));
                view.info(&a.encoding.endian, &format!("{context} endian"));
                view.info(
                    &a.encoding.encoded_bits,
                    &format!("{context} encoding width"),
                );
                let ids = view.location(&a.location, &context);
                let default =
                    argument_value(&a.rules.default, view, &format!("{context} CPC default"));
                let value = argument_value(
                    &a.rules.element_value,
                    view,
                    &format!("{context} element value"),
                );
                // The rule families register their own definitions and problems under this context.
                for d in &a.rules.supporting_definitions {
                    view.definition(d, &format!("{context} rules"));
                }
                argument_rules(a, view, &context, rules);
                rows.push(vec![
                    name,
                    string(&a.description),
                    format!("{}{}", position(&a.location.position), markers(&ids)),
                    width(&a.location.encoded_bits),
                    format!(
                        "{} (PTC {} / PFC {})",
                        encoding_kind(&a.encoding),
                        number(&a.encoding.ptc),
                        number(&a.encoding.pfc)
                    ),
                    string(&a.units),
                    default,
                    value,
                ]);
            }
            Layout::Element(CommandElement::Fixed(f)) => {
                let context = format!("Fixed area at {}", position(&f.location.position));
                view.definition(&f.definition, &context);
                let ids = view.location(&f.location, &context);
                let value = argument_value(&f.value, view, &context);
                let description = field_value(&f.definition, "CDF_DESCR")
                    .map(scalar)
                    .unwrap_or_else(|| "unavailable".into());
                rows.push(vec![
                    "Fixed area".into(),
                    description,
                    format!("{}{}", position(&f.location.position), markers(&ids)),
                    width(&f.location.encoded_bits),
                    "fixed bits".into(),
                    "".into(),
                    "".into(),
                    value,
                ]);
            }
            Layout::Repeat {
                definition,
                repetition,
                children,
            } => {
                view.definition(definition, "Command repetition");
                view.repetition(repetition, "Command repetition");
                command_layout(children, view, rows, rules);
            }
            Layout::Conditional {
                definition,
                condition,
                children,
            } => {
                view.definition(definition, "Command condition");
                view.declaration(condition, "Command condition");
                command_layout(children, view, rows, rules);
            }
        }
    }
}

/// The declared range, alias and conversion rules of one argument, in nested blocks. An
/// argument with no declared rules contributes nothing, while an unavailable family keeps
/// its problem markers and the argument's other information.
fn argument_rules(a: &CommandArgument, view: &mut View, context: &str, lines: &mut Vec<String>) {
    let range_ids = view.info(&a.rules.ranges, &format!("{context} ranges"));
    let alias_ids = view.info(&a.rules.aliases, &format!("{context} aliases"));
    let calibration_ids = view.info(&a.rules.calibrations, &format!("{context} calibrations"));
    if declared_empty(a.rules.ranges.value.as_ref())
        && declared_empty(a.rules.aliases.value.as_ref())
        && declared_empty(a.rules.calibrations.value.as_ref())
    {
        return;
    }
    lines.push(context.to_owned());
    match &a.rules.ranges.value {
        Some(ranges) if ranges.is_empty() => {}
        Some(ranges) => {
            lines.push(format!("  Ranges{}", markers(&range_ids)));
            for range in ranges {
                let context = format!("{context} range");
                view.definition(&range.definition, &context);
                let mut ids = view.info(&range.low, &context);
                ids.extend(view.info(&range.high, &context));
                ids.sort_unstable();
                ids.dedup();
                lines.push(format!(
                    "    {} .. {} [{}] [{}]{}",
                    scalar_value(&range.low),
                    scalar_value(&range.high),
                    string(&range.representation),
                    source(&range.definition.source),
                    markers(&ids)
                ));
            }
        }
        None => {
            lines.push(format!("  Ranges{}", markers(&range_ids)));
            lines.push("    unavailable".into());
        }
    }
    match &a.rules.aliases.value {
        Some(aliases) if aliases.is_empty() => {}
        Some(aliases) => {
            lines.push(format!("  Aliases{}", markers(&alias_ids)));
            for alias in aliases {
                let context = format!("{context} alias");
                view.definition(&alias.definition, &context);
                let ids = view.info(&alias.raw, &context);
                lines.push(format!(
                    "    {} -> {} [{}]{}",
                    scalar_value(&alias.raw),
                    string(&alias.text),
                    source(&alias.definition.source),
                    markers(&ids)
                ));
            }
        }
        None => {
            lines.push(format!("  Aliases{}", markers(&alias_ids)));
            lines.push("    unavailable".into());
        }
    }
    match &a.rules.calibrations.value {
        Some(alternatives) if alternatives.is_empty() => {}
        Some(alternatives) => {
            lines.push(format!("  Calibrations{}", markers(&calibration_ids)));
            for alternative in alternatives {
                match &alternative.calibration.value {
                    Some(calibration) => {
                        let context = format!("{context} {}", reference(&calibration.reference));
                        let ids = view.info(&alternative.calibration, &context);
                        view.calibration(calibration, &context);
                        lines.push(format!(
                            "    {} at {}{}",
                            reference(&calibration.reference),
                            source(&calibration.definition.source),
                            markers(&ids)
                        ));
                        lines.extend(
                            calibration_lines(calibration)
                                .into_iter()
                                .map(|line| format!("    {line}")),
                        );
                    }
                    None => {
                        let ids =
                            view.info(&alternative.calibration, &format!("{context} calibrations"));
                        lines.push(format!("    unavailable{}", markers(&ids)));
                    }
                }
            }
        }
        None => {
            lines.push(format!("  Calibrations{}", markers(&calibration_ids)));
            lines.push("    unavailable".into());
        }
    }
}

/// A family that is declared without entries says none, which needs no block of its own.
fn declared_empty<T>(value: Option<&Vec<T>>) -> bool {
    value.is_some_and(Vec::is_empty)
}

fn argument_value(i: &Info<ArgumentValue>, view: &mut View, context: &str) -> String {
    let ids = view.info(i, context);
    let value = match &i.value {
        None => "unavailable".into(),
        Some(v) => {
            view.definition(&v.definition, context);
            view.info(&v.representation, &format!("{context} representation"));
            match &v.source {
                ValueSource::Literal(s) => {
                    let default = ["CPC_INTER", "CDF_INTER"]
                        .into_iter()
                        .map(|name| default_suffix(&v.definition, name))
                        .find(|suffix| !suffix.is_empty())
                        .unwrap_or("");
                    format!("{} [{}{default}]", scalar(s), string(&v.representation))
                }
                ValueSource::Telemetry {
                    parameter,
                    declaration,
                } => {
                    view.declaration(declaration, context);
                    format!("telemetry {}", text(&parameter.0))
                }
                ValueSource::Runtime(declaration) => {
                    view.declaration(declaration, context);
                    "runtime input".into()
                }
            }
        }
    };
    format!("{value}{}", markers(&ids))
}
