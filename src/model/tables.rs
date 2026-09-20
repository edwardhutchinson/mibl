//! The MIB tables the loader reads, and what each loaded table turned out to be.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Table {
    Pcf,
    Pid,
    Tpcf,
    Pic,
    Plf,
    Vpd,
    Cur,
    Caf,
    Cap,
    Mcf,
    Lgf,
    Txf,
    Txp,
    Ccf,
    Cdf,
    Cpc,
    Cca,
    Ccs,
    Paf,
    Pas,
    Prf,
    Prv,
    Tcp,
    Pcdf,
    Pcpc,
}
impl Table {
    /// Every table the loader reads, ordered by code. The file name orders identically,
    /// because each file name is its lowercase code with a `.dat` suffix.
    pub const ALL: [Table; 25] = [
        Table::Caf,
        Table::Cap,
        Table::Cca,
        Table::Ccf,
        Table::Ccs,
        Table::Cdf,
        Table::Cpc,
        Table::Cur,
        Table::Lgf,
        Table::Mcf,
        Table::Paf,
        Table::Pas,
        Table::Pcdf,
        Table::Pcf,
        Table::Pcpc,
        Table::Pic,
        Table::Pid,
        Table::Plf,
        Table::Prf,
        Table::Prv,
        Table::Tcp,
        Table::Tpcf,
        Table::Txf,
        Table::Txp,
        Table::Vpd,
    ];
    /// The declared code, as it appears in field names, reference labels and rows.
    pub fn code(self) -> &'static str {
        match self {
            Table::Pcf => "PCF",
            Table::Pid => "PID",
            Table::Tpcf => "TPCF",
            Table::Pic => "PIC",
            Table::Plf => "PLF",
            Table::Vpd => "VPD",
            Table::Cur => "CUR",
            Table::Caf => "CAF",
            Table::Cap => "CAP",
            Table::Mcf => "MCF",
            Table::Lgf => "LGF",
            Table::Txf => "TXF",
            Table::Txp => "TXP",
            Table::Ccf => "CCF",
            Table::Cdf => "CDF",
            Table::Cpc => "CPC",
            Table::Cca => "CCA",
            Table::Ccs => "CCS",
            Table::Paf => "PAF",
            Table::Pas => "PAS",
            Table::Prf => "PRF",
            Table::Prv => "PRV",
            Table::Tcp => "TCP",
            Table::Pcdf => "PCDF",
            Table::Pcpc => "PCPC",
        }
    }
    /// The file name the loader reads, relative to the MIB directory.
    pub fn file(self) -> &'static str {
        match self {
            Table::Pcf => "pcf.dat",
            Table::Pid => "pid.dat",
            Table::Tpcf => "tpcf.dat",
            Table::Pic => "pic.dat",
            Table::Plf => "plf.dat",
            Table::Vpd => "vpd.dat",
            Table::Cur => "cur.dat",
            Table::Caf => "caf.dat",
            Table::Cap => "cap.dat",
            Table::Mcf => "mcf.dat",
            Table::Lgf => "lgf.dat",
            Table::Txf => "txf.dat",
            Table::Txp => "txp.dat",
            Table::Ccf => "ccf.dat",
            Table::Cdf => "cdf.dat",
            Table::Cpc => "cpc.dat",
            Table::Cca => "cca.dat",
            Table::Ccs => "ccs.dat",
            Table::Paf => "paf.dat",
            Table::Pas => "pas.dat",
            Table::Prf => "prf.dat",
            Table::Prv => "prv.dat",
            Table::Tcp => "tcp.dat",
            Table::Pcdf => "pcdf.dat",
            Table::Pcpc => "pcpc.dat",
        }
    }
    /// What the table's rows declare, in the glossary's terms.
    pub fn meaning(self) -> &'static str {
        match self {
            Table::Pcf => "Monitoring parameter definitions",
            Table::Pid => "Telemetry packet definitions",
            Table::Tpcf => "Recorded packet names and sizes",
            Table::Pic => "Packet identification criteria",
            Table::Plf => "Parameter occurrence locations",
            Table::Vpd => "Variable packet layout elements",
            Table::Cur => "Conditional calibration alternatives",
            Table::Caf => "Numerical calibration definitions",
            Table::Cap => "Numerical calibration curve points",
            Table::Mcf => "Polynomial calibration coefficients",
            Table::Lgf => "Logarithmic calibration coefficients",
            Table::Txf => "Textual calibration definitions",
            Table::Txp => "Textual calibration intervals",
            Table::Ccf => "Telecommand definitions",
            Table::Cdf => "Telecommand argument elements",
            Table::Cpc => "Command argument definitions",
            Table::Cca => "Command conversion definitions",
            Table::Ccs => "Command conversion curve points",
            Table::Paf => "Alias definitions for argument values",
            Table::Pas => "Alias values",
            Table::Prf => "Allowed range definitions",
            Table::Prv => "Range boundary values",
            Table::Tcp => "Outgoing packet header definitions",
            Table::Pcdf => "Command header field definitions",
            Table::Pcpc => "Command header parameters",
        }
    }
    /// The root kind a table's rows are looked up as, when a lookup addresses them directly.
    pub fn root(self) -> Option<TableRoot> {
        match self {
            Table::Pcf => Some(TableRoot::Parameter),
            Table::Pid => Some(TableRoot::Packet),
            Table::Ccf => Some(TableRoot::Command),
            _ => None,
        }
    }
}
/// The kind of root definition a table's rows are addressed as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TableRoot {
    Parameter,
    Packet,
    Command,
}
/// One table as the loader left it. Every read attempt is reported, not only the failures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableReport {
    pub table: Table,
    pub rows: TableRows,
}
/// A readable table is distinguished from an absent one even when it retains no rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TableRows {
    Missing,
    Unreadable,
    Read { rows: usize },
}
