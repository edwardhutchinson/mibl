# TUI follow-up tasks

Requested on September 25, 2026, after issue #52's browser and keyboard refinements.
Work on `eh/issue-52-tui`, starting from `cf8d235`. Keep `main` unchanged.

Document the work now. Begin implementation no earlier than September 26, 2026,
00:57:34 UTC, which is 02:57:34 CEST. This is the requested six-hour delay.

Complete A through D in order, with one implementation commit per task. Update
its checkbox and completion record in that commit. These task IDs and acceptance
criteria are the reference for subsequent work without the chat history.

## Progress

- [ ] A. Sticky column headers for definition lists
- [ ] B. Structured definition inspection
- [ ] C. PUS browsing view
- [ ] D. Table navigation and editor mode

## A. Sticky column headers for definition lists

Replace the repeated `packet`, `parameter`, and `command` prefixes in definition
rows with a proper table presentation. The view's tab identifies its kind.

Acceptance criteria:

- Packet, parameter, and command lists have labeled, aligned columns for their
  available identity, name, description, and source information.
- Column headers stay visible while users scroll or page through rows.
- Mixed search and PUS results retain a Kind column where needed to distinguish
  definitions. Homogeneous lists avoid repeating the kind on every row.
- Selection remains visible, and Enter retains exact-identity lookup and duplicate
  ambiguity behavior. Long text and narrow terminals remain usable.
- Screen tests exercise scrolling with persistent headers and selection.

Completion record: pending.

## B. Structured definition inspection

When Enter opens a packet, parameter, or command, present its information in
labeled sections with borders and restrained color. Help users find the useful
parts of a definition without reading a single uninterrupted text dump.

Acceptance criteria:

- Make identity and summary clear, with labeled sections for the relevant layout,
  occurrences, calibrations, arguments, groups, value rules, and command header.
- Visually distinguish usable values, unavailable information, and problems.
  Color supplements labels so meaning remains readable without color.
- Preserve complete recorded fields, supporting definitions, source locations,
  and problem evidence, including partial and ambiguous definitions.
- Keyboard navigation and scrolling reach every section, including long or wide
  content. Returning to the list preserves the selected entry.
- Build presentation from typed query results or shared presentation structures;
  do not invoke CLI commands or reconstruct domain information from CLI text.
- Synthetic screen tests cover packet, parameter, and command views, problematic
  definitions, and access to complete evidence.

Completion record: pending.

## C. PUS browsing view

Add a dedicated PUS tab with a populated service/subtype listing. Users should
be able to discover definitions by their PUS coordinates without typing a filter.

Acceptance criteria:

- Show existing PUS services and subtypes in numeric service-then-subtype order.
- Selecting a service/subtype shows its telemetry packet and telecommand
  definitions; selecting a definition opens the inspection from task B.
- Retain current library filtering and duplicate-identity semantics. Represent
  unavailable coordinates explicitly wherever listed; do not invent coordinates.
- Provide visible keyboard shortcuts and a route back to the PUS listing.
- Keep scoped search and the existing `f` PUS filter usable.
- Synthetic tests cover ordering, packet and command results, selection,
  unavailable coordinates, and return navigation.

Completion record: pending.

## D. Table navigation and editor mode

Make the Tables view a selectable list. Up/Down and paging move between supported
tables; Enter opens the selected table in editor mode.

Editor choice: clarification was requested. The proposed default is an editable
grid inside the TUI with an explicit Save action. Alternatives offered were
opening the file in `$EDITOR` or a read-only inspector. Record the user's answer
here before implementation; if no answer arrives during the delay, use the
embedded editor default. This is a new request beyond #52's read-only scope.

Acceptance criteria for the default embedded editor:

- Table selection, paging, Enter, and return navigation work with visible help.
- Display columns and rows from the selected source table. Preserve source data
  that domain parsing does not retain, including unknown trailing fields.
- Support keyboard cell editing, explicit save, and cancellation. Opening a table
  or navigating away must not silently write changes.
- Preserve untouched data, including empty and omitted cells and duplicate rows.
  Report unavailable files and read/write failures clearly.
- Detect external file changes before overwriting them. Save through a temporary
  file and replacement so a failed write does not truncate the original.
- Keep the loaded domain snapshot's lifetime explicit. After saving, tell the
  user that restarting reloads definitions; do not silently mix snapshot versions.
- Use synthetic temporary files to verify navigation, editing, save/cancel,
  preservation, conflicts, and errors. Never modify mission/reference fixtures.

Completion record: pending.

## Shared constraints and verification

Keep the established shortcuts: `P` packets, `p` parameters, `c` or `C` commands,
`t` tables, `/` search, and `f` PUS filter. Numeric definition shortcuts remain
aliases. Add and document a discoverable shortcut for the new PUS tab.

Preserve existing CLI output, flags, exit behavior, exact lookups, and search
ordering. Domain queries continue to use public `Mib` operations. Terminal
cleanup must continue to work on exit, initialization failure, errors, and panic.

The user confirmed these test seams: public library APIs, keyboard actions and
rendered TUI screens with synthetic fixtures, and CLI/terminal process behavior.
Editor file I/O can be exercised through the same keyboard and process seams
against disposable synthetic files.

For each task, run the relevant tests, type checking, formatting, and strict
Clippy checks. Review standards and task acceptance criteria before marking it
complete. Run the full test suite at the end and record results with the final
completion update. The branch is intended for a later PR into `main`; this
request does not ask to publish or merge that PR yet.
