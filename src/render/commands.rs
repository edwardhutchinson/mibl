//! The command view: the command's own summary, its application-data arguments with their
//! declared rules, and the expanded packet header.

use mibl::model::*;
use std::io::{self, Write};

use super::{
    calibrations::calibration_lines,
    evidence::View,
    format::{
        default_suffix, encoding_kind, field_value, markers, number, position, recorded_text,
        reference, scalar, scalar_value, source, string, table, text, width,
    },
};

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
        command_layout(layout, &mut view, &mut rows, &mut rules, 0);
        if rows.len() == 1 {
            writeln!(out, "No elements declared")?;
        } else if nested(layout) {
            // Declared groups print as two-space nested blocks so group membership stays visible.
            for row in rows.iter().skip(1) {
                writeln!(out, "{}", row.join("  "))?;
            }
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
    command_header(&c.header, &mut view, out)?;
    view.finish(details, out)
}

/// The expanded packet header, separate from the application-data arguments: the TCP
/// declaration and its PCDF elements in declared offset order. A command whose header cannot
/// be resolved keeps its other information and reports the problem here.
fn command_header(
    header: &Info<CommandHeader>,
    view: &mut View,
    out: &mut dyn Write,
) -> io::Result<()> {
    writeln!(out, "\nHeader")?;
    let ids = view.info(header, "Header");
    let Some(header) = &header.value else {
        return writeln!(out, "unavailable{}", markers(&ids));
    };
    view.definition(&header.definition, "Command header");
    writeln!(
        out,
        "TCP {}  {}\nSource: {}{}",
        text(recorded_text(&header.definition, "TCP_ID").unwrap_or("unavailable")),
        text(recorded_text(&header.definition, "TCP_DESC").unwrap_or("unavailable")),
        source(&header.definition.source),
        markers(&ids)
    )?;
    let field_ids = view.info(&header.fields, "Header elements");
    let mut rows = vec![vec![
        "Field".into(),
        "Parameter".into(),
        "Location".into(),
        "Width".into(),
        "Kind".into(),
        "Value".into(),
    ]];
    if let Some(fields) = &header.fields.value {
        if fields.is_empty() {
            writeln!(out, "No header elements declared")?;
        } else {
            for field in fields {
                header_field(field, view, &mut rows);
            }
            table(&rows, out)?;
        }
    } else {
        writeln!(out, "unavailable{}", markers(&field_ids))?;
    }
    Ok(())
}

fn header_field(field: &HeaderField, view: &mut View, rows: &mut Vec<Vec<String>>) {
    let description = text(recorded_text(&field.definition, "PCDF_DESC").unwrap_or("unavailable"));
    let context = format!(
        "Header element {description} at {}",
        position(&field.location.position)
    );
    view.definition(&field.definition, &context);
    let kind_ids = view.info(&field.field_kind, &format!("{context} kind"));
    let parameter_ids = view.info(&field.parameter, &format!("{context} parameter"));
    if let Some(target) = &field.parameter.value {
        view.target(target, &format!("{context} parameter"));
    }
    let ids = view.location(&field.location, &context);
    let value = header_value(field, view, &format!("{context} value"));
    rows.push(vec![
        description,
        match recorded_text(&field.definition, "PCDF_PNAME") {
            Some(name) => format!("{}{}", text(name), markers(&parameter_ids)),
            None => markers(&parameter_ids).trim_start().to_owned(),
        },
        format!("{}{}", position(&field.location.position), markers(&ids)),
        width(&field.location.encoded_bits),
        format!("{}{}", kind_label(&field.field_kind), markers(&kind_ids)),
        value,
    ]);
}

/// The header element types ICD 7.0 declares, with the source each value is taken from when
/// a command is loaded.
fn kind_label(kind: &Info<String>) -> String {
    match kind.value.as_deref() {
        Some("F") => "F fixed".into(),
        Some("A") => "A APID from CCF_APID".into(),
        Some("T") => "T service type from CCF_TYPE".into(),
        Some("S") => "S service subtype from CCF_STYPE".into(),
        Some("K") => "K acknowledgement flags from CCF_ACK".into(),
        Some("P") => "P set automatically at invocation".into(),
        Some(other) => text(other),
        None => "unavailable".into(),
    }
}

/// The recorded value of one header element, which the shared argument-value renderer already
/// prints with its representation and declaration evidence. A fixed area's value is the fixed
/// content of the header, while a parameter element's recorded value is the default its command
/// definition or the command subsystem may replace at invocation.
fn header_value(field: &HeaderField, view: &mut View, context: &str) -> String {
    let value = argument_value(&field.value, view, context);
    let declared_default = field.field_kind.value.as_deref() != Some("F")
        && matches!(
            field.value.value.as_ref().map(|value| &value.source),
            Some(ValueSource::Literal(_))
        );
    if declared_default {
        format!("{value} (declared default)")
    } else {
        value
    }
}

/// Declared repetition and condition structures print as nested blocks, not as one flat table.
fn nested(layout: &[Layout<CommandElement>]) -> bool {
    layout
        .iter()
        .any(|node| !matches!(node, Layout::Element(_)))
}

fn command_layout(
    layout: &[Layout<CommandElement>],
    view: &mut View,
    rows: &mut Vec<Vec<String>>,
    rules: &mut Vec<String>,
    depth: usize,
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
                    format!("{}{name}", indent(depth)),
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
                    format!("{}Fixed area", indent(depth)),
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
                let ids = view.info(repetition, "Command repetition");
                rows.push(vec![
                    format!("{}Repeat group", indent(depth)),
                    source(&definition.source),
                    format!("{}{}", repetition_label(repetition), markers(&ids)),
                ]);
                command_layout(children, view, rows, rules, depth + 1);
                rows.push(vec![format!("{}End repeat", indent(depth))]);
            }
            Layout::Conditional {
                definition,
                condition,
                children,
            } => {
                view.definition(definition, "Command condition");
                view.declaration(condition, "Command condition");
                rows.push(vec![
                    format!("{}Conditional structure", indent(depth)),
                    source(&definition.source),
                    text(&condition.expression),
                ]);
                command_layout(children, view, rows, rules, depth + 1);
            }
        }
    }
}

/// Declared groups nest by two spaces per level, matching the packet layout convention.
fn indent(depth: usize) -> String {
    "  ".repeat(depth)
}

fn repetition_label(repetition: &Info<Repetition>) -> String {
    match &repetition.value {
        Some(Repetition::Fixed { count, stride_bits }) => {
            format!("fixed count {count}, stride {}", width(stride_bits))
        }
        Some(Repetition::Runtime(declaration)) => {
            format!("runtime {}", text(&declaration.expression))
        }
        None => "unavailable".into(),
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
