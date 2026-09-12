# mibl

Read-only SCOS MIB viewer, currently at the interface declaration stage.

The [interface contract](docs/interfaces/contract.md) describes module boundaries,
loading and query outcomes, schema interpretation, and review exchanges.
Library operations are explicit unimplemented placeholders. The CLI prints an
unimplemented message and exits with status 2.

Check declarations with `cargo check --all-targets` and compile-only examples with
`cargo check --example contracts`. No viewer functionality is implemented yet.
