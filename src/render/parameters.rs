//! The parameter view: the parameter's own summary, the packets that contain it and where it
//! occurs in each, and its declared calibration alternatives.

use mibl::model::*;
use std::{
    collections::BTreeMap,
    io::{self, Write},
};

use super::{
    calibrations::calibration_lines,
    evidence::View,
    format::{
        field_value, location_text, markers, parameter_summary, reference, repeat, scalar, source,
        string, table, text, width,
    },
};

pub(crate) fn parameter(
    p: &ParameterDescription,
    details: bool,
    out: &mut dyn Write,
) -> io::Result<()> {
    let mut view = View::default();
    view.parameter(&p.parameter, "Parameter");
    writeln!(out, "Parameter {}", text(&p.parameter.name.0))?;
    for line in parameter_summary(&p.parameter) {
        writeln!(out, "{line}")?;
    }
    writeln!(
        out,
        "Source: {}\n\nPackets",
        source(&p.parameter.definition.source)
    )?;
    view.info(&p.occurrences, "Packets");
    let mut rows = vec![vec![
        "SPID".into(),
        "Name".into(),
        "Location".into(),
        "Width".into(),
        "Repeat".into(),
    ]];
    let mut seen = BTreeMap::new();
    if let Some(packets) = &p.occurrences.value {
        for packet in packets {
            view.info(&packet.packet, "Containing packet");
            if let Some(p) = &packet.packet.value {
                view.packet(p, &format!("Packet {}", p.spid.0));
            }
            for o in &packet.occurrences {
                let spid = packet
                    .packet
                    .value
                    .as_ref()
                    .map(|p| p.spid.0.to_string())
                    .or_else(|| field_value(&o.definition, "PLF_SPID").map(scalar))
                    .unwrap_or_else(|| "unavailable".into());
                let context = format!(
                    "Packet {spid} {} at {}",
                    text(&o.reference.0),
                    location_text(o)
                );
                let ids = view.occurrence(o, &context);
                if let Some(parameter) = &o.parameter.value {
                    view.parameter(parameter, "Parameter");
                }
                rows.push(vec![
                    spid,
                    packet
                        .packet
                        .value
                        .as_ref()
                        .map_or_else(|| "unavailable".into(), |p| string(&p.name)),
                    location_text(o),
                    width(&o.location.encoded_bits),
                    format!("{}{}", repeat(o, &mut seen), markers(&ids)),
                ]);
            }
        }
        if rows.len() == 1 {
            writeln!(out, "none")?;
        } else {
            table(&rows, out)?;
        }
    } else {
        writeln!(out, "unavailable")?;
    }
    writeln!(out, "\nCalibrations")?;
    match &p.parameter.calibrations.value {
        Some(alternatives) if alternatives.is_empty() => writeln!(out, "none declared")?,
        Some(alternatives) => {
            for (i, a) in alternatives.iter().enumerate() {
                writeln!(out, "Alternative {}", i + 1)?;
                if let Some(condition) = &a.condition {
                    writeln!(
                        out,
                        "  Condition: {} [runtime dependent]",
                        text(&condition.expression)
                    )?;
                }
                if let Some(c) = &a.calibration.value {
                    writeln!(
                        out,
                        "  {} at {}",
                        reference(&c.reference),
                        source(&c.definition.source)
                    )?;
                    for line in calibration_lines(c) {
                        writeln!(out, "  {line}")?;
                    }
                } else {
                    writeln!(out, "  unavailable")?;
                }
            }
        }
        None => writeln!(out, "unavailable")?,
    }
    view.finish(details, out)
}
