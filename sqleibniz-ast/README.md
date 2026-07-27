# sqleibniz-ast

Parse SQLite into tokens, an abstract syntax tree (AST), and diagnostics.

`sqleibniz-ast` is the standalone syntax layer of
[sqleibniz](https://github.com/xnacly/sqleibniz). It uses a handwritten lexer
and recursive-descent parser, follows SQLite's documented syntax, and recovers
from errors where possible. It is intended for tools that need to inspect SQL,
such as linters, editors, formatters, and source analyzers.

> [!WARNING]
> The crate is under active development. Its public API and supported SQLite
> grammar may change before 1.0.

## Installation

```toml
[dependencies]
sqleibniz-ast = "0.0.3"
```

## Parse SQL

`sqleibniz_ast::parse` is the usual entry point. It always returns the lexer output, parsed
statements, and any diagnostics, allowing tools to retain useful syntax even
while a source file is incomplete or invalid.

```rust
use sqleibniz_ast::parse;

let parsed = parse(
    "CREATE TABLE users (id INTEGER) STRICT; SELECT id FROM users;",
    "schema.sql",
);

assert!(parsed.is_ok());
assert_eq!(parsed.ast.len(), 2);
assert!(!parsed.tokens.is_empty());
```

`ParseResult` contains:

- `tokens`: the lexer output;
- `ast`: successfully parsed statements as `Box<dyn Node>`;
- `errors`: lexer and parser diagnostics.

Parsing is recoverable, so `ast` may be non-empty when `errors` is non-empty.
Report the diagnostics before accepting SQL as valid:

```rust
use sqleibniz_ast::parse;

let parsed = parse("SELECT FROM users;", "query.sql");

for diagnostic in &parsed.errors {
    eprintln!(
        "{}:{}: {}",
        diagnostic.file,
        diagnostic.location.line + 1,
        diagnostic.msg,
    );
}
```

## Serialize AST as JSON

Enable the `serde` feature and add a JSON serializer to your application:

```toml
[dependencies]
serde_json = "1"
sqleibniz-ast = { version = "0.0.3", features = ["serde"] }
```

AST nodes serialize as JSON objects with a `type` field. This makes the output
suited to debugging, snapshots, and integrations with tools outside Rust.

```rust
use sqleibniz_ast::parse;

let parsed = parse("SELECT id FROM users;", "query.sql");
assert!(parsed.is_ok());

let json = serde_json::to_string_pretty(&parsed.ast)?;
println!("{json}");
```

```json
[
  {
    "type": "Select",
    "quantifier": null,
    "columns": [
      {
        "Expr": {
          "expr": {
            "type": "Expr",
            "literal": null,
            "bind": null,
            "schema": null,
            "table": null,
            "column": "id",
            "function": null,
            "operator": null,
            "arguments": []
          },
          "alias": null
        }
      }
    ],
    "from": [
      {
        "Table": {
          "name": {
            "Table": "users"
          },
          "alias": null
        }
      }
    ],
    "where_expr": null,
    "group_by": [],
    "having": null,
    "order_by": [],
    "limit": null
  }
]
```

The serialized AST is a representation of the crate's public syntax model; it
is not a stable wire format before 1.0.

## Scope

The parser follows SQLite's documented syntax diagrams and shares the syntax
coverage of sqleibniz. See the parent project's
[supported statement matrix](https://github.com/xnacly/sqleibniz#supported-sql-statements)
for the current coverage.
