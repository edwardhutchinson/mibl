//! Encoding and declared placement, including repetition and conditions.

use super::evidence::{Definition, Info, RuntimeDeclaration};

#[derive(Clone, Debug)]
pub struct Encoding {
    pub ptc: Info<u16>,
    pub pfc: Info<u32>,
    pub endian: Info<String>,
    pub encoded_bits: Info<u64>,
}

#[derive(Clone, Debug)]
pub enum Position {
    PacketAbsolute {
        byte: u64,
        bit: u8,
    },
    /// CDF_BIT is after the header with repetition counts taken as one.
    ApplicationDeclaredBit(u64),
    HeaderBit(u64),
    RelativeBits(i64),
    Runtime(RuntimeDeclaration),
}

#[derive(Clone, Debug)]
pub struct Location {
    pub position: Info<Position>,
    pub encoded_bits: Info<u64>,
    pub constraints: Vec<RuntimeDeclaration>,
}

#[derive(Clone, Debug)]
pub enum Repetition {
    Fixed { count: u64, stride_bits: Info<u64> },
    Runtime(RuntimeDeclaration),
}

#[derive(Clone, Debug)]
pub enum Enclosure {
    Repetition(Info<Repetition>),
    Condition(RuntimeDeclaration),
}

/// Siblings in declared order; duplicate positions survive with source tie-breaks.
#[derive(Clone, Debug)]
pub enum Layout<T> {
    Element(T),
    Repeat {
        definition: Definition,
        repetition: Info<Repetition>,
        children: Vec<Layout<T>>,
    },
    Conditional {
        definition: Definition,
        condition: RuntimeDeclaration,
        children: Vec<Layout<T>>,
    },
}
