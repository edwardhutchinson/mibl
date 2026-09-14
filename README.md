# mibl

Read-only SCOS MIB viewer. Load a directory and look up a monitoring parameter by
its exact, case-sensitive name, a packet by SPID, or a telecommand by name:

```sh
MIB_DIR=/path/to/mib cargo run -- packet 89000
MIB_DIR=/path/to/mib cargo run -- parameter TEMP
MIB_DIR=/path/to/mib cargo run -- command DEMO_TC
MIB_DIR=/path/to/mib cargo run -- search mode --scope all
MIB_DIR=/path/to/mib cargo run -- --debug parameter TEMP
```

The CLI defaults to a compact overview with encoding, identification, aligned
occurrence tables and all returned problems. Add `--details` for recorded fields,
empty and omitted presence, documented default rules and problem evidence:

```sh
MIB_DIR=/path/to/mib cargo run -- --details parameter TEMP
MIB_DIR=/path/to/mib cargo run -- --debug --details packet 89000
```

Syntax is `mibl [OPTIONS] parameter NAME`, `packet SPID`, `command NAME`,
or `search QUERY [--scope parameters|packets|commands|all]`. Search defaults to
all three kinds. Global `--debug` and `--details` flags work before or after the
verb. `--debug` sends loading and query events to stderr without changing stdout.

Search matches names and descriptions case-insensitively, including packet SPIDs.
Exact identities rank first, identity prefixes next, then fuzzy matches, favoring
names over descriptions. Ties use kind, identity and source order, with numeric
packet identities. All matches appear in a plain candidate table with source
locations, including duplicates. Blank or unmatched queries print only the table
header and succeed. `--details` does not change search output. Copy a returned
identity into the corresponding exact lookup command; search never selects one.

Details print each reachable definition once, with stable IDs and references
for its uses. Packet details include parameter descriptions and units. Parameter
views list containing packets in numeric SPID order. Fixed repetitions retain
each expanded occurrence and its bit stride. Values derived from documented
defaults carry a `[default]` suffix in the overview.

Tables use spaces aligned by Unicode display width. Printable text is preserved
in full, controls are escaped, and output is identical on terminals and when
redirected. There is no wrapping, truncation, color or pager.

Duplicate roots produce a candidate table in either mode and exit with status 3.
Found results with local problems also exit with status 0. Missing identities
exit silently with status 1 outside debug mode. Configuration, loading and output
errors print to stderr and exit with status 2.

The library takes an explicit path and returns owned descriptions:

```rust,no_run
use mibl::{Mib, model::ParameterName};
use std::path::Path;

let mib = Mib::load(Path::new("/path/to/mib"))?;
let result = mib.parameter(&ParameterName("TEMP".into()));
# Ok::<(), mibl::LoadError>(())
```

A snapshot reads its files once. Queries use retained records and indexes, even
if the source files change or disappear. Results can outlive the snapshot.

The loader reads `pcf.dat`, `pid.dat`, `tpcf.dat`, `pic.dat`, `plf.dat`, and CAF
headers in `caf.dat`, plus `ccf.dat`, `cdf.dat` and `cpc.dat`. Supporting tables can form a usable snapshot even without
root definitions. The loader accepts omitted optional fields and extra trailing
columns, and keeps usable rows when neighboring rows or other supported files fail.

Missing or ambiguous links remain local problems on found definitions. Duplicate
positions and conflicting identification widths retain their source definitions.
Fixed extraction widths come from PTC/PFC, not the PCF_WIDTH padding declaration.

Command views show ordered basic arguments and fixed application-data areas,
CPC defaults, recorded element values and explicit telemetry dependencies.
Positions are declared unexpanded application-data bits. Argument widths come
from CPC PTC/PFC; disagreements with CDF_ELLEN remain visible. CPC suffix fields
retain their raw text and candidate meanings without guessing description or
endian interpretation.

Variable packet layouts, calibration expansion, command headers, nested command
repetitions and supporting argument rules belong to later slices.
Unimplemented relationships
carry unsupported-interpretation problems.

The [interface contract](docs/interfaces/contract.md) describes the complete
accepted interface set. Local references and example mission data stay unpublished.
Tests use synthetic temporary directories.

Run `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test` to validate the implementation.
