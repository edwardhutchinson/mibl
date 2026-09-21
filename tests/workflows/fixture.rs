//! The combined synthetic snapshot every workflow scenario loads.
//!
//! One definition carries all twenty-five canonical supporting table families at
//! once, with the bytes and the load order every scenario expects. The constants
//! a scenario reads directly stay visible to the suite; the rest are private to
//! this file. The temporary directory still comes from `tests/common/mod.rs`.

use crate::common::Fixture;

/// A status-category parameter whose referenced key belongs to the textual family, so its
/// category and reference disagree, plus a parameter whose calibration key resolves nowhere.
pub(crate) const PCF: &str = "\
DEMO_MODE\tOperational mode\t\t\t3\t4\t\t\t\tN\tR\tDEMO_MODE_TXF
DEMO_TEMP\tTemperature\t\tK\t3\t12\t\t\t\tN\tR\tDEMO_TEMP_CAF
DEMO_STATE\tState\t\t\t3\t4\t\t\t\tS\tR\tDEMO_STATE_TXF
DEMO_COUNT\tCount\t\t\t3\t4\t\t\t\tN\tR
DEMO_ABSENT\tParameter whose calibration is undeclared\t\t\t3\t4\t\t\t\tN\tR\tDEMO_ABSENT_CAL
";

/// A fixed packet with a repeated occurrence beside a variable packet sharing its parameters.
const PID: &str = "\
3\t25\t42\t7\t0\t89000\tDemonstration housekeeping\t\t-1\t10
3\t26\t42\t0\t0\t89001\tDemonstration variable packet\t\t7\t10
";

const TPCF: &str = "\
89000\tDEMO_HK\t32
89001\tDEMO_VAR
";

const PIC: &str = "\
3\t25\t16\t8\t-1\t0\t42
3\t26\t16\t8\t-1\t0\t42
";

const PLF: &str = "\
DEMO_MODE\t89000\t19\t0\t1\t0\t0\t1
DEMO_TEMP\t89000\t20\t0\t2\t16\t0\t1
DEMO_STATE\t89000\t24\t0\t1\t0\t0\t1
";

pub(crate) const VPD: &str = "\
7\t1\tDEMO_COUNT\t2\t0\tN\tN\t\t0
7\t2\tDEMO_TEMP\t0\t0\tN\tN\t\t0
7\t3\tDEMO_MODE\t0\t0\tN\tN\t\t0
";

/// Two conditional alternatives for `DEMO_TEMP` beside its direct `PCF_CURTX` declaration.
const CUR: &str = "\
DEMO_TEMP\t1\tDEMO_MODE\t0\tDEMO_TEMP_MCF
DEMO_TEMP\t2\tDEMO_MODE\t1\tDEMO_TEMP_LGF
";

pub(crate) const CAF: &str = "DEMO_TEMP_CAF\tTemperature calibration\tR\tU\tH";

pub(crate) const CAP: &str = "\
DEMO_TEMP_CAF\tA\t1.5
DEMO_TEMP_CAF\tFF\t2.5
";

const MCF: &str = "DEMO_TEMP_MCF\tTemperature polynomial\t1.5";

const LGF: &str = "DEMO_TEMP_LGF\tTemperature logarithm\t2";

/// Two textual headers share one key, so its reference keeps both definitions.
const TXF: &str = "\
DEMO_STATE_TXF\tState calibration\tU\t2
DEMO_STATE_TXF\tState calibration duplicate\tU\t2
DEMO_MODE_TXF\tMode calibration\tU\t2
";

const TXP: &str = "\
DEMO_STATE_TXF\t0\t1\tIDLE
DEMO_STATE_TXF\t2\t3\tACTIVE
DEMO_MODE_TXF\t0\t0\tOFFLINE
DEMO_MODE_TXF\t1\t1\tONLINE
";

const CCF: &str = "\
DEMO_TC001\tDemonstration command one\tDistribute demonstration commands\t\tN\tDEMO_HDR01\t3\t25\t42\t3
DEMO_TC074\tDemonstration command two\tEnable forwarding of demonstration packets\t\tN\tDEMO_HDR02\t14\t1\t28\t6
DEMO_TC003\tThird demonstration command\tDeclared widths and an empty header\t\tN\tDEMO_HDR03\t3\t25\t42\t2
";

/// One malformed row is dropped while every neighbouring declaration stays usable.
const CDF: &str = "\
DEMO_TC001\tE\tFirst argument\t8\t0\t\tARG1\tR
DEMO_TC001\tE\tSecond argument\t8\t8\t\tARG2\tT\t0\tDEMO_COUNT
DEMO_TC001\tE\tThird argument\t8\t16\t\tARG3\tR
DEMO_TC074\tF\tGroup count\t8\t0\t6\tN1\tR\t1
DEMO_TC074\tE\tApplication id\t16\t8\t\tAPID\tR
DEMO_TC074\tA\tPadding\t4\t24\t\t\t\tF
DEMO_TC074\tF\tType count\t8\t28\t3\tN2\tR\t1
DEMO_TC074\tE\tType\t8\t36\t\tTYPE\tR
DEMO_TC074\tF\tSubtype count\t8\t44\t1\tN3\tR\t1
DEMO_TC074\tE\tSubtype\t8\t52\t\tSUBTYPE\tR
DEMO_TC003\tE\tAgreeing width\t8\t0\t\tARG4\tR
DEMO_TC003\tE\tDisagreeing width\t16\t8\t\tARG5\tR
DEMO_ORPHAN\tX\tUnsupported element type\t8\t0
";

const CPC: &str = "\
ARG1\tFirst argument\t3\t4\tR\tH\t\tN\tDEMO_RANGE_1\tDEMO_CONV_1\tDEMO_ALIAS_1
ARG2\tSecond argument\t3\t4\tR\tO\t\tN
ARG3\tThird argument\t3\t4\tR\tH\t\tN\tDEMO_ABSENT_RANGE
N1\tGroup count\t3\t4
APID\tApplication id\t3\t12
N2\tType count\t3\t4
TYPE\tType\t3\t4\tR\tH\t\tN\tDEMO_RANGE_2
N3\tSubtype count\t3\t4
SUBTYPE\tSubtype\t3\t4
ARG4\tAgreeing argument\t3\t4
ARG5\tDisagreeing argument\t3\t4
ORPHAN_ARG\tOrphan argument\tbroken\t4
";

const PRF: &str = "\
DEMO_RANGE_1\tFirst argument range\tE\tU\tH\t2\tmAmp
DEMO_RANGE_2\tType range\tR\tU\tD\t1
";

pub(crate) const PRV: &str = "\
DEMO_RANGE_1\t20\t7F
DEMO_RANGE_1\t0\t3
DEMO_RANGE_2\t0\t2
";

const CCA: &str = "DEMO_CONV_1\tFirst argument conversion\tR\tU\tH\tmAmp\t2";

const CCS: &str = "\
DEMO_CONV_1\t10\t1.5
DEMO_CONV_1\tFF\t3.5
";

const PAF: &str = "DEMO_ALIAS_1\tFirst argument alias\tR\t2";

const PAS: &str = "\
DEMO_ALIAS_1\tLOW\t0
DEMO_ALIAS_1\tHIGH\t2.5
";

/// One row carries unknown extra trailing columns the compatibility policy ignores.
const TCP: &str = "\
DEMO_HDR01\tDemonstration command header
DEMO_HDR02\tExtended demonstration header
DEMO_HDR03\tDemonstration header without elements\t\t\textra\tcolumns
";

const PCDF: &str = "\
DEMO_HDR01\tPkt Version Number\tF\t3\t0\t\t1\tD
DEMO_HDR01\tPkt Type\tF\t1\t3\t\t1\tD
DEMO_HDR01\tAPID\tA\t11\t5\tHP002\t2A\t
DEMO_HDR01\tSequence Count\tP\t14\t16\tHP004\t0\t
DEMO_HDR01\tAck Flags\tK\t4\t30\tHP001\t0\t
DEMO_HDR01\tService Type\tT\t8\t34\tHP005\t0\t
DEMO_HDR01\tService Subtype\tS\t8\t42\tHP006\t0\t
DEMO_HDR01\tPkt Length\tP\t16\t50\tHP003\t0\t
DEMO_HDR02\tPkt Version Number\tF\t3\t0\t\t0\tD
DEMO_HDR02\tAPID\tA\t11\t5\tHP002\t2A\t
DEMO_HDR02\tService Type\tT\t8\t16\tHP005\t0\t
DEMO_HDR02\tService Subtype\tS\t8\t24\tHP006\t0\t
";

pub(crate) const PCPC: &str = "\
HP001\tAcknowledgement flags\tU
HP002\tApplication process id\tU
HP003\tPacket length\tI
HP004\tSequence count\tI
HP005\tService type\tU
HP006\tService subtype\tU
";

/// Every canonical supporting table family the contract names, in load order.
pub(crate) const TABLES: [(&str, &str); 25] = [
    ("pcf.dat", PCF),
    ("pid.dat", PID),
    ("tpcf.dat", TPCF),
    ("pic.dat", PIC),
    ("plf.dat", PLF),
    ("vpd.dat", VPD),
    ("cur.dat", CUR),
    ("caf.dat", CAF),
    ("cap.dat", CAP),
    ("mcf.dat", MCF),
    ("lgf.dat", LGF),
    ("txf.dat", TXF),
    ("txp.dat", TXP),
    ("ccf.dat", CCF),
    ("cdf.dat", CDF),
    ("cpc.dat", CPC),
    ("prf.dat", PRF),
    ("prv.dat", PRV),
    ("cca.dat", CCA),
    ("ccs.dat", CCS),
    ("paf.dat", PAF),
    ("pas.dat", PAS),
    ("tcp.dat", TCP),
    ("pcdf.dat", PCDF),
    ("pcpc.dat", PCPC),
];

fn write_snapshot(dir: &Fixture) {
    for (file, text) in TABLES {
        dir.write(file, text);
    }
}

pub(crate) fn combined() -> Fixture {
    let dir = Fixture::new();
    write_snapshot(&dir);
    dir
}
