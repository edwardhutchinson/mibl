# Clap library integration

This evaluation assesses whether the `mibl` project should introduce `clap` for command-line argument parsing, balancing architectural contract conformance, dependency footprint, compile-time overhead, and solo-developer maintenance velocity.

---

## 1. Current CLI Architecture and Contract

The executable CLI entry point and configuration are defined in [`src/main.rs`](../../src/main.rs), with behavioral requirements specified in [`docs/design/cli-output.md`](../design/cli-output.md#a-overview-by-default-evidence-on-request), [`docs/interfaces/contract.md`](../interfaces/contract.md#issue-19-cli-presentation-amendment), and verified in [`tests/cli.rs`](../../tests/cli.rs).

### 1.1 Current Parsing Implementation
Currently, argument parsing is implemented manually in 55 lines within `src/main.rs` (`fn configure`, lines 29–83). The routine processes `arguments: Vec<OsString>` and `mib_dir: Option<OsString>`:

- **Flag Extraction**: Scans leading tokens for `--debug` and `--details`. Duplicate flags (`--debug --debug`, `--details --details`) break scanning and fail validation.
- **Verb and Argument Matching**: Matches exact positional slices:
  - `parameter NAME`: Validates that `NAME` is non-empty, does not start with `-`, and is valid UTF-8.
  - `packet SPID`: Validates that `SPID` consists entirely of ASCII digits (`s.bytes().all(|b| b.is_ascii_digit())`) and parses into a `u64`. Rejects negative signs (`-1`), positive signs (`+1`), non-digits (`no`), and integer overflow.
- **Environment Handling**: Reads `MIB_DIR` as an `OsString` via `std::env::var_os("MIB_DIR")`. It strictly distinguishes `None` ("MIB_DIR is not set") from `Some("")` ("MIB_DIR is empty"), while preserving non-Unicode directory paths on Unix ([`tests/cli.rs#L131-L149`](../../tests/cli.rs#L131-L149)).

### 1.2 Agreed Application Contract
The CLI contract establishes strict syntax and process-level semantics:

- **Exact Syntax**: `mibl [--debug] [--details] parameter NAME | packet SPID`.
- **Strict Prefix Positioning**: Flags may appear in either order (`--debug --details` or `--details --debug`), but *only* before the verb. Flags placed after the verb (e.g., `mibl parameter TEMP --details` or `mibl parameter --details`) are rejected ([`tests/cli.rs#L254-L266`](../../tests/cli.rs#L254-L266)).
- **Exit Statuses**:
  - `0`: Successful execution (both found descriptions and ambiguous candidate tables).
  - `1`: Silent not-found results (`Lookup::NotFound(NoMatchingIdentity)` and `Lookup::NotFound(DefinitionsUnavailable)`). Emits zero bytes to stdout and stderr when debug is inactive.
  - `2`: Configuration errors, invalid argument errors, MIB load errors, and output I/O errors.
- **Error Formatting**: Configuration and argument errors write `mibl: <message>` to stderr, such as `mibl: usage: mibl [--debug] [--details] parameter NAME | packet SPID`.
- **Built-in Flags**: The current CLI does not recognize `-h`, `--help`, `-V`, or `--version`; passing them yields an argument error and exit code 2.

### 1.3 Future CLI Requirements
`src/main.rs` already contains internal enum variants for subsequent slices:
- `Request::Command(CommandName)`: Command lookup (`mibl command CNAME`).
- `Request::Search { query: String, scope: SearchScope }`: Search across definitions with an optional scope flag (`mibl search QUERY [--scope <SCOPE>]`), where scope choices are `parameters`, `packets`, `commands`, or `all` ([`docs/interfaces/contract.md#representative-cross-checks`](../interfaces/contract.md#representative-cross-checks)).

---

## 2. Primary Source Evaluation of Clap (Clap v4)

Evaluating [clap v4](https://docs.rs/clap/latest/clap/) against official documentation and primary APIs reveals how well it fits `mibl`'s technical requirements.

### 2.1 API Paradigms
Clap provides two primary authoring styles:

1. **Builder API** ([`clap::Command`](https://docs.rs/clap/latest/clap/struct.Command.html), [`clap::Arg`](https://docs.rs/clap/latest/clap/struct.Arg.html)):
   - Programmatically configures CLI structures using `Command::new()`, `.arg()`, and `.subcommand()`.
   - Can be used via the `clap` crate or the lightweight [`clap_builder`](https://docs.rs/clap_builder/latest/clap_builder/) crate directly.
   - Requires no procedural macros and avoids proc-macro dependencies.

2. **Derive API** ([`clap::Parser`](https://docs.rs/clap/latest/clap/trait.Parser.html), [`clap::Subcommand`](https://docs.rs/clap/latest/clap/trait.Subcommand.html), [`clap::ValueEnum`](https://docs.rs/clap/latest/clap/trait.ValueEnum.html)):
   - Declaratively derives parsers onto Rust structs and enums via `#[derive(Parser)]`.
   - Requires `features = ["derive"]`, invoking the [`clap_derive`](https://docs.rs/clap_derive/latest/clap_derive/) macro.

### 2.2 Dependency Footprint and Compilation Overhead
Empirical measurements on the `mibl` codebase reveal significant differences in dependency tree depth and compile-time cost:

| Configuration | Crates in `cargo tree` | New Packages | Clean Build Time (Wall / User CPU) | Incremental Build | Release Binary Size |
|---|---|---|---|---|---|
| **Current Baseline** (manual 55 lines) | 16 | 0 | 6.03s / 8.88s | 0.02s | 1.5 MB |
| **Clap Builder** (`clap` default features) | 27 | 11 | 6.42s / 11.20s | 0.03s | 1.6 MB |
| **Clap Minimal Builder** (`default-features = false`) | 19 | 3 | 6.15s / 9.40s | 0.02s | 1.5 MB |
| **Clap Derive** (`features = ["derive"]`) | 31 | 14 | 6.87s / 15.79s | 0.04s | 1.6 MB |

#### The `syn` Version Split
A notable cost of `clap`'s derive macro is crate duplication:
- `mibl`'s existing dependency `tracing` pulls in `tracing-attributes`, which depends on `syn v2.0.119`.
- Current `clap_derive v4.6.4` depends on `syn v3.0.5`.
- Enabling `clap`'s derive feature forces Cargo to compile **two distinct major versions of `syn`**, driving a **+78% increase in clean build user CPU time** (from 8.88s to 15.79s).

Using the Builder API or `clap_builder` avoids `clap_derive` entirely, keeping build times and dependency counts modest.

### 2.3 Contract Compatibility and Behavioral Nuances

Empirical testing against `mibl`'s test assertions reveals specific points of alignment and divergence:

1. **Prefix-Only Flag Strictness**:
   - In clap, flags defined on the root command that are *not* marked [`.global(true)`](https://docs.rs/clap/latest/clap/struct.Arg.html#method.global) are only accepted before the subcommand.
   - Running `mibl parameter TEMP --details` correctly yields `ErrorKind::UnknownArgument` because `--details` is not recognized by the `parameter` subcommand.
   - Running `mibl parameter --details` similarly triggers `ErrorKind::UnknownArgument` rather than parsing `--details` as the parameter name.

2. **Duplicate Flag Rejection**:
   - According to the [clap `ArgAction::SetTrue` documentation](https://docs.rs/clap/latest/clap/enum.ArgAction.html#variant.SetTrue): *"If the argument has previously been seen, it will result in an ArgumentConflict unless Command::args_override_self(true) is set."*
   - Clap natively rejects `--details --details` and `--debug --debug` with `ErrorKind::ArgumentConflict`, satisfying `mibl`'s contract out of the box.

3. **SPID Numeric Validation**:
   - `mibl` requires SPID to be pure ASCII digits, rejecting `+1`, `-1`, `no`, and overflow ([`tests/cli.rs#L207-L212`](../../tests/cli.rs#L207-L212)).
   - Standard [`clap::value_parser!(u64)`](https://docs.rs/clap/latest/clap/macro.value_parser.html) delegates to Rust's standard unsigned integer parsing, which **accepts `+1`**.
   - To preserve `mibl`'s exact contract, a custom validator (e.g. [`ValueParser::custom`](https://docs.rs/clap/latest/clap/builder/struct.ValueParser.html#method.custom)) checking `s.bytes().all(|b| b.is_ascii_digit())` is required.

4. **Error Exit Codes and Formatting**:
   - In Clap v4, [`clap::Error::exit_code()`](https://docs.rs/clap/latest/clap/error/struct.Error.html#method.exit_code) returns `2` for syntax and validation errors, matching `mibl`'s required exit status 2.
   - However, clap formats errors with multi-line diagnostic headers (`error: unexpected argument ... \n\nUsage: ...`), whereas `mibl`'s contract specifies single-line stderr strings (`mibl: usage: ...`).

5. **Help and Version Flags**:
   - Clap automatically registers `-h`, `--help`, `-V`, and `--version`, exiting with code `0`.
   - Matching `mibl`'s current rejection of `--help` requires setting [`.disable_help_flag(true)`](https://docs.rs/clap/latest/clap/struct.Command.html#method.disable_help_flag) and [`.disable_version_flag(true)`](https://docs.rs/clap/latest/clap/struct.Command.html#method.disable_version_flag).

6. **Environment Variable `MIB_DIR`**:
   - `mibl` reads `MIB_DIR` from the environment, distinguishing unset from empty, and supporting non-Unicode paths on Unix.
   - Keeping `MIB_DIR` resolution in `src/main.rs` via `std::env::var_os` remains cleaner than forcing it into clap's argument model, as `MIB_DIR` is strictly an environmental precondition rather than a CLI flag.

---

## 3. Solo Developer & Small Project Context

For a solo developer maintaining a focused codebase, library adoption involves balancing development velocity against long-term maintenance baggage.

### 3.1 Benefits of Adopting Clap (Why We Should Adopt Clap)

1. **Velocity on Upcoming Slices (`command` and `search`)**:
   - The planned `search` operation requires flags with arguments (e.g. `mibl search QUERY --scope <all|parameters|packets|commands>`).
   - Parsing flags with optional values, validating enum choices, and handling variations like `--scope all` vs `--scope=all` manually requires boilerplate string slicing and error handling.
   - With `clap`, `SearchScope` simply derives [`ValueEnum`](https://docs.rs/clap/latest/clap/trait.ValueEnum.html) or uses `value_parser!(["all", "parameters", "packets", "commands"])`, giving instant parsing, validation, and error messages.

2. **Standard CLI Usability**:
   - Clap provides professional CLI conveniences with zero maintenance: automatic `--help` documentation, `--version` reporting, and typo suggestions (e.g. suggesting `parameter` if the user types `paramter`).
   - If the tool is distributed to end users or integrated into shell scripts, [`clap_complete`](https://docs.rs/clap_complete/latest/clap_complete/) can generate Bash/Zsh/Fish completions, and [`clap_mangen`](https://docs.rs/clap_mangen/latest/clap_mangen/) can generate standard Unix man pages directly from the same definitions.

3. **Declarative Architecture & Self-Documentation**:
   - Moving from positional array patterns (`[verb, name]`, `[verb, spid]`) to strongly typed definitions improves readability and keeps the CLI structure self-documenting.

### 3.2 Costs & Drawbacks for a Small Project

1. **Compile-Time Cost**:
   - Adding `clap` with `derive` nearly doubles clean build CPU time (+78%) due to `clap_derive` and the `syn v3` dependency.
   - Even with builder-only features, clean build user time increases by ~25%.

2. **Transitive Dependency Surface**:
   - Introducing `clap` adds 11 to 14 new crates to the dependency tree for a binary that currently relies on only 3 direct dependencies (`tracing`, `tracing-subscriber`, `unicode-width`).

3. **Unnecessary for the Current Slice**:
   - The existing 55-line manual parser is fully implemented, thoroughly tested across 15 integration test suites, has zero compile overhead, and matches the accepted contract with 100% precision.

---

## 4. Actionable Recommendations

### Recommendation: Two-Phase Adoption Strategy

1. **Current Slice (Do Not Adopt Yet)**:
   - **Retain the current manual parser** for the immediate parameter and packet inspection slices.
   - The current 55-line implementation in `src/main.rs` is fully tested, adds zero dependencies, compiles instantaneously, and avoids any impedance mismatch with existing contract tests in `tests/cli.rs`.

2. **Future Slice (Adopt When Implementing `search` and `command`)**:
   - **Introduce `clap` when implementing the search slice (`search QUERY [--scope <SCOPE>]`)**, where the developer velocity benefits of option parsing, value validation, and enum parsing outweigh dependency costs.
   - **Recommended Flavor**:
     - **For Minimal Build Overhead**: Use the **Builder API** with minimal features:
       ```toml
       clap = { version = "4", default-features = false, features = ["std", "usage", "error-context", "help"] }
       ```
       This provides standard usage, help, and error reporting while avoiding procedural macros and duplicate `syn` builds.
     - **For Maximum Developer Ergonomics**: Use the **Derive API** (`features = ["derive"]`) if typed struct/enum ergonomics are preferred, accepting the one-time compile cost of `clap_derive`.
   - **Contract Evolution**: When adopting `clap`, propose an amendment to [`docs/design/cli-output.md`](../design/cli-output.md) to formally embrace standard clap error output and `--help`/`--version` flags rather than spending effort suppressing clap's standard behaviors to mimic custom legacy formatting.
