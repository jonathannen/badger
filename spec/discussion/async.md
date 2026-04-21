# Async

Badger has no `async` keyword, no `await`, no `Future`/`Promise` type, and no function coloring. This is not a deliberate omission, but just a natural extension of the execution model.

## Why Badger doesn't need async

In most languages, `async` exists to solve a specific problem: a function that performs IO needs to yield control so other work can run while it waits. The language threads that concern through its type system — a function either is or isn't async, and async-ness is viral up the call graph.

Badger's execution model solves that problem a different way. A program is a dataflow graph ([lang.md §1.2](../lang.md#L13)). Nodes fire when their inputs are ready, and nodes with no dependency between them may run concurrently — including while other nodes are blocked on IO. Concurrency is the _default_, not an opt-in annotation. There's no "main thread" to yield from, because there's no linear thread of execution in the first place.

Concretely, given:

```
a = init.io.stdout.print("one");
b = init.io.stdout.print("two");
c = init.io.fs.read_file("config.toml");
```

The three nodes have no data edges between them. The runtime may fire them in any order, and may fire them concurrently. If `read_file` blocks for 50ms, nothing about `a` or `b` is affected — they simply fire whenever their inputs (just `init.io.stdout` in both cases) are available. No `await` is needed because there's no caller waiting in the sequential sense.

Where a data edge _does_ exist, the dependent node waits automatically:

```
config = init.io.fs.read_file("config.toml");
result = process(config);
```

`result` cannot fire until `config` has a value. That's not asynchrony managed by the programmer — it's the normal dataflow rule. The same mechanism that expresses "b depends on a" expresses "b waits for a's IO to complete."

## Ordering effects without async

Async languages use `await` both to wait for a value _and_ to impose happens-before ordering on effects that don't otherwise depend on each other. Badger splits these:

1. **Data dependencies** — the natural mechanism. A node that consumes the output of another waits for it.
2. **Capability threading** — effectful functions return the capability, so the next call data-depends on it. `stdout.print("a")` returns a `Stdout`; `stdout.print("b")` on that return value is forced to happen after. No `await` needed; the type system forces the ordering.
3. **`!depend`** — the escape hatch when neither of the above is available ([lang.md §7.3](../lang.md#L265-L272)).

In an async language, `await` is load-bearing for both value and ordering semantics. In Badger, value semantics are handled by data edges and ordering is handled by the three mechanisms above, in that preference order.

## Blocking IO and the runtime

A practical question: what happens when an effectful node makes a blocking syscall? Badger's spec doesn't mandate an implementation, but the model permits several:

- **One OS thread per effectful node.** Simple; probably fine for small programs.
- **A work-stealing thread pool with blocking-detection**, where the scheduler spawns additional threads when one is blocked in a syscall. This is what Tokio's `spawn_blocking` does, promoted to the default.
- **Platform-level async IO under the hood**, with effect implementations using `io_uring` / `epoll` / `kqueue` to avoid blocking threads at all. The Badger program never sees this — it just sees capabilities returning values.

The key property is that none of these choices are visible in Badger source. A program written against the `Stdout` interface runs identically on a runtime that uses blocking IO and a runtime that uses `io_uring`. This is the payoff for surfacing effects through interfaces ([lang.md §7.4](../lang.md#L284-L294)) rather than baking them into the language.

## What's still open

- **Cancellation semantics.** If a consumer decides it no longer needs a value, can the producing subgraph be cancelled? What happens to effects already in flight?
- **Streaming / incremental values.** Nodes currently produce a single value. First-class streams would need either a new node kind or a standard-library `Iterator`/channel abstraction.
- **Back-pressure.** If a producer is faster than a consumer, where does the buffering happen, and how is memory bounded?
- **Fairness.** When many nodes are ready at once and resources are finite, what scheduling guarantees does the runtime provide? (§9 already lists this as open.)
- **Effect ordering across unrelated capabilities.** If two subgraphs both touch the filesystem but hold different capability values, do the underlying effects interleave freely, or does the runtime linearize them? This probably wants to be a property of the capability, not the language.

The short version: Badger replaces `async`/`await` with "dataflow is already concurrent." You get parallelism for free by not introducing dependencies; you get ordering by introducing them. The language stays small, and the host runtime gets to decide how to actually execute the graph.
