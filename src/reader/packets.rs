//! Telemetry packet, occurrence and layout rows and their schemas.

use super::Row;
use super::fields::{CellType, Column, cell, integer_cell, parse_definition, text_cell};
use crate::model::{Info, PacketSpid, ParameterName, Scalar, Source};
use CellType::{Code, Integer, Text};

pub(crate) struct Pid {
    pub r#type: Info<i64>,
    pub stype: Info<i64>,
    pub apid: Info<i64>,
    pub pi1_val: Info<i64>,
    pub pi2_val: Info<i64>,
    pub spid: Info<PacketSpid>,
    pub descr: Info<String>,
    pub unit: Info<String>,
    pub tpsd: Info<i64>,
    pub dfhsize: Info<i64>,
    pub time: Info<String>,
    pub r#inter: Info<i64>,
    pub valid: Info<String>,
    pub check: Info<i64>,
    pub event: Info<String>,
    pub evid: Info<String>,
}

const PID: &[Column] = &[
    Column {
        name: "PID_TYPE",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "PID_STYPE",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "PID_APID",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "PID_PI1_VAL",
        kind: Integer,
        required: false,
        default: Some("0"),
    },
    Column {
        name: "PID_PI2_VAL",
        kind: Integer,
        required: false,
        default: Some("0"),
    },
    Column {
        name: "PID_SPID",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "PID_DESCR",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "PID_UNIT",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "PID_TPSD",
        kind: Integer,
        required: false,
        default: Some("-1"),
    },
    Column {
        name: "PID_DFHSIZE",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "PID_TIME",
        kind: Code("YN"),
        required: false,
        default: Some("N"),
    },
    Column {
        name: "PID_INTER",
        kind: Integer,
        required: false,
        default: None,
    },
    Column {
        name: "PID_VALID",
        kind: Code("YN"),
        required: false,
        default: Some("Y"),
    },
    Column {
        name: "PID_CHECK",
        kind: Integer,
        required: false,
        default: Some("0"),
    },
    Column {
        name: "PID_EVENT",
        kind: Code("NIEW"),
        required: false,
        default: Some("N"),
    },
    Column {
        name: "PID_EVID",
        kind: Text,
        required: false,
        default: None,
    },
];

pub(super) fn parse_pid(text: &str, source: Source) -> Result<Row<Pid>, String> {
    let definition = parse_definition(text, source, PID)?;
    let spid = integer_cell(&definition, 5)
        .value
        .and_then(|n| u64::try_from(n).ok())
        .ok_or("SPID must be nonnegative")?;
    let cells = Pid {
        r#type: integer_cell(&definition, 0),
        stype: integer_cell(&definition, 1),
        apid: integer_cell(&definition, 2),
        pi1_val: integer_cell(&definition, 3),
        pi2_val: integer_cell(&definition, 4),
        spid: cell(&definition, 5, |_| Some(PacketSpid(spid))),
        descr: text_cell(&definition, 6),
        unit: text_cell(&definition, 7),
        tpsd: integer_cell(&definition, 8),
        dfhsize: integer_cell(&definition, 9),
        time: text_cell(&definition, 10),
        r#inter: integer_cell(&definition, 11),
        valid: text_cell(&definition, 12),
        check: integer_cell(&definition, 13),
        event: text_cell(&definition, 14),
        evid: text_cell(&definition, 15),
    };
    Ok(Row { definition, cells })
}

pub(crate) struct Tpcf {
    pub spid: Info<PacketSpid>,
    pub name: Info<String>,
    pub size: Info<i64>,
}

const TPCF: &[Column] = &[
    Column {
        name: "TPCF_SPID",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "TPCF_NAME",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "TPCF_SIZE",
        kind: Integer,
        required: false,
        default: None,
    },
];

pub(super) fn parse_tpcf(text: &str, source: Source) -> Result<Row<Tpcf>, String> {
    let definition = parse_definition(text, source, TPCF)?;
    let spid = integer_cell(&definition, 0)
        .value
        .and_then(|n| u64::try_from(n).ok())
        .ok_or("SPID must be nonnegative")?;
    let cells = Tpcf {
        spid: cell(&definition, 0, |_| Some(PacketSpid(spid))),
        name: text_cell(&definition, 1),
        size: integer_cell(&definition, 2),
    };
    Ok(Row { definition, cells })
}

pub(crate) struct Pic {
    pub r#type: Info<i64>,
    pub stype: Info<i64>,
    pub pi1_off: Info<i64>,
    pub pi1_wid: Info<i64>,
    pub pi2_off: Info<i64>,
    pub pi2_wid: Info<i64>,
    pub apid: Info<i64>,
}

const PIC: &[Column] = &[
    Column {
        name: "PIC_TYPE",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "PIC_STYPE",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "PIC_PI1_OFF",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "PIC_PI1_WID",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "PIC_PI2_OFF",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "PIC_PI2_WID",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "PIC_APID",
        kind: Integer,
        required: false,
        default: Some("99999"),
    },
];

pub(super) fn parse_pic(text: &str, source: Source) -> Result<Row<Pic>, String> {
    let definition = parse_definition(text, source, PIC)?;
    let cells = Pic {
        r#type: integer_cell(&definition, 0),
        stype: integer_cell(&definition, 1),
        pi1_off: integer_cell(&definition, 2),
        pi1_wid: integer_cell(&definition, 3),
        pi2_off: integer_cell(&definition, 4),
        pi2_wid: integer_cell(&definition, 5),
        apid: integer_cell(&definition, 6),
    };
    Ok(Row { definition, cells })
}

pub(crate) struct Plf {
    pub name: Info<ParameterName>,
    pub spid: Info<PacketSpid>,
    pub offby: Info<i64>,
    pub offbi: Info<i64>,
    pub nbocc: Info<i64>,
    pub lgocc: Info<i64>,
    pub time: Info<i64>,
    pub tdocc: Info<i64>,
}

const PLF: &[Column] = &[
    Column {
        name: "PLF_NAME",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "PLF_SPID",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "PLF_OFFBY",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "PLF_OFFBI",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "PLF_NBOCC",
        kind: Integer,
        required: false,
        default: Some("1"),
    },
    Column {
        name: "PLF_LGOCC",
        kind: Integer,
        required: false,
        default: Some("0"),
    },
    Column {
        name: "PLF_TIME",
        kind: Integer,
        required: false,
        default: Some("0"),
    },
    Column {
        name: "PLF_TDOCC",
        kind: Integer,
        required: false,
        default: Some("1"),
    },
];

pub(super) fn parse_plf(text: &str, source: Source) -> Result<Row<Plf>, String> {
    let definition = parse_definition(text, source, PLF)?;
    let spid = integer_cell(&definition, 1)
        .value
        .and_then(|n| u64::try_from(n).ok())
        .ok_or("SPID must be nonnegative")?;
    let cells = Plf {
        name: cell(&definition, 0, |s| match s {
            Scalar::Text(s) => Some(ParameterName(s.clone())),
            _ => None,
        }),
        spid: cell(&definition, 1, |_| Some(PacketSpid(spid))),
        offby: integer_cell(&definition, 2),
        offbi: integer_cell(&definition, 3),
        nbocc: integer_cell(&definition, 4),
        lgocc: integer_cell(&definition, 5),
        time: integer_cell(&definition, 6),
        tdocc: integer_cell(&definition, 7),
    };
    Ok(Row { definition, cells })
}
