<p align="center">
  <img src="assets/badger.png" alt="Badger logo" width="200" />
</p>

# Badger

Badger is a dataflow language, that aims to have source code ergonomics closer to languages like Rust or Zig. It uses strong types and immutable data structures.

Instead of step-by-step control flow, Badger programs are graphs of values and transformations that run when their inputs are ready. However, the syntax follows a familiar paradigm (and is heavily inspired by Rust and Haskell).

See [spec/lang.md](spec/lang.md) for the full language specification. [examples/hello-world/main.badger](examples/hello-world/main.badger) and [spec.badger](spec.badger) contain runnable examples.

## Language at a Glance

- **Immutable, dataflow execution.** A program is a graph; nodes fire when their inputs are ready. Independent subgraphs run in parallel with no ceremony.
- **Effect cascading.** Pure nodes (no effectful inputs) are freely reordered, memoized, or elided. Effectful nodes fire exactly as the graph dictates.
- **Expression-oriented, Rust-shaped syntax.** `struct`, `enum`, `interface`, `implement`, pattern-matching `match`, generics with `<T>` — familiar shapes with strong static typing.
- **No loops — `@recurse` only.** Iteration is tail-recursive self-reference inside a function; nearest-enclosing `fn` scope.
- **Capability-passing for effects.** The program entry point receives an `Init` value; all IO and runtime access is threaded through it explicitly. Effects are also tracked in types (`!effect(io)`).
- **Ordering.** Prefer data edges → capability threading (effectful functions return the capability) → `!depend` as an escape hatch for explicit happens-before.
- **Runtime boundary.** Capability interfaces declared in `.badger`; the host crate provides implementations. Low-level primitives use `extern` with `@intrinsic(...)`.

## Hello World

```
pub main = fn(init: Init) -> () {
  init.io.stdout.print_line("Hello World");
};
```

## Repository Layout

- [lib/](lib/) — Badger-level standard library (`.badger` sources).
- [examples/](examples/) — Example Badger programs, one directory per example.
- [crates/](crates/index.md) — Rust crates implementing the toolchain (parser, graph IR, optimizer, runtime, interpreter, compiler, host). See [crates/index.md](crates/index.md) for the full breakdown.
- [spec/](spec/) — language specification and design docs.
  - [lang.md](spec/lang.md) — language specification.
  - [grammar.md](spec/grammar.md) — formal grammar (W3C EBNF).
  - [ir.md](spec/ir.md) — intermediate representation (dataflow graph).

## Giants

Badger stands on the shoulders of giants. It borrows ideas from:

- **[Rust](https://www.rust-lang.org/)** — syntax and type-system shape.
- **[Zig](https://ziglang.org/)** — explicit runtime boundaries and capability passing.
- **[Haskell](https://www.haskell.org/)** — purity and effect tracking.
- **[TensorFlow](https://www.tensorflow.org/)** — dataflow-graph execution.

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE), at your option.
