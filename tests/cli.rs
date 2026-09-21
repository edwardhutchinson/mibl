//! Process-level integration tests for the `mibl` executable.
//!
//! This file is the crate root of the `cli` test target, so the child
//! modules live in `tests/cli/` and are declared with explicit `#[path]`
//! attributes to keep Rust's module lookup unambiguous.

mod common;

#[path = "cli/commands.rs"]
mod commands;

#[path = "cli/configuration.rs"]
mod configuration;

#[path = "cli/output.rs"]
mod output;

#[path = "cli/packets.rs"]
mod packets;

#[path = "cli/parameters.rs"]
mod parameters;

#[path = "cli/pus.rs"]
mod pus;

#[path = "cli/search.rs"]
mod search;

#[path = "cli/support.rs"]
mod support;
