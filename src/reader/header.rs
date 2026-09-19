use super::*;

const TCP: &[Column] = &[
    Column {
        name: "TCP_ID",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "TCP_DESC",
        kind: Text,
        required: false,
        default: None,
    },
];
pub(super) fn parse_tcp(text: &str, source: Source) -> Result<Row<Tcp>, String> {
    let definition = parse_definition(text, source, TCP)?;
    let cells = Tcp {
        id: text_cell(&definition, 0),
        desc: text_cell(&definition, 1),
    };
    Ok(Row { definition, cells })
}

/// ICD 7.0 gives the header element types: `F` fixed area, `A` APID, `T` service type,
/// `S` service sub type, `K` acknowledgement flags and `P` an automatically set packet
/// parameter. An undeclared code is malformed and drops its own row.
const PCDF: &[Column] = &[
    Column {
        name: "PCDF_TCNAME",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "PCDF_DESC",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "PCDF_TYPE",
        kind: Code("FATSKP"),
        required: true,
        default: None,
    },
    Column {
        name: "PCDF_LEN",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "PCDF_BIT",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "PCDF_PNAME",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "PCDF_VALUE",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "PCDF_RADIX",
        kind: Code("DHO"),
        required: false,
        default: Some("H"),
    },
];
pub(super) fn parse_pcdf(text: &str, source: Source) -> Result<Row<Pcdf>, String> {
    let definition = parse_definition(text, source, PCDF)?;
    let cells = Pcdf {
        tcname: text_cell(&definition, 0),
        desc: text_cell(&definition, 1),
        r#type: text_cell(&definition, 2),
        len: integer_cell(&definition, 3),
        bit: integer_cell(&definition, 4),
        pname: text_cell(&definition, 5),
        value: text_cell(&definition, 6),
        radix: text_cell(&definition, 7),
    };
    Ok(Row { definition, cells })
}

/// ICD 7.0 declares the code that PCDF_VALUE is written in: `I` signed integer and
/// `U` unsigned integer.
const PCPC: &[Column] = &[
    Column {
        name: "PCPC_PNAME",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "PCPC_DESC",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "PCPC_CODE",
        kind: Code("IU"),
        required: false,
        default: Some("U"),
    },
];
pub(super) fn parse_pcpc(text: &str, source: Source) -> Result<Row<Pcpc>, String> {
    let definition = parse_definition(text, source, PCPC)?;
    let cells = Pcpc {
        pname: text_cell(&definition, 0),
        desc: text_cell(&definition, 1),
        code: text_cell(&definition, 2),
    };
    Ok(Row { definition, cells })
}
