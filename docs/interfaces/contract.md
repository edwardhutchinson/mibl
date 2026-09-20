# MIB viewer interface contract

Status: accepted by the maintainer at commit `ef6ea86`; issue #8 is closed.
[Acceptance record](https://github.com/edwardhutchinson/mibl/issues/8#issuecomment-5648856740).
The complete interface set remains the contract for the implementation slices.
Issue #9 implements explicit-directory loading, PCF lookup and CLI rendering.
Issue #10 implements fixed packet lookup and containing occurrences.
Issue #13 implements basic command lookup. Issue #12 implements monitoring
calibration resolution. Issue #14 implements command argument ranges, aliases
and conversions. Issue #15 implements nested command argument groups. Issue #16
implements expanded command headers. Search, variable packet layouts and PUS
lookup follow in #17, #11 and #22.

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

Each table's typed cells, column schema and parser are colocated in the reader
module that owns its family: `reader/fields.rs` holds the cell declarations and
the recorded-field reading they share, `reader/parameters.rs`, `reader/packets.rs`,
`reader/variable.rs`, `reader/calibrations.rs`, `reader/commands.rs` with
`reader/commands/rules.rs`, and `reader/header.rs` hold their rows, schemas and
parsers. `src/reader.rs` describes snapshot loading and keeps only the shared
`Row<T>`, `TableLoad<T>` and `Records` types, the load and the typed row names it
re-exports to the catalog. Column order, optionality and default evidence are in
[schema.md](schema.md). `Row<T>` owns `Definition` and typed `Info` cells. No
table returns a borrowed buffer. The catalog never reparses a raw row to discover
relationships. Text-valued numeric calibration data requires format/radix
interpretation in the catalog.

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

Search matches every recorded name and the descriptions case-insensitively and
includes packet SPIDs. Rank exact identities, identity prefixes, then fuzzy
matches, preferring names over descriptions. Ties sort by kind
parameter/packet/command, identity with numeric SPIDs, then source. Return every
qualifying row without a cap. Blank queries give an empty vector. Scoring and
threshold remain implementation tuning. Loading scales with supported input size;
exact queries and relationships use indexes, while search may scan candidates.

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

## Issue #12 monitoring calibration resolution

`ParameterSummary.calibrations: Info<Vec<CalibrationAlternative>>` carries every
retained definition with no public interface amendment. The reader adds the CUR,
CAF, CAP, MCF, LGF, TXF and TXP tables under the shared partial-loading and
recorded-field rules; a directory holding only these supporting rows still loads.

Each alternative holds an optional `selection` (the CUR row), an optional
`condition` and the calibration `Info`. CUR rows sort by CUR_POS and then source
line, so the declared evaluation order survives, and duplicate positions attach
InconsistentDefinition with every competing row. A condition records the declared
expression `raw(CUR_RLCHK) = CUR_VALPAR` plus a typed dependency on the referenced
monitoring parameter, whose targets are recorded PCF definitions. CUR_SELECT
remains the calibration reference. No telemetry is evaluated.

PCF_CATEG names the calibration family a reference means, because the numerical
and textual keys share one key space. ICD 7.0 declares that PCF_CURTX and
CUR_SELECT match TXF_NUMBR for status parameters and CAF_NUMBR, MCF_IDENT or
LGF_IDENT otherwise, and that neither field applies to text parameters or to
string and time encodings. A reference therefore resolves within its declared
family first: CAF/CAP curve points, MCF polynomial and LGF logarithmic
coefficients for every non-status category, TXF/TXP intervals for `S`.
Definitions that the declared family cannot supply stay available with
InconsistentDefinition attached, so the surveyed category/reference disagreement
loses no usable evidence. A simultaneous PCF_CURTX reference and CUR rows keep
both declaration sets in the declared family with the same problem, because the
family stays PCF_CATEG's either way; a prohibited encoding or category keeps its
calibration with the disagreement recorded. A key that resolves nowhere attaches
MissingReference for the declared family's table, and a key with several targets
retains all of them with AmbiguousReference. No missing or ambiguous target
removes the matching parameter. Because that ambiguity spans supporting tables,
the renderer names every table holding a candidate.

The parameter overview lists alternatives in order with their interpreted values,
marking the ones derived from documented defaults, and `--details` adds the
recorded fields, defaults, sources and reference evidence of each CUR, CAF, CAP,
MCF, LGF, TXF and TXP row. This supersedes the #19
calibration placeholder problem, which no longer appears in parameter views;
packet views embed the same resolved summaries.

Cross-check: the reader supplies all five CUR cells, seven CAF/CAP cells, seven
MCF/LGF cells and four TXF/TXP cells; catalog consumers join PNAME, POS, RLCHK,
VALPAR and SELECT with NUMBR and IDENT and keep display fields in definitions.
Public library and parameter CLI tests cover each family, conditional ordering,
shared and three-way duplicate positions, missing and ambiguous references, the
synthetic equivalent of the surveyed status-reference disagreement and the
simultaneous-declaration case.
The unpublished sample MIB resolves every parameter without a panic and shows the
surveyed category/reference disagreement as an InconsistentDefinition.

## Issue #14 command argument rules

`ValueRules.ranges: Info<Vec<AllowedRange>>`, `aliases: Info<Vec<Alias>>` and
`calibrations: Info<Vec<CalibrationAlternative>>` carry every retained range
boundary, alias mapping and command conversion with no public interface
amendment. The reader adds PRF, PRV, PAF, PAS, CCA and CCS under the shared
partial-loading and recorded-field rules; a directory holding only these
supporting rows still loads.

CPC_PRFREF, CPC_PAFREF and CPC_CCAREF name the three families, and each key
resolves in its own table's key space. A reference the CPC row does not declare
yields a known-empty collection, which the CLI reports as no declared rule. A
declared reference without a usable target yields MissingReference for the header
table, or for the value table when the header resolved but its value rows did
not, and never removes the argument. Several header rows under one key keep every
set: each set emits its own entries interpreted by its own declarations, and one
AmbiguousReference problem lists every candidate definition. The resolved range
set and alias set header rows are retained in `ValueRules.supporting_definitions`,
so their recorded fields stay reachable with their provenance.

AllowedRange carries the PRF representation code (PRF_INTER) and one PRV row
definition per declared boundary. PRF_DSPFMT and PRF_RADIX interpret PRV_MINVAL
and PRV_MAXVAL: `U` with the declared radix as an unsigned value, `I` as a signed
integer and `R` as a decimal. `A`, `T` and `D` declare a character or time token
that has no numeric reduction, so those bounds retain their recorded text as a
text scalar; the command slice applies that rule around the shared numeric
interpretation, leaving calibration point interpretation unchanged. An omitted
PRV_MAXVAL stays unavailable instead of becoming an empty range. Alias carries
one PAS mapping: the raw value interpreted with PAF_RAWFMT beside the declared
text. PAF declares no radix column, so alias values are decimal.

PRF_NRANGE, PAF_NALIAS and CCA_NCURVE are compared with the PRV, PAS and CCS rows
retained for their key. A declared count that disagrees with the number of
retained rows attaches InconsistentDefinition naming both declarations, their
values and every available definition, and no row is dropped. A header without
value rows reports its missing value table instead, and an omitted count is not a
disagreement.

A CalibrationAlternative holds the CCA row as the calibration definition and its
CCS points in a CommandConversion form. CCS_XVALS is interpreted with CCA_RAWFMT
and CCA_RADIX, and CCS_YVALS with CCA_ENGFMT. CCA declares no interpolation and
CPC_INTER names a raw or engineering input rather than an extrapolation rule, so
the conversion's interpolation stays unavailable without an invented
declaration. Malformed established cells drop only their own rows, and unusable
boundary or point interpretations keep the recorded text beside a structured
UnsupportedInterpretation problem.

The CLI prints an Argument rules section after the application-data table. An
argument with declared rules shows its ranges, aliases and conversions in
declared order, each boundary, mapping and point with its source, and the
argument with no declared rules contributes nothing. Problem markers attach to
the affected family or value line, and `--details` adds the recorded fields of
every reachable PRF, PRV, PAF, PAS, CCA and CCS row.

Cross-check: the reader supplies all seven PRF cells, three PRV cells, four PAF
cells, three PAS cells, seven CCA cells and three CCS cells; catalog consumers
join CPC_PRFREF, CPC_PAFREF and CPC_CCAREF with NUMBR and keep display fields in
definitions. Public library and CLI tests cover each family, combined rules,
undeclared families, missing and ambiguous references, conflicting declared
counts, malformed neighbouring rows, omitted optional fields, unavailable
interpretations, an unavailable argument layout and unchanged snapshots. The
unpublished sample MIB resolves every command without a panic and shows range,
alias and conversion values beside the unavailable interpretations its own
declarations carry.

## Issue #15 nested command argument groups

`CommandDescription.arguments: Info<Vec<Layout<CommandElement>>>` carries the
declared group structure with no public interface amendment. The existing tree
types and `Location` fields hold every declared element, group, repetition and
position rule, so neither a public type, field nor operation is added. The reader
already supplies the ten CDF cells, so no reader change is needed either.

CDF declares the unexpanded application data: `CDF_ELTYPE` is `A` for a fixed
area, `F` for a non-editable parameter and `E` for an editable parameter, and an
element declaring `CDF_GRPSIZE` is a repeater whose value at command invocation
repeats the following declared elements. The catalog therefore resolves a
command's elements recursively over declared layout order: `CDF_BIT` then source
line, with duplicates in source order. A repeater stays a declared element
beside the `Layout::Repeat` node holding the elements it repeats, so a counter
transmits once while its group repeats. `CDF_GRPSIZE` counts following declared
records, including nested repeaters, so groups nest as subtrees: S2KTC074's
declaration gives `N1`, `Layout::Repeat` of `APID`, `N2` and the `Layout::Repeat`
of `Type`, `N3` and the `Layout::Repeat` of `Subtype`.

CDF has no fixed-count column, so every group repetition is
`Repetition::Runtime` with a `RuntimeDependent` problem and a dependency on the
repeater's declared value source: the telemetry parameter of `CDF_INTER=T`, or
the CPC definition of `CDF_PNAME` that the MIB records or an operator supplies.
The declaration names which of those supplies the count and the recorded value
where one is declared, which keeps statically known information visible without
turning it into a guaranteed count. No group is expanded, and nothing evaluates a
command invocation. Members therefore carry no enclosure list of their own: the
tree preserves group membership, and each member and following element keeps the
group's unexpanded-position rule in `Location.constraints`, which has no
dependencies of its own so that one group does not repeat the same target
reference at every member.

`CDF_BIT` is never presented as a guaranteed absolute runtime offset. Every
command element keeps `Position::ApplicationDeclaredBit` with its declared value,
the group's declaration states that the declared positions assume one repetition,
and `Location.constraints` on members and following elements records that their
offsets shift with the group.

Contradictory and incomplete declarations stay beside the affected group or
element while retaining usable data. A fixed area declaring `CDF_GRPSIZE` cannot
supply a repetition count, so its repetition is unavailable with
InconsistentDefinition naming `CDF_ELTYPE`, `CDF_GRPSIZE` and every available
definition, and its members remain in the tree. A `CDF_GRPSIZE` that declares more
elements than its group retains keeps the retained members with
InconsistentDefinition naming the declared size and every available definition, and
a declared size above the schema's 1 to 99 range keeps the declared group clamped
to the retained elements with that disagreement. A size below one element, or
nesting beyond the supported depth, leaves the repeater an element with that
problem, and the elements it declared stay declared elements at the enclosing
level; that is the unsupported-extension behavior of VPD repetition. A declared
source whose target never resolved keeps its missing or ambiguous reference
problem beside the group. Duplicate declared positions keep every element in source
order with the existing InconsistentDefinition evidence, inside groups as well as
at the top level.

A group node's `definition` is the repeater's CDF row, so the declared
`CDF_GRPSIZE` value and every other recorded cell stay structurally available
beside the runtime declaration, and `children` carries the retained members.

The CLI prints the application data of a command with declared groups as
two-space nested blocks with `Repeat group` and `End repeat` boundaries, matching
the packet layout convention, and keeps the aligned table when no group is
declared. Group rows carry their declaration source and a problem marker. A
declared condition would print the same way under a `Conditional structure` row, so
every variant of the shared tree type renders as nesting instead of flattening; CDF
declares no conditions.

Cross-check: the reader supplies all ten CDF cells; the catalog consumes
`CDF_CNAME`, `CDF_ELTYPE`, `CDF_ELLEN`, `CDF_BIT`, `CDF_GRPSIZE`, `CDF_PNAME`,
`CDF_INTER`, `CDF_VALUE` and `CDF_TMID` and joins `CDF_PNAME` with CPC and
`CDF_TMID` with PCF. Public library tests cover the synthetic equivalent of
S2KTC074, editable and telemetry repetition sources, missing targets, fixed areas
inside groups and as repeaters, incomplete groups, out-of-range and clamped sizes,
elements following a group, the supported nesting depth and duplicate positions
inside a group. Focused CLI checks cover nested rendering in both output modes, the
flat table for a command without declared groups, and fixed-area and incomplete
declarations whose problems and usable elements render beside each other. The unpublished sample MIB resolves every command
without a panic and renders S2KTC074's nested repetitions with their recorded
counter values.

## Issue #16 command header expansion

The public `CommandHeader` and `HeaderField` types already carry the expansion, so
neither a type, field nor operation is added. The reader adds the TCP, PCDF and
PCPC tables under the shared partial-loading and recorded-field rules; a directory
holding only these supporting rows still loads.

`CCF_PKTID` names the TCP row whose PCDF records describe the packet header the
command is encoded into. TCP resolves by TCP_ID, PCDF rows by PCDF_TCNAME and PCPC
rows by PCPC_PNAME, each keeping duplicates in source order. A resolving reference
that has no retained row is a missing reference, and several rows under one key
keep every definition with the ambiguity attached. The header's own `definition`
is the first retained TCP row in source order, so an ambiguous declaration still
describes the header.

`CommandHeader.fields` holds every PCDF row in declared order: PCDF_BIT, then
source line, which keeps duplicate offsets in source order. Each `HeaderField`
carries its PCDF row as `definition`, PCDF_TYPE as `field_kind`, PCDF_BIT as
`Position::HeaderBit` with PCDF_LEN as the encoded width, the PCPC row named by
PCDF_PNAME as a typed `parameter` target, and PCDF_VALUE as its value.

ICD 7.0 defines the recorded value: a fixed area (`F`) states its content in hex,
an unsigned integer parameter (`PCPC_CODE` absent or `U`) states its default in
PCDF_RADIX, and a signed integer parameter (`I`) states it in decimal, where
PCDF_RADIX is irrelevant. The declared representation therefore reports the radix
that governs the interpretation, while every recorded cell, including PCDF_RADIX,
stays in the definition. Without a linked parameter the declared format is
unknown, so the value stays unavailable and the link carries the reference problem.
A value the declared radix cannot interpret keeps its recorded text and an
UnsupportedInterpretation problem.

Header element types are `F` fixed area, `A` APID, `T` service type, `S` service
sub type, `K` acknowledgement flags and `P` an automatically set packet parameter.
PCDF_TYPE is validated against those codes, so an undeclared type drops its own row
and leaves every other element usable. The renderer names each type and the source
a command load takes its value from, which keeps the invocation-time source visible
beside the recorded default. No telemetry or command invocation is evaluated.

Contradictory and incomplete declarations stay beside the usable elements. ICD 7.0
requires PCDF_PNAME to be absent for a fixed area, so a fixed area declaring one
keeps both declarations with InconsistentDefinition, and a parameter element
declaring none reports the same way. Duplicate PCDF_BIT declarations keep every
element in source order with InconsistentDefinition naming every declaration. Every
PCDF record declaring one PCDF_PNAME must declare the same PCDF_LEN, within one
packet header or across several, so a disagreement attaches
InconsistentDefinition naming PCDF_PNAME, PCDF_LEN and every available definition
to the affected element widths. Missing, ambiguous, contradictory or unsupported
header information never removes the command or changes a unique match into
NotFound.

The CLI prints a `Header` section after the `Argument rules` section, separate from
the application data: the expanded TCP identity with its description and source,
then an aligned table of `Field  Parameter  Location  Width  Kind  Value`. A field
shows its recorded PCDF_DESC, its declared PCDF_PNAME, its header bit, its declared
length in bits, its declared type with the source that supplies its value, and its
interpreted value with the governing radix, marked as the declared default for
every parameter element. A resolved header with no retained PCDF row prints
`No header elements declared`; an unavailable header or field table prints
`unavailable` with its problem markers. `--details` adds every reachable TCP, PCDF
and PCPC definition with its recorded fields, presence, defaults and reference
evidence.

Cross-check: the reader supplies all eight PCDF cells, three PCPC cells and two TCP
cells; the catalog consumes CCF_PKTID, PCDF_TCNAME, PCDF_TYPE, PCDF_LEN, PCDF_BIT,
PCDF_PNAME, PCDF_VALUE and PCDF_RADIX and joins PCDF_PNAME with PCPC. Public
library tests cover declared ordering, fixed content, signed and unsigned
interpretation, documented radix defaults, absent supporting files, missing and
duplicate linked definitions, duplicate offsets, disagreeing declared lengths,
contradictory and malformed declarations, ambiguous packet headers and a
supporting-only snapshot. Focused CLI checks cover the section's placement, its
table in both output modes, and problems rendering beside usable arguments. The
unpublished sample MIB expands every command header without a panic and renders
the nominal, test and no-header declarations with their recorded content.

This supersedes the #13 note that header expansion remains outside that slice, and
the #19 placeholder header problem, which no longer appears in command views.

## Issue #18 combined workflow verification

Issue #18 requested no new public behaviour. It adds `tests/workflows.rs`, one
publishable synthetic snapshot carrying all twenty-five canonical table families
at once, as the environment where calibration, packet structures, command rules,
search and incomplete-data behaviour meet. It reproduces no supplied reference
row; `DEMO_MODE` in packet `89000` at byte 19 bit 0 with a non-status category
beside a textual key stands in for `ZUT00002`, `DEMO_TC001` for `S2KTC001`,
`DEMO_TC074` for `S2KTC074` and `DEMO_TC003` covers the CDF_ELLEN/CPC width
disagreement.

Public-library scenarios cover: a parameter carrying a fixed PLF occurrence, a
variable VPD occurrence inside a runtime group and a conditional calibration
alternative beside its direct `PCF_CURTX` declaration; the surveyed
category/reference disagreement keeping its TXF definition, intervals and
provenance; a duplicated TXF key keeping both candidate targets while its
intervals stay usable; a key resolving nowhere naming every table of the
declared family; a fixed packet combining identification criteria, a bounded
two-place repetition and embedded parameter summaries with calibrations; a
variable packet keeping declared groups, runtime dependencies and recorded-only
dependency targets; a command combining declared argument positions, ranges,
aliases, conversions, a telemetry value source, a missing range table, nested
repeat groups, a fixed area and separate expanded header fields; the
CDF_ELLEN/CPC width disagreement staying local to the affected argument;
duplicate packet, command and parameter roots staying separate and in source
order; partial snapshots keeping usable information beside missing tables; a
supporting-only snapshot answering both absence reasons; snapshot independence
when sources change or disappear; owned results outliving the snapshot with
relative one-based provenance; and deterministic ordering across independent
loads and separate processes.

CLI process scenarios cover every lookup kind against the same snapshot, the
`mode` candidate table and its scope, PUS coordinates, `--details` reaching a
definition from each of the twenty-five table files, the silent status-1 exact
miss outside debug mode for both absence reasons and all three kinds, and
`--debug` reporting dropped rows, ignored extra columns, skipped files and the
query decisions without changing stdout.

One integration gap was resolved: the packet lookup emitted its query event but
not the absence reason its parameter and command siblings report, so a missing
packet could not be told from an unloaded one under `--debug`. The catalog now
emits the same `packet not found` event. No contract statement changed; this only
completes the existing debug-event requirement.

Two behaviours are recorded rather than changed, because the contract does not
prescribe the first and the second is its documented consequence. Duplicate
fixed PLF roots for one SPID keep a single occurrence entry whose packet summary
stays unavailable and whose ambiguity problem retains both candidate roots, at
the declared location, while duplicate variable roots produce separate entries;
the generic "sorted by numeric SPID, then source for duplicate packet
definitions" sentence and the #11 variable-only statement can be read as
disagreeing, and the observable locations differ only in the variable case.
Missing tables and unreadable tables both reach public results as missing
references, distinguished only by the debug `skipped file` reason.

Full `cargo check --all-targets`, `cargo clippy --all-targets -- -D warnings`,
`cargo fmt --check` and the complete `cargo test --all-targets` suite pass, 132
tests including these nine. The unpublished sample MIB resolves `ZUT00002`,
packet `89000`, `S2KTC001`, `S2KTC074` and the `mode` search, and every one of
its own command, parameter and packet identities renders in both output modes
without a panic or unexpected error: 528 checked runs. This verification uses
synthetic data and a sample MIB, so it does not claim exhaustive ICD
conformance. No supplied reference file or example MIB row was published.

## Issue #29 occurrence availability

The public types already distinguish an unknown collection from an empty one:
`ParameterDescription.occurrences.value` is `None` when the occurrence sources
cannot establish a result, and `Some` with no element only when the declared
occurrences are known to be none. `Catalog::parameter_occurrences` now applies
that distinction instead of asserting a known empty result for every found root.
An absent occurrence list stays `None` unless a readable PLF table accounts for
the fixed occurrences and every declared variable packet structure has VPD data.
A declared structure has no VPD data when the table is missing or unreadable, or
when it is readable with no retained row under the declared `PID_TPSD` key. The
declared variable packet structure is what triggers this check, so a snapshot
without `vpd.dat` still reports a known empty result when no PID root declares a
variable layout.

Independently known occurrences are unaffected. A fixed occurrence from PLF, or a
variable occurrence from VPD, keeps its containing packet beside the
missing-reference problem for whichever source is unavailable, and every declared
`PID_TPSD` key without VPD rows reports one missing `VPD` reference under that
key. Exact parameter lookup stays `Found` with status 0 whenever the root
definition is usable. The renderer is unchanged: `none` remains the presentation
of a present empty value and `unavailable` that of an absent one.

Cross-check: the catalog consults only its own `TableLoad` states and existing
supporting indexes, so no new reader column or public type is needed. Public
library tests cover a missing PLF table, an unreadable PLF table, a readable empty
PLF table, a fixed occurrence from a readable PLF row, a declared variable
structure with missing, empty or unrelated VPD rows, a known empty result from a
readable VPD table, a variable occurrence from a matching VPD row, one reference
for two PID roots sharing a TPSD, a variable occurrence kept beside a missing PLF
table, and a fixed occurrence kept beside an unavailable variable structure. CLI
tests cover `none` against `unavailable` for the `Packets` section in both output
modes with the missing-reference problem retained beside a usable definition.

## Issue #30 fixed occurrence width consensus

A fixed PLF occurrence derived its encoded width only from a uniquely resolved
parameter summary, so an ambiguous PCF reference discarded a width that every
candidate established independently. `Catalog::occurrence` now derives that width
from every retained candidate through the same shared consensus the variable
layout uses: the width survives when all candidates establish the same value, and
stays `None` when candidates disagree, when a candidate cannot establish a width,
or when no definition matches the declared name. The ambiguous parameter summary,
its `AmbiguousReference` problem and every candidate definition stay on the
occurrence and on its width unchanged, no candidate is selected, and the exact
parameter lookup for a duplicated name stays `Ambiguous`.

PTC/PFC still determines the encoded extraction width. `PCF_WIDTH` remains the
padding declaration and is never consulted for a fixed occurrence's width, so
candidates declaring equal PTC/PFC with different `PCF_WIDTH` keep the shared
PTC/PFC width. Sharing one candidate-width derivation also gives a fixed
occurrence's width the sources the variable layout already retains: the declaring
PLF row beside every candidate PCF row. Nothing reads that list for a fixed
occurrence, so the rendered output is unchanged.

Cross-check: the candidate-width derivation and the consensus helper are both
shared with the variable layout, so the fixed and variable paths cannot disagree
about what a candidate establishes or what agreement means. Public library
tests cover two candidates that agree while their `PCF_WIDTH` declarations
disagree, candidates whose PTC/PFC establish different widths, a candidate whose
PTC/PFC declares no supported width, and a name with no matching definition, and
confirm the duplicate exact parameter lookup stays `Ambiguous`. A CLI test covers
the packet `Layout` width and the retained candidates in both output modes. Both
paths were also compared over every parameter, packet and command identity of the
unpublished sample MIB in both output modes: 256 runs, no output or status
difference. Every fixture is synthetic; no supplied reference row was published.

## Issue #31 search over every recorded packet name

A packet root's name comes from a TPCF reference, and `PacketSummary::name` is
deliberately unavailable while that reference is ambiguous. Search scored only
that resolved value, so a query matching one of the recorded names found nothing
unless it also matched the SPID or the description, making an otherwise usable
partial dataset undiscoverable by name.

`Catalog::search` now scores every name a candidate records. `Catalog::packet_names`
returns each TPCF name retained under one packet root, in source order with
duplicates kept, and `Catalog::searchable_names` combines those with the resolved
name a candidate already carries, so parameters and commands keep their single
recorded name. Scoring takes the best match across the identity and those names,
which keeps the contractual ranking unchanged: exact identity, identity prefix,
fuzzy identity or name, then description. Existing tie-breaks stay. Each PID root
still yields exactly one candidate however many of its names match, and duplicate
PID roots stay separate candidates in source order.

Ambiguity is preserved exactly as before. No TPCF definition is chosen to display
a hit: the candidate's `name` stays unavailable with its `AmbiguousReference`
problem and every candidate definition, the candidate table keeps printing
`unavailable`, and the returned identity resolves in the exact packet lookup to
the same ambiguous evidence. A root recording one TPCF row is unaffected and
still shows that recorded name.

Cross-check: only the catalog's existing TPCF supporting index and the existing
`Candidate` fields are read, so no reader column, public type or renderer changed.
Public library tests cover distinct competing names, duplicate identical TPCF
names, duplicate PID roots, case folding and unicode-preserving folding, the
`Packets` and `All` scopes, one candidate per matching PID root, the retained
`AmbiguousReference` evidence and the reusable exact-lookup identity. A CLI test
covers the candidate table for names that never appear in it and the `--details`
problem evidence naming both TPCF definitions. Every fixture is synthetic; no
supplied reference row was published.

## Table listing

The maintainer agreed this slice directly, without an issue: a `tables` verb that
reads the configured `MIB_DIR` like every other verb and reports each supported
table's code, file name, declared content, the lookup that addresses its rows and
the state the load left it in. The agreed scope stops there. Files present in the
directory that the loader does not read are not named, and no per-table column
inventory is added to the CLI; `--details` adds nothing, as it adds nothing to
search.

`Table` now carries the loader's own facts about a table: `Table::ALL` lists every
supported table in code order, and `code`, `file` and `meaning` name it. `reader::read_table`
takes a `Table`, so the file name a read attempt uses and the file name the listing
prints cannot drift apart, and it records the attempt as a `TableReport` beside the
`TableLoad` state it already produced. `Mib::tables` returns those reports, so the
listing comes from the one load rather than a second directory read, in the same
`Read`/`Unreadable`/`Missing` distinction the loader already preserved: a readable
table with no retained rows reports `0` and stays distinct from an absent one.
`Table::root` names the lookup kind a root table's rows are addressed as, and the
CLI renders it: `PCF` is `parameter`, `PID` is `packet`, `CCF` is `command`, and
every supporting table is `-`. Code order is also file-name order, because each
file is the lowercase code with a `.dat` suffix.

This adds one `Request` and `Response` pair for the existing
`Mib` operation, so every request still has a public operation and every response
still has a variant. No reader column, retained record, catalog index, public
description or existing rendering changed. The listing requires a usable
directory like every other verb, so the absent and empty `MIB_DIR` errors, the
inaccessible-directory load error and the no-usable-rows load error keep their
statuses, and no identity is involved, so no status-1 or status-3 outcome exists
for this verb.

Cross-check: the listing is generated from `Table` alone, so library tests assert
that codes and file names are unique, that every file name matches its code, that
code order is the presented order, that a directory providing every catalogued
file reports every table as read, and that absent, readable-empty and unusable
files report three different states. A CLI test asserts every cell of every row
against `Table` and the fixture's load states, `--details` and `--debug` leaving
stdout unchanged, and the two `MIB_DIR` errors. The unpublished sample MIB lists
all 25 tables with its retained row counts, including `vpd.dat` read with no usable
row. Every fixture is synthetic; no supplied reference row was published.
