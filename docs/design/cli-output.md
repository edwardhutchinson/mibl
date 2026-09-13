# CLI output alternatives

Status: awaiting maintainer decision for [#19](https://github.com/edwardhutchinson/mibl/issues/19).
These are proposals, not an amendment to the accepted interface contract.
No renderer behavior or CLI flags have changed. All identities, descriptions,
values and source locations below are invented and publishable.

## Cases to compare

Normal: monitoring parameter DEMO_TEMP is an unsigned 16-bit value with units K,
big endian, PTC 3 / PFC 12. It occurs once at packet byte 16, bit 0 in packet
42001, DEMO_HK, with APID 42 and service 3/25. PIC disables additional criteria.
Calibration expansion is currently unsupported and must be visible even here.

Problematic: DEMO_MODE is an 8-bit enumerated parameter, PTC 2 / PFC 8. Packet
42002 contains it twice, at bytes 20 and 22, from one PLF row with count 2 and
stride 16 bits. DEMO_UNKNOWN has no PCF target at byte 24. DEMO_DUP at byte 25
has two PCF targets. Two TPCF rows disagree on the packet name. Two PIC rows
declare different PI1 widths. Both alternatives remain evidence; neither wins.
The DEMO_MODE parameter view exposes the containing packet's problems, but does
not expand unrelated DEMO_UNKNOWN or DEMO_DUP occurrences.

Examples show proposed presentation of current library information. They do not
claim that unsupported calibration or variable-layout joins are implemented.

## A: overview by default, evidence on request

Proposed syntax: `mibl [--debug] [--details] parameter NAME` or
`mibl [--debug] [--details] packet SPID`. Each flag may occur once, in either
order before the verb. No short alias, flags after the verb, or explicit overview
flag. Without `--details`, show the overview. `--debug` independently controls
loading/query diagnostics on stderr and never changes result verbosity.

Normal parameter overview:

```text
Parameter DEMO_TEMP
Description: Demonstration temperature
Encoding: unsigned integer, 16 bits, big endian (PTC 3 / PFC 12)
Units: K
Source: pcf.dat:1

Packets
SPID   Name     Location       Width    Repeat
42001  DEMO_HK  byte 16 bit 0  16 bits  once

Problems
[P1] Calibration: unsupported interpretation; expansion is not implemented.
Use --details for recorded fields and problem evidence.
```

Normal packet overview:

```text
Packet 42001  DEMO_HK
Description: Demonstration housekeeping
Source: pid.dat:1

Identification
APID: 42
Service: type 3, subtype 25
Additional criteria: none declared

Layout
Parameter  Location       Width    Repeat
DEMO_TEMP  byte 16 bit 0  16 bits  once

Problems
[P1] DEMO_TEMP calibration: unsupported interpretation; expansion is not implemented.
Use --details for recorded fields and problem evidence.
```

Problematic parameter overview:

```text
Parameter DEMO_MODE
Description: Demonstration mode
Encoding: enumerated, 8 bits, big endian (PTC 2 / PFC 8)
Units: unavailable
Source: pcf.dat:2

Packets
SPID   Name         Location       Width   Repeat
42002  unavailable  byte 20 bit 0  8 bits  1/2, stride 16 bits
42002  unavailable  byte 22 bit 0  8 bits  2/2, stride 16 bits

Problems
[P1] Calibration: unsupported interpretation; expansion is not implemented.
[P2] Packet 42002 name: ambiguous reference; 2 TPCF candidates.
Use --details for recorded fields and problem evidence.
```

Unavailable units remain visible beside the field without inventing a library
Problem. Numbered problems represent actual problems carried by the result.

Problematic packet overview:

```text
Packet 42002  unavailable
Description: Demonstration packet with partial definitions
Source: pid.dat:2

Identification
APID: 42
Service: type 3, subtype 26
PI1 expected: 7; extraction: unavailable [P2]

Layout
Parameter     Location       Width        Repeat
DEMO_MODE     byte 20 bit 0  8 bits       1/2, stride 16 bits
DEMO_MODE     byte 22 bit 0  8 bits       2/2, stride 16 bits
DEMO_UNKNOWN  byte 24 bit 0  unavailable  once [P3]
DEMO_DUP      byte 25 bit 0  unavailable  once [P4]

Problems
[P1] Packet name: ambiguous reference; 2 TPCF candidates.
[P2] PI1 extraction width: inconsistent definition; 8 and 16 bits.
[P3] DEMO_UNKNOWN: missing reference; no matching PCF definition.
[P4] DEMO_DUP: ambiguous reference; 2 PCF candidates.
[P5] DEMO_MODE calibration: unsupported interpretation; expansion is not implemented.
Use --details for recorded fields and problem evidence.
```

### A details rules and excerpt

`--details` starts with the same overview, omits the advice line, then appends
Problem evidence and Definitions. Show every recorded field of every definition
reachable in the public result, including dependency and ambiguous target
records. Preserve all supplied interpretations and problems. Do not invent
unavailable joins or infer extraction widths from contradictory evidence.

Assign definition IDs D1, D2, ... in first encounter order, traversing the root,
identification, then occurrences in public result order, with attached problem
targets visited where encountered. A definition at the same table/source line
is printed once and all uses refer to its ID. Different source lines remain
separate even if their values match. Every use retains its relationship and
problem context. No recursive queries for additional definitions.

The following is an excerpt, not the complete details output. A full rendering
must include every supplied column and definition, not just those illustrated.

```text
Problem evidence
[P1] Packet 42002 name, reference TPCF SPID 42002
  Candidate DEMO_HK_A: tpcf.dat:2 [D2]
  Candidate DEMO_HK_B: tpcf.dat:3 [D3]
[P2] PI1 extraction width
  8 bits: pic.dat:2, PIC_PI1_WID [D4]
  16 bits: pic.dat:3, PIC_PI1_WID [D5]
[P3] DEMO_UNKNOWN, reference PCF NAME DEMO_UNKNOWN
  Declared at plf.dat:3 [D8]; no target.
[P4] DEMO_DUP, reference PCF NAME DEMO_DUP
  Candidate: pcf.dat:3 [D10]
  Candidate: pcf.dat:4 [D11]

Definitions
[D6] DEMO_MODE parameter, pcf.dat:2
  Column 1  PCF_NAME
    Recorded: text "DEMO_MODE"
    Interpreted: "DEMO_MODE", recorded
  Column 3  PCF_PID
    Recorded: empty
    Interpreted: unavailable
  Column 12  PCF_CURTX
    Recorded: omitted
    Interpreted: unavailable
```

Default interpretation example, when supported by the supplied schema meaning:

```text
  Column 7  PLF_TIME
    Recorded: omitted
    Interpreted: 0, documented default
    Rule: <the full rule supplied by the library>
```

In real output, print the actual rule, never the placeholder above. Empty and
omitted remain distinct. Quote text, escape controls, and label interpreted
values as recorded or documented defaults. Multiple possible field meanings get
separate named entries. Unknown interpretations say unavailable. Details include
source filenames, one-based lines and physical columns, references, candidate
definitions, conflicting values, runtime declarations and their dependencies.
Overview shows useful interpreted values, including defaults, but no raw-field
inventory or default-rule text. An interpreted value derived from a documented
default gets a `[default]` suffix where shown. Root provenance appears in the
header; supporting provenance is in details. Supporting definitions contribute
useful names, encoding, locations and problems to the overview without repeating
their complete records. Packet layout parameter descriptions and units appear
in details as a parameter summary before that parameter's recorded fields.

## B: one complete report, with evidence beside each use

No verbosity modes or new syntax. Existing `--debug` remains diagnostic only.
Keep full details in every successful lookup, but group them by subject. Every
use repeats supporting records in place, avoiding cross-references at the cost
of longer output. Normal parameter opening, with the field list abbreviated only
for this comparison:

```text
Parameter DEMO_TEMP
  Description: Demonstration temperature
  Type: unsigned integer, PTC 3 / PFC 12
  Encoded width: 16 bits
  Endian: big endian
  Units: K
  Definition: pcf.dat:1
    Column 1 PCF_NAME: text "DEMO_TEMP" -> "DEMO_TEMP" (recorded)
    ... every recorded field, presence and interpretation ...
  Calibration problem: unsupported interpretation; expansion is not implemented.

Packet 42001
  Name: DEMO_HK
  Definition: pid.dat:1
    ... every recorded field ...
  Characteristics: tpcf.dat:1
    ... every recorded field ...
  Occurrence DEMO_TEMP
    Location: byte 16 bit 0
    Encoded width: 16 bits
    Repeat: once
    Definition: plf.dat:1
      ... every recorded field ...
```

Normal packet opening:

```text
Packet 42001
  Name: DEMO_HK
  Description: Demonstration housekeeping
  Definition: pid.dat:1
    ... every recorded field ...
  Characteristics: tpcf.dat:1
    ... every recorded field ...
  Identification
    APID: 42
    Service type: 3
    Service subtype: 25
    Additional criteria: none declared
    Definition: pic.dat:1
      ... every recorded field, including disabled extraction declarations ...
  Occurrence DEMO_TEMP
    Location: byte 16 bit 0
    Encoded width: 16 bits
    Repeat: once
    Definition: plf.dat:1
      ... every recorded field ...
    Parameter DEMO_TEMP
      ... complete parameter summary and PCF fields ...
      Calibration problem: unsupported interpretation; expansion is not implemented.
```

Problematic packet structure, again an abbreviated comparison:

```text
Packet 42002
  Name: unavailable
  Problem: ambiguous reference TPCF SPID 42002
    Candidate DEMO_HK_A, tpcf.dat:2
      ... complete candidate fields ...
    Candidate DEMO_HK_B, tpcf.dat:3
      ... complete candidate fields ...
  Identification
    PI1 expected: 7
    Extraction width: unavailable
    Problem: inconsistent definition
      8 bits, pic.dat:2
        ... complete fields ...
      16 bits, pic.dat:3
        ... complete fields ...
  Occurrence DEMO_MODE
    Location: byte 20 bit 0
    Encoded width: 8 bits
    Repeat: 1/2, stride 16 bits
    ... full PLF and parameter details ...
  Occurrence DEMO_MODE
    Location: byte 22 bit 0
    Encoded width: 8 bits
    Repeat: 2/2, stride 16 bits
    ... full PLF and parameter details repeated ...
  Occurrence DEMO_UNKNOWN
    Location: byte 24 bit 0
    Encoded width: unavailable
    Problem: missing reference PCF NAME DEMO_UNKNOWN, plf.dat:3
  Occurrence DEMO_DUP
    Location: byte 25 bit 0
    Encoded width: unavailable
    Problem: ambiguous reference PCF NAME DEMO_DUP
      Candidate pcf.dat:3
        ... complete fields ...
      Candidate pcf.dat:4
        ... complete fields ...
```

For the problematic parameter, use the normal B parameter block, followed by
packet 42002 with both TPCF candidates and two occurrence blocks. Repeat PLF
fields for each occurrence. Do not include unrelated parameters or PIC criteria
absent from ParameterDescription. Both formats preserve this public boundary.

B shows every recorded field, omitted/empty distinction, documented default rule,
provenance and alternative beside the relevant value. Use A's field vocabulary
without definition IDs. No data truncation or omitted repeated records is allowed.

## Comparison and recommendation

| Decision | A | B |
|---|---|---|
| First useful information | Identity, encoding or identification, then location table | Identity followed by full root fields before occurrences |
| Labels | Short summary labels; explicit field labels in details | Explicit labels at every nesting level |
| Tables | Aligned spaces for occurrences and candidates | Nested labelled blocks for found results |
| Supporting records | Once per source in details, with references | Repeated at each use |
| Problems | Overview list, full evidence in details | Full evidence beside affected value |
| Cost to inspect raw data | Rerun with --details | Scroll within one report |
| Contract impact | Intentionally relax default full-details requirement | Preserve full-details default |

I recommend A because packet locations remain easy to scan when parameters repeat.
B is useful if inspecting original MIB cells is the usual task and rerunning the
query is undesirable. Neither choice is accepted yet.

## Shared proposed rules

- Preserve exact case-sensitive parameter lookup and unsigned numeric SPID lookup.
  Duplicate roots still produce every candidate in a plain aligned table with
  Kind, Identity, Name, Description and Source. Never pick a candidate. Candidate
  output is the same in both A modes. Ambiguity remains status 0.
- Both not-found reasons remain silent status 1 outside debug mode. Configuration,
  loading and output errors remain visible on stderr with status 2. Found results
  with local problems remain status 0. Do not add interactive selection or a TUI.
- Locations spell out `byte N bit M` for packet-absolute positions. Use the library's
  numbering unchanged. Width and stride always state bits; unavailable is never
  zero. Do not infer width from PCF_WIDTH. Order occurrences as supplied by the
  library, including duplicate locations. Print each expanded fixed occurrence,
  with instance/count and declared stride, without expanding it a second time.
- Use one blank line between sections and two spaces per nesting level. Align
  table columns with spaces and two spaces between columns. Measure displayed
  text in Unicode display columns. Never use tabs for alignment. Do not impose
  column width limits: long identities or descriptions can exceed terminal width.
- No renderer wrapping, truncation, colors, pager or terminal detection initially.
  Terminal and redirected stdout have identical bytes. Terminals may visually
  wrap long lines themselves. Escape tabs, newlines, carriage returns, escapes
  and other control characters in source text so records cannot inject lines or
  terminal control sequences. Preserve full printable Unicode text.
- In A, every actual Problem reachable in the returned public result appears in
  the overview, including field-interpretation problems. Group identical repeated
  problems only when kind, explanation, evidence and targets match; list every
  affected context. Deterministic numbering follows display traversal order.
  Preserve known values beside warnings. Missing values say unavailable even
  when no Problem is supplied. Empty collections say none only when known empty;
  unavailable collections say unavailable. Do not represent unsupported as empty.
- Later calibration views reuse identity, interpreted summary, problems and
  recorded evidence. Conditional alternatives keep their declared order.
  Variable packet and command views use two-space nested blocks for repeats and
  conditions, without expanding runtime counts. Label application-declared bit
  positions separately from packet-absolute positions. Command headers get their
  own section. Search uses the candidate table without selection. These are
  conventions for later tickets, not new behavior in #20.

A future nesting sketch:

```text
Repeat: runtime COUNT
  Condition: SELECTOR = 1
    Parameter DEMO_VALUE
    Location: runtime dependent
    Width: 8 bits
```

Long-text check for #20: use an identity longer than 100 characters, a description
longer than 200, printable non-ASCII text and embedded controls. Verify all text
survives, controls are escaped, and redirection does not change formatting.

## Proposed contract amendment and implementation handoff

If A is accepted, amend the CLI query → render row of
[contract.md](../interfaces/contract.md) to specify overview by default and complete
recorded details through `--details`. Qualify the issue #9 and #10 coverage text
where it says the CLI renders recorded fields and provenance: that guarantee
moves to details mode; root source remains in overview. Update README and CLI
usage in #20 when the behavior ships. Library descriptions, retained definitions,
problem payloads and information ownership remain unchanged.

If B is accepted, record the grouped presentation and unchanged complete default
in the contract. No new flag or information-reduction amendment is needed.

After explicit acceptance, replace #20's dependency brief with these concrete
requirements and a link to the accepted document commit:

1. Implement the selected section order, labels, field and occurrence presentation.
2. Implement exactly the chosen modes and parser syntax, or explicitly no modes.
3. Preserve all problems, known values, evidence and candidate alternatives using
   the chosen access path. Retain public-library information unchanged.
4. Apply the chosen support-record reuse rule to both lookup directions without
   recursively loading or querying new relationships.
5. Add synthetic CLI process checks for these normal and problematic cases, each
   mode, duplicate roots, misses, errors, controls, long text and redirection.
   Validate field evidence against the current public model rather than treating
   abbreviated comparison excerpts as complete golden output.
6. Update README/usage, run cargo check, clippy and the full suite, and link the
   implementation to workflow verification #18. Later renderer features stay in
   their owning tickets.

## Acceptance record

Pending. The maintainer must select A, B or a concrete revision, confirm syntax,
problem visibility, evidence access and width rules, and explicitly accept the
resulting examples. Record their response and accepted commit here, cross-check
and amend the contract, then update #20. Do not mark #19 complete or #20 ready
before that response. No choice has been made on the maintainer's behalf.
