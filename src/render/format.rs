//! Shared text presentation: escaping, recorded and interpreted value formatting, column
//! alignment, default annotations, and the labels that name sources, references, positions and
//! encodings. Feature views and evidence collection both read their text through here, so this
//! module never names a feature entry point or an evidence `View`.
use mibl::model::*;
use std::{
    collections::BTreeMap,
    io::{self, Write},
};
use unicode_width::UnicodeWidthStr;

pub(super) fn text(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_control() {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}
pub(super) fn quoted(s: &str) -> String {
    let escaped: String = s
        .chars()
        .map(|c| match c {
            '\\' => "\\\\".into(),
            '"' => "\\\"".into(),
            c if c.is_control() => c.escape_default().to_string(),
            c => c.to_string(),
        })
        .collect();
    format!("\"{escaped}\"")
}

pub(super) fn source(s: &Source) -> String {
    format!("{}:{}", text(&s.file.to_string_lossy()), s.line)
}
pub(super) fn number<T: std::fmt::Display>(i: &Info<T>) -> String {
    i.value
        .as_ref()
        .map_or_else(|| "unavailable".into(), ToString::to_string)
}
pub(super) fn string(i: &Info<String>) -> String {
    i.value
        .as_deref()
        .map_or_else(|| "unavailable".into(), text)
}
pub(super) fn width(i: &Info<u64>) -> String {
    i.value
        .map_or_else(|| "unavailable".into(), |n| format!("{n} bits"))
}
pub(super) fn scalar_value(i: &Info<Scalar>) -> String {
    i.value
        .as_ref()
        .map_or_else(|| "unavailable".into(), scalar)
}
pub(super) fn scalar(s: &Scalar) -> String {
    match s {
        Scalar::Text(s) | Scalar::Code(s) | Scalar::Decimal(s) => quoted(s),
        Scalar::Integer(n) => n.to_string(),
        Scalar::Unsigned(n) => n.to_string(),
        Scalar::Boolean(b) => b.to_string(),
    }
}
/// Rows aligned on display width, with two spaces between columns and no borders or wrapping.
pub(super) fn table(rows: &[Vec<String>], out: &mut dyn Write) -> io::Result<()> {
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let widths: Vec<_> = (0..columns)
        .map(|col| {
            rows.iter()
                .filter_map(|r| r.get(col))
                .map(|s| s.width())
                .max()
                .unwrap_or(0)
        })
        .collect();
    for row in rows {
        for (col, cell) in row.iter().enumerate() {
            write!(out, "{cell}")?;
            if col + 1 < row.len() {
                write!(out, "{}", " ".repeat(widths[col] - cell.width() + 2))?;
            }
        }
        writeln!(out)?;
    }
    Ok(())
}
pub(super) fn default_suffix(d: &Definition, name: &str) -> &'static str {
    if d.fields.iter().flat_map(|f| &f.meanings).any(|m| {
        m.schema_name == name
            && m.interpretation
                .value
                .as_ref()
                .is_some_and(|i| matches!(i.origin, InterpretationOrigin::DocumentedDefault { .. }))
    }) {
        " [default]"
    } else {
        ""
    }
}
pub(super) fn field_value<'a>(d: &'a Definition, name: &str) -> Option<&'a Scalar> {
    d.fields
        .iter()
        .flat_map(|f| &f.meanings)
        .find(|m| m.schema_name == name)?
        .interpretation
        .value
        .as_ref()
        .map(|i| &i.value)
}
/// A recorded textual field, printed without the quoting that marks an interpreted value.
pub(super) fn recorded_text<'a>(d: &'a Definition, name: &str) -> Option<&'a str> {
    match field_value(d, name)? {
        Scalar::Text(s) | Scalar::Code(s) | Scalar::Decimal(s) => Some(s),
        _ => None,
    }
}
pub(super) fn parameter_summary(p: &ParameterSummary) -> Vec<String> {
    let e = &p.encoding;
    let kind = encoding_kind(e);
    let endian = match e.endian.value.as_deref() {
        Some("B") => "big endian".into(),
        Some("L") => "little endian".into(),
        _ => string(&e.endian),
    };
    vec![
        format!(
            "Description: {}{}",
            string(&p.description),
            default_suffix(&p.definition, "PCF_DESCR")
        ),
        format!(
            "Encoding: {kind}, {}, {endian}{} (PTC {} / PFC {})",
            width(&e.encoded_bits),
            default_suffix(&p.definition, "PCF_ENDIAN"),
            number(&e.ptc),
            number(&e.pfc)
        ),
        format!(
            "Units: {}{}",
            string(&p.units),
            default_suffix(&p.definition, "PCF_UNIT")
        ),
    ]
}
pub(super) fn encoding_kind(e: &Encoding) -> &'static str {
    match e.ptc.value {
        Some(1) => "boolean",
        Some(2) => "enumerated",
        Some(3) => "unsigned integer",
        Some(4) => "signed integer",
        Some(5) => "real",
        Some(6) => "bit string",
        Some(7) => "octet string",
        Some(8) => "character string",
        Some(9) => "absolute time",
        Some(10) => "relative time",
        Some(11) => "deduced",
        Some(13) => "saved synthetic",
        _ => "unavailable",
    }
}
pub(super) fn position(p: &Info<Position>) -> String {
    match &p.value {
        Some(Position::PacketAbsolute { byte, bit }) => format!("byte {byte} bit {bit}"),
        Some(Position::ApplicationDeclaredBit(n)) => format!("application-declared bit {n}"),
        Some(Position::HeaderBit(n)) => format!("header bit {n}"),
        Some(Position::RelativeBits(n)) => format!("relative {n} bits"),
        Some(Position::Runtime(r)) => format!("runtime dependent: {}", text(&r.expression)),
        None => "unavailable".into(),
    }
}
pub(super) fn table_label(table: &Table) -> String {
    format!("{table:?}").to_uppercase()
}
pub(super) fn reference(r: &Reference) -> String {
    match r {
        Reference::Root(Identity::Parameter(n)) => format!("PCF NAME {}", text(&n.0)),
        Reference::Root(Identity::Packet(n)) => format!("PID SPID {}", n.0),
        Reference::Root(Identity::Command(n)) => format!("CCF NAME {}", text(&n.0)),
        Reference::Supporting { table, key } => {
            format!("{} {}", table_label(table), text(key))
        }
        Reference::Deferred { concept, key } => format!("{} {}", text(concept), text(key)),
    }
}
/// The problem markers of the ids collected while rendering one value, as ` [P1] [P2]`.
pub(super) fn markers(ids: &[usize]) -> String {
    ids.iter().map(|id| format!(" [P{id}]")).collect()
}

/// How one occurrence's declared enclosing repetition reads. Both the parameter and the packet
/// occurrence tables present occurrences, so this label is shared rather than owned by a feature.
pub(super) fn repeat(o: &ParameterOccurrence, seen: &mut BTreeMap<Source, u64>) -> String {
    let instance = seen.entry(o.definition.source.clone()).or_default();
    *instance += 1;
    let mut parts = Vec::new();
    for e in &o.enclosing {
        parts.push(match e {
            Enclosure::Repetition(r) => match &r.value {
                Some(Repetition::Fixed { count, stride_bits })
                    if field_value(&o.definition, "VPD_TPSD").is_some() =>
                {
                    format!("fixed count {count}, stride {}", width(stride_bits))
                }
                Some(Repetition::Fixed { count, stride_bits }) => format!(
                    "{instance}/{count}, stride {}{}",
                    width(stride_bits),
                    default_suffix(&o.definition, "PLF_LGOCC")
                ),
                Some(Repetition::Runtime(r)) => format!("runtime {}", text(&r.expression)),
                None => "unavailable".into(),
            },
            Enclosure::Condition(r) => format!("condition {}", text(&r.expression)),
        });
    }
    if parts.is_empty() {
        format!("once{}", default_suffix(&o.definition, "PLF_NBOCC"))
    } else {
        parts.join("; ")
    }
}
/// How one occurrence's declared location reads, with its offset default marker. Shared between
/// the parameter and packet occurrence tables for the same reason as [`repeat`].
pub(super) fn location_text(o: &ParameterOccurrence) -> String {
    format!(
        "{}{}",
        position(&o.location.position),
        if default_suffix(&o.definition, "PLF_OFFBY").is_empty() {
            default_suffix(&o.definition, "PLF_OFFBI")
        } else {
            " [default]"
        }
    )
}
