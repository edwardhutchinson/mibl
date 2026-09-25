# mibl

A read-only Rust library, CLI, and terminal browser for exploring SCOS-2000 Mission Information Bases
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
cargo run
cargo run -- packet 89000
cargo run -- parameter TEMP
cargo run -- command DEMO_TC
cargo run -- search mode --scope all
cargo run -- pus 3,25
cargo run -- tables
cargo run -- --debug parameter TEMP
```

Replace the example names and packet SPID with identities from your MIB.
Output defaults to a compact overview. Add `--details` for recorded fields and
source evidence, or `--debug` for diagnostics. Run `cargo run -- --help` for options.

## Terminal browser

Run `mibl` without a subcommand in an interactive terminal. It loads `MIB_DIR`
once; restart to reload changes. Definitions always include recorded fields and
problem evidence. The top tabs highlight the displayed definition kind or table
reports. Use `1`, `2`, `3`, or `t` to switch views. `--debug` diagnostics apply to
CLI subcommands.

| Key | Action |
| --- | --- |
| `1`, `2`, `3` | Browse packets, parameters, commands |
| `Tab` | Cycle definition lists, or search scopes while searching |
| `/` | Enter a search query; `Enter` applies it, `Esc` cancels |
| `p` | Enter a PUS service or service,subtype filter |
| `t` | Supported-table load reports |
| `Enter` | Inspect the selected identity |
| `Esc` | Return to the list |
| `↑`/`↓`, `j`/`k` | Select entries or scroll a definition |
| `PageUp`/`PageDown`, `Home`/`End` | Move through long lists and definitions |
| `←`/`→`, `h`/`l` | Scroll wide definition tables horizontally |
| `?` | Scrollable keyboard help |
| `q`, `Ctrl-C` | Quit, with `q` treated as text while entering a filter |

Search uses the library's matching and ordering. Duplicate identities remain
ambiguous when opened; selecting a row does not choose a particular duplicate.
Use a CLI subcommand when piping or redirecting output.

## Commands

Run these with `cargo run -- <command>` from the repository, or run `cargo install` to
execute as `mibl <command>`.

### Monitoring parameters

`mibl parameter NAME`

Look up a monitoring parameter by its exact, case-sensitive name, such as
`parameter TEMP`. Shows its type, units, calibration definitions, and occurrences
within telemetry packets.

### Telemetry packets

`mibl packet SPID`

Inspect a telemetry packet definition by its numeric SPID, such as `packet 89000`.
Shows identification fields and the fixed or variable layout, including parameter
locations, widths, and repetition groups.

### Telecommands

`mibl command NAME`

Look up a telecommand by its exact, case-sensitive name, such as `command DEMO_TC`.
Shows arguments, defaults, allowed ranges, aliases, conversions, and nested
repetitions, with packet header fields in a separate section.

### Search

`mibl search QUERY`

Fuzzy-search names, descriptions, and packet SPIDs, such as `search mode`.
Returns a ranked table of matching definitions with identities, PUS coordinates,
descriptions, and source locations. Narrow results with
`--scope parameters|packets|commands|all`.

### PUS services

`mibl pus SERVICE[,SUBTYPE]`

Find telemetry packets and telecommands for a PUS service. Use `pus 3` for the
whole service or `pus 3,25` for one subtype. Shows a table of matching definitions
with identities, PUS coordinates, names, descriptions, and source locations.

### MIB tables

`mibl tables`

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
