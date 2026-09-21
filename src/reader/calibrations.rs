//! Calibration, polynomial, textual and condition rows and their schemas.

use super::Row;
use super::fields::{CellType, Column, cell, integer_cell, parse_definition, text_cell};
use crate::model::{Info, ParameterName, Scalar, Source};
use CellType::{Code, Integer, Text};

pub(crate) struct Cur {
    pub pname: Info<ParameterName>,
    pub pos: Info<i64>,
    pub rlchk: Info<ParameterName>,
    pub valpar: Info<i64>,
    pub select: Info<String>,
}

pub(super) fn parse_cur(text: &str, source: Source) -> Result<Row<Cur>, String> {
    let columns = [
        Column {
            name: "CUR_PNAME",
            kind: Text,
            required: true,
            default: None,
        },
        Column {
            name: "CUR_POS",
            kind: Integer,
            required: true,
            default: None,
        },
        Column {
            name: "CUR_RLCHK",
            kind: Text,
            required: true,
            default: None,
        },
        Column {
            name: "CUR_VALPAR",
            kind: Integer,
            required: true,
            default: None,
        },
        Column {
            name: "CUR_SELECT",
            kind: Text,
            required: true,
            default: None,
        },
    ];
    let definition = parse_definition(text, source, &columns)?;
    let name = |index| {
        cell(&definition, index, |s| match s {
            Scalar::Text(s) => Some(ParameterName(s.clone())),
            _ => None,
        })
    };
    let cells = Cur {
        pname: name(0),
        pos: integer_cell(&definition, 1),
        rlchk: name(2),
        valpar: integer_cell(&definition, 3),
        select: text_cell(&definition, 4),
    };
    Ok(Row { definition, cells })
}

pub(crate) struct Caf {
    pub numbr: Info<String>,
    pub descr: Info<String>,
    pub engfmt: Info<String>,
    pub rawfmt: Info<String>,
    pub radix: Info<String>,
    pub unit: Info<String>,
    pub ncurve: Info<i64>,
    pub r#inter: Info<String>,
}

const CAF: &[Column] = &[
    Column {
        name: "CAF_NUMBR",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "CAF_DESCR",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "CAF_ENGFMT",
        kind: Code("RIU"),
        required: true,
        default: None,
    },
    Column {
        name: "CAF_RAWFMT",
        kind: Code("RIU"),
        required: true,
        default: None,
    },
    Column {
        name: "CAF_RADIX",
        kind: Code("DHO"),
        required: false,
        default: None,
    },
    Column {
        name: "CAF_UNIT",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "CAF_NCURVE",
        kind: Integer,
        required: false,
        default: None,
    },
    Column {
        name: "CAF_INTER",
        kind: Code("PF"),
        required: false,
        default: Some("F"),
    },
];

pub(super) fn parse_caf(text: &str, source: Source) -> Result<Row<Caf>, String> {
    let definition = parse_definition(text, source, CAF)?;
    let cells = Caf {
        numbr: text_cell(&definition, 0),
        descr: text_cell(&definition, 1),
        engfmt: text_cell(&definition, 2),
        rawfmt: text_cell(&definition, 3),
        radix: text_cell(&definition, 4),
        unit: text_cell(&definition, 5),
        ncurve: integer_cell(&definition, 6),
        r#inter: text_cell(&definition, 7),
    };
    Ok(Row { definition, cells })
}

pub(crate) struct Cap {
    pub numbr: Info<String>,
    pub xvals: Info<String>,
    pub yvals: Info<String>,
}

pub(super) fn parse_cap(text: &str, source: Source) -> Result<Row<Cap>, String> {
    let columns = ["CAP_NUMBR", "CAP_XVALS", "CAP_YVALS"].map(|name| Column {
        name,
        kind: Text,
        required: true,
        default: None,
    });
    let definition = parse_definition(text, source, &columns)?;
    let cells = Cap {
        numbr: text_cell(&definition, 0),
        xvals: text_cell(&definition, 1),
        yvals: text_cell(&definition, 2),
    };
    Ok(Row { definition, cells })
}

pub(crate) struct Mcf {
    pub ident: Info<String>,
    pub descr: Info<String>,
    pub pol1: Info<String>,
    pub pol2: Info<String>,
    pub pol3: Info<String>,
    pub pol4: Info<String>,
    pub pol5: Info<String>,
}

pub(super) fn parse_mcf(text: &str, source: Source) -> Result<Row<Mcf>, String> {
    let columns = [
        Column {
            name: "MCF_IDENT",
            kind: Text,
            required: true,
            default: None,
        },
        Column {
            name: "MCF_DESCR",
            kind: Text,
            required: false,
            default: None,
        },
        Column {
            name: "MCF_POL1",
            kind: Text,
            required: true,
            default: None,
        },
        Column {
            name: "MCF_POL2",
            kind: Text,
            required: false,
            default: Some("0"),
        },
        Column {
            name: "MCF_POL3",
            kind: Text,
            required: false,
            default: Some("0"),
        },
        Column {
            name: "MCF_POL4",
            kind: Text,
            required: false,
            default: Some("0"),
        },
        Column {
            name: "MCF_POL5",
            kind: Text,
            required: false,
            default: Some("0"),
        },
    ];
    let definition = parse_definition(text, source, &columns)?;
    let cells = Mcf {
        ident: text_cell(&definition, 0),
        descr: text_cell(&definition, 1),
        pol1: text_cell(&definition, 2),
        pol2: text_cell(&definition, 3),
        pol3: text_cell(&definition, 4),
        pol4: text_cell(&definition, 5),
        pol5: text_cell(&definition, 6),
    };
    Ok(Row { definition, cells })
}

pub(crate) struct Lgf {
    pub ident: Info<String>,
    pub descr: Info<String>,
    pub pol1: Info<String>,
    pub pol2: Info<String>,
    pub pol3: Info<String>,
    pub pol4: Info<String>,
    pub pol5: Info<String>,
}

pub(super) fn parse_lgf(text: &str, source: Source) -> Result<Row<Lgf>, String> {
    let columns = [
        Column {
            name: "LGF_IDENT",
            kind: Text,
            required: true,
            default: None,
        },
        Column {
            name: "LGF_DESCR",
            kind: Text,
            required: false,
            default: None,
        },
        Column {
            name: "LGF_POL1",
            kind: Text,
            required: true,
            default: None,
        },
        Column {
            name: "LGF_POL2",
            kind: Text,
            required: false,
            default: Some("0"),
        },
        Column {
            name: "LGF_POL3",
            kind: Text,
            required: false,
            default: Some("0"),
        },
        Column {
            name: "LGF_POL4",
            kind: Text,
            required: false,
            default: Some("0"),
        },
        Column {
            name: "LGF_POL5",
            kind: Text,
            required: false,
            default: Some("0"),
        },
    ];
    let definition = parse_definition(text, source, &columns)?;
    let cells = Lgf {
        ident: text_cell(&definition, 0),
        descr: text_cell(&definition, 1),
        pol1: text_cell(&definition, 2),
        pol2: text_cell(&definition, 3),
        pol3: text_cell(&definition, 4),
        pol4: text_cell(&definition, 5),
        pol5: text_cell(&definition, 6),
    };
    Ok(Row { definition, cells })
}

pub(crate) struct Txf {
    pub numbr: Info<String>,
    pub descr: Info<String>,
    pub rawfmt: Info<String>,
    pub nalias: Info<i64>,
}

pub(super) fn parse_txf(text: &str, source: Source) -> Result<Row<Txf>, String> {
    let columns = [
        Column {
            name: "TXF_NUMBR",
            kind: Text,
            required: true,
            default: None,
        },
        Column {
            name: "TXF_DESCR",
            kind: Text,
            required: false,
            default: None,
        },
        Column {
            name: "TXF_RAWFMT",
            kind: Code("RIU"),
            required: true,
            default: None,
        },
        Column {
            name: "TXF_NALIAS",
            kind: Integer,
            required: false,
            default: None,
        },
    ];
    let definition = parse_definition(text, source, &columns)?;
    let cells = Txf {
        numbr: text_cell(&definition, 0),
        descr: text_cell(&definition, 1),
        rawfmt: text_cell(&definition, 2),
        nalias: integer_cell(&definition, 3),
    };
    Ok(Row { definition, cells })
}

pub(crate) struct Txp {
    pub numbr: Info<String>,
    pub from: Info<String>,
    pub to: Info<String>,
    pub altxt: Info<String>,
}

pub(super) fn parse_txp(text: &str, source: Source) -> Result<Row<Txp>, String> {
    let columns = [
        Column {
            name: "TXP_NUMBR",
            kind: Text,
            required: true,
            default: None,
        },
        Column {
            name: "TXP_FROM",
            kind: Text,
            required: true,
            default: None,
        },
        Column {
            name: "TXP_TO",
            kind: Text,
            required: true,
            default: None,
        },
        Column {
            name: "TXP_ALTXT",
            kind: Text,
            required: true,
            default: None,
        },
    ];
    let definition = parse_definition(text, source, &columns)?;
    let cells = Txp {
        numbr: text_cell(&definition, 0),
        from: text_cell(&definition, 1),
        to: text_cell(&definition, 2),
        altxt: text_cell(&definition, 3),
    };
    Ok(Row { definition, cells })
}
