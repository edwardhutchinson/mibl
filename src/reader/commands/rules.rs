//! Command argument conversion, alias and range rows and their schemas.

use super::super::Row;
use super::super::fields::{CellType, Column, integer_cell, parse_definition, text_cell};
use crate::model::{Info, Source};
use CellType::{Code, Integer, Text};

pub(crate) struct Cca {
    pub numbr: Info<String>,
    pub descr: Info<String>,
    pub engfmt: Info<String>,
    pub rawfmt: Info<String>,
    pub radix: Info<String>,
    pub unit: Info<String>,
    pub ncurve: Info<i64>,
}

const CCA: &[Column] = &[
    Column {
        name: "CCA_NUMBR",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "CCA_DESCR",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "CCA_ENGFMT",
        kind: Code("RIU"),
        required: false,
        default: Some("R"),
    },
    Column {
        name: "CCA_RAWFMT",
        kind: Code("RIU"),
        required: false,
        default: Some("U"),
    },
    Column {
        name: "CCA_RADIX",
        kind: Code("DHO"),
        required: false,
        default: Some("D"),
    },
    Column {
        name: "CCA_UNIT",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "CCA_NCURVE",
        kind: Integer,
        required: false,
        default: None,
    },
];

pub(in crate::reader) fn parse_cca(text: &str, source: Source) -> Result<Row<Cca>, String> {
    let definition = parse_definition(text, source, CCA)?;
    let cells = Cca {
        numbr: text_cell(&definition, 0),
        descr: text_cell(&definition, 1),
        engfmt: text_cell(&definition, 2),
        rawfmt: text_cell(&definition, 3),
        radix: text_cell(&definition, 4),
        unit: text_cell(&definition, 5),
        ncurve: integer_cell(&definition, 6),
    };
    Ok(Row { definition, cells })
}

pub(crate) struct Ccs {
    pub numbr: Info<String>,
    pub xvals: Info<String>,
    pub yvals: Info<String>,
}

const CCS: &[Column] = &[
    Column {
        name: "CCS_NUMBR",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "CCS_XVALS",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "CCS_YVALS",
        kind: Text,
        required: true,
        default: None,
    },
];

pub(in crate::reader) fn parse_ccs(text: &str, source: Source) -> Result<Row<Ccs>, String> {
    let definition = parse_definition(text, source, CCS)?;
    let cells = Ccs {
        numbr: text_cell(&definition, 0),
        xvals: text_cell(&definition, 1),
        yvals: text_cell(&definition, 2),
    };
    Ok(Row { definition, cells })
}

pub(crate) struct Paf {
    pub numbr: Info<String>,
    pub descr: Info<String>,
    pub rawfmt: Info<String>,
    pub nalias: Info<i64>,
}

const PAF: &[Column] = &[
    Column {
        name: "PAF_NUMBR",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "PAF_DESCR",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "PAF_RAWFMT",
        kind: Code("RIU"),
        required: false,
        default: Some("U"),
    },
    Column {
        name: "PAF_NALIAS",
        kind: Integer,
        required: false,
        default: None,
    },
];

pub(in crate::reader) fn parse_paf(text: &str, source: Source) -> Result<Row<Paf>, String> {
    let definition = parse_definition(text, source, PAF)?;
    let cells = Paf {
        numbr: text_cell(&definition, 0),
        descr: text_cell(&definition, 1),
        rawfmt: text_cell(&definition, 2),
        nalias: integer_cell(&definition, 3),
    };
    Ok(Row { definition, cells })
}

pub(crate) struct Pas {
    pub numbr: Info<String>,
    pub altxt: Info<String>,
    pub alval: Info<String>,
}

const PAS: &[Column] = &[
    Column {
        name: "PAS_NUMBR",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "PAS_ALTXT",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "PAS_ALVAL",
        kind: Text,
        required: true,
        default: None,
    },
];

pub(in crate::reader) fn parse_pas(text: &str, source: Source) -> Result<Row<Pas>, String> {
    let definition = parse_definition(text, source, PAS)?;
    let cells = Pas {
        numbr: text_cell(&definition, 0),
        altxt: text_cell(&definition, 1),
        alval: text_cell(&definition, 2),
    };
    Ok(Row { definition, cells })
}

pub(crate) struct Prf {
    pub numbr: Info<String>,
    pub descr: Info<String>,
    pub r#inter: Info<String>,
    pub dspfmt: Info<String>,
    pub radix: Info<String>,
    pub nrange: Info<i64>,
    pub unit: Info<String>,
}

const PRF: &[Column] = &[
    Column {
        name: "PRF_NUMBR",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "PRF_DESCR",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "PRF_INTER",
        kind: Code("RE"),
        required: false,
        default: Some("R"),
    },
    Column {
        name: "PRF_DSPFMT",
        kind: Code("AIURTD"),
        required: false,
        default: Some("R"),
    },
    Column {
        name: "PRF_RADIX",
        kind: Code("DHO"),
        required: false,
        default: Some("D"),
    },
    Column {
        name: "PRF_NRANGE",
        kind: Integer,
        required: false,
        default: None,
    },
    Column {
        name: "PRF_UNIT",
        kind: Text,
        required: false,
        default: None,
    },
];

pub(in crate::reader) fn parse_prf(text: &str, source: Source) -> Result<Row<Prf>, String> {
    let definition = parse_definition(text, source, PRF)?;
    let cells = Prf {
        numbr: text_cell(&definition, 0),
        descr: text_cell(&definition, 1),
        r#inter: text_cell(&definition, 2),
        dspfmt: text_cell(&definition, 3),
        radix: text_cell(&definition, 4),
        nrange: integer_cell(&definition, 5),
        unit: text_cell(&definition, 6),
    };
    Ok(Row { definition, cells })
}

pub(crate) struct Prv {
    pub numbr: Info<String>,
    pub minval: Info<String>,
    pub maxval: Info<String>,
}

const PRV: &[Column] = &[
    Column {
        name: "PRV_NUMBR",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "PRV_MINVAL",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "PRV_MAXVAL",
        kind: Text,
        required: false,
        default: None,
    },
];

pub(in crate::reader) fn parse_prv(text: &str, source: Source) -> Result<Row<Prv>, String> {
    let definition = parse_definition(text, source, PRV)?;
    let cells = Prv {
        numbr: text_cell(&definition, 0),
        minval: text_cell(&definition, 1),
        maxval: text_cell(&definition, 2),
    };
    Ok(Row { definition, cells })
}
