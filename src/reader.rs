//! Private typed rows and shared loading. Table families are added in their owning slices.
use crate::{LoadError, model::*};
use std::{io, path::Path};
mod calibrations;
mod commands;
use calibrations::*;
mod header;
mod variable;
use commands::{
    parse_cca, parse_ccf, parse_ccs, parse_cdf, parse_cpc, parse_paf, parse_pas, parse_prf,
    parse_prv,
};
use header::{parse_pcdf, parse_pcpc, parse_tcp};
use variable::parse_vpd;

/// Every retained row owns recorded fields and independent interpreted cells.
pub(crate) struct Row<T> {
    pub definition: Definition,
    pub cells: T,
}
/// Missing/unreadable tables differ from readable tables with no retained rows.
pub(crate) enum TableLoad<T> {
    Missing,
    Unreadable(io::Error),
    Read { rows: Vec<Row<T>> },
}
/// Consumes available supported rows, including supporting-only snapshots.
pub(crate) struct Records {
    pub reports: Vec<TableReport>,
    pub pcf: TableLoad<Pcf>,
    pub pid: TableLoad<Pid>,
    pub tpcf: TableLoad<Tpcf>,
    pub pic: TableLoad<Pic>,
    pub plf: TableLoad<Plf>,
    pub vpd: TableLoad<Vpd>,
    pub cur: TableLoad<Cur>,
    pub caf: TableLoad<Caf>,
    pub cap: TableLoad<Cap>,
    pub mcf: TableLoad<Mcf>,
    pub lgf: TableLoad<Lgf>,
    pub txf: TableLoad<Txf>,
    pub txp: TableLoad<Txp>,
    pub ccf: TableLoad<Ccf>,
    pub cdf: TableLoad<Cdf>,
    pub cpc: TableLoad<Cpc>,
    pub cca: TableLoad<Cca>,
    pub ccs: TableLoad<Ccs>,
    pub paf: TableLoad<Paf>,
    pub pas: TableLoad<Pas>,
    pub prf: TableLoad<Prf>,
    pub prv: TableLoad<Prv>,
    pub tcp: TableLoad<Tcp>,
    pub pcdf: TableLoad<Pcdf>,
    pub pcpc: TableLoad<Pcpc>,
}

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

pub(crate) struct Tpcf {
    pub spid: Info<PacketSpid>,
    pub name: Info<String>,
    pub size: Info<i64>,
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

pub(crate) struct Vpd {
    pub tpsd: Info<i64>,
    pub pos: Info<i64>,
    pub name: Info<ParameterName>,
    pub grpsize: Info<i64>,
    pub fixrep: Info<i64>,
    pub choice: Info<String>,
    pub pidref: Info<String>,
    pub disdesc: Info<String>,
    pub width: Info<i64>,
    pub justify: Info<String>,
    pub newline: Info<String>,
    pub dchar: Info<i64>,
    pub form: Info<String>,
    pub offset: Info<i64>,
}

pub(crate) struct Cur {
    pub pname: Info<ParameterName>,
    pub pos: Info<i64>,
    pub rlchk: Info<ParameterName>,
    pub valpar: Info<i64>,
    pub select: Info<String>,
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

pub(crate) struct Cap {
    pub numbr: Info<String>,
    pub xvals: Info<String>,
    pub yvals: Info<String>,
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

pub(crate) struct Lgf {
    pub ident: Info<String>,
    pub descr: Info<String>,
    pub pol1: Info<String>,
    pub pol2: Info<String>,
    pub pol3: Info<String>,
    pub pol4: Info<String>,
    pub pol5: Info<String>,
}

pub(crate) struct Txf {
    pub numbr: Info<String>,
    pub descr: Info<String>,
    pub rawfmt: Info<String>,
    pub nalias: Info<i64>,
}

pub(crate) struct Txp {
    pub numbr: Info<String>,
    pub from: Info<String>,
    pub to: Info<String>,
    pub altxt: Info<String>,
}

pub(crate) struct Ccf {
    pub cname: Info<CommandName>,
    pub descr: Info<String>,
    pub descr2: Info<String>,
    pub ctype: Info<String>,
    pub critical: Info<String>,
    pub pktid: Info<String>,
    pub r#type: Info<i64>,
    pub stype: Info<i64>,
    pub apid: Info<i64>,
    pub npars: Info<i64>,
    pub plan: Info<String>,
    pub exec: Info<String>,
    pub ilscope: Info<String>,
    pub ilstage: Info<String>,
    pub subsys: Info<i64>,
    pub hipri: Info<String>,
    pub mapid: Info<i64>,
    pub defset: Info<String>,
    pub rapid: Info<i64>,
    pub ack: Info<i64>,
    pub subschedid: Info<i64>,
}

pub(crate) struct Cdf {
    pub cname: Info<CommandName>,
    pub eltype: Info<String>,
    pub descr: Info<String>,
    pub ellen: Info<i64>,
    pub bit: Info<i64>,
    pub grpsize: Info<i64>,
    pub pname: Info<String>,
    pub r#inter: Info<String>,
    pub value: Info<String>,
    pub tmid: Info<String>,
}

pub(crate) struct Cpc {
    pub name: Info<String>,
    pub descr: Info<String>,
    pub ptc: Info<i64>,
    pub pfc: Info<i64>,
    pub dispfmt: Info<String>,
    pub radix: Info<String>,
    pub unit: Info<String>,
    pub categ: Info<String>,
    pub prfref: Info<String>,
    pub ccaref: Info<String>,
    pub pafref: Info<String>,
    pub r#inter: Info<String>,
    pub defval: Info<String>,
    pub corr: Info<String>,
    pub obtip: Info<i64>,
    pub descr2: Info<String>,
    pub endian: Info<String>,
}

pub(crate) struct Cca {
    pub numbr: Info<String>,
    pub descr: Info<String>,
    pub engfmt: Info<String>,
    pub rawfmt: Info<String>,
    pub radix: Info<String>,
    pub unit: Info<String>,
    pub ncurve: Info<i64>,
}

pub(crate) struct Ccs {
    pub numbr: Info<String>,
    pub xvals: Info<String>,
    pub yvals: Info<String>,
}

pub(crate) struct Paf {
    pub numbr: Info<String>,
    pub descr: Info<String>,
    pub rawfmt: Info<String>,
    pub nalias: Info<i64>,
}

pub(crate) struct Pas {
    pub numbr: Info<String>,
    pub altxt: Info<String>,
    pub alval: Info<String>,
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

pub(crate) struct Prv {
    pub numbr: Info<String>,
    pub minval: Info<String>,
    pub maxval: Info<String>,
}

pub(crate) struct Tcp {
    pub id: Info<String>,
    pub desc: Info<String>,
}

pub(crate) struct Pcdf {
    pub tcname: Info<String>,
    pub desc: Info<String>,
    pub r#type: Info<String>,
    pub len: Info<i64>,
    pub bit: Info<i64>,
    pub pname: Info<String>,
    pub value: Info<String>,
    pub radix: Info<String>,
}

pub(crate) struct Pcpc {
    pub pname: Info<String>,
    pub desc: Info<String>,
    pub code: Info<String>,
}

pub(crate) fn load(directory: &Path) -> Result<Records, LoadError> {
    std::fs::read_dir(directory).map_err(|cause| LoadError::InaccessibleDirectory {
        directory: directory.to_owned(),
        cause,
    })?;
    let mut reports = Vec::with_capacity(Table::ALL.len());
    let pcf = read_table(directory, Table::Pcf, parse_pcf, &mut reports);
    let caf = read_table(directory, Table::Caf, parse_caf, &mut reports);
    let pid = read_table(directory, Table::Pid, parse_pid, &mut reports);
    let tpcf = read_table(directory, Table::Tpcf, parse_tpcf, &mut reports);
    let pic = read_table(directory, Table::Pic, parse_pic, &mut reports);
    let plf = read_table(directory, Table::Plf, parse_plf, &mut reports);
    let vpd = read_table(directory, Table::Vpd, parse_vpd, &mut reports);
    let ccf = read_table(directory, Table::Ccf, parse_ccf, &mut reports);
    let cdf = read_table(directory, Table::Cdf, parse_cdf, &mut reports);
    let cpc = read_table(directory, Table::Cpc, parse_cpc, &mut reports);
    let cap = read_table(directory, Table::Cap, parse_cap, &mut reports);
    let mcf = read_table(directory, Table::Mcf, parse_mcf, &mut reports);
    let lgf = read_table(directory, Table::Lgf, parse_lgf, &mut reports);
    let txf = read_table(directory, Table::Txf, parse_txf, &mut reports);
    let txp = read_table(directory, Table::Txp, parse_txp, &mut reports);
    let cur = read_table(directory, Table::Cur, parse_cur, &mut reports);
    let cca = read_table(directory, Table::Cca, parse_cca, &mut reports);
    let ccs = read_table(directory, Table::Ccs, parse_ccs, &mut reports);
    let paf = read_table(directory, Table::Paf, parse_paf, &mut reports);
    let pas = read_table(directory, Table::Pas, parse_pas, &mut reports);
    let prf = read_table(directory, Table::Prf, parse_prf, &mut reports);
    let prv = read_table(directory, Table::Prv, parse_prv, &mut reports);
    let tcp = read_table(directory, Table::Tcp, parse_tcp, &mut reports);
    let pcdf = read_table(directory, Table::Pcdf, parse_pcdf, &mut reports);
    let pcpc = read_table(directory, Table::Pcpc, parse_pcpc, &mut reports);
    debug_assert_eq!(reports.len(), Table::ALL.len());
    if cur.rows().is_empty()
        && mcf.rows().is_empty()
        && txp.rows().is_empty()
        && txf.rows().is_empty()
        && lgf.rows().is_empty()
        && cap.rows().is_empty()
        && cdf.rows().is_empty()
        && cpc.rows().is_empty()
        && ccf.rows().is_empty()
        && pcf.rows().is_empty()
        && caf.rows().is_empty()
        && pid.rows().is_empty()
        && tpcf.rows().is_empty()
        && pic.rows().is_empty()
        && plf.rows().is_empty()
        && vpd.rows().is_empty()
        && cca.rows().is_empty()
        && ccs.rows().is_empty()
        && paf.rows().is_empty()
        && pas.rows().is_empty()
        && prf.rows().is_empty()
        && prv.rows().is_empty()
        && tcp.rows().is_empty()
        && pcdf.rows().is_empty()
        && pcpc.rows().is_empty()
    {
        return Err(LoadError::NoUsableSupportedRows {
            directory: directory.to_owned(),
        });
    }
    Ok(Records {
        reports: in_code_order(reports),
        pcf,
        pid,
        tpcf,
        pic,
        plf,
        vpd,
        cur,
        caf,
        cap,
        mcf,
        lgf,
        txf,
        txp,
        ccf,
        cdf,
        cpc,
        cca,
        ccs,
        paf,
        pas,
        prf,
        prv,
        tcp,
        pcdf,
        pcpc,
    })
}

impl<T> TableLoad<T> {
    pub(crate) fn rows(&self) -> &[Row<T>] {
        match self {
            Self::Read { rows } => rows,
            _ => &[],
        }
    }
    /// What this attempt reported, for the table listing.
    fn report(&self, table: Table) -> TableReport {
        TableReport {
            table,
            rows: match self {
                Self::Missing => TableRows::Missing,
                Self::Unreadable(_) => TableRows::Unreadable,
                Self::Read { rows } => TableRows::Read { rows: rows.len() },
            },
        }
    }
}

/// Reports read in the loader's own order, presented in code order.
/// Every loader read has one report, and `Table::ALL` lists every table once.
fn in_code_order(mut reports: Vec<TableReport>) -> Vec<TableReport> {
    let position = |table: Table| {
        Table::ALL
            .iter()
            .position(|t| *t == table)
            .expect("every loaded table is catalogued")
    };
    reports.sort_by_key(|report| position(report.table));
    reports
}

fn read_table<T>(
    directory: &Path,
    table: Table,
    parse: fn(&str, Source) -> Result<Row<T>, String>,
    reports: &mut Vec<TableReport>,
) -> TableLoad<T> {
    let file = table.file();
    let bytes = match std::fs::read(directory.join(file)) {
        Ok(bytes) => bytes,
        Err(cause) => {
            tracing::debug!(file, reason = %cause, "skipped file");
            let skipped = if cause.kind() == io::ErrorKind::NotFound {
                TableLoad::Missing
            } else {
                TableLoad::Unreadable(cause)
            };
            reports.push(skipped.report(table));
            return skipped;
        }
    };
    let mut rows = Vec::new();
    for (index, bytes) in bytes.split(|b| *b == b'\n').enumerate() {
        if bytes.is_empty() && index > 0 {
            continue;
        }
        let source = Source {
            file: file.into(),
            line: (index + 1).try_into().unwrap(),
        };
        let text = std::str::from_utf8(bytes).map(|s| s.strip_suffix('\r').unwrap_or(s));
        let parsed = match text {
            Ok(text) => parse(text, source),
            Err(e) => Err(e.to_string()),
        };
        match parsed {
            Ok(row) => rows.push(row),
            Err(reason) => {
                tracing::debug!(file, line = index + 1, %reason, original = %String::from_utf8_lossy(bytes), "dropped row")
            }
        }
    }
    tracing::debug!(file, retained = rows.len(), "loaded table");
    let loaded = TableLoad::Read { rows };
    reports.push(loaded.report(table));
    loaded
}

#[derive(Clone, Copy)]
enum CellType {
    Text,
    Integer,
    Code(&'static str),
}
struct Column {
    name: &'static str,
    kind: CellType,
    required: bool,
    default: Option<&'static str>,
}
use CellType::{Code, Integer, Text};
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

fn parse_definition(text: &str, source: Source, schema: &[Column]) -> Result<Definition, String> {
    let cells: Vec<_> = text.split('\t').collect();
    if cells.len() > schema.len() {
        tracing::debug!(file = %source.file.display(), line = source.line.get(), extra = cells.len() - schema.len(), "ignored extra columns");
    }
    let mut fields = Vec::with_capacity(schema.len());
    for (index, column) in schema.iter().enumerate() {
        let recorded = cells.get(index).copied();
        let presence = match recorded {
            None => Presence::Omitted,
            Some("") => Presence::Empty,
            Some(s) => Presence::Text(s.to_owned()),
        };
        let value = recorded.filter(|s| !s.is_empty()).or(column.default);
        if column.required && value.is_none() {
            return Err(format!("{} is required", column.name));
        }
        let interpretation = value
            .map(|s| {
                let value = match column.kind {
                    Text => Scalar::Text(s.to_owned()),
                    Integer => Scalar::Integer(
                        s.parse()
                            .map_err(|_| format!("{}: invalid integer {s:?}", column.name))?,
                    ),
                    Code(allowed) if s.len() == 1 && allowed.contains(s) => {
                        Scalar::Code(s.to_owned())
                    }
                    Code(_) => return Err(format!("{}: invalid code {s:?}", column.name)),
                };
                let origin = if recorded.is_some_and(|s| !s.is_empty()) {
                    InterpretationOrigin::Recorded
                } else {
                    InterpretationOrigin::DocumentedDefault {
                        rule: format!("{} defaults to {s:?} when omitted or empty", column.name),
                    }
                };
                Ok(Interpretation { value, origin })
            })
            .transpose()?;
        fields.push(RecordedField {
            column: (index + 1).try_into().unwrap(),
            presence,
            meanings: vec![FieldMeaning {
                schema_name: column.name.into(),
                interpretation: Info {
                    value: interpretation,
                    problems: vec![],
                    sources: vec![source.clone()],
                },
            }],
        });
    }
    Ok(Definition { source, fields })
}

fn cell<T>(
    definition: &Definition,
    index: usize,
    convert: impl FnOnce(&Scalar) -> Option<T>,
) -> Info<T> {
    let interpretation = &definition.fields[index].meanings[0].interpretation;
    Info {
        value: interpretation
            .value
            .as_ref()
            .and_then(|i| convert(&i.value)),
        problems: interpretation.problems.clone(),
        sources: interpretation.sources.clone(),
    }
}
fn text_cell(definition: &Definition, index: usize) -> Info<String> {
    cell(definition, index, |s| match s {
        Scalar::Text(s) | Scalar::Code(s) => Some(s.clone()),
        _ => None,
    })
}
fn integer_cell(definition: &Definition, index: usize) -> Info<i64> {
    cell(definition, index, |s| match s {
        Scalar::Integer(n) => Some(*n),
        _ => None,
    })
}
fn parse_pcf(text: &str, source: Source) -> Result<Row<Pcf>, String> {
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
fn parse_caf(text: &str, source: Source) -> Result<Row<Caf>, String> {
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
fn parse_pid(text: &str, source: Source) -> Result<Row<Pid>, String> {
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
fn parse_tpcf(text: &str, source: Source) -> Result<Row<Tpcf>, String> {
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
fn parse_pic(text: &str, source: Source) -> Result<Row<Pic>, String> {
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
fn parse_plf(text: &str, source: Source) -> Result<Row<Plf>, String> {
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
