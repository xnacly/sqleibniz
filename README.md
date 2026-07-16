# sqleibniz

Static analysis and LSP for SQL in Rust. Check for valid syntax, semantics and
perform dynamic analysis.

> [!WARNING]  
> Sqleibniz is in development, please keep this in mind before
> creating issues.

## Features

Sqleibniz is a command line tool to analyse sql statements by checking for their static and
dynamic correctness. See below for a list of currently implemented features.

### Supported features

- [ ] static analysis (syntax and semantic analysis)
  - [x] syntax analysis - sqleibniz aims to implement the syntax [sqlite understands](https://www.sqlite.org/lang.html)
  - [x] warn for sqlites [quirks](https://www.sqlite.org/quirks.html)
  - [ ] do the used tables exist / were they created beforehand
  - [ ] do the used columns exist / were they created beforehand
  - [ ] do the used functions exist / were they created beforehand
  - [ ] are all used types compatible
- [ ] dynamic analysis (runtime analysis via embedded sqlite)
  - [ ] assertions via `@sqleibniz::assert`
  - [ ] were all tables and their columns created correctly (with correct storage classes)
  - [ ] were all stmts executed successfully
- [ ] pretty errors
  - [x] faulty code display with line numbers
  - [x] link to sqlite documentation for each diagnostic
  - [x] ability to omit specific errors depending on their group (Rule)
  - [x] highlighting the error in the faulty code snippet
  - [x] explanation why the specific error was ommitted based on its Rule
  - [x] syntax highlighting in terminal errors
  - [ ] possible fix suggestions
  - [x] suggestions for unknown and possible misspelled keywords
- [ ] language server protocol
  - [x] diagnostics for full sqleibniz analysis
  - [x] apply disabled diagnostics from `leibniz.lua`
  - [ ] snippets
  - [ ] intelligent completions
- [ ] lua scripting
  - [x] configure sqleibniz with lua
  - [x] scripting to hook into node analysis for custom diagnostics
  - [x] execute hooks when encountering the defined node while analysing

### Supported Sql statements

| `sqlite` specification                                                     | syntax analysis | semantic analysis | Example                                                   |
| -------------------------------------------------------------------------- | --------------- | ----------------- | --------------------------------------------------------- |
| [`explain-stmt`](https://www.sqlite.org/lang_explain.html)                 | ✅              | ❌                | `EXPLAIN QUERY PLAN VACUUM;`                              |
| [`alter-table-stmt`](https://www.sqlite.org/lang_altertable.html)          | ✅              | ✅                | `ALTER TABLE schema.table_name ADD new_column_name TEXT;` |
| [`analyze-stmt`](https://www.sqlite.org/lang_analyze.html)                 | ✅              | ❌                | `ANALYZE my_table;`                                       |
| [`attach-stmt`](https://www.sqlite.org/lang_attach.html)                   | ✅              | ❌                | `ATTACH DATABASE 'users.db' AS users;`                    |
| [`begin-stmt`](https://www.sqlite.org/lang_transaction.html)               | ✅              | ❌                | `BEGIN DEFERRED TRANSACTION;`                             |
| [`commit-stmt`](https://www.sqlite.org/lang_transaction.html)              | ✅              | ❌                | `END TRANSACTION;`                                        |
| [`create-index-stmt`](https://www.sqlite.org/lang_createindex.html)        | ✅              | ❌                | `CREATE INDEX idx_users_id ON users (id);`                |
| [`create-table-stmt`](https://www.sqlite.org/lang_createtable.html)        | ✅              | ✅                | `CREATE TABLE users (id INTEGER) STRICT;`                 |
| [`create-trigger-stmt`](https://www.sqlite.org/lang_createtrigger.html)    | ✅              | ❌                | `CREATE TRIGGER user_ai AFTER INSERT ON users BEGIN SELECT 1; END;` |
| [`create-view-stmt`](https://www.sqlite.org/lang_createview.html)          | ✅              | ✅                | `CREATE VIEW active_users AS SELECT id FROM users;`       |
| [`create-virtual-table-stmt`](https://www.sqlite.org/lang_createvtab.html) | ✅              | ✅                | `CREATE VIRTUAL TABLE docs USING fts5(content);`          |
| [`delete-stmt`](https://www.sqlite.org/lang_delete.html)                   | ✅              | ✅                | `DELETE FROM users WHERE id = 1 RETURNING *;`             |
| [`detach-stmt`](https://www.sqlite.org/lang_detach.html)                   | ✅              | ❌                | `DETACH DATABASE my_database;`                            |
| [`drop-index-stmt`](https://www.sqlite.org/lang_dropindex.html)            | ✅              | ❌                | `DROP INDEX my_index;`                                    |
| [`drop-table-stmt`](https://www.sqlite.org/lang_droptable.html)            | ✅              | ❌                | `DROP TABLE my_table;`                                    |
| [`drop-trigger-stmt`](https://www.sqlite.org/lang_droptrigger.html)        | ✅              | ❌                | `DROP TRIGGER my_trigger;`                                |
| [`drop-view-stmt`](https://www.sqlite.org/lang_dropview.html)              | ✅              | ❌                | `DROP VIEW my_view;`                                      |
| [`insert-stmt`](https://www.sqlite.org/lang_insert.html)                   | ✅              | ✅                | `INSERT INTO users (id) VALUES (1) RETURNING *;`          |
| [`pragma-stmt`](https://www.sqlite.org/pragma.html)                        | ✅              | ✅                | `PRAGMA schema.optimize(0xfffe);`                         |
| [`reindex-stmt`](https://www.sqlite.org/lang_reindex.html)                 | ✅              | ❌                | `REINDEX my_schema.my_table;`                             |
| [`release-stmt`](https://www.sqlite.org/lang_savepoint.html)               | ✅              | ❌                | `RELEASE SAVEPOINT latest_savepoint;`                     |
| [`rollback-stmt`](https://www.sqlite.org/lang_transaction.html)            | ✅              | ❌                | `ROLLBACK TO latest_savepoint;`                           |
| [`savepoint-stmt`](https://www.sqlite.org/lang_savepoint.html)             | ✅              | ❌                | `SAVEPOINT latest_savepoint;`                             |
| [`select-stmt`](https://www.sqlite.org/lang_select.html)                   | ✅              | ✅                | `SELECT id FROM users WHERE active = true;`               |
| [`update-stmt`](https://www.sqlite.org/lang_update.html)                   | ✅              | ✅                | `UPDATE users SET name = 'Ada' WHERE id = 1;`             |
| [`vacuum-stmt`](https://www.sqlite.org/lang_vacuum.html)                   | ✅              | ❌                | `VACUUM INTO 'repacked.db';`                              |

See [example/stmt.sql](./example/stmt.sql) for the executable statement support
matrix used by the examples.

## Installation

### cargo

```shell
cargo install sqleibniz
```

#### from source

```shell
git clone https://github.com/xnacly/sqleibniz
cd sqleibniz
cargo install --path sqleibniz
```

Uninstall via:

```shell
cargo uninstall sqleibniz
```

## Command line interface usage

```shell
sqleibniz [OPTIONS] [PATHS]...
```

Run `sqleibniz --help` for the current CLI reference, including all flags and
diagnostic rules accepted by `-D`.

### Configuration

Sqleibniz can be configured via a `leibniz.lua` file. By default, the CLI reads
`./leibniz.lua`; pass `--config <path>` to use another file or `--ignore-config`
to skip configuration entirely.

The language server reads `disabled_rules` from the workspace `leibniz.lua`.
Lua hooks are only executed in LSP diagnostics when `--lsp-enable-hooks` is
passed explicitly.

See [leibniz.lua](./leibniz.lua) for the canonical example configuration,
including disabled rules and Lua hook examples. That file includes the current
SQLite-specific rules such as `sqlite/unknown-pragma`.

Each hook runs for contexts matching every field in `match`. Token hooks can
use `node.content` for the token text and report a diagnostic by calling
`sqleibniz.diagnostic(node, "message")`.

### sqleibniz instructions

A sqleibniz instruction is prefixed with `@sqleibniz::` and written inside of a
sql single line comment.

#### `expect`

In a similar fashion to ignoring diagnostics via the configuration in
`leibniz.lua`, sqleibniz allows the user to expect diagnostics in the source
file and omit them on a statement by statement basis. To do so, a comment
containing a sqleibniz instruction has to be issued:

```sql
-- will not cause a diagnostic
-- @sqleibniz::expect <explanation for instruction usage here>
-- incorrect, because EXPLAIN wants a sql stmt
EXPLAIN 25;

-- will not cause a diagnostic
-- @sqleibniz::expect <explanation for instruction usage here>
-- incorrect, because 'unknown_table' does not exist
SELECT * FROM unknown_table;

-- will cause a diagnostic
-- incorrect, because EXPLAIN wants a sql stmt, not a literal
EXPLAIN QUERY PLAN 25;
```

Passing the above file to `sqleibniz`:

```text
======================== example/sqleibniz.sql =========================
sql/syntax: Unexpected Literal
 -> /home/teo/programming/sqleibniz/example/sqleibniz.sql:11:20
 09 |
 10 | -- will not cause a diagnostic
 11 | EXPLAIN QUERY PLAN 25;
    |                    ~~ error occurs here.
    |
    ~ note: Literal Number(25.0) can not start a statement
    ~ docs: https://www.sqlite.org/syntax/sql-stmt.html
 * sql/syntax: The source file contains a structure with incorrect syntax
=============================== Summary ================================
[-] example/sqleibniz.sql:
    1 Diagnostic(s) detected
    0 Diagnostic(s) ignored

=> 0/1 Files verified successfully, 1 verification failed.
```

`@sqleibniz::expect` is implemented by inserting a token with the type
`Type::InstructionExpect`. The parser encounters this token and consumes all
token until a token with the type `Type::Semicolon` is found. Thus sqleibniz is
skipping the analysis of the statement directly after the sqleibniz
instruction. A statement is terminated via `;`. `@sqleibniz::expect` therefore
supports ignoring diagnostics for statements spanning either a single line or
multiple lines.

## Language Server Protocol (lsp)

Sqleibniz has an LSP provider included, with in-editor diagnostics, hover info and other dx helpers.
The language server loads `leibniz.lua` from the first workspace folder, then
from the LSP `rootUri`, then from the current working directory. Only
`disabled_rules` are applied in LSP mode by default. Lua hooks are ignored
unless the server is started with `--lsp-enable-hooks`.

### Setup in Neovim

> requires installation beforehand via `cargo install`

As simple as adding the following to the neovim lua config:

```lua
vim.lsp.config.sqleibniz = {
    cmd = { 'sqleibniz', '--lsp' },
    filetypes = { "sql" },
    root_markers = { "leibniz.lua" }
}
vim.lsp.enable('sqleibniz')
```
