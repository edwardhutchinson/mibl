//! Monitoring parameter rows and their schema.

use super::Row;
use super::fields::{CellType, Column, cell, integer_cell, parse_definition, text_cell};
use crate::model::{Info, ParameterName, Scalar, Source};
use CellType::{Code, Integer, Text};

pub(crate) struct Pcf {
    pub name: Info<ParameterName>,
    pub descr: Info<String>,
    pub pid: Info<i64>,
    pub unit: Info<String>,
    pub ptc: Info<i64>,
    pub pfc: Info<i64>,
    pub width: Info<i64>,
    pub valid: Info<String>,
    pub related: Info<String>,
    pub categ: Info<String>,
    pub natur: Info<String>,
    pub curtx: Info<String>,
    pub r#inter: Info<String>,
    pub uscon: Info<String>,
    pub decim: Info<i64>,
    pub parval: Info<String>,
    pub subsys: Info<String>,
    pub valpar: Info<i64>,
    pub sptype: Info<String>,
    pub corr: Info<String>,
    pub obtid: Info<i64>,
    pub darc: Info<String>,
    pub endian: Info<String>,
    pub descr2: Info<String>,
}

const PCF: &[Column] = &[
    Column {
        name: "PCF_NAME",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "PCF_DESCR",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "PCF_PID",
        kind: Integer,
        required: false,
        default: None,
    },
    Column {
        name: "PCF_UNIT",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "PCF_PTC",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "PCF_PFC",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "PCF_WIDTH",
        kind: Integer,
        required: false,
        default: None,
    },
    Column {
        name: "PCF_VALID",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "PCF_RELATED",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "PCF_CATEG",
        kind: Code("NST"),
        required: true,
        default: None,
    },
    Column {
        name: "PCF_NATUR",
        kind: Code("RDPHSC"),
        required: true,
        default: None,
    },
    Column {
        name: "PCF_CURTX",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "PCF_INTER",
        kind: Code("PF"),
        required: false,
        default: Some("F"),
    },
    Column {
        name: "PCF_USCON",
        kind: Code("YN"),
        required: false,
        default: Some("N"),
    },
    Column {
        name: "PCF_DECIM",
        kind: Integer,
        required: false,
        default: None,
    },
    Column {
        name: "PCF_PARVAL",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "PCF_SUBSYS",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "PCF_VALPAR",
        kind: Integer,
        required: false,
        default: Some("1"),
    },
    Column {
        name: "PCF_SPTYPE",
        kind: Code("ER"),
        required: false,
        default: None,
    },
    Column {
        name: "PCF_CORR",
        kind: Code("YN"),
        required: false,
        default: Some("Y"),
    },
    Column {
        name: "PCF_OBTID",
        kind: Integer,
        required: false,
        default: None,
    },
    Column {
        name: "PCF_DARC",
        kind: Code("01"),
        required: false,
        default: Some("0"),
    },
    Column {
        name: "PCF_ENDIAN",
        kind: Code("BL"),
        required: false,
        default: Some("B"),
    },
    Column {
        name: "PCF_DESCR2",
        kind: Text,
        required: false,
        default: Some(""),
    },
];

pub(super) fn parse_pcf(text: &str, source: Source) -> Result<Row<Pcf>, String> {
    let definition = parse_definition(text, source, PCF)?;
    let cells = Pcf {
        name: cell(&definition, 0, |s| match s {
            Scalar::Text(s) => Some(ParameterName(s.clone())),
            _ => None,
        }),
        descr: text_cell(&definition, 1),
        pid: integer_cell(&definition, 2),
        unit: text_cell(&definition, 3),
        ptc: integer_cell(&definition, 4),
        pfc: integer_cell(&definition, 5),
        width: integer_cell(&definition, 6),
        valid: text_cell(&definition, 7),
        related: text_cell(&definition, 8),
        categ: text_cell(&definition, 9),
        natur: text_cell(&definition, 10),
        curtx: text_cell(&definition, 11),
        r#inter: text_cell(&definition, 12),
        uscon: text_cell(&definition, 13),
        decim: integer_cell(&definition, 14),
        parval: text_cell(&definition, 15),
        subsys: text_cell(&definition, 16),
        valpar: integer_cell(&definition, 17),
        sptype: text_cell(&definition, 18),
        corr: text_cell(&definition, 19),
        obtid: integer_cell(&definition, 20),
        darc: text_cell(&definition, 21),
        endian: text_cell(&definition, 22),
        descr2: text_cell(&definition, 23),
    };
    Ok(Row { definition, cells })
}
