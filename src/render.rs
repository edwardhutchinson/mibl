//! Plain, deterministic views of the returned description. Never queries the MIB.
//!
//! Each implementation module owns one kind of output: `parameters`, `packets`, `commands` and
//! `listings` render the five entry points re-exported below, `evidence` collects the definitions
//! and problems those views share, `calibrations` summarises a calibration definition, and
//! `format` holds the text presentation every view reads through.

mod calibrations;
mod commands;
mod evidence;
mod format;
mod listings;
mod packets;
mod parameters;

pub(super) use commands::command;
pub(crate) use format::text;
pub(super) use listings::{candidates, tables};
pub(super) use packets::packet;
pub(super) use parameters::parameter;

#[cfg(test)]
mod tests;
