# mibl

Read-only SCOS MIB viewer. Load a directory and look up a monitoring parameter by
its exact, case-sensitive name, a packet by SPID, or a telecommand by name:

```sh
MIB_DIR=/path/to/mib cargo run -- packet 89000
MIB_DIR=/path/to/mib cargo run -- parameter TEMP
MIB_DIR=/path/to/mib cargo run -- command DEMO_TC
MIB_DIR=/path/to/mib cargo run -- search mode --scope all
MIB_DIR=/path/to/mib cargo run -- pus 3,25
MIB_DIR=/path/to/mib cargo run -- tables
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
`search QUERY [--scope parameters|packets|commands|all]`, or `tables`.
Search defaults to all three kinds. Global `--debug` and `--details` flags work
before or after the verb. `--debug` sends loading and query events to stderr
without changing stdout.

Search matches every recorded name and the descriptions case-insensitively,
including packet SPIDs. A packet root matches every TPCF name it records, so a
name query still finds the packet when its TPCF reference is ambiguous; the table
keeps showing `unavailable` because no recorded definition is chosen to display a
hit. Exact identities rank first, identity prefixes next, then fuzzy matches,
favoring names over descriptions. Ties use kind, identity and source order, with
numeric packet identities. All matches appear in a plain candidate table with
source locations, including duplicates. Blank or unmatched queries print only the
table header and succeed. `--details` does not change search output. Copy a
returned identity into the corresponding exact lookup command; search never
selects one.

Details print each reachable definition once, with stable IDs and references
for its uses. Packet details include parameter descriptions and units. Parameter
views list containing packets in numeric SPID order. Fixed repetitions retain
each expanded occurrence and its bit stride. Values derived from documented
defaults carry a `[default]` suffix in the overview.

Occurrence and layout sections say `none` only when the sources show the
collection is empty, and `unavailable` when they cannot: a missing or unreadable
PLF table, or a declared variable packet structure whose VPD rows are absent,
leaves the occurrences unavailable with its missing-reference problem. Known
fixed and variable occurrences stay visible beside that problem.

Tables use spaces aligned by Unicode display width. Printable text is preserved
in full, controls are escaped, and output is identical on terminals and when
redirected. There is no wrapping, truncation, color or pager.

Command application data keeps declared group nesting: an element declaring
`CDF_GRPSIZE` is a repeater whose value repeats the elements that follow it, so
such a command prints nested `Repeat group` blocks naming the element that
supplies the count, its recorded value, its source and its problem markers.
Repetition stays runtime-dependent, declared positions keep their unexpanded
`CDF_BIT` values, and no command invocation is expanded.

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

The loader reads `pcf.dat`, `pid.dat`, `tpcf.dat`, `pic.dat`, `plf.dat`, `vpd.dat`,
`cur.dat`, `caf.dat`, `cap.dat`, `mcf.dat`, `lgf.dat`, `txf.dat`, `txp.dat`,
`ccf.dat`, `cdf.dat`, `cpc.dat`, `prf.dat`, `prv.dat`, `paf.dat`, `pas.dat`,
`cca.dat`, `ccs.dat`, `tcp.dat`, `pcdf.dat` and `pcpc.dat`. Supporting tables can
form a usable snapshot even without root definitions. The loader accepts omitted
optional fields and extra trailing columns, and keeps usable rows when neighboring
rows or other supported files fail.

Missing or ambiguous links remain local problems on found definitions. Duplicate
positions and conflicting identification widths retain their source definitions.
Fixed extraction widths come from PTC/PFC, not the PCF_WIDTH padding declaration.
An ambiguous parameter reference keeps the width every candidate establishes, and
loses it only when the candidates disagree, cannot establish one, or are absent.

Command views show ordered basic arguments and fixed application-data areas,
CPC defaults, recorded element values and explicit telemetry dependencies.
Positions are declared unexpanded application-data bits. Argument widths come
from CPC PTC/PFC; disagreements with CDF_ELLEN remain visible. CPC suffix fields
retain their raw text and candidate meanings without guessing description or
endian interpretation.

Variable packet layouts retain nested groups without expanding repetitions.
Counts, runtime selectors, parameter-ID dependencies and relative locations remain
visible. VPD display width does not determine encoded width. PCF padding and
signed VPD offsets determine subsequent locations where possible.

Monitoring calibrations show numerical curve points, polynomial and logarithmic
coefficients, and textual intervals. Conditional alternatives follow CUR_POS order
and retain selectors, raw-value conditions and monitoring-parameter dependencies.
The viewer does not evaluate live telemetry. PCF_CATEG names the calibration
family its reference means, so a reference resolves within its declared numerical
or textual family first; a definition available only in the other family stays
available with a structured problem. Declaring both a PCF_CURTX reference and CUR
calibrations keeps both sets with the disagreement visible. Parameter overviews
show interpreted calibration values; `--details` adds original fields, defaults,
sources and reference evidence.

Command argument rules resolve the CPC range, alias and conversion references.
Allowed ranges keep each declared boundary with the range set's representation,
input format and radix, where an omitted upper bound stays unavailable. Alias
mappings keep the interpreted raw value beside their declared text. Command
conversions keep their raw and engineering point interpretations from the CCA
formats, and no interpolation is invented where none is declared. An argument
that declares no reference reports no rules, a reference without a usable target
stays a local problem, several matching sets keep every definition with the
ambiguity attached, and a declared count that disagrees with the retained rows
stays visible. An unavailable argument layout is never reported as a command
that declares none. The command view prints an `Argument rules` section with each
value and its source; `--details` adds the recorded range, alias and conversion
rows.

Command headers expand as their own section, separate from the application data.
`CCF_PKTID` names the TCP packet header, and its PCDF records describe each header
element in declared bit-offset order: a fixed area keeps the recorded content the
command is encoded with, and a parameter element links the PCPC definition that
describes it and keeps that element's declared default interpreted with the linked
parameter's code and radix. The element type names the source a command load takes
the value from, such as `CCF_APID` for an APID element or the command subsystem for
an element it sets automatically. Missing, ambiguous, contradictory and unsupported
stay beside the elements that remain usable, and a command whose header cannot be
resolved stays found with its arguments and a missing reference.

The [interface contract](docs/interfaces/contract.md) describes the complete
accepted interface set. Local references and example mission data stay unpublished.
Tests use synthetic temporary directories.

Run `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`, and
`cargo test` to validate the implementation.

### MIB tables

Use `mibl tables` to see which file holds what, when a file name such as
`pcf.dat` does not say enough on its own:

```sh
MIB_DIR=/path/to/mib cargo run -- tables
```

Every supported table gets one row with its code, the file it comes from, what
its rows declare, and the lookup that addresses those rows directly:
`parameter` for `pcf.dat`, `packet` for `pid.dat`, `command` for `ccf.dat`, and
`-` for every supporting table, which is reached through one of them.

```
Code  File      Defines                                Lookup     Rows
CDF   cdf.dat   Telecommand argument elements          -          378
PCF   pcf.dat   Monitoring parameter definitions       parameter  116
PID   pid.dat   Telemetry packet definitions           packet       4
VPD   vpd.dat   Variable packet layout elements        -            0
```

Rows report what the loaded directory provided: retained row count, `missing` for
an absent file, or `unreadable` for a file that cannot be read. A readable table
with no usable row is `0`, which answers differently from `missing`. Codes are
listed in alphabetical order, which is also file-name order. `--details` adds
nothing here, as with search.

### PUS lookup

Use `mibl pus 3` to list telemetry packet definitions and telecommand definitions
for service 3, or `mibl pus 3,25` to restrict the results to subtype 25.
Both coordinates accept unsigned 16-bit integers, separated by a comma.
Results sort by subtype, with missing subtypes last, then packets before
commands, then numeric packet SPID or command name. Duplicate definitions remain
separate rows. No matches produces a header-only table and exit status 0.

Candidate tables for PUS lookup, search, and ambiguous exact lookups contain
`Kind  Identity  PUS  Name  Description  Source`. PUS coordinates appear as
`TM(3,25)` or `TC(3,25)`. A missing subtype appears as `-` inside the
coordinates; parameters and definitions without a service show `-`.
The PUS column does not participate in search matching.
