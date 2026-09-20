# mibl

A read-only Rust library and CLI for exploring SCOS-2000 Mission Information Bases
(MIBs), the definitions used to interpret spacecraft telemetry and describe commands.

- Inspect monitoring parameters, calibrations, and fixed or variable telemetry layouts.
- Explore telecommands, argument rules, nested repetitions, and packet headers.
- Find definitions with fuzzy search or PUS service lookup, and list available MIB tables.

Loads a MIB directory into an immutable snapshot. Usable definitions remain available
when data is incomplete or ambiguous, with problems and source evidence attached.
It inspects definitions; it does not evaluate live telemetry or execute commands.

## Quick start

With Rust installed, run from this repository and point `MIB_DIR` at your MIB directory:

```sh
export MIB_DIR=/path/to/mib
cargo run -- parameter TEMP
```

Replace the example names and packet SPID with identities from your MIB.
Output defaults to a compact overview. Add `--details` for recorded fields and
source evidence, or `--debug` for diagnostics. Run `cargo run -- --help` for options.

## Commands

Run these with `cargo run -- <command>` from the repository.

### `parameter NAME`

Look up a monitoring parameter by its exact, case-sensitive name, such as
`parameter TEMP`. Shows its type, units, calibration definitions, and occurrences
within telemetry packets.

### `packet SPID`

Inspect a telemetry packet definition by its numeric SPID, such as `packet 89000`.
Shows identification fields and the fixed or variable layout, including parameter
locations, widths, and repetition groups.

### `command NAME`

Look up a telecommand by its exact, case-sensitive name, such as `command DEMO_TC`.
Shows arguments, defaults, allowed ranges, aliases, conversions, and nested
repetitions, with packet header fields in a separate section.

### `search QUERY`

Fuzzy-search names, descriptions, and packet SPIDs, such as `search mode`.
Returns a ranked table of matching definitions with identities, PUS coordinates,
descriptions, and source locations. Narrow results with
`--scope parameters|packets|commands|all`.

### `pus SERVICE[,SUBTYPE]`

Find telemetry packets and telecommands for a PUS service. Use `pus 3` for the
whole service or `pus 3,25` for one subtype. Shows a table of matching definitions
with identities, PUS coordinates, names, descriptions, and source locations.

### `tables`

List every supported MIB table with its file name, purpose, and direct lookup
command where available. Shows the loaded row count or whether the file is
missing or unreadable.

## Rust library

Load a snapshot and query it directly:

```rust,no_run
use mibl::{Mib, model::ParameterName};
use std::path::Path;

let mib = Mib::load(Path::new("/path/to/mib"))?;
let result = mib.parameter(&ParameterName("TEMP".into()));
# Ok::<(), mibl::LoadError>(())
```

See the [library example](examples/contracts.rs) and 
[table schema](docs/interfaces/schema.md) for API and data-format details.

## Development

Tests use synthetic data; mission data is not included.

```sh
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test
```
