//! Helpers more than one workflow scenario needs: loading the combined
//! fixture, extracting a found description, naming provenance and declared
//! positions, and digesting every query kind of one snapshot.
//!
//! A helper used by a single scenario stays beside that scenario instead.

use crate::common::Fixture;
use mibl::{
    Mib,
    model::{
        CommandArgument, CommandDescription, CommandElement, CommandName, Info, Layout, Lookup,
        PacketDescription, PacketOccurrences, PacketSpid, ParameterDescription, ParameterName,
        Position, Repetition, SearchScope, Source,
    },
};
use std::{num::NonZeroUsize, path::PathBuf};

pub(crate) fn loaded(dir: &Fixture) -> Mib {
    Mib::load(dir.path()).unwrap()
}

pub(crate) fn parameter(mib: &Mib, name: &str) -> ParameterDescription {
    let Lookup::Found(description) = mib.parameter(&ParameterName(name.into())) else {
        panic!("expected parameter {name}")
    };
    description
}

pub(crate) fn packet(mib: &Mib, spid: u64) -> PacketDescription {
    let Lookup::Found(description) = mib.packet(PacketSpid(spid)) else {
        panic!("expected packet {spid}")
    };
    description
}

pub(crate) fn command(mib: &Mib, name: &str) -> CommandDescription {
    let Lookup::Found(description) = mib.command(&CommandName(name.into())) else {
        panic!("expected command {name}")
    };
    description
}

pub(crate) fn source(file: &str, line: usize) -> Source {
    Source {
        file: PathBuf::from(file),
        line: NonZeroUsize::new(line).unwrap(),
    }
}

pub(crate) fn spids(packets: &[PacketOccurrences]) -> Vec<u64> {
    packets
        .iter()
        .map(|group| group.packet.value.as_ref().unwrap().spid.0)
        .collect()
}

/// Declared positions exactly as the contract describes them.
pub(crate) fn position_text(position: &Info<Position>) -> String {
    match &position.value {
        Some(Position::PacketAbsolute { byte, bit }) => format!("byte {byte} bit {bit}"),
        Some(Position::RelativeBits(bits)) => format!("relative {bits} bits"),
        Some(Position::ApplicationDeclaredBit(bit)) => format!("application-declared bit {bit}"),
        Some(Position::HeaderBit(bit)) => format!("header bit {bit}"),
        Some(Position::Runtime(_)) => "runtime".into(),
        None => "unavailable".into(),
    }
}

pub(crate) fn repetition_text(repetition: &Info<Repetition>) -> String {
    match &repetition.value {
        Some(Repetition::Fixed { count, stride_bits }) => {
            format!("fixed {count} stride {:?}", stride_bits.value)
        }
        Some(Repetition::Runtime(declaration)) => format!("runtime {}", declaration.expression),
        None => "unavailable".into(),
    }
}

/// Every declared argument, whether or not a group encloses it.
pub(crate) fn declared_arguments(layout: &[Layout<CommandElement>]) -> Vec<&CommandArgument> {
    let mut arguments = Vec::new();
    for node in layout {
        match node {
            Layout::Element(CommandElement::Argument(a)) => arguments.push(a.as_ref()),
            Layout::Element(CommandElement::Fixed(_)) => {}
            Layout::Repeat { children, .. } | Layout::Conditional { children, .. } => {
                arguments.extend(declared_arguments(children));
            }
        }
    }
    arguments
}

/// Every query kind of the combined snapshot, formatted for comparison.
pub(crate) fn query_digest(mib: &Mib) -> String {
    format!(
        "parameter {:?}\npacket {:?}\ncommand {:?}\nsearch {:?}\npus {:?}",
        mib.parameter(&ParameterName("DEMO_TEMP".into())),
        mib.packet(PacketSpid(89001)),
        mib.command(&CommandName("DEMO_TC074".into())),
        mib.search("mode", SearchScope::All),
        mib.pus(3, Some(25)),
    )
}
