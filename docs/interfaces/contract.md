# MIB viewer interface contract

Status: accepted by the maintainer at commit `ef6ea86`; issue #8 is closed.
[Acceptance record](https://github.com/edwardhutchinson/mibl/issues/8#issuecomment-5648856740).
The complete interface set remains the contract for the implementation slices.
Issue #9 implements explicit-directory loading, PCF lookup and CLI rendering.
Issue #10 implements fixed packet lookup and containing occurrences.
Issue #13 implements basic command lookup. Search and variable packet operations
remain explicit placeholders.

Sources: [issue #8](https://github.com/edwardhutchinson/mibl/issues/8),
[canonical contract](https://github.com/edwardhutchinson/mibl/issues/6#issuecomment-5648605950),
[workflows](https://github.com/edwardhutchinson/mibl/issues/4#issuecomment-5646945884),
and [compatibility](https://github.com/edwardhutchinson/mibl/issues/5#issuecomment-5647045608).
Local reference files and example MIB remain unpublished.

## Boundary inventory and cross-check

| Producer → consumer | Input | Output / errors | Ownership and invariants | Representative exchange |
|---|---|---|---|---|
| Environment/argv → CLI configure | OS strings, optional MIB_DIR | Configuration or MissingMibDir, EmptyMibDir, InvalidArguments | CLI owns configuration; paths need not be Unicode | MIB_DIR plus `parameter ZUT00002` gives explicit path and ParameterName |
| CLI → Mib::load | borrowed explicit Path | owned Mib or typed LoadError | library never reads environment or installs subscriber | inaccessible directory carries path and io::Error |
| Mib → reader::load | same Path | owned Records or LoadError | one load, no later file reads; every table has TableLoad state | missing CAF with usable PCF still loads |
| Reader → catalog::new | owned Records | owned Catalog, no relationship error return | rows retain duplicates; schema interpretations separate from original fields | two PCF rows under one name create two root index entries |
| Catalog → Mib | immutable typed lookup/search arguments | owned Lookup descriptions / Vec<Candidate> | no borrowing from snapshot, no silent target selection | duplicate identity gives AtLeastTwo candidates |
| Mib → CLI query | immutable snapshot and Request | typed Response | CLI imports public library only | packet SPID goes to packet, never a fuzzy lookup |
| CLI query → render | borrowed Response, output writer | ExitCode or io::Error | overview for Found, complete recorded details with --details per #19; plain candidate table for ambiguity/search | both NotFound reasons emit no bytes and exit 1 |
| CLI errors → report_error | CliError, stderr writer | status 2 | visible independently of tracing | unset MIB_DIR reports configuration error |
| Library tracing facade → CLI subscriber | debug/info events | stderr when enabled | no saved diagnostics or logging handle; debug off cannot break unmatched silence | dropped row event has relative file, line, reason, original text |

Every query output has a Response variant. Every Request has a public Mib
operation with the same typed identity. Every public description input has the
reader producer listed below. CLI errors cover configuration, loading and output
I/O. Load errors distinguish inaccessible directory from no usable supported
rows; a directory containing only usable supporting rows still produces a Mib.
TableLoad distinguishes missing, unreadable, and read with zero or more rows.
Dropped rows are ordinary tracing events, not retained diagnostic records.

## Reader → catalog table exchanges

All table-specific cell declarations are in `src/reader.rs`; column order,
optionality and default evidence are in [schema.md](schema.md). `Row<T>` owns
`Definition` and typed `Info` cells. No table returns a borrowed buffer. The
catalog never reparses a raw row to discover relationships. Text-valued numeric
calibration data requires format/radix interpretation in the catalog.

| Producer tables | Keys and joins consumed by catalog | Public output |
|---|---|---|
| PCF | NAME; PTC/PFC encoding; CURTX plus category; related parameter references return to PCF | ParameterSummary, Encoding, recorded definition and units |
| PID | SPID roots; TYPE/STYPE/APID; PI1/PI2 expected values; TPSD selects variable layout | PacketSummary, PacketIdentification |
| TPCF | SPID → PID, preserve duplicate names | packet name and recorded metadata |
| PIC | TYPE/STYPE and APID, including wildcard 99999; offsets and widths for PI1/PI2 | IdentificationCriterion with extraction Location and source definitions |
| PLF | SPID → PID, NAME → PCF; byte/bit, NBOCC/LGOCC and time declarations | fixed occurrences, repetitions and location constraints |
| VPD | TPSD → PID; POS order, NAME → PCF; GRPSIZE/FIXREP/CHOICE/PIDREF/OFFSET | nested Layout, runtime selectors, repetitions and dependencies |
| CUR | PNAME → PCF, POS order, RLCHK → PCF, VALPAR condition, SELECT calibration reference | ordered CalibrationAlternative with selection definition |
| CAF/CAP | NUMBR header → points, format/radix/interpolation | Numerical calibration with independently known points |
| MCF / LGF | IDENT, POL1..POL5 | Polynomial / Logarithmic coefficients with per-field uncertainty |
| TXF/TXP | NUMBR header → intervals; FROM/TO/ALTXT | Textual calibration |
| CCF | CNAME root, PKTID → TCP | CommandDescription and separate header |
| CDF | CNAME → CCF; BIT order, PNAME → CPC; ELLEN, GRPSIZE, INTER/VALUE, TMID → PCF | ordered arguments/fixed areas, nested repetitions, element values and telemetry dependencies |
| CPC | normalized parameter identity; PTC/PFC; PRFREF/CCAREF/PAFREF; DEFVAL | CommandArgument encoding/units and ValueRules |
| CCA/CCS | NUMBR conversion header → points | CommandConversion calibration, recorded format/radix data |
| PAF/PAS | NUMBR alias header → values | Alias list plus supporting definitions |
| PRF/PRV | NUMBR range header → intervals | AllowedRange list with representation and supporting definitions |
| TCP/PCDF/PCPC | TCP ID → PCDF TCNAME; ordered BIT fields; PNAME → PCPC | CommandHeader and HeaderField, separate from application arguments |

Root indexes map identities to vectors, never a single row. Supporting indexes
also retain all rows. Their RowId is scoped to its table; composite supporting
keys must use a collision-free encoding of the table's declared key fields.
Related missing/ambiguous definitions become Info problems with typed references
and candidate Target definitions. A missing support table cannot remove a root.
Unusable established numeric fields drop their row; contradictory references do
not. Unsupported schema interpretation alone does not drop otherwise usable rows.

## Schema and recorded information

Definition.fields is in schema column order. Each RecordedField has a one-based
physical column and Omitted, Empty or Text presence. A FieldMeaning carries the
schema name and separate interpreted scalar; documented defaults name their
rule. Source filenames are relative within the loaded directory and line numbers
are one-based. Reader construction must enforce relative filenames without `..`.

PCF column 24 is optional DESCR2. CPC column 1 normalizes PNAME/NAME to the
internal `name` cell. CPC column 15 normalizes OBTID/OBTIP to the internal `obtip`
clock reference without assigning new semantics. Recorded meanings preserve both
spellings where the source cannot distinguish them. CPC physical column 16 can
mean ENDIAN or DESCR2; 17 can be ENDIAN under the extended layout. Without
unambiguous evidence, preserve candidate meanings and original presence, leave
dependent interpretations unavailable, and attach UnsupportedInterpretation.
Do not use plausible cell text or a missing suffix to choose a version or apply
an endian default. The schema inventory lists the extended cells, not an
instruction to assume that layout. No version flag or detection is declared.
Unknown extra trailing columns are ignored with debug events.

CDF BIT is a declared application-data position after the header with repetition
counts taken as one, not an absolute runtime position. CPC PTC/PFC supply parameter
width; CDF ELLEN is a consistency declaration, or width for a fixed area. Preserve
disagreements in Location problems and recorded fields. PCF WIDTH is not a general
encoded width. VPD width also must not be substituted for encoding semantics.

## Public information and finite expansion

Info preserves a usable value alongside multiple structured problems and source
locations. MissingReference, AmbiguousReference, InconsistentDefinition,
UnsupportedInterpretation and RuntimeDependent have distinct payloads. Ambiguous
references retain at least two Target definitions. Inconsistencies retain fields,
conflicting values and available targets. RuntimeDeclaration retains the declared
expression, condition or selector, typed dependencies and source locations.
Dependency targets contain recorded definitions only, preventing recursive joins.
Deferred concepts stay in recorded fields or Deferred references; no limits,
validity, verification, sequences, displays or groups are expanded.

ParameterDescription contains PacketOccurrences sorted by numeric SPID, then
source for duplicate packet definitions. Each occurrence has its complete ordered
enclosure path. PacketDescription contains ParameterSummary, which has no
containing-packet list. Layout trees preserve nested declared groups and never
expand runtime counts. Calibration alternatives retain CUR evaluation order.
Duplicate positions remain visible, carry InconsistentDefinition problems and
sort by source. Header fields remain distinct from CommandElement arguments and
fixed areas. Parent definitions and leaf definitions preserve provenance for all
supporting data even when their interpreted value is unavailable.

Exactly one root gives Found even with missing links. Multiple roots give
Ambiguous with two mandatory candidates plus any remainder. No roots of that
kind gives DefinitionsUnavailable; otherwise absent exact identity gives
NoMatchingIdentity. Exact names are case-sensitive. Candidates retain identity,
optional name/description and source, so missing display text loses no definition.

Search matches names/descriptions case-insensitively and includes packet SPIDs.
Rank exact identities, identity prefixes, then fuzzy matches, preferring names
over descriptions. Ties sort by kind parameter/packet/command, identity with
numeric SPIDs, then source. Return every qualifying row without a cap. Blank
queries give an empty vector. Scoring and threshold remain implementation tuning.
Loading scales with supported input size; exact queries and relationships use
indexes, while search may scan candidates.

## Representative cross-checks

These are review exchanges, not assertions of executed behavior or copied mission
rows. `examples/contracts.rs` compiles callers but never runs placeholder queries.

| Exchange | Producer → consumer trace and required result |
|---|---|
| `parameter ZUT00002` | PCF → ParameterSummary, PLF → occurrence at byte 19/bit 0 in PID 89000. Category/reference conflict remains an InconsistentDefinition beside available calibration data. CLI can show Mode description, definition, location and calibration together. |
| `packet 89000` | PID/TPCF → name ZUY_HK_00001, PID/PIC → identification and extraction criteria, PLF/VPD → ordered layout. ParameterSummary terminates expansion without embedding containing packets. |
| `command S2KTC001` | CCF/CDF/CPC → S2KCP001/002/003 arguments at ApplicationDeclaredBit 0/8/16. CDF TMID → explicit telemetry ValueSource and PCF dependency. TCP/PCDF/PCPC → separate expanded header. |
| `command S2KTC074` | CDF group declarations → nested Layout::Repeat; runtime repetition remains a declaration with dependencies. Fixed areas retain their own variant. |
| `search mode --scope all` | root records and names/descriptions → ordered Vec<Candidate> → plain table, no automatic selection. Parameter-only scope excludes other kinds. |
| Missing calibration | retained PCF root → Found; calibration Info has MissingReference and any independently usable fields. Missing files remain distinguishable internally from empty files. |
| Duplicate definition | two root rows with same typed identity → separate candidates with sources → AtLeastTwo → candidate table. Neither row overwrites the other. |
| Duplicate supporting target | one root plus two matching calibrations → Found with AmbiguousReference and both Targets; root lookup is not Ambiguous. |
| Ambiguous CPC suffix | original columns 16/17 → multiple FieldMeanings and UnsupportedInterpretation; unambiguous name/type survives; endian-dependent information unavailable. |
| Absent identity | usable roots of kind → NoMatchingIdentity; none → DefinitionsUnavailable. Both render no output and status 1 outside debug. |
| Supporting-only directory | usable CAF with no roots → successful Mib; exact queries DefinitionsUnavailable, search empty table. No usable rows anywhere → visible LoadError. |

## Review and acceptance

Compile validation proves signatures connect, not that behavior works. No
behavioral tests belong in this ticket. Later implementation tickets supply
fixtures for variable structures and repeated fixed occurrences absent from the
sample. Full ICD conformance has not been established.

Maintainer acceptance: recorded in issue #8 for the complete set in commit
`ef6ea86`, including this file, schema.md, all src declarations and the compile-only
example.

Agent review completed against starting commit
`762f80e06093aaec0f81c8b54a2459164e9d7aa1` with separate standards and spec reviewers.
Standards: no documented violations or actionable smells. Spec: no findings;
maintainer acceptance was subsequently recorded in issue #8.

## Issue #9 implementation coverage

The reader loads PCF and the CAF headers needed for the supporting-only exchange.
It validates established numeric and coded fields, preserves field presence and
unambiguous defaults, and retains PCF_DESCR2 at column 24. There is no conflicting
PCF column interpretation. CPC ambiguity and additional table families are deferred
to their owning slices. The existing typed reader contracts remain unchanged.

The catalog indexes retained PCF rows by case-sensitive name, preserving every
duplicate. Parameter results include the definition, units, PTC/PFC, endian and
static encoded width where known from ICD 7.0 Appendix A. Unknown widths retain
original codes and an UnsupportedInterpretation problem. PCF_WIDTH is never used
as an encoded width. Unexpanded calibrations and occurrences also explicitly
report UnsupportedInterpretation until their owning slices implement the joins.
Recorded references remain available even when their targets are missing.

Cross-check: PCF rows produce owned ParameterSummary fields; CAF rows make the
supporting-only load possible; catalog root counts distinguish both absence
reasons; the CLI consumes Found, Ambiguous and NotFound through public Mib calls.
Source locations and interpreted defaults reach the field renderer unchanged.
Library events reach only the application-owned tracing subscriber. Public Mib
and CLI process tests cover these exchanges using synthetic temporary files.

## Issue #10 contract amendment and cross-check

`PacketSummary.characteristics: Info<Vec<Definition>>` carries all matching TPCF
rows in source order. The existing `name` carries the resolved name and local
missing or ambiguous reference problems. Characteristics retain every recorded
field, including TPCF_SIZE, even if the name cannot be resolved. Missing TPCF
rows produce a missing-reference problem and no characteristics value.

The reader already produces these definitions in `Row<Tpcf>`. The catalog copies
them into owned packet summaries, including summaries in parameter results. The
CLI renders their recorded fields and provenance. This closes the missing path
from the existing reader declaration to the recorded-metadata consumer without
exposing private rows. Duplicate TPCF rows remain available in both this collection
and ambiguous-reference targets. PID remains the packet root definition.

Fixed PLF repetitions are expanded into individual layout elements, each carrying
its fixed count and bit stride in `enclosing`. Positions sort by byte and bit,
then source location, then repetition instance. Parameter results use the same
occurrences, grouped numerically by SPID. This is finite expansion of the bounded
PLF_NBOCC count; runtime and variable structures belong to the VPD slice.

`PacketIdentification.definitions: Vec<Definition>` retains all matching PIC rows
in source order, including rows that disable both additional criteria with -1
offsets. Reader `Row<Pic>` definitions feed this catalog-owned collection; the CLI
renders it even when `criteria` is empty. Criterion definitions continue to include
PID and all matching PIC evidence. Thus disabling extraction cannot hide the
recorded definition. The collection is empty when no PIC rows match; the criteria
information carries the missing-reference problem.

A PLF occurrence linked to a PID with a variable TPSD remains inspectable in a
parameter result, with an inconsistent-definition problem containing both rows.
Issue #11 replaces the variable-layout placeholder with the VPD behavior below.
Missing VPD data is reported as a missing reference while fixed occurrences remain
available.

## Issue #19 CLI presentation amendment

The maintainer accepted option A and its presentation rules on 2026-09-13 with
"I accept option A" after reviewing the examples in commit `ad3f7c9`.
The [accepted output decision](../design/cli-output.md) defines the examples,
section order, labels, tables, problem evidence and reusable conventions.
Issue #20 implements this amendment; existing rendering remains in place until
that ticket ships.

`mibl [--debug] [--details] parameter NAME` and
`mibl [--debug] [--details] packet SPID` use overview output by default.
As amended by #21, flags may appear before or after the verb or identity,
and repeated flags are idempotent. No short aliases or explicit overview flag
are accepted. `--debug` remains
independent stderr diagnostics and does not change result verbosity.

Overview shows identity, description, root source, encoding/units for parameters,
identification for packets, and occurrence tables. Every reachable public-result
problem remains visible with its affected context, including problems in recorded
field interpretations. Known values survive alongside problems; unavailable
information never becomes zero or a falsely empty collection.

`--details` includes the overview followed by problem evidence and definitions,
omitting the overview's details advice. It preserves every reachable recorded
field, presence, interpretation, documented-default rule, source, reference and
alternative. Supporting definitions print once per table/source line with stable
references to each use. No recursive lookup is introduced.

This intentionally supersedes the full-details default in the original contract
and the issue #9/#10 rendering coverage above. Their recorded-field and supporting
provenance guarantees now apply to details mode; root provenance remains in the
overview. Reader, catalog and public library information are unchanged.

Use aligned spaces, full untruncated printable text and escaped controls, with
no renderer wrapping, colors, pager or terminal detection. Redirected and terminal
stdout are identical. Exact lookup, duplicate candidates, status-1 silent misses,
status-0 found results, visible status-2 errors and --debug behavior
retain their accepted semantics. Issue #21 changes root ambiguous matches to
status 3, preserving their candidate output. Future rendering tickets follow these conventions.

## Issue #21 CLI parsing amendment

Clap v4 owns argument validation and help/version output. `--help` and
`--version` succeed without resolving `MIB_DIR`. Lookup requests still resolve
`MIB_DIR` as an OS string, preserving non-Unicode directory paths. Packet SPIDs
accept only nonempty ASCII digits within the u64 range, including leading zeros.
The builder API enables std, usage, error-context, help and suggestions; derive
macros are unnecessary. Completion and man-page generators can consume this
command definition in a future tooling change.

## Issue #13 implementation coverage

CCF, CDF and CPC use the shared reader policies. CCF roots and CDF/CPC supporting
indexes preserve duplicates. Public `Mib::command` returns owned descriptions;
`command NAME` uses the same overview, details, candidate and exit conventions
as the parameter and packet verbs.

The catalog sorts basic elements by CDF_BIT and source. Argument widths use the
shared PTC/PFC mapping; fixed areas use CDF_ELLEN. Duplicate positions and width
disagreements retain structured evidence. Missing or ambiguous CPC definitions
leave the root found and affect only the associated argument information.

CPC identity and clock-reference fields preserve both known spellings. Columns
16 and 17 retain physical presence and candidate meanings, with unavailable
interpretations and no inferred endian default. Literal values remain recorded
text with their representation and defining row. CPC defaults and CDF values
stay separate; CDF_INTER=D refers to the CPC default, while T carries a runtime
telemetry declaration with finite PCF targets. The CDF definition retains the
static fallback text. Editable arguments without values and variable widths
explicitly require runtime input.

Header expansion, repetition trees and supporting argument rules remain outside
this slice. Repetition declarations retain all basic elements and report the
unimplemented structure. Missing or unreadable CDF tables produce unavailable
layout information rather than a falsely empty layout.

Cross-check: existing reader rows produce every field in the existing command
model; public Mib results and CLI checks exercise lookup, ordering, partial links,
width conflicts, defaults and schema ambiguity using synthetic data. No public
interface amendment is needed for this slice.


## Issue #11 variable packet resolution

The existing public types carry VPD results without an interface extension.
Reader `Row<Vpd>` feeds ordered `Layout<ParameterOccurrence>` trees in packet
lookup. Parameter lookup flattens only their declared elements, retaining each
occurrence's enclosing groups. Neither traversal expands repetition counts or
follows runtime-selected TPSD structures. Missing packet roots leave orphan VPD
occurrences available with a missing PID/TPSD reference.

The catalog interprets GRPSIZE as a count of following records, including nested
markers. A positive FIXREP defines a fixed group whose marker has a PCF definition
but no transmitted occurrence. Zero FIXREP uses the marker's runtime value.
The negative FIXREP extension is unsupported and its children remain inspectable.
CHOICE produces a conditional node with a selector dependency and no statically
selected children. PIDREF and deduced PCF_RELATED retain parameter-ID dependencies.

Extraction begins at PID_DFHSIZE bytes. VPD_OFFSET adds a signed bit displacement
from the preceding slot end. PTC/PFC supplies encoded width; PCF_WIDTH supplies the
padded slot when declared, with padding preceding the value. A contradictory slot
width preserves the declared slot end and encoded width, but cannot establish the
value's extraction position. VPD_WIDTH remains display metadata. Repeated children
use positions relative to each repetition, with group-start constraints; following
locations use fixed counts and known strides, or retain runtime dependencies.

Duplicate positions keep source ordering and attach all competing definitions.
Shared widths survive ambiguous PCF references when all candidates agree. Packet
and parameter rendering consume the same occurrence information, with explicit
group boundaries in packet output. Recorded fields, defaults and source locations
continue through the existing definition and problem-evidence renderers.

Cross-check: the reader supplies all fourteen schema fields; catalog consumers
use TPSD/POS/NAME, repetition, selection and offset cells and preserve display
fields in definitions. Public packet and parameter results terminate at parameter
summaries and packet summaries respectively. The CLI accesses only those public
results. Synthetic public-library and CLI tests cover these exchanges.

### Enclosing definition amendment

`ParameterOccurrence.enclosing_definitions: Vec<Definition>` preserves the
recorded declarations for its enclosure path, outermost first. The catalog copies
each VPD group definition into its children's occurrences; fixed PLF repetition
uses the occurrence's PLF definition. Parameter lookup retains this collection
when flattening a layout. The renderer registers these definitions before
rendering enclosure interpretations, so reverse lookup can show all group fields.

Cross-check: VPD/PLF reader definitions already supply the required recorded data.
The catalog produces an owned collection with one definition per enclosure; both
packet and parameter views consume it through the public occurrence. Definitions
contain no relationship expansion, so the new collection remains finite. This
closes the review's missing producer-to-consumer path without requiring a query
from the renderer.

Duplicate variable PID definitions produce separate containing-packet entries.
Each entry retains its own root summary and locations, plus an ambiguous-identity
problem listing the other candidates. Entries sort by SPID and root source.
