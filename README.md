# mibl

Read-only SCOS MIB viewer. Load a directory and look up a monitoring parameter by
its exact, case-sensitive name:

```sh
MIB_DIR=/path/to/mib cargo run -- packet 89000
MIB_DIR=/path/to/mib cargo run -- parameter TEMP
MIB_DIR=/path/to/mib cargo run -- --debug parameter TEMP
```

The CLI defaults to a compact overview with encoding, identification, aligned
occurrence tables and all returned problems. Add `--details` for recorded fields,
empty and omitted presence, documented default rules and problem evidence:

```sh
MIB_DIR=/path/to/mib cargo run -- --details parameter TEMP
MIB_DIR=/path/to/mib cargo run -- --debug --details packet 89000
```

Syntax is `mibl [--debug] [--details] parameter NAME` or
`mibl [--debug] [--details] packet SPID`. Each flag is allowed once, in either
order before the verb. `--debug` enables loading and lookup events on stderr
without changing stdout. There are no short aliases or flags after the verb.

Details print each reachable definition once, with stable IDs and references
for its uses. Packet details include parameter descriptions and units. Parameter
views list containing packets in numeric SPID order. Fixed repetitions retain
each expanded occurrence and its bit stride. Values derived from documented
defaults carry a `[default]` suffix in the overview.

Tables use spaces aligned by Unicode display width. Printable text is preserved
in full, controls are escaped, and output is identical on terminals and when
redirected. There is no wrapping, truncation, color or pager.

Duplicate roots produce a candidate table in either mode and exit with status 0.
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
