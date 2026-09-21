//! Table listings: the candidates of an ambiguous or searched identity, and every supported
//! table with what the loaded directory provided for it.

use mibl::model::*;
use std::io::{self, Write};

use super::format::{source, string, table, text};

pub(crate) fn candidates<'a>(
    candidates: impl Iterator<Item = &'a Candidate>,
    out: &mut dyn Write,
) -> io::Result<()> {
    let mut rows = vec![vec![
        "Kind".into(),
        "Identity".into(),
        "PUS".into(),
        "Name".into(),
        "Description".into(),
        "Source".into(),
    ]];
    for c in candidates {
        let (kind, identity) = match &c.identity {
            Identity::Parameter(n) => ("parameter", text(&n.0)),
            Identity::Packet(n) => ("packet", n.0.to_string()),
            Identity::Command(n) => ("command", text(&n.0)),
        };
        rows.push(vec![
            kind.into(),
            identity,
            match (&c.identity, c.service_type) {
                (Identity::Packet(_) | Identity::Command(_), Some(service)) => {
                    let prefix = if matches!(c.identity, Identity::Packet(_)) {
                        "TM"
                    } else {
                        "TC"
                    };
                    let subtype = c
                        .service_subtype
                        .map_or_else(|| "-".into(), |s| s.to_string());
                    format!("{prefix}({service},{subtype})")
                }
                _ => "-".into(),
            },
            string(&c.name),
            string(&c.description),
            source(&c.source),
        ]);
    }
    table(&rows, out)
}

/// Every supported table, with the file name it comes from, what its rows declare, the
/// lookup that addresses them directly, and what the loaded directory provided. The
/// listing never depends on `--details`, which adds nothing here.
pub(crate) fn tables<'a>(
    reports: impl Iterator<Item = &'a TableReport>,
    out: &mut dyn Write,
) -> io::Result<()> {
    let mut rows = vec![vec![
        "Code".into(),
        "File".into(),
        "Defines".into(),
        "Lookup".into(),
        "Rows".into(),
    ]];
    for report in reports {
        rows.push(vec![
            report.table.code().into(),
            report.table.file().into(),
            report.table.meaning().into(),
            match report.table.root() {
                Some(TableRoot::Parameter) => "parameter".into(),
                Some(TableRoot::Packet) => "packet".into(),
                Some(TableRoot::Command) => "command".into(),
                None => "-".into(),
            },
            match report.rows {
                TableRows::Missing => "missing".into(),
                TableRows::Unreadable => "unreadable".into(),
                TableRows::Read { rows } => rows.to_string(),
            },
        ]);
    }
    table(&rows, out)
}
