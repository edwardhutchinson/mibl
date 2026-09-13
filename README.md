# mibl

Read-only SCOS MIB viewer. Load a directory and look up a monitoring parameter by
its exact, case-sensitive name:

```sh
MIB_DIR=/path/to/mib cargo run -- packet 89000
MIB_DIR=/path/to/mib cargo run -- parameter TEMP
MIB_DIR=/path/to/mib cargo run -- --debug parameter TEMP
```

The CLI shows recorded PCF fields, original field presence, interpreted defaults,
PTC/PFC encoding, units and source locations. Packet details include identification
criteria and fixed parameter occurrences. Parameter details list containing packets
in numeric SPID order. Fixed repetitions retain each occurrence and its bit stride. Duplicate definitions produce a
candidate table. A missing name exits silently with status 1. Configuration and
loading errors print to stderr and exit with status 2. `--debug` enables loading
and lookup events on stderr.

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
headers in `caf.dat`. Supporting tables can form a usable snapshot even without
root definitions. The loader accepts omitted optional fields and extra trailing
columns, and keeps usable rows when neighboring rows or other supported files fail.

Missing or ambiguous links remain local problems on found definitions. Duplicate
positions and conflicting identification widths retain their source definitions.
Fixed extraction widths come from PTC/PFC, not the PCF_WIDTH padding declaration.

Variable packet layouts, calibration expansion, command lookup and search belong
to later slices. Command and search library operations still have explicit
placeholders. The CLI accepts `parameter NAME` and `packet SPID`. Unimplemented
relationships carry unsupported-interpretation problems.

The [interface contract](docs/interfaces/contract.md) describes the complete
accepted interface set. Local references and example mission data stay unpublished.
Tests use synthetic temporary directories.

Run `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test` to validate the implementation.
