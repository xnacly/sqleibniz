# sqleibniz-ast

SQLite syntax primitives, lexer, parser, and abstract syntax tree.

`sqleibniz-ast` is the syntax layer behind
[sqleibniz](https://github.com/xnacly/sqleibniz). It parses the SQLite syntax
that sqleibniz understands and returns the tokens, AST, and recoverable
diagnostics together. It does so via handwritten tokenisation and recursive
descent parsing.

> [!WARNING]
> This crate is under active development. Its public API and supported SQLite
> grammar may change before 1.0.

## Usage

Add the crate to your project:

```toml
[dependencies]
sqleibniz-ast = "0.0.1"
```

Use [`parse`] for the usual one-step workflow:

```rust
use sqleibniz_ast::parse;

let parsed = parse(
    "CREATE TABLE users (id INTEGER) STRICT; SELECT id FROM users;",
    "schema.sql",
);

if parsed.is_ok() {
    assert_eq!(parsed.ast.len(), 2);
} else {
    for diagnostic in parsed.errors {
        eprintln!("{}:{}: {}", diagnostic.file, diagnostic.location.line + 1, diagnostic.msg);
    }
}
```

`ParseResult` contains:

- `tokens`: the token stream produced by the lexer;
- `ast`: successfully parsed statements as `Box<dyn Node>`;
- `errors`: lexer and parser diagnostics, including a source location, rule,
  note, and SQLite documentation link where available.

The parser recovers where it can. An AST can therefore be non-empty even when
`errors` is non-empty—useful for editors and other tools that must work with
incomplete SQL.

Enable the optional `serde` feature to serialize AST nodes directly with
Serde:

```toml
sqleibniz-ast = { version = "0.0.1", features = ["serde"] }
```

For lower-level control, use `Lexer::new(...).run()` and then
`Parser::new(tokens, name).parse()` directly. AST node definitions are exposed
from `sqleibniz_ast::parser::nodes`.

## Scope

The crate follows SQLite's documented syntax diagrams and shares the same
statement coverage as sqleibniz's syntax analysis. See the parent project's
[supported statement matrix](https://github.com/xnacly/sqleibniz#supported-sql-statements)
for the current coverage.
