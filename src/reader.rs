//! Private typed rows and shared loading. Each table family's cells, schema
//! and parser live together in its own module.

use crate::{LoadError, model::*};
use std::{io, path::Path};

mod calibrations;
mod commands;
mod fields;
mod header;
mod packets;
mod parameters;
mod variable;

pub(crate) use calibrations::{Caf, Cap, Cur, Lgf, Mcf, Txf, Txp};
use calibrations::{parse_caf, parse_cap, parse_cur, parse_lgf, parse_mcf, parse_txf, parse_txp};
pub(crate) use commands::{Cca, Ccf, Ccs, Cdf, Cpc, Paf, Pas, Prf, Prv};
use commands::{
    parse_cca, parse_ccf, parse_ccs, parse_cdf, parse_cpc, parse_paf, parse_pas, parse_prf,
    parse_prv,
};
pub(crate) use header::{Pcdf, Pcpc, Tcp};
use header::{parse_pcdf, parse_pcpc, parse_tcp};
pub(crate) use packets::{Pic, Pid, Plf, Tpcf};
use packets::{parse_pic, parse_pid, parse_plf, parse_tpcf};
pub(crate) use parameters::Pcf;
use parameters::parse_pcf;
pub(crate) use variable::Vpd;
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
