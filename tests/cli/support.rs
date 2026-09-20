use crate::common::Fixture;
use std::process::Command;
use std::process::Output;

pub(crate) fn run(dir: &Fixture, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mibl"))
        .env("MIB_DIR", dir.path())
        .args(args)
        .output()
        .unwrap()
}

pub(crate) fn accepted_fixture() -> Fixture {
    let dir = Fixture::new();
    dir.write("pcf.dat", "DEMO_TEMP\tDemonstration temperature\t\tK\t3\t12\t16\t\t\tN\tR\t\t\t\t\t\t\t\t\t\t\t\tB\nDEMO_MODE\tDemonstration mode\t\t\t2\t8\t8\t\t\tS\tR\t\t\t\t\t\t\t\t\t\t\t\tB\nDEMO_DUP\tFirst duplicate\t\t\t3\t4\t8\t\t\tN\tR\nDEMO_DUP\tSecond duplicate\t\t\t3\t4\t8\t\t\tN\tR");
    dir.write("pid.dat", "3\t25\t42\t0\t0\t42001\tDemonstration housekeeping\t\t-1\t10\n3\t26\t42\t7\t0\t42002\tDemonstration packet with partial definitions\t\t-1\t10");
    dir.write(
        "tpcf.dat",
        "42001\tDEMO_HK\n42002\tDEMO_HK_A\n42002\tDEMO_HK_B",
    );
    dir.write(
        "pic.dat",
        "3\t25\t-1\t0\t-1\t0\n3\t26\t10\t8\t-1\t0\n3\t26\t10\t16\t-1\t0",
    );
    dir.write("plf.dat", "DEMO_TEMP\t42001\t16\t0\t1\nDEMO_MODE\t42002\t20\t0\t2\t16\nDEMO_UNKNOWN\t42002\t24\t0\t1\nDEMO_DUP\t42002\t25\t0\t1");
    dir
}
