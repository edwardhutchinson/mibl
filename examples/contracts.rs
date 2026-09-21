//! Compile-only consumers. main deliberately does not invoke the placeholder operations.
#![allow(dead_code)]

use mibl::{LoadError, Mib, model::*};
use std::path::Path;

fn snapshot(directory: &Path) -> Result<ParameterDescription, LoadError> {
    let mib = Mib::load(directory)?;
    let result = mib.parameter(&ParameterName("ZUT00002".into()));
    drop(mib); // Owned descriptions outlive the snapshot.
    match result {
        Lookup::Found(description) => Ok(description),
        Lookup::NotFound(NotFoundReason::NoMatchingIdentity) => panic!("absent identity"),
        Lookup::NotFound(NotFoundReason::DefinitionsUnavailable) => panic!("no usable PCF roots"),
        Lookup::Ambiguous(AtLeastTwo {
            first,
            second,
            rest,
        }) => {
            let _candidates = (first, second, rest);
            panic!("candidate table required")
        }
    }
}

fn workflows(mib: &Mib) {
    let _: Lookup<PacketDescription> = mib.packet(PacketSpid(89000));
    let _: Lookup<CommandDescription> = mib.command(&CommandName("S2KTC001".into()));
    let _: Lookup<CommandDescription> = mib.command(&CommandName("S2KTC074".into()));
    let _: Vec<Candidate> = mib.search("mode", SearchScope::All);
}

fn incomplete(info: Info<Calibration>) {
    let _usable_and_problematic = (info.value, info.problems, info.sources);
}

fn duplicate(first: Candidate, second: Candidate) -> Lookup<ParameterDescription> {
    Lookup::Ambiguous(AtLeastTwo {
        first: Box::new(first),
        second: Box::new(second),
        rest: Vec::new(),
    })
}

fn parameter_details(value: ParameterDescription) {
    let ParameterDescription {
        parameter,
        occurrences,
    } = value;
    let _definition_type_units_calibration = (
        parameter.definition.fields,
        parameter.encoding,
        parameter.units,
        parameter.calibrations,
    );
    if let Some(packets) = occurrences.value {
        for packet in packets {
            for occurrence in packet.occurrences {
                let _location_and_enclosures = (occurrence.location, occurrence.enclosing);
            }
        }
    }
}

fn command_details(value: CommandDescription) {
    let _separate_header = value.header;
    if let Some(layout) = value.arguments.value {
        visit(layout);
    }
}

fn visit(layout: Vec<Layout<CommandElement>>) {
    for node in layout {
        match node {
            Layout::Element(CommandElement::Argument(argument)) => {
                let _rules = (
                    argument.rules.default,
                    argument.rules.element_value,
                    argument.rules.ranges,
                    argument.rules.aliases,
                    argument.rules.calibrations,
                );
            }
            Layout::Element(CommandElement::Fixed(area)) => {
                let _fixed = (area.location, area.value);
            }
            Layout::Repeat {
                repetition,
                children,
                ..
            } => {
                let _declared_count = repetition;
                visit(children);
            }
            Layout::Conditional {
                condition,
                children,
                ..
            } => {
                let _runtime_dependencies = condition.dependencies;
                visit(children);
            }
        }
    }
}

fn main() {}
