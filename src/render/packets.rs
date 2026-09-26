//! The packet view: identification criteria and the flattened layout of the packet's parameter
//! occurrences, including declared repetition and condition groups.

use super::sections::Role;
use mibl::model::*;
use std::{
    collections::BTreeMap,
    io::{self, Write},
};

use super::{
    evidence::View,
    format::{
        default_suffix, field_value, location_text, markers, number, position, repeat, source,
        string, table, text, width,
    },
};

pub(crate) fn packet(p: &PacketDescription, details: bool, out: &mut dyn Write) -> io::Result<()> {
    packet_sections(p, details, &mut super::sections::Plain(out))
}

pub(crate) fn packet_sections(
    p: &PacketDescription,
    details: bool,
    out: &mut dyn super::sections::Output,
) -> io::Result<()> {
    out.section("Summary", Role::Summary)?;
    let mut view = View::default();
    view.packet(&p.packet, "Packet");
    writeln!(
        out,
        "Packet {}  {}\nDescription: {}\nSource: {}",
        p.packet.spid.0,
        string(&p.packet.name),
        string(&p.packet.description),
        source(&p.packet.definition.source)
    )?;
    let i = &p.identification;
    for d in &i.definitions {
        view.definition(d, "Identification");
    }
    view.info(&i.apid, "APID");
    view.info(&i.service_type, "Service type");
    view.info(&i.service_subtype, "Service subtype");
    out.section("Identification", Role::Content)?;
    writeln!(
        out,
        "APID: {}{}\nService: type {}{}, subtype {}{}",
        number(&i.apid),
        default_suffix(&p.packet.definition, "PID_APID"),
        number(&i.service_type),
        default_suffix(&p.packet.definition, "PID_TYPE"),
        number(&i.service_subtype),
        default_suffix(&p.packet.definition, "PID_STYPE")
    )?;
    match &i.criteria.value {
        Some(cs) if cs.is_empty() => writeln!(out, "Additional criteria: none declared")?,
        Some(cs) => {
            for (index, c) in cs.iter().enumerate() {
                // The result omits disabled criteria; identify PI2 by its retained PID field sources/values and PIC declarations.
                let pi = criterion_number(i, index);
                let context = format!("PI{pi} extraction");
                let ids = view.location(&c.extraction, &context);
                view.info(&c.expected, &format!("PI{pi} expected"));
                for d in &c.definitions {
                    view.definition(d, &format!("PI{pi}"));
                }
                writeln!(
                    out,
                    "PI{pi} expected: {}{}; location: {}; width: {}{}",
                    number(&c.expected),
                    default_suffix(&p.packet.definition, &format!("PID_PI{pi}_VAL")),
                    position(&c.extraction.position),
                    width(&c.extraction.encoded_bits),
                    markers(&ids)
                )?;
            }
        }
        None => writeln!(out, "Additional criteria: unavailable")?,
    }
    out.section("Layout", Role::availability(&p.layout.value))?;
    view.info(&p.layout, "Layout");
    let mut occurrences = Vec::new();
    let mut rows = vec![vec![
        "Parameter".into(),
        "Location".into(),
        "Width".into(),
        "Repeat".into(),
    ]];
    if let Some(layout) = &p.layout.value {
        collect_layout(
            layout,
            &mut occurrences,
            &mut view,
            &mut rows,
            &mut BTreeMap::new(),
            0,
        );
    }
    if rows.len() > 1 {
        if p.layout.value.as_ref().is_some_and(|layout| {
            layout
                .iter()
                .any(|node| !matches!(node, Layout::Element(_)))
        }) {
            for row in rows.iter().skip(1) {
                writeln!(out, "{}", row.join("  "))?;
            }
        } else {
            table(&rows, out)?;
        }
    } else {
        writeln!(
            out,
            "{}",
            if p.layout.value.is_some() {
                "none"
            } else {
                "unavailable"
            }
        )?;
    }
    for o in occurrences {
        if let Some(parameter) = &o.parameter.value {
            view.parameter(parameter, &text(&o.reference.0));
        }
    }
    view.info(&i.criteria, "Identification criteria");
    view.finish(details, out)
}

fn criterion_number(i: &PacketIdentification, index: usize) -> usize {
    if index == 0
        && !i.definitions.is_empty()
        && i.definitions
            .iter()
            .all(|d| matches!(field_value(d, "PIC_PI1_OFF"), Some(Scalar::Integer(-1))))
    {
        2
    } else {
        index + 1
    }
}

fn collect_layout<'a>(
    layout: &'a [Layout<ParameterOccurrence>],
    occurrences: &mut Vec<&'a ParameterOccurrence>,
    view: &mut View,
    rows: &mut Vec<Vec<String>>,
    seen: &mut BTreeMap<Source, u64>,
    depth: usize,
) {
    for item in layout {
        match item {
            Layout::Element(o) => {
                occurrences.push(o);
                let context = format!("{} at {}", text(&o.reference.0), location_text(o));
                let ids = view.occurrence(o, &context);
                rows.push(vec![
                    format!("{}{}", "  ".repeat(depth), text(&o.reference.0)),
                    location_text(o),
                    width(&o.location.encoded_bits),
                    format!("{}{}", repeat(o, seen), markers(&ids)),
                ]);
            }
            Layout::Repeat {
                definition,
                repetition,
                children,
            } => {
                view.definition(definition, "Layout repeat");
                view.repetition(repetition, "Layout repeat");
                let label = match &repetition.value {
                    Some(Repetition::Fixed { count, stride_bits }) => {
                        format!("fixed count {count}, stride {}", width(stride_bits))
                    }
                    Some(Repetition::Runtime(r)) => format!("runtime {}", text(&r.expression)),
                    None => "unavailable".into(),
                };
                rows.push(vec![
                    format!("{}Repeat group", "  ".repeat(depth)),
                    source(&definition.source),
                    String::new(),
                    label,
                ]);
                collect_layout(children, occurrences, view, rows, seen, depth + 1);
                rows.push(vec![format!("{}End repeat", "  ".repeat(depth))]);
            }
            Layout::Conditional {
                definition,
                condition,
                children,
            } => {
                view.definition(definition, "Layout condition");
                view.declaration(condition, "Layout condition");
                rows.push(vec![
                    format!("{}Conditional structure", "  ".repeat(depth)),
                    source(&definition.source),
                    String::new(),
                    text(&condition.expression),
                ]);
                collect_layout(children, occurrences, view, rows, seen, depth + 1);
            }
        }
    }
}
