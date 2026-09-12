# mibl

Read-only SCOS MIB viewer. Load a directory and look up a monitoring parameter by
its exact, case-sensitive name:

```sh
MIB_DIR=/path/to/mib cargo run -- parameter TEMP
MIB_DIR=/path/to/mib cargo run -- --debug parameter TEMP
```

The CLI shows recorded PCF fields, original field presence, interpreted defaults,
PTC/PFC encoding, units and source locations. Duplicate definitions produce a
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

This slice reads `pcf.dat` and CAF headers in `caf.dat`. CAF permits a usable
supporting-only snapshot, for which parameter definitions are unavailable. The
loader accepts omitted optional fields and extra trailing columns, and keeps
usable rows when neighboring rows or other supported files fail.

Packet occurrences, calibration expansion, packet and command lookups, and search
belong to later slices. Those library operations still have explicit placeholders;
the CLI currently accepts only `parameter NAME`. Unexpanded relationships carry
an unsupported-interpretation problem, so an empty relationship list does not
claim that no relationships exist. CPC schema reconciliation belongs to the
command slice; the PCF suffix supported here is unambiguous.

The [interface contract](docs/interfaces/contract.md) describes the complete
accepted interface set. Local references and example mission data stay unpublished.
Tests use synthetic temporary directories.

Run `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test` to validate the implementation.
