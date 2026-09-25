mod common;

use common::{Fixture, PARAMETER};
use mibl::{Mib, model::*};

#[test]
fn inventory_lists_every_root_in_identity_and_source_order_without_changing_blank_search() {
    let dir = Fixture::new();
    dir.write("pcf.dat", &format!("{PARAMETER}\n{PARAMETER}"));
    dir.write(
        "pid.dat",
        "3\t25\t42\t7\t0\t100\tMode\t\t-1\t10\n3\t25\t42\t7\t0\t20\tMode\t\t-1\t10",
    );
    dir.write("ccf.dat", "SET_STATE\tMode\t\t\t\tHEADER");
    let mib = Mib::load(dir.path()).unwrap();
    let all = mib.inventory(SearchScope::All);
    assert_eq!(
        all.iter().map(|c| c.identity.clone()).collect::<Vec<_>>(),
        vec![
            Identity::Parameter(ParameterName("TEMP".into())),
            Identity::Parameter(ParameterName("TEMP".into())),
            Identity::Packet(PacketSpid(20)),
            Identity::Packet(PacketSpid(100)),
            Identity::Command(CommandName("SET_STATE".into())),
        ]
    );
    assert_eq!(all[0].source.line.get(), 1);
    assert_eq!(all[1].source.line.get(), 2);
    assert!(matches!(
        mib.parameter(&ParameterName("TEMP".into())),
        Lookup::Ambiguous(_)
    ));
    for (scope, count) in [
        (SearchScope::Parameters, 2),
        (SearchScope::Packets, 2),
        (SearchScope::Commands, 1),
    ] {
        assert_eq!(mib.inventory(scope).len(), count);
        assert!(mib.search("", scope).is_empty());
    }
    drop(dir);
    assert_eq!(mib.inventory(SearchScope::All).len(), 5);
}
