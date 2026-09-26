# TUI follow-up tasks

Requested on September 25, 2026, after issue #52's browser and keyboard refinements.
Work on `eh/issue-52-tui`, starting from `cf8d235`. Keep `main` unchanged.

Document the work now. Begin implementation no earlier than September 26, 2026,
00:57:34 UTC, which is 02:57:34 CEST. This is the requested six-hour delay.

Complete A through D in order, with one implementation commit per task. Update
its checkbox and completion record in that commit. These task IDs and acceptance
criteria are the reference for subsequent work without the chat history.

## Progress

- [x] A. Sticky column headers for definition lists
- [x] B. Structured definition inspection
- [x] C. PUS browsing view
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

Completion record: implemented sticky Ratatui column headers, a Kind column for
mixed results, and horizontal column navigation. TUI tests, type checking,
formatting, and strict Clippy pass.

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

Completion record: definition views now have bordered sections, colored headings,
problem and unavailable-value highlighting, and `[` / `]` section navigation.
Shared section boundaries retain complete details without parsing CLI output.
TUI tests and CLI compatibility tests pass, as do type checking and strict Clippy.

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

Completion record: `u` opens a PUS tab grouped by service and subtype, sorted
numerically with unavailable coordinates last. Enter opens the group's definitions;
Esc returns through inspection, results, and groups with selection retained.
Synthetic screen tests cover ordering, duplicates, missing coordinates, and the
existing filter. Type checking, formatting, and strict Clippy pass.

## D. Table navigation and editor mode

Make the Tables view a selectable list. Up/Down and paging move between supported
tables; Enter opens the selected table in editor mode.

Editor choice: the user selected opening the source file in `$EDITOR`.

Acceptance criteria:

- Table selection, paging, Enter, and return navigation work with visible help.
- Enter launches the configured `$EDITOR` with the selected table's source path.
  Support editor arguments and paths containing spaces without interpolating the
  source path into shell code.
- Restore normal terminal mode before launching the editor, wait for it to exit,
  then resume the TUI with the selected table retained.
- Missing or empty `$EDITOR`, unavailable files, launch failures, and unsuccessful
  editor exits produce clear feedback without leaving the terminal corrupted.
- Editing and saving are owned by the external editor. The browser never rewrites
  or normalizes the table file itself.
- Keep the loaded domain snapshot unchanged and explain that restarting reloads
  edited definitions.
- Synthetic process tests verify selected paths, editor invocation and arguments,
  return navigation, failures, and terminal restoration.

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
External editor invocation can be exercised through the same keyboard and
process seams against disposable synthetic files.

For each task, run the relevant tests, type checking, formatting, and strict
Clippy checks. Review standards and task acceptance criteria before marking it
complete. Run the full test suite at the end and record results with the final
completion update. The branch is intended for a later PR into `main`; this
request does not ask to publish or merge that PR yet.
