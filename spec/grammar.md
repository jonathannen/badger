# Badger Grammar

Status: rough draft, tracks [lang.md](lang.md).

Notation: **W3C EBNF**.

- `A ::= B` — production.
- `A B` — sequence.
- `A | B` — alternation. In the canonical parser, `|` is **ordered choice** (first match wins) to resolve ambiguity.
- `A?` — optional.
- `A*` — zero or more.
- `A+` — one or more.
- `(A B)` — grouping.
- `"foo"` — literal terminal.
- `/* ... */` — grammar-level comment.

Railroad diagrams are not included; the grammar will be rendered separately once it stabilizes.

---

## Lexical

```ebnf
Whitespace    ::= (" " | "\t" | "\r" | "\n")+
LineComment   ::= "//" (~[\r\n])*
Comment       ::= LineComment      /* block comments: open */

Identifier    ::= IdentStart IdentCont*
IdentStart    ::= [A-Za-z_]
IdentCont     ::= [A-Za-z0-9_]

IntLiteral    ::= DecDigits IntSuffix?
DecDigits     ::= [0-9] ([0-9_]* [0-9])?
IntSuffix     ::= ("i" | "u") ("8" | "16" | "32" | "64" | "128")

FloatLiteral  ::= DecDigits "." DecDigits FloatSuffix?
FloatSuffix   ::= "f" ("32" | "64")

CharLiteral   ::= "'" (CharEscape | ~['\\]) "'"
StringLiteral ::= "\"" (CharEscape | ~["\\])* "\""
CharEscape    ::= "\\" ("n" | "r" | "t" | "\\" | "\"" | "'" | "0")

BoolLiteral   ::= "true" | "false"
UnitLiteral   ::= "(" ")"

Keyword       ::= "enum" | "struct" | "interface" | "implement" | "as"
                | "fn" | "match" | "type" | "pub" | "extern"
                | "Self"
```

## Types

```ebnf
Type          ::= TypeAtom ("!" EffectSpec)?

TypeAtom      ::= UnitType
                | TupleType
                | ArrayType
                | SliceType
                | FnType
                | NamedType

UnitType      ::= "(" ")"
TupleType     ::= "(" Type ("," Type)+ ","? ")"
ArrayType     ::= "[" IntLiteral "]" Type
SliceType     ::= "[" "]" Type
FnType        ::= "fn" "(" TypeList? ")" "->" Type
NamedType     ::= Identifier GenericArgs?

GenericArgs   ::= "<" Type ("," Type)* ">"
TypeList      ::= Type ("," Type)*

EffectSpec    ::= "effect" "(" Identifier ("," Identifier)* ")"
```

## Declarations

```ebnf
Module        ::= Item*

Item          ::= Visibility? (
                    TypeAlias
                  | EnumDecl
                  | StructDecl
                  | InterfaceDecl
                  | ImplementBlock
                  | ExternDecl
                  | Binding
                  )

Visibility    ::= "pub"

TypeAlias     ::= "type" Identifier GenericParams? "=" Type ";"

EnumDecl      ::= "enum" Identifier GenericParams? "{" EnumVariant+ "}"
EnumVariant   ::= Identifier ("(" TypeList ")")? ";"

StructDecl    ::= "struct" Identifier GenericParams? "{" StructField* "}"
StructField   ::= Identifier ":" Type ";"

InterfaceDecl ::= "interface" Identifier GenericParams? "{" InterfaceMember+ "}"
InterfaceMember ::= Identifier "(" ParamList? ")" "->" Type ";"

ImplementBlock ::= "implement" Type ("as" Type)? "{" Method* "}"
Method        ::= Identifier GenericParams? "(" ParamList? ")" "->" Type Block

ExternDecl    ::= "extern" Identifier "=" FnType Attribute* ";"
Attribute     ::= "@" Identifier ("(" AttrArg ("," AttrArg)* ")")?
AttrArg       ::= StringLiteral | IntLiteral | Identifier

GenericParams ::= "<" Identifier ("," Identifier)* ">"
ParamList     ::= Param ("," Param)*
Param         ::= Identifier ":" Type
```

## Bindings and Expressions

```ebnf
Binding       ::= Identifier GenericParams? "=" Expr ";"

Expr          ::= Pipeline

Pipeline      ::= Ternary ("|>" Ternary)*
Ternary       ::= Or ("?" Expr ":" Expr)?
Or            ::= And ("||" And)*
And           ::= Cmp ("&&" Cmp)*
Cmp           ::= Add (CmpOp Add)*
CmpOp         ::= "==" | "!=" | "<" | "<=" | ">" | ">="
Add           ::= Mul (("+" | "-") Mul)*
Mul           ::= Unary (("*" | "/" | "%") Unary)*
Unary         ::= ("-" | "!")? Postfix

Postfix       ::= Primary PostfixTail*
PostfixTail   ::= "." Identifier                        /* field / method name */
                | "(" ArgList? ")"                      /* call */
                | "[" Expr "]"                          /* index */
                | "!depend" "(" ArgList ")"             /* explicit dependency */
                | "!effect" "(" Identifier ")"          /* effect annotation at call site */

ArgList       ::= Expr ("," Expr)*

Primary       ::= Literal
                | UnitLiteral
                | Block
                | Tuple
                | ArrayLit
                | Lambda
                | Match
                | Recurse
                | Path

Literal       ::= IntLiteral | FloatLiteral | BoolLiteral
                | CharLiteral | StringLiteral

Block         ::= "{" Stmt* Expr? "}"
Stmt          ::= Binding | (Expr ";")

Tuple         ::= "(" Expr "," (Expr ("," Expr)*)? ","? ")"
ArrayLit      ::= "[" (Expr ("," Expr)*)? "]"

Lambda        ::= "fn" "(" ParamList? ")" "->" Type LambdaBody
LambdaBody    ::= Block
                | "=>" Expr

Match         ::= "match" Expr "{" MatchArm+ "}"
MatchArm      ::= Pattern ":" Expr ";"

Recurse       ::= "@recurse" "(" ArgList? ")"

Path          ::= Identifier ("::" Identifier)*
```

## Patterns

```ebnf
Pattern       ::= PatternAtom

PatternAtom   ::= "_"                                      /* wildcard */
                | Literal                                  /* literal pattern */
                | Identifier                               /* binding */
                | Path ("(" PatternList ")")?              /* variant / constructor */
                | "(" PatternList ")"                      /* tuple pattern */

PatternList   ::= Pattern ("," Pattern)*
```

---

## Open / Unfinished Productions

- **Block comments** — `/* ... */` form.
- **String interpolation** — not yet decided.
- **Struct literals** — e.g. `Socket { bytes_sent: 0 }`; pending SPEC decision.
- **Arm guards** — `Ok(x) if x > 0: ...`; pending SPEC decision.
- **Or-patterns, `@`-bindings** — pending SPEC decision.
- **Operator precedence / associativity** — the current table is provisional; full precedence decisions are tracked in [lang.md §9](lang.md).
- **Generic bounds** — syntax for `T: Interface` not yet decided.
- **Attribute placement** — currently only shown on `extern`; broader placement rules TBD.
- **Trailing `;` rules at the top level** — consistency between `Binding` and `Item` terminators.
