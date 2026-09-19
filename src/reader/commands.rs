use super::*;

const CCF: &[Column] = &[
    Column {
        name: "CCF_CNAME",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "CCF_DESCR",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "CCF_DESCR2",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "CCF_CTYPE",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "CCF_CRITICAL",
        kind: Code("YN"),
        required: false,
        default: Some("N"),
    },
    Column {
        name: "CCF_PKTID",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "CCF_TYPE",
        kind: Integer,
        required: false,
        default: None,
    },
    Column {
        name: "CCF_STYPE",
        kind: Integer,
        required: false,
        default: None,
    },
    Column {
        name: "CCF_APID",
        kind: Integer,
        required: false,
        default: None,
    },
    Column {
        name: "CCF_NPARS",
        kind: Integer,
        required: false,
        default: None,
    },
    Column {
        name: "CCF_PLAN",
        kind: Code("AFSN"),
        required: false,
        default: Some("N"),
    },
    Column {
        name: "CCF_EXEC",
        kind: Code("YN"),
        required: false,
        default: Some("Y"),
    },
    Column {
        name: "CCF_ILSCOPE",
        kind: Code("GLSBFTN"),
        required: false,
        default: Some("N"),
    },
    Column {
        name: "CCF_ILSTAGE",
        kind: Code("RUOAC"),
        required: false,
        default: Some("C"),
    },
    Column {
        name: "CCF_SUBSYS",
        kind: Integer,
        required: false,
        default: None,
    },
    Column {
        name: "CCF_HIPRI",
        kind: Code("YN"),
        required: false,
        default: Some("N"),
    },
    Column {
        name: "CCF_MAPID",
        kind: Integer,
        required: false,
        default: None,
    },
    Column {
        name: "CCF_DEFSET",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "CCF_RAPID",
        kind: Integer,
        required: false,
        default: None,
    },
    Column {
        name: "CCF_ACK",
        kind: Integer,
        required: false,
        default: None,
    },
    Column {
        name: "CCF_SUBSCHEDID",
        kind: Integer,
        required: false,
        default: None,
    },
];
pub(super) fn parse_ccf(text: &str, source: Source) -> Result<Row<Ccf>, String> {
    let definition = parse_definition(text, source, CCF)?;
    let cells = Ccf {
        cname: cell(&definition, 0, |s| match s {
            Scalar::Text(s) => Some(CommandName(s.clone())),
            _ => None,
        }),
        descr: text_cell(&definition, 1),
        descr2: text_cell(&definition, 2),
        ctype: text_cell(&definition, 3),
        critical: text_cell(&definition, 4),
        pktid: text_cell(&definition, 5),
        r#type: integer_cell(&definition, 6),
        stype: integer_cell(&definition, 7),
        apid: integer_cell(&definition, 8),
        npars: integer_cell(&definition, 9),
        plan: text_cell(&definition, 10),
        exec: text_cell(&definition, 11),
        ilscope: text_cell(&definition, 12),
        ilstage: text_cell(&definition, 13),
        subsys: integer_cell(&definition, 14),
        hipri: text_cell(&definition, 15),
        mapid: integer_cell(&definition, 16),
        defset: text_cell(&definition, 17),
        rapid: integer_cell(&definition, 18),
        ack: integer_cell(&definition, 19),
        subschedid: integer_cell(&definition, 20),
    };
    Ok(Row { definition, cells })
}

const CDF: &[Column] = &[
    Column {
        name: "CDF_CNAME",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "CDF_ELTYPE",
        kind: Code("AFE"),
        required: true,
        default: None,
    },
    Column {
        name: "CDF_DESCR",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "CDF_ELLEN",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "CDF_BIT",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "CDF_GRPSIZE",
        kind: Integer,
        required: false,
        default: Some("0"),
    },
    Column {
        name: "CDF_PNAME",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "CDF_INTER",
        kind: Code("REDT"),
        required: false,
        default: Some("R"),
    },
    Column {
        name: "CDF_VALUE",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "CDF_TMID",
        kind: Text,
        required: false,
        default: None,
    },
];
pub(super) fn parse_cdf(text: &str, source: Source) -> Result<Row<Cdf>, String> {
    let definition = parse_definition(text, source, CDF)?;
    let cells = Cdf {
        cname: cell(&definition, 0, |s| match s {
            Scalar::Text(s) => Some(CommandName(s.clone())),
            _ => None,
        }),
        eltype: text_cell(&definition, 1),
        descr: text_cell(&definition, 2),
        ellen: integer_cell(&definition, 3),
        bit: integer_cell(&definition, 4),
        grpsize: integer_cell(&definition, 5),
        pname: text_cell(&definition, 6),
        r#inter: text_cell(&definition, 7),
        value: text_cell(&definition, 8),
        tmid: text_cell(&definition, 9),
    };
    Ok(Row { definition, cells })
}

const CPC: &[Column] = &[
    Column {
        name: "CPC_NAME",
        kind: Text,
        required: true,
        default: None,
    },
    Column {
        name: "CPC_DESCR",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "CPC_PTC",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "CPC_PFC",
        kind: Integer,
        required: true,
        default: None,
    },
    Column {
        name: "CPC_DISPFMT",
        kind: Code("AIURTD"),
        required: false,
        default: Some("R"),
    },
    Column {
        name: "CPC_RADIX",
        kind: Code("DHO"),
        required: false,
        default: Some("D"),
    },
    Column {
        name: "CPC_UNIT",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "CPC_CATEG",
        kind: Code("CTBAPN"),
        required: false,
        default: Some("N"),
    },
    Column {
        name: "CPC_PRFREF",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "CPC_CCAREF",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "CPC_PAFREF",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "CPC_INTER",
        kind: Code("RE"),
        required: false,
        default: Some("R"),
    },
    Column {
        name: "CPC_DEFVAL",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "CPC_CORR",
        kind: Code("YN"),
        required: false,
        default: Some("Y"),
    },
    Column {
        name: "CPC_OBTIP",
        kind: Integer,
        required: false,
        default: Some("0"),
    },
    Column {
        name: "CPC_DESCR2",
        kind: Text,
        required: false,
        default: None,
    },
    Column {
        name: "CPC_ENDIAN",
        kind: Text,
        required: false,
        default: None,
    },
];
pub(super) fn parse_cpc(text: &str, source: Source) -> Result<Row<Cpc>, String> {
    let mut definition = parse_definition(text, source, CPC)?;
    reconcile_cpc(&mut definition);
    let cells = Cpc {
        name: text_cell(&definition, 0),
        descr: text_cell(&definition, 1),
        ptc: integer_cell(&definition, 2),
        pfc: integer_cell(&definition, 3),
        dispfmt: text_cell(&definition, 4),
        radix: text_cell(&definition, 5),
        unit: text_cell(&definition, 6),
        categ: text_cell(&definition, 7),
        prfref: text_cell(&definition, 8),
        ccaref: text_cell(&definition, 9),
        pafref: text_cell(&definition, 10),
        r#inter: text_cell(&definition, 11),
        defval: text_cell(&definition, 12),
        corr: text_cell(&definition, 13),
        obtip: integer_cell(&definition, 14),
        descr2: text_cell(&definition, 15),
        endian: text_cell(&definition, 16),
    };
    Ok(Row { definition, cells })
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
pub(super) fn parse_cca(text: &str, source: Source) -> Result<Row<Cca>, String> {
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
pub(super) fn parse_ccs(text: &str, source: Source) -> Result<Row<Ccs>, String> {
    let definition = parse_definition(text, source, CCS)?;
    let cells = Ccs {
        numbr: text_cell(&definition, 0),
        xvals: text_cell(&definition, 1),
        yvals: text_cell(&definition, 2),
    };
    Ok(Row { definition, cells })
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
pub(super) fn parse_paf(text: &str, source: Source) -> Result<Row<Paf>, String> {
    let definition = parse_definition(text, source, PAF)?;
    let cells = Paf {
        numbr: text_cell(&definition, 0),
        descr: text_cell(&definition, 1),
        rawfmt: text_cell(&definition, 2),
        nalias: integer_cell(&definition, 3),
    };
    Ok(Row { definition, cells })
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
pub(super) fn parse_pas(text: &str, source: Source) -> Result<Row<Pas>, String> {
    let definition = parse_definition(text, source, PAS)?;
    let cells = Pas {
        numbr: text_cell(&definition, 0),
        altxt: text_cell(&definition, 1),
        alval: text_cell(&definition, 2),
    };
    Ok(Row { definition, cells })
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
pub(super) fn parse_prf(text: &str, source: Source) -> Result<Row<Prf>, String> {
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
pub(super) fn parse_prv(text: &str, source: Source) -> Result<Row<Prv>, String> {
    let definition = parse_definition(text, source, PRV)?;
    let cells = Prv {
        numbr: text_cell(&definition, 0),
        minval: text_cell(&definition, 1),
        maxval: text_cell(&definition, 2),
    };
    Ok(Row { definition, cells })
}

fn reconcile_cpc(d: &mut Definition) {
    for (index, names) in [
        (0, ["CPC_PNAME", "CPC_NAME"]),
        (14, ["CPC_OBTID", "CPC_OBTIP"]),
    ] {
        let meaning = d.fields[index].meanings[0].clone();
        d.fields[index].meanings = names
            .into_iter()
            .map(|name| FieldMeaning {
                schema_name: name.into(),
                interpretation: meaning.interpretation.clone(),
            })
            .collect();
    }
    // Neither cell text nor row length identifies the source schema version.
    for (index, names) in [
        (15, vec!["CPC_ENDIAN", "CPC_DESCR2"]),
        (16, vec!["CPC_ENDIAN"]),
    ] {
        let meanings: Vec<_> = names
            .into_iter()
            .map(|name| FieldMeaning {
                schema_name: name.into(),
                interpretation: Info {
                    value: None,
                    problems: vec![],
                    sources: vec![d.source.clone()],
                },
            })
            .collect();
        let problem = Problem {
            kind: ProblemKind::UnsupportedInterpretation { column: Some(d.fields[index].column), meanings: meanings.clone() },
            sources: vec![d.source.clone()],
            explanation: "CPC suffix layout is ambiguous; description and endian cannot be selected or defaulted".into(),
        };
        d.fields[index].meanings = meanings
            .into_iter()
            .map(|mut meaning| {
                meaning.interpretation.problems.push(problem.clone());
                meaning
            })
            .collect();
    }
}
