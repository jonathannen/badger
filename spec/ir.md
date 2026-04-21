# Badger Intermediate Representation (IR)

Status: rough draft.

The Badger IR **is** the dataflow graph. Use-def edges between SSA values are execution edges: a node fires when all of its incoming edges carry values. Source order in the IR is irrelevant — the scheduler reads the graph, not the text.

## Line Format

Every instruction has the shape:

```
%N: T = opcode operands [!effect(...)]
```

- `%N` is an SSA name. Every value is single-assignment and single-typed.
- `T` is the value's type. Types are always explicit in the IR.
- `opcode` names the operation. Juxtaposition never means application; every op is named.
- Operands are `%N` references, literals, or symbols.
- `!effect(...)` annotations mark effectful ops.

## Example

```
%1: i32 = const 1
%2: i32 = const 2
%3: i32 = add %1 %2
%4: (i32, i32) = tuple %1 %2
%5: i32 = tuple.get %4 0
```

`%1` and `%2` have no dependency on each other and may fire in parallel. `%3`, `%4`, `%5` fire when their inputs are ready.

## Types

Every SSA value carries its type. The IR is fully typed and, in its primary form, monomorphic — generics are instantiated before the IR is handed to the optimizer or scheduler. Types are integral to well-formedness: validation (§Validation) checks type agreement on every operand.

Effect annotations are part of a value's type: `() !effect(io)` is a distinct type from `()`. An op that produces an effectful value must be reachable from an effectful input or an effect source (capability).

Types on operands are determined by the operand's definition; only the result type appears on each line.

### Struct Types

Struct types are **nominal** with layout defined in a module-level type table. The text on each instruction references structs by name; the table holds the layout.

```
// Module-level type table:
type Socket = { bytes_sent: u32 }

// Field access and functional update:
%b:  u32    = field.get   %s "bytes_sent"
%s2: Socket = struct.with %s { bytes_sent: %new }
```

Structs are immutable at the IR level (SPEC §1.1). `struct.with` produces a new value; the original is unchanged.

### Interface Types and Dispatch

Interface bounds are resolved through **monomorphization**. Generic code with an interface bound is specialized to one concrete function per instantiation; the primary IR form is monomorphic and contains no unresolved interface calls.

```
// Higher form (during lowering, generics still present):
%r: bool = call @Stoppable::stop<Socket> %sock

// Primary form (after monomorphization):
%r: bool = call @stop_Socket %sock
```

Capability interfaces are not special-cased. `init.io.stdout` has a concrete host-provided type (e.g. `HostStdout`) that implements the `Stdout` interface declared in the stdlib; calls through it monomorphize to `extern` intrinsics (SPEC §7.4) the same way any other interface call does.

Dynamic dispatch (`dyn Trait`) is not part of the IR at this stage. _(open: when and how to add it.)_

## Functions and Regions

A function is a named sub-graph with input ports and a result port:

```
fn add_one(%x: i32) -> i32 {
  %1: i32 = const 1
  %2: i32 = add %x %1
  yield %2
}
```

- Parameters are named SSA values scoped to the region.
- `yield` exits the region with a value.
- A region is a dataflow sub-graph. Nothing inside a region escapes except via `yield`.

Call sites consume a function value and argument edges, producing the result:

```
%r: i32 = call @add_one %x
```

## Iteration: `@recurse` as a Feedback Edge

`@recurse` is the only cycle permitted in the IR. It is an edge from a call site inside a function region back to that function's input ports, consumed on the next iteration. The top-level graph is acyclic; iteration is isolated to function regions.

```
fn loop_a(%i: u8) -> u8 {
  %c:  u8   = const 10
  %lt: bool = lt %i %c
  %n:  u8   = add %i 1
  %r:  u8   = select %lt (@recurse %n) (%i)
  yield %r
}
```

`@recurse` produces a value of the function's return type; it does not "return" imperatively. `select` fires once both branches resolve (one of which is a tail recursion).

## Control Flow

Structured control flow lowers to dataflow nodes, not basic blocks. There are no phi nodes — every reader of a value reads a single, unique SSA name.

- `cond ? a : b` → `select cond a b`
- `match` → `match` node with pattern operands per arm; produces bound SSA values in the matched arm's sub-region.
- Both branches of a `select` are always materialized at the IR level. Lazy firing (only evaluate the taken branch) is a scheduler optimization enabled by purity, not an IR concept.

## Effects and Ordering

All effects in the IR are expressed as **data edges on effect tokens**. An effectful op consumes a token (typically a capability value like `Stdout`) and produces a new one. Downstream effectful ops on the same capability take the new token as input, creating a deterministic happens-before edge.

```
%s0: Stdout = capability.get %init "io.stdout"
%s1: Stdout !effect(io) = call @print      %s0 "Hello"
%s2: Stdout !effect(io) = call @print_line %s1 "World"
```

Two things to note:
- The token is a real value. The scheduler sees the edge and orders the effects without special cases.
- The capability-threading pattern from SPEC §7.3 is the *only* mechanism at the IR level. `!depend(a)` in source code lowers to a synthetic token edge — there is no `!depend` opcode.

### Pure vs Effectful

Effectfulness cascades through the IR per SPEC §1.2:

- An op is effectful if it carries `!effect(...)` or any of its inputs are effectful.
- Pure ops are freely reorderable, memoizable, and elidable by any pass.
- Effectful ops must fire exactly as the graph dictates.

Every pass must preserve this distinction. An optimizer that hoists a pure op past an effectful one is legal; the reverse is not.

## Validation

A well-formed IR satisfies:

1. **Single assignment.** Each `%N` is defined exactly once.
2. **Use-after-def.** Every use of `%N` is reachable from its definition along data edges.
3. **Type agreement.** Every operand's type matches the opcode's signature.
4. **Acyclicity at top level.** The only cycles in the graph are `@recurse` edges inside function regions.
5. **Effect monotonicity.** An op marked `!effect(X)` has at least one input marked `!effect(X)` or is a primitive effect source (capability).

Every pass (lowering, optimization, scheduling) must produce IR that still validates.

## Open Questions

- **Pattern representation.** How `match` arms expose bound values — sub-regions with their own SSA scope, or flat with explicit destructure ops?
- **Closure representation.** How captured values are attached to function values.
- **Dynamic dispatch.** When and how `dyn Trait` enters the IR; new opcode (`dyn.call`) and type form.
- **Value equality.** Whether struct and enum equality opcodes exist at the IR level, or equality is always lowered to user-defined pure functions.
- **Scheduling hints.** Whether the IR carries parallelism hints or leaves them entirely to the scheduler.
- **Textual vs in-memory canonical form.** Whether the text format shown here is normative or just a printer for the real in-memory graph.
