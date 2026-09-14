use super::*;

const VPD: &[Column] = &[
    Column {
        name: "VPD_TPSD",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "VPD_POS",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "VPD_NAME",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "VPD_GRPSIZE",
        kind: Integer,
        required: false,
        default: Some("0"),
    },
    Column {
        name: "VPD_FIXREP",
        kind: Integer,
        required: false,
        default: Some("0"),
    },
    Column {
        name: "VPD_CHOICE",
        kind: Code("YN"),
        required: false,
        default: Some("N"),
    },
    Column {
        name: "VPD_PIDREF",
        kind: Code("YN"),
        required: false,
        default: Some("N"),
    },
    Column {
        name: "VPD_DISDESC",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "VPD_WIDTH",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "VPD_JUSTIFY",
        kind: Code("LCR"),
        required: false,
        default: Some("L"),
    },
    Column {
        name: "VPD_NEWLINE",
        kind: Code("YN"),
        required: false,
        default: Some("N"),
    },
    Column {
        name: "VPD_DCHAR",
        kind: Integer,
        required: false,
        default: Some("0"),
    },
    Column {
        name: "VPD_FORM",
        kind: Code("NBODH"),
        required: false,
        default: Some("N"),
    },
    Column {
        name: "VPD_OFFSET",
        kind: Integer,
        required: false,
        default: Some("0"),
    },
];
pub(super) fn parse_vpd(text: &str, source: Source) -> Result<Row<Vpd>, String> {
    let definition = parse_definition(text, source, VPD)?;
    let cells = Vpd {
        tpsd: integer_cell(&definition, 0),
        pos: integer_cell(&definition, 1),
        name: cell(&definition, 2, |s| match s {
            Scalar::Text(s) => Some(ParameterName(s.clone())),
            _ => None,
        }),
        grpsize: integer_cell(&definition, 3),
        fixrep: integer_cell(&definition, 4),
        choice: text_cell(&definition, 5),
        pidref: text_cell(&definition, 6),
        disdesc: text_cell(&definition, 7),
        width: integer_cell(&definition, 8),
        justify: text_cell(&definition, 9),
        newline: text_cell(&definition, 10),
        dchar: integer_cell(&definition, 11),
        form: text_cell(&definition, 12),
        offset: integer_cell(&definition, 13),
    };
    Ok(Row { definition, cells })
}
