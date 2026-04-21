# vscode-badger

Syntax highlighting for the [Badger](../../spec/lang.md) programming language.

## Features

- Keywords: `fn`, `enum`, `struct`, `interface`, `implement`, `extends`, `type`, `extern`, `match`, `pub`, `as`
- `@`-forms: `@import`, `@recurse`, `@intrinsic`
- `!`-forms: `!effect`, `!depend`
- Primitive types: `i8`–`i128`, `u8`–`u128`, `isize`, `usize`, `f32`, `f64`, `bool`, `char`
- Stdlib types: `String`, `Option`, `Result`, `Void`, `Init`, `Main`
- Doc comments (`///`) vs. line comments (`//`)
- String, raw-string, char, and numeric literals (with type suffixes)
- Operators: `=>`, `->`, `|>`, `::`, ternary `?:`, comparison, logical, arithmetic
- Identifier heuristics: PascalCase → type, lowercase before `(` → function call, lowercase before `= fn` → function definition

## Install locally

Symlink the extension into your VS Code extensions folder:

```sh
ln -s "$(pwd)/packages/vscode-badger" ~/.vscode/extensions/vscode-badger-0.0.1
```

Then reload VS Code. Any `.badger` file should highlight.

## Package as a `.vsix`

```sh
npm install -g @vscode/vsce
cd packages/vscode-badger
vsce package
```

## Scope reference

The grammar uses standard TextMate scopes so it works with any VS Code color theme. See
`syntaxes/badger.tmLanguage.json` for the full pattern list.
