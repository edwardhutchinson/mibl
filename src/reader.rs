//! Private typed rows. No parsing or schema detection is implemented.
use crate::{LoadError, model::*};
use std::{io, path::Path};

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

pub(crate) fn load(_directory: &Path) -> Result<Records, LoadError> {
    todo!("interface only")
}
