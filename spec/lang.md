# Badger Language Specification (BLS)

Badger is an immutable, dataflow-oriented programming language with strong static typing. Programs describe a graph of value-producing nodes; nodes execute as soon as their inputs are available. Instead of step-by-step control flow, a Badger program is a graph of values and transformations that run when their inputs are ready — but the surface syntax is deliberately familiar, modeled on Rust with selective borrowings from Haskell.

Badger's design goals, in priority order:

1. **Dataflow as the execution model.** A program's meaning is its dependency graph. Evaluation order falls out of data dependencies, not source order; independent subgraphs run in parallel without ceremony.
2. **Immutability by default.** No `mut`, no in-place mutation. "Updating" a value produces a new one. This is what makes dataflow tractable — nodes are pure functions of their inputs unless they explicitly carry effects.
3. **Familiar syntax.** `struct`, `enum`, `interface`, `implement`, `match`, `fn`, generics with `<T>`. A programmer from Rust, Swift, or TypeScript should be able to read Badger source without a decoder ring, even though the execution model underneath is different.
4. **Explicit effects and capabilities.** Side effects are tracked in types (`!effect(io)`) and performed through capabilities threaded from `Init` (§7). There is no ambient IO.
5. **Small, principled core.** One form for function values (`fn`), one form for iteration (`@recurse`), one form for modules (`@import`). Features earn their keep by composing with the dataflow model, not by adding escape hatches from it.

Ideas Badger borrows from:

- **[Rust](https://www.rust-lang.org/)** — the shape of the type system and most of the syntactic surface.
- **[Zig](https://ziglang.org/)** — explicit runtime boundaries; capabilities passed rather than ambient.
- **[Haskell](https://www.haskell.org/)** — purity as the default; effects tracked in types.
- **[TensorFlow](https://www.tensorflow.org/)** — computation as a dataflow graph, not a control-flow program.

This document describes the language as currently specified. Sections marked _(open)_ indicate decisions that have not yet been made; provisional sections are subject to change.

For the intermediate representation (the dataflow graph a program lowers to), see [ir.md](ir.md).

## 1. Core Model

### 1.1 Immutability

All bindings are immutable. There is no `mut` keyword and no in-place mutation. "Updating" a value means producing a new value derived from the old one. A binding name can be shadowed in a nested scope (see §1.4), but the original value is unchanged.

### 1.2 Dataflow Execution

Once parsed, a program is a graph. Each binding is a node; each reference to a binding is an incoming edge. A node fires as soon as all of its incoming edges carry values. Nodes with no dependency between them may execute in any order, including concurrently.

Consequences:

- Evaluation order is not determined by source order.
- Independent subgraphs are implicitly parallel.
- A node with no consumers and no effects may be elided. Effectful nodes must fire regardless of whether their value is consumed — effects are observable by definition, so the compiler and runtime cannot elide them on the grounds that nothing reads the result.

**Effect cascading.** Effectfulness is a property of individual nodes and propagates transitively: a node is effectful if it directly performs an effect or if any of its inputs are effectful.

A non-effectful node is considered _pure_ — given the same inputs, it produces the same result, and the compiler and runtime are free to memoize, reorder, elide, or re-execute it. Effectful nodes must fire exactly as the dataflow graph dictates; pure nodes are fully substitutable by their results.

### 1.3 Expression-Oriented

To keep the graph resolved, every construct must produce a value.

Functions, `match`, ternaries, and blocks are all expressions. There is no statement/expression distinction at the grammar level — terminating `;` turns an expression into a binding or sequencing form.

**Block value.** A block's value is its trailing expression — the final expression not terminated by `;`. There is no `return` keyword: a `fn` body, like any block, yields its trailing expression. Early exit from a computation is expressed with `match`, ternaries, or a future propagation operator (see below), not with a control-flow escape.

A `;` after an expression is a **terminator**: it sequences the expression, discards its value, and ensures the expression is not the block's trailing expression. The two forms therefore differ in meaning:

```
{ x }    // block evaluates to x
{ x; }   // block evaluates to () — the `;` discards x's value
```

A block whose contents are all `;`-terminated has no trailing expression and evaluates to `()` (§3.1); this is the normal shape of a block composed of effectful calls whose values are not consumed:

```
pub main = fn(init: Init) -> () {
  init.stdout.print_line("Hello World");   // effectful, value discarded
  // no trailing expression — block evaluates to ()
}
```

The canonical form for a value-producing block is no trailing `;`. Writing `;` after the tail expression is legal but means something different — it changes the block's value to `()`.

When a block body is itself the body of a top-level declaration or binding, a `;` after the closing `}` is optional declaration punctuation. It does not change the value of the block. These forms are equivalent:

```
pub main = fn(init: Init) -> () {
  init.stdout.print_line("Hello World");
}

pub main = fn(init: Init) -> () {
  init.stdout.print_line("Hello World");
};
```

This is a deliberate consequence of the dataflow model (§1.2). A block is a subgraph whose output edge is its trailing expression; introducing `return` would mean that some _other_ node in the block becomes the output depending on runtime control flow, which is not expressible as a single static output edge. Keeping blocks expression-shaped preserves the property that the graph is fully determined by the source text.

_(open: whether Badger adopts a `?`-style propagation operator for short-circuiting on `Result` / `Option`, analogous to Rust's `?`. This is the intended replacement for the use cases `return` would otherwise cover.)_

### 1.4 Shadowing

A binding may reuse a name that is already in scope. The new binding shadows the previous one from that point onward; the previous binding's value is unchanged. Shadowing follows Rust's conventions, and the rebinding idiom — including same-name rebinds within a single block — is the idiomatic way to "update" a value step-by-step under immutability.

```
x = 1;
y = {
  x = x + 1;   // shadows outer `x`; RHS sees outer `x` (= 1)
  x = x * 10;  // shadows the previous `x`; RHS sees `x` (= 2)
  x            // block evaluates to 20
};
// outer `x` is still 1 here
```

Shadowing rules:

- **A name may be bound more than once in the same block.** Successive `=` bindings form a sequence in source order; each binding shadows all earlier ones under the same name for the remainder of the enclosing block.
- **Nested scopes may shadow outer scopes.** Block bodies, `fn` parameter lists, `match` arm bindings, and other nested scopes may reintroduce a name from any enclosing scope.
- **Each binding's RHS sees the previous binding of the name, not itself.** The RHS is evaluated against the name's meaning _before_ the new binding is introduced; the new binding becomes visible only to code textually after it. This is what makes `x = x + 1` well-defined rather than self-referential, and extends to longer chains like `x = x * 10` that follow an earlier `x`.
- **References are resolved lexically.** Each textual use of a name refers to the most recent binding of that name visible at that lexical point. Each binding is its own dataflow node (§1.2); shadowing only decides which node a textual occurrence points to — it never retargets an existing edge.
- **Shadowing is not mutation.** Earlier bindings retain their values and remain reachable from anything already holding a reference to them — notably closures that captured the name before the shadow.

Because source order within a block defines the shadowing sequence, same-name rebindings are one of the few places where lexical order carries meaning in Badger. The dataflow graph itself is still order-independent — the ordering is a property of how names resolve, not of when nodes fire.

_(open: whether pattern bindings in `match` arms and destructuring forms follow the same rules uniformly; whether shadowing across `pub` / module boundaries is permitted; diagnostics for shadowing that is likely unintentional.)_

### 1.5 Bindings

A binding attaches a name (or pattern) to the value of an expression with `=`.

**Plain binding.** The type is inferred from the RHS:

```
x = 1;
greet = fn(name: String) -> String => "hello, " + name;
```

**Type-annotated binding.** An optional `: Type` between the name and `=` asserts the binding's type. The RHS must be assignable to that type; the annotation is checked, not merely recorded:

```
x: i32 = 1;
pub main: Main = fn(init) { ... }
```

Type annotations are useful when the RHS's type is wider than what you want the binding to expose, when the RHS's type is not fully inferrable on its own (e.g. an untyped `fn` literal whose parameter types the annotation pins down — see §3.1), or to document intent at module boundaries.

**Destructuring binding.** The left-hand side may be a pattern that decomposes the RHS. Record/namespace destructuring uses braces and binds each named field to a name of the same name:

```
{ Init } = @import("std").process;
{ HttpServer, HttpClient } = @import("net").http;
```

Tuple destructuring uses parentheses:

```
(first, second) = some_pair;
```

Destructuring bindings follow the same shadowing and immutability rules as plain bindings (§1.1, §1.4). Each introduced name is an independent binding; none of them is an alias for the RHS as a whole.

_(open: renaming in destructuring (e.g. `{ Init as I }`); default / fallback forms; rest patterns; whether the LHS may mix destructuring with a type annotation on the whole pattern.)_

## 2. Types

### 2.0 Comments

Comments are trivia: they do not affect evaluation, name resolution, or typing, but they are part of the source text and must be preserved by tools that round-trip source.

**Line comments.** `//` starts a line comment that runs to the end of the line:

```
// this is a comment
x = 1;
```

**Doc comments.** `///` starts a doc comment. A doc comment is line-oriented like `//`, but is intended to document the declaration that immediately follows it:

```
/// Option represents a value that is optionally available.
pub enum Option<T> {
  None;
  Some(T);
}
```

Multiple consecutive `///` lines belong to the same doc-comment block.

_(open: block comments such as `/* ... */`; exact attachment rules for doc comments; whether doc comments are permitted on non-declaration forms.)_

### 2.1 Primitive and Built-in Types

Badger's primitive types follow Rust's conventions. See the [Rust Reference — Types](https://doc.rust-lang.org/reference/types.html) for the canonical semantics; Badger inherits naming, widths, and bit-level behavior unless noted otherwise.

Initial set:

- Signed integers: `i8`, `i16`, `i32`, `i64`, `i128`, `isize`
- Unsigned integers: `u8`, `u16`, `u32`, `u64`, `u128`, `usize`
- Floating point: `f32`, `f64`
- Boolean: `bool`
- Character: `char` (Unicode scalar value)

`usize` and `isize` are pointer-sized integers, following Rust's convention. They are the expected types for sizes, lengths, and indices (e.g. `String.size`, array indices in collection interfaces).

`String` is _not_ a primitive — it is provided by the standard library on top of `[]u8`. String contents are assumed to be UTF-8 encoded.

**Integer literal inference.** An integer literal with no contextual type is `i32`. When the literal appears in a context that pins a numeric type (assignment to a typed binding, function argument, return position), the literal takes that type if it fits. A literal that does not fit its inferred type is a compile error.

_(open: overflow behavior — wrap/trap/saturate; float literal inference defaults; explicit numeric suffixes (`3000_u32`).)_

### 2.2 Unit

The empty tuple `()` is the unit type. There is no separate void type; `Void` is an alias for `()`.

```
type Unit = ();
```

`()` is a real, first-class value — it can be bound, passed, and returned.

### 2.3 Tuples

```
type ATuple = (i32, i32);
```

### 2.4 Arrays and Slices

Sized array:

```
type AnArray = [3]i32;
```

Undimensioned slice:

```
type UndimensionedSlice = []i32;
```

### 2.5 Enumerations

```
pub enum Result<T, E> {
  Ok(T);
  Error(E);
}
```

Variants may carry payloads. Generic parameters are declared with angle brackets.

**Variant access.** Variants are reached by `.` on the enum type name, the same operator used for module members (§8) and struct fields (§2.6):

```
ok  = Result.Ok(42);
err = Result.Error("nope");
```

Badger does not distinguish a separate `::` path operator for type-level paths; `.` is the one namespace-access operator. In contexts where the enum is already in scope from a `match` or `use`-style binding, variants may be written unqualified (`Ok(42)`) — this follows the normal name-resolution rules, not a special form.

### 2.6 Structs

```
struct Socket {
  bytes_sent: u32;
}
```

Fields are immutable. "Updating" a struct means constructing a new one.

### 2.7 String Literals

String literals are written with double quotes: `"hello"`. Contents are UTF-8 (§2.1). Escape syntax, numeric/Unicode escapes, and raw-string forms follow [Rust's string-literal rules](https://doc.rust-lang.org/reference/tokens.html#string-literals) — `\n`, `\t`, `\\`, `\"`, `\xNN`, `\u{...}`, and raw strings (`r"..."`, `r#"..."#`) — except where explicitly diverged from. Single quotes are reserved for `char` literals, not strings.

_(open: string interpolation; multi-line string forms; whether byte-string literals (`b"..."`) are supported; exact divergences from Rust, if any.)_

## 3. Functions

### 3.1 Function Values

Functions are values bound with `=`. There is no separate `fn name()` declaration form.

Block body:

```
lambda_a = fn(data: []u8) -> Result<bool, bool> {
  Result.Ok(true)
}
```

Arrow (single-expression) body:

```
lambda_b = fn(data: []u8) -> Result<bool, bool> => Result.Ok(true);
```

**Block bodies vs. arrow bodies.** A block body yields its trailing expression, following the general block-value rules in §1.3. Arrow bodies have no trailing-`;` subtlety: `=> expr` always yields `expr`'s value, and there is no way to write an arrow body that yields `()` other than by making the expression itself be `()`.

The return type annotation is still required for a block-bodied `fn` without external type context; the block's `()`-by-default behavior for all-`;`-terminated bodies applies to what the block _produces_, not to what the signature declares.

**Parameter and return-type inference.** Parameter type annotations and the return arrow may be omitted when the function's type is fixed by context — typically, an enclosing type annotation on the binding (§1.5) or the expected type of a call argument:

```
pub main: Main = fn(init) {
  init.stdout.print_line("Hello World");
}
```

Here `Main` pins `init`'s type and the return type, so `fn(init) { ... }` needs no annotations of its own — and because the return type is pinned by context, the `-> ()` arrow is redundant and should be omitted. A `fn` literal with no external type context must declare its parameter types and (if it isn't obvious from the body) its return type.

_(open: precise rules for when parameter types can be elided; whether return types are ever inferred from body in the absence of context; interaction with generics.)_

### 3.2 Function Types

Function types are written with the `fn` keyword — the same shape as the value syntax (§3.1), minus the body:

```
// parameter to a higher-order function
apply: fn(value: In, index: usize) -> Out

// type aliases
pub type MapApply<In, Out> = fn(value: In, index: usize) -> Out;
pub type Main = fn(init: Init) -> ();
```

There is no bare-arrow function-type form: `(a: A) -> B` as a _type_ is not a function type. Keeping a single spelling (`fn(...) -> T`) avoids two-syntaxes-for-one-thing and keeps function types distinguishable from tuple types at the first token.

Parameter names in a function _type_ are documentation only; they do not bind at call sites. A function type may omit parameter names entirely (e.g. `fn(In, usize) -> Out`).

_(open: whether named parameters in function types ever carry meaning beyond documentation — e.g. for keyword-style calls.)_

### 3.3 Generics

```
map<In, Out> = fn(values: []In, apply: fn(value: In, index: usize) -> Out) -> []Out {
  iter = fn(index: usize, acc: []Out) -> []Out =>
    index < values.length
      ? @recurse(index + 1, acc.append(apply(values[index], index)))
      : acc;

  iter(0, [])
}
```

_(open: generic bounds / trait constraints.)_

### 3.4 Recursion and Iteration

Badger has no loop keyword. Iteration is expressed through self-recursion using the `@recurse` form, which refers to the enclosing `fn`:

```
loop_a = fn(i: u8) -> u8 {
  i < 10 ? @recurse(i + 1) : i
}

loop_b = fn(i: u8) -> u8 => i < 10 ? @recurse(i + 1) : i;
```

`@recurse` always refers to the immediately enclosing `fn` — it does not cross nested function boundaries. The idiomatic pattern for iteration is an outer `fn` that sets up state and an inner tail-recursive lambda that does the looping (see `map` above). Mutual recursion and cross-boundary recursion are expressed by giving the target function a name and calling it directly.

_(open: whether `@recurse` is guaranteed tail-call-optimized. Future extension: labeled `@recurse` to target a named outer `fn` if the nearest-enclosing restriction proves limiting in practice.)_

### 3.5 Method Chaining

Nothing in the language prohibits chained method calls such as `response.status(200).set_header(...).write(...).end()`. Each call is an independent expression whose receiver is the value returned by the previous call; chaining composes with the dataflow rules of §1.2 and the capability-threading pattern of §7 without special treatment.

_(open: trailing-lambda sugar for callback-shaped APIs — deferred.)_

## 4. Methods and Interfaces

### 4.1 Inherent Methods (`implement`)

Methods are attached to a type with `implement`:

```
implement Socket {
  send(self: Self, data: []u8) -> Result<bool, bool> {
  }
}
```

Methods take `self: Self` by value. Because values are immutable, a method that "modifies" state returns a new value of `Self`.

**Static methods.** A method declared without `self: Self` as its first parameter is a _static method_ — it is called on the type itself rather than on an instance:

```
implement HttpServer {
  listen(port: u16, handler: Handler) -> HttpServer !effect(io) {
  }
}

// called on the type, not on an instance:
HttpServer.listen(3000, my_handler);
```

The presence or absence of `self: Self` is the sole distinction between instance and static methods.

### 4.2 Interfaces

```
interface Stoppable {
  stop(self: Self) -> Result<bool, bool>;
}
```

**Generic interfaces.** _(provisional)_ Interfaces may declare generic parameters in angle brackets, and those parameters may appear in method signatures:

```
interface Iterator<Item> {
  next(self: Self) -> Option<Item>;
}
```

**Constraint-style generic parameters.** _(provisional)_ A generic parameter may be written as a _shape constraint_ rather than a bare name — the parameter stands for any type whose structure matches the given shape. Inside the interface body, the parameter refers to the matched type as a whole, and any names inside the shape are brought into scope as references to the corresponding parts.

Syntactic distinction: a generic parameter is a bare parameter name iff the slot is a single identifier (`<Item>`, `<T, U>`); any other form in the slot (`<[]Type>`, `<Iterator<Type>>`) is a shape pattern. The CST distinguishes the two purely by the shape of the token(s) in the parameter position — no lookahead into the interface body is required.

```
interface ArrayCollection<[]Type> {
  // `Self` is the matching `[]Type`; `Type` names the element type.
  filter(self: Self, apply: FilterApply<Type>) -> Self;
  map<Out>(self: Self, apply: MapApply<Type, Out>) -> []Out;
}

interface IteratorCollection<Iterator<Type>> {
  // `Self` is the matching `Iterator<Type>`; `Type` names the item type.
  collect(self: Self) -> []Type;
}
```

This form is useful for expressing interfaces that apply uniformly to a family of shapes (all slices, all iterators) without having to name the container type separately.

**Interface inheritance.** An interface may extend one or more other interfaces with `extends`; implementers of the derived interface must satisfy all inherited method signatures:

```
interface Stdio extends Read, Write {}
```

An `extends` clause takes a comma-separated list of interface types. The body may be empty (as above) or may add further methods. Implementing an `extends`-derived interface implicitly implements its parents.

_(open: name collisions across multiple parents; whether `extends` permits default-method inheritance; full rules for matching the shape in constraint-style parameters, including multi-parameter shapes; whether constraint-style parameters may carry bounds.)_

### 4.3 Interface Implementation

```
implement Socket as Stoppable {
  stop(self: Self) -> Result<bool, bool> {
  }
}
```

_(open: coherence rules, default methods, dynamic dispatch.)_

## 5. Control Flow

All control-flow constructs are expressions. They produce a value.

### 5.1 Ternary

```
i < 10 ? @recurse(i + 1) : i
```

### 5.2 Match

```
a = match b {
  true: 1;
  _: 2;
}
```

`match` supports pattern destructuring with variant binding. The pattern `Ok(result)` tests that the value is the `Ok` variant and binds its payload to `result`:

```
maybe_value = maybe_function;
result = match maybe_value {
  Ok(result): process(result);
  _: ();
}
```

Compound patterns work over tuples, enabling multiple narrowing binds at once:

```
maybe_first = maybe_function;
maybe_second = maybe_function;
result = match (maybe_first, maybe_second) {
  (Ok(first), Ok(second)): process(first, second);
  _: ();
}
```

Because `maybe_first` and `maybe_second` have no data dependency on each other, both can be computed in parallel; the `match` is the join point.

_(open: exhaustiveness checking, arm guards, or-patterns, `@`-bindings.)_

## 6. Pipelining

```
result = values |> map()
```

_(open: exact threading rule — left-hand value as first argument, placeholder `_`, or other.)\_

## 7. Effects and Capabilities

Badger uses two complementary mechanisms for managing effects.

### 7.1 Capability Passing

The program entry point receives an `Init` value carrying all runtime capabilities. Effects are invoked by calling methods on capabilities threaded through values:

```
pub main = fn(init: Init) -> () {
  init.stdout.print_line("Hello World");
}
```

Capabilities are ordinary values. They are passed explicitly, not ambient.

### 7.2 Effect Annotations

A function's arrow may carry an effect annotation indicating the effects it performs:

```
read_file = fn(filename: String) -> Void !effect(io) {
  //
}
```

_(open: full set of effects; inference vs. declaration; propagation to callers; relationship to capability passing.)_

### 7.3 Explicit Dependencies _(under consideration)_

Effects on the same resource that do not carry a data dependency can — provisionally — be ordered with `!depend`:

```
hello = init.stdout.print("Hello");
init.stdout.print_line("Hello World") !depend(hello);
```

`!depend` adds a happens-before edge in the dataflow graph regardless of the dependency's value.

**Preferred ordering mechanisms, strongest to weakest:**

1. **Data edges** — the natural dataflow way. Prefer when possible.
2. **Capability threading** — have effectful functions return the capability so the next call data-depends on it (e.g. `print` returns `stdout`).
3. **`!depend`** — escape hatch when neither data nor capability threading is available.

Every unnecessary `!depend` is lost parallelism.

**Why this is under consideration.** `!depend` is an escape hatch, and escape hatches in a dataflow language are load-bearing if they're used routinely — every `!depend` is a place the compiler cannot parallelize. The stronger position is that a well-designed effectful API should make `!depend` unnecessary: if two effects on the same resource need ordering, the API should thread the resource (mechanism 2) so the data-edge rule forces ordering automatically. Whether `!depend` stays in the language depends on whether real Badger code finds cases that capability threading genuinely cannot express, or whether it becomes a crutch for stdlib authors who didn't design the threading carefully.

_(open: whether `!depend` is kept at all; if kept, whether it accepts multiple arguments or stacks; failure semantics when the depended-on node fails; whether `!depend` is legal on pure expressions.)_

### 7.4 Runtime-Provided Functions

Most runtime functionality is surfaced through **interfaces** (§4.2) that Badger source declares and the runtime implements. Capabilities threaded from `Init` (§8) are values whose types implement these interfaces. Badger source never references runtime implementations directly — swapping the host (native, wasm, test mock) requires no changes to `.badger` files.

```
// lib/std/io/stdout.badger
interface Stdout {
  print(self: Self, s: String) -> Self !effect(io);
  print_line(self: Self, s: String) -> Self !effect(io);
}
```

Low-level primitives that do not fit an interface shape are currently a runtime/compiler concern, not part of the Badger source syntax specified here.

## 8. Modules _(provisional)_

A module is brought into scope with the `@import` form, which takes a module path as a string literal and yields the module as a value:

```
std = @import("std");
http = @import("net").http;
```

The result of `@import(...)` is an ordinary value — a namespace whose members are the module's public bindings. Member access uses `.`, and the result is freely composed with destructuring bindings (§1.5):

```
{ Init } = @import("std").process;
{ HttpServer } = @import("net").http;
```

`@import` is resolved at compile time. Its string argument must be a literal, not a computed expression; dynamic or first-class module values are not supported.

**Relative paths.** A path beginning with `./` or `../` resolves relative to the file containing the `@import`. Relative paths name a specific file and include the `.badger` extension:

```
pub @import("./string.badger");
```

Paths _without_ a leading `./` or `../` are package paths — they resolve against the package name (e.g. `"std"`, `"net"`), not against the current file.

**Re-export.** _(provisional)_ An `@import` form prefixed with `pub` at the top level re-exports the imported module's public bindings as part of the current module's public surface:

```
// lib/std/builtin/index.badger
pub @import("./string.badger");
```

The bindings exported by `./string.badger` become public members of `std.builtin`. Re-export is a top-level form; it does not bind a name locally (use a normal `{ ... } = @import(...)` destructuring binding for that). A `pub @import(...)` is a top-level declaration that yields the imported module as a value — the same value produced by a bare `@import(...)`; the `pub` prefix additionally re-exports its public bindings.

_(open: module path resolution beyond relative-vs-package; whether package paths may contain subpath segments inside the string (`"net/http"`) or only top-level names accessed via `.`; visibility rules beyond `pub`; file-to-module mapping (e.g. `index.badger` as the module root); cyclic imports; versioning; vendored vs. stdlib modules; whether `@import` accepts identifiers in addition to string literals; whether re-export can be narrowed to a subset of the imported module's bindings.)_

### 8.1 Prelude

The public bindings of `std.builtin` are _implicitly imported_ into every Badger module. Source files may refer to these names without an explicit `@import`:

```
// No import needed — Option, Result, and String come from std.builtin.
parse = fn(s: String) -> Result<i32, ParseError> { ... };
```

The current prelude contents are the public surface of `lib/std/builtin` — notably `Option<T>`, `Result<T, E>`, and `String`. Because prelude bindings are ordinary stdlib definitions re-exported implicitly, a module that shadows one of their names with a local binding (§1.4) is free to do so — shadowing rules apply uniformly.

_(open: the exact prelude roster; whether prelude membership is fixed by the language or configurable per package; whether a module can opt out of the prelude.)_

## 9. Entry Point

A program's entry point is a public binding named `main`:

```
pub main = fn(init: Init) -> () {
  init.stdout.print_line("Hello World");
}
```

`Init` and `Main` are defined by `std.process`:

```
// lib/std/process
pub struct Init {
  arguments: []String;
  stdin:     Stdin;
  stdout:    Stdout;
  stderr:    Stderr;
  // ... further capability fields supplied by the runtime
}

pub type Main = fn(init: Init) -> ();
```

`Init` is an ordinary struct whose fields are the capabilities the runtime provides for this program (arguments, standard I/O, and other host-supplied capabilities; see §7.1). Capability fields sit directly on `Init` — there is no intermediate `io` grouping — so a program reads `init.stdout` rather than `init.io.stdout`. `Main` is the expected type of the entry-point binding; annotating the binding as `pub main: Main = fn(init) { ... }` lets the `fn` literal omit its parameter and return-type annotations (§3.1).

_(open: the full set of capability fields on `Init`; whether programs may declare a narrower `Init` to request only a subset of capabilities; how `Init` is supplied across targets (native, wasm, test).)_

## 10. Visibility

Top-level declarations default to module-private. Prefixing a declaration with `pub` makes it part of the module's public surface — the set of names accessible via `@import` (§8) and eligible for re-export.

`pub` may appear on:

- **Bindings**, including function values: `pub main = fn(init: Init) -> () { ... }`
- **Type aliases**: `pub type String = []u8;`
- **Structs**: `pub struct Init { ... }`
- **Enums**: `pub enum Option<T> { ... }`
- **Interfaces**: `pub interface Read { ... }`
- **Inherent `implement` blocks**: `pub implement String { ... }`
- **Re-exports**: `pub @import("./string.badger");` (§8)

A `pub implement` block makes the methods defined on that type part of the module's public surface — it does not change the visibility of the type itself (the type's own `pub` controls that).

Interface _implementation_ blocks (`implement T as I`, §4.3) inherit their visibility from the interface and the implementing type; they do not take a `pub` modifier of their own.

Within structs and enums, field- and variant-level visibility is not yet specified — all fields of a `pub struct` and all variants of a `pub enum` are currently treated as public.

_(open: field- and variant-level visibility; additional visibility scopes such as package-private or `pub(crate)`-style modifiers; visibility and interface method signatures; whether `pub` on a non-exported type that appears in a public signature is an error or an inference.)_

## 11. Doc Comments

A line beginning with `///` is a **doc comment**. Doc comments attach to the declaration that immediately follows them and are preserved by tooling for documentation generation. Multiple `///` lines in a row form a single doc block for the following declaration.

```
/// Option represents a value that is optionally available.
///
/// See: https://doc.rust-lang.org/std/option/index.html
pub enum Option<T> {
  None;
  Some(T);
}
```

Doc comments are distinct from ordinary `//` line comments, which are ignored by tooling and carry no semantic association with nearby declarations.

_(open: module-level doc comments (e.g. `//!`); formatting conventions inside doc bodies (Markdown, cross-references); whether doc comments on `pub` items are required or merely idiomatic; doc comments on struct fields and enum variants.)_

## 12. Open Questions

The following aspects of Badger are not yet specified:

- **Scheduling & concurrency semantics.** What is the unit of scheduling? What guarantees exist about parallel firing and the ordering of effects on a shared capability?
- **Error model.** Handling of integer overflow, array out-of-bounds, division by zero, stack overflow from `@recurse`. Whether Badger has panics/traps and how they interact with the dataflow graph.
- **Numeric behavior.** Overflow semantics (wrap/trap/saturate), literal inference defaults, additional float forms.
- **Effect system details.** Full set of effects; inference vs. declaration; propagation; relationship to capabilities.
- **Module system.** File-to-module mapping; package path resolution; cyclic imports; versioning (see §8 for the provisional `@import` form).
- **Generics.** Bounds/constraints, associated types, higher-kinded types, generic `implement`.
- **Interface resolution.** Coherence, default methods, dynamic dispatch, multi-interface implementation, inheritance semantics for `extends`.
- **Match features.** Exhaustiveness checking, arm guards, or-patterns, `@`-bindings.
- **Collection & struct literals.** Array literals, struct construction syntax, string interpolation.
- **Type inference rules.** Scope and algorithm.
- **Operator set.** Full list, precedence, overloading.
- **Equality.** Structural vs nominal equality; deep vs shallow semantics for compound types; NaN handling for floats; whether equality is user-definable via an interface (e.g. `Eq`); equality on function values; distinction (if any) between `==` and an identity/reference check.
- **`@recurse` semantics.** Tail-position requirement, TCO guarantee, behavior across nested `fn`s.
- **Visibility.** Field- and variant-level visibility; additional scopes beyond `pub` (see §10).
- **Prelude.** Exact roster, configurability, opt-out (see §8.1).
- **FFI / runtime boundary.** How `Init` is supplied; linking; target platforms.
- **Dataflow specifics.** Whether cycles are permitted; laziness rules.
