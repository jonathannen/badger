# Crates

The Badger toolchain is implemented as a set of Rust crates, each with a narrow responsibility. Source flows from text through a concrete syntax tree to a typed dataflow graph, which is then either executed by the interpreter or lowered by the compiler. The runtime and host crates provide the shared machinery and platform-specific capability implementations.

## Pipeline

- [`badger-cli`](badger-cli/) — the `badger` binary; parses command-line arguments and dispatches to the other crates.
- [`badger-cst`](badger-cst/) — source → concrete syntax tree (lexer + parser, tokens, spans).
- [`badger-graph`](badger-graph/) — CST → graph IR (lowering + typecheck); defines the graph types.
- [`badger-optimizer`](badger-optimizer/) — graph-to-graph optimization passes.

## Execution

- [`badger-runtime`](badger-runtime/) — shared core: values, scheduler, effect machinery, capability traits.
- [`badger-interpreter`](badger-interpreter/) — executes the graph via the runtime.
- [`badger-compiler`](badger-compiler/) — emits code that calls into the runtime.
- [`badger-host`](badger-host/) — Rust-side implementations of runtime capabilities (io, fs, ...).
