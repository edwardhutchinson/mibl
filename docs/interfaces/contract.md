# MIB viewer interface contract

Status: accepted by the maintainer at commit `ef6ea86`; issue #8 is closed.
[Acceptance record](https://github.com/edwardhutchinson/mibl/issues/8#issuecomment-5648856740).
The complete interface set remains the contract for the implementation slices.
Issue #9 implements explicit-directory loading, PCF lookup and CLI rendering.
Packet, command and search operations remain explicit placeholders.

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
| CLI query → render | borrowed Response, output writer | ExitCode or io::Error | details for Found; plain candidate table for ambiguity/search | both NotFound reasons emit no bytes and exit 1 |
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
