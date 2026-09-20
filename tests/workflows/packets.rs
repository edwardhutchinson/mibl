//! Packet acceptance scenarios over the combined snapshot: identification
//! criteria, a fixed layout with a bounded repetition, a variable layout with
//! declared runtime groups, and deterministic ordering.
//!
//! Each test loads the combined snapshot itself, so no test depends on another
//! test's assertions or on the order they run in.

use crate::{
    fixture::combined,
    support::{loaded, packet, position_text, query_digest, repetition_text, source},
};
use mibl::model::{Enclosure, Layout, PacketSpid, Repetition};

#[test]
fn packet_identification_keeps_declared_criteria_and_extraction() {
    let dir = combined();
    let mib = loaded(&dir);
    let housekeeping = packet(&mib, 89000);
    assert_eq!(housekeeping.packet.spid, PacketSpid(89000));
    assert_eq!(housekeeping.packet.name.value.as_deref(), Some("DEMO_HK"));
    assert_eq!(
        housekeeping.packet.description.value.as_deref(),
        Some("Demonstration housekeeping")
    );
    assert_eq!(housekeeping.identification.apid.value, Some(42));
    assert_eq!(housekeeping.identification.service_type.value, Some(3));
    assert_eq!(housekeeping.identification.service_subtype.value, Some(25));
    let criteria = housekeeping.identification.criteria.value.as_ref().unwrap();
    assert_eq!(criteria.len(), 1);
    assert_eq!(criteria[0].expected.value, Some(7));
    assert_eq!(
        position_text(&criteria[0].extraction.position),
        "byte 16 bit 0"
    );
    assert_eq!(criteria[0].extraction.encoded_bits.value, Some(8));
}

#[test]
fn packet_layout_lists_declared_occurrences_and_expands_bounded_repetition() {
    let dir = combined();
    let mib = loaded(&dir);
    let housekeeping = packet(&mib, 89000);
    // Declared order, bounded expansion of the two-place repetition and embedded calibrations.
    let layout = housekeeping.layout.value.as_ref().unwrap();
    let described: Vec<String> = layout
        .iter()
        .map(|node| {
            let Layout::Element(occurrence) = node else {
                panic!("expected a fixed occurrence: {node:?}")
            };
            format!(
                "{} at {} width {:?} enclosing {} calibrations {}",
                occurrence.reference.0,
                position_text(&occurrence.location.position),
                occurrence.location.encoded_bits.value,
                occurrence.enclosing.len(),
                occurrence
                    .parameter
                    .value
                    .as_ref()
                    .unwrap()
                    .calibrations
                    .value
                    .as_ref()
                    .map_or(0, Vec::len)
            )
        })
        .collect();
    assert_eq!(
        described,
        vec![
            "DEMO_MODE at byte 19 bit 0 width Some(8) enclosing 0 calibrations 1",
            "DEMO_TEMP at byte 20 bit 0 width Some(16) enclosing 1 calibrations 3",
            "DEMO_TEMP at byte 22 bit 0 width Some(16) enclosing 1 calibrations 3",
            "DEMO_STATE at byte 24 bit 0 width Some(8) enclosing 0 calibrations 2",
        ]
    );
    for node in &layout[1..3] {
        let Layout::Element(occurrence) = node else {
            panic!()
        };
        let [Enclosure::Repetition(repetition)] = &occurrence.enclosing[..] else {
            panic!()
        };
        assert_eq!(repetition_text(repetition), "fixed 2 stride Some(16)");
    }
}

#[test]
fn variable_packet_layout_keeps_declared_groups_and_runtime_dependencies() {
    let dir = combined();
    let mib = loaded(&dir);
    // The variable packet keeps its declared groups and runtime dependencies unexpanded.
    let variable = packet(&mib, 89001);
    assert_eq!(variable.packet.name.value.as_deref(), Some("DEMO_VAR"));
    let layout = variable.layout.value.as_ref().unwrap();
    assert_eq!(layout.len(), 2);
    let Layout::Element(count) = &layout[0] else {
        panic!()
    };
    assert_eq!(count.reference.0, "DEMO_COUNT");
    assert_eq!(position_text(&count.location.position), "byte 10 bit 0");
    let Layout::Repeat {
        definition,
        repetition,
        children,
    } = &layout[1]
    else {
        panic!("expected the declared group")
    };
    assert_eq!(definition.source, source("vpd.dat", 1));
    assert!(repetition_text(repetition).contains("value of DEMO_COUNT"));
    let Some(Repetition::Runtime(declaration)) = &repetition.value else {
        panic!()
    };
    let targets = declaration.dependencies[0].targets.value.as_ref().unwrap();
    assert_eq!(
        targets.len(),
        1,
        "dependency targets are recorded definitions only"
    );
    assert_eq!(targets[0].definition.source, source("pcf.dat", 4));
    let described: Vec<String> = children
        .iter()
        .map(|node| {
            let Layout::Element(occurrence) = node else {
                panic!()
            };
            format!(
                "{} at {} width {:?} enclosing {}",
                occurrence.reference.0,
                position_text(&occurrence.location.position),
                occurrence.location.encoded_bits.value,
                occurrence.enclosing.len()
            )
        })
        .collect();
    assert_eq!(
        described,
        vec![
            "DEMO_TEMP at relative 0 bits width Some(16) enclosing 1",
            "DEMO_MODE at relative 16 bits width Some(8) enclosing 1",
        ]
    );
}

#[test]
fn packet_queries_keep_ordering_deterministic_across_independent_loads() {
    let dir = combined();
    let mib = loaded(&dir);
    // Ordering is deterministic across independent loads of the same declarations.
    assert_eq!(query_digest(&loaded(&combined())), query_digest(&mib));
}
