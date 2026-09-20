//! Cross-feature acceptance scenarios for the complete viewer (issue #18).
//!
//! One synthetic snapshot carries every canonical supporting table at once, so
//! calibration, packet structures, command rules, search and incomplete-data
//! behaviour are exercised together rather than one slice at a time. It is a
//! publishable equivalent of the surveyed `ZUT00002`, packet `89000`,
//! `S2KTC001`, `S2KTC074` and `mode` search cases. No supplied reference file or
//! example MIB row is reproduced here.
//!
//! This parent only declares the suite. The snapshot is defined once in
//! `workflows/fixture.rs`, helpers more than one scenario needs live in
//! `workflows/support.rs`, and each scenario file isolates one area:
//! `workflows/parameters.rs`, `workflows/packets.rs`, `workflows/commands.rs`,
//! `workflows/snapshots.rs` and `workflows/cli.rs`.
mod common;

#[path = "workflows/cli.rs"]
mod cli;
#[path = "workflows/commands.rs"]
mod commands;
#[path = "workflows/fixture.rs"]
mod fixture;
#[path = "workflows/packets.rs"]
mod packets;
#[path = "workflows/parameters.rs"]
mod parameters;
#[path = "workflows/snapshots.rs"]
mod snapshots;
#[path = "workflows/support.rs"]
mod support;
