# sqleibniz

Static analysis and LSP for SQL in Rust. Check for valid syntax, semantics and
perform dynamic analysis.

> [!WARNING]  
> Sqleibniz is in early stages of development, please keep this in mind before
> creating issues. Contributions are always welcome 💗

## Features

Sqleibniz is a command line tool to analyse sql statements by checking for their static and
dynamic correctness. See below for a list of currently implemented features.

### Supported features

- [ ] static analysis (syntax and semantic analysis)
  - [x] syntax analysis - sqleibniz aims to implement the syntax [sqlite understands](https://www.sqlite.org/lang.html)
  - [ ] warn for sqlites [quirks](https://www.sqlite.org/quirks.html)
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
  - [ ] snippets
  - [ ] intelligent completions
- [ ] lua scripting
  - [x] configure sqleibniz with lua
  - [x] scripting to hook into node analysis for custom diagnostics
  - [ ] execute hooks when encountering the defined node while analysing

### Supported Sql statements

| `sqlite` specification                                                     | syntax analysis | semantic analysis | Example                                                   |
| -------------------------------------------------------------------------- | --------------- | ----------------- | --------------------------------------------------------- |
| [`explain-stmt`](https://www.sqlite.org/lang_explain.html)                 | ✅              | ❌                | `EXPLAIN QUERY PLAN;`                                     |
| [`alter-table-stmt`](https://www.sqlite.org/lang_altertable.html)          | ✅              | ❌                | `ALTER TABLE schema.table_name ADD new_column_name TEXT;` |
| [`analyze-stmt`](https://www.sqlite.org/lang_analyze.html)                 | ✅              | ❌                | `ANALYZE my_table;`                                       |
| [`attach-stmt`](https://www.sqlite.org/lang_attach.html)                   | ✅              | ❌                | `ATTACH DATABASE 'users.db' AS users;`                    |
| [`begin-stmt`](https://www.sqlite.org/lang_transaction.html)               | ✅              | ❌                | `BEGIN DEFERRED TRANSACTION;`                             |
| [`commit-stmt`](https://www.sqlite.org/lang_transaction.html)              | ✅              | ❌                | `END TRANSACTION;`                                        |
| [`create-index-stmt`](https://www.sqlite.org/lang_createindex.html)        | ❌              | ❌                |                                                           |
| [`create-table-stmt`](https://www.sqlite.org/lang_createtable.html)        | ❌              | ❌                |                                                           |
| [`create-trigger-stmt`](https://www.sqlite.org/lang_createtrigger.html)    | ❌              | ❌                |                                                           |
| [`create-view-stmt`](https://www.sqlite.org/lang_createview.html)          | ❌              | ❌                |                                                           |
| [`create-virtual-table-stmt`](https://www.sqlite.org/lang_createvtab.html) | ❌              | ❌                |                                                           |
| [`delete-stmt`](https://www.sqlite.org/lang_delete.html)                   | ❌              | ❌                |                                                           |
| [`detach-stmt`](https://www.sqlite.org/lang_detach.html)                   | ✅              | ❌                | `DETACH DATABASE my_database`                             |
| [`drop-index-stmt`](https://www.sqlite.org/lang_dropindex.html)            | ✅              | ❌                | `DROP INDEX my_index;`                                    |
| [`drop-table-stmt`](https://www.sqlite.org/lang_droptable.html)            | ✅              | ❌                | `DROP TABLE my_table;`                                    |
| [`drop-trigger-stmt`](https://www.sqlite.org/lang_droptrigger.html)        | ✅              | ❌                | `DROP TRIGGER my_trigger;`                                |
| [`drop-view-stmt`](https://www.sqlite.org/lang_dropview.html)              | ✅              | ❌                | `DROP VIEW my_view;`                                      |
| [`insert-stmt`](https://www.sqlite.org/lang_insert.html)                   | ❌              | ❌                |                                                           |
| [`pragma-stmt`](https://www.sqlite.org/pragma.html)                        | ✅              | ❌                | `PRAGMA schema.optimize(0xfffe);`                         |
| [`reindex-stmt`](https://www.sqlite.org/lang_reindex.html)                 | ✅              | ❌                | `REINDEX my_schema.my_table`                              |
| [`release-stmt`](https://www.sqlite.org/lang_savepoint.html)               | ✅              | ❌                | `RELEASE SAVEPOINT latest_savepoint`                      |
| [`rollback-stmt`](https://www.sqlite.org/lang_transaction.html)            | ✅              | ❌                | `ROLLBACK TO latest_savepoint;`                           |
| [`savepoint-stmt`](https://www.sqlite.org/lang_savepoint.html)             | ✅              | ❌                | `SAVEPOINT latest_savepoint`                              |
| [`select-stmt`](https://www.sqlite.org/lang_select.html)                   | ❌              | ❌                |                                                           |
| [`update-stmt`](https://www.sqlite.org/lang_update.html)                   | ❌              | ❌                |                                                           |
| [`vacuum-stmt`](https://www.sqlite.org/lang_vacuum.html)                   | ✅              | ❌                | `VACUUM INTO 'repacked.db'`                               |

## Installation

### cargo

```
cargo install --git https://github.com/xnacly/sqleibniz
```

#### from source

```shell
git clone https://github.com/xnacly/sqleibniz
cargo install --path .
```

### via `make`

> this builds the project with cargo and moves the resulting binary to
> `/usr/bin/`.

```shell
git clone https://github.com/xnacly/sqleibniz
make
```

Uninstall via:

```shell
make uninstall
```

## Command line interface usage

```text
Static analysis and LSP for SQL in Rust

Usage: sqleibniz [OPTIONS] [PATHS]...

Arguments:
  [PATHS]...
          files to analyse

Options:
  -i, --ignore-config
          instruct sqleibniz to ignore the configuration, if specified

  -c, --config <CONFIG>
          path to the configuration

          [default: leibniz.lua]

  -s, --silent
          disable stdout/stderr output

  -D <DISABLE>
          disable diagnostics by their rules, all are enabled by default - this may change in the future

          Possible values:
          - no-content:                Source file is empty
          - no-statements:             Source file is not empty but holds no statements
          - unimplemented:             Source file contains constructs sqleibniz does not yet understand
          - unknown-keyword:           Source file contains an unknown keyword
          - bad-sqleibniz-instruction: Source file contains invalid sqleibniz instruction
          - sqlite-unsupported:        Source file uses sql features sqlite does not support
          - quirk:                     Sqlite or SQL quirk: https://www.sqlite.org/quirks.html
          - unterminated-string:       Source file contains an unterminated string
          - unknown-character:         The source file contains an unknown character
          - invalid-numeric-literal:   The source file contains an invalid numeric literal, either overflow or incorrect syntax
          - invalid-blob:              The source file contains an invalid blob literal, either bad hex data (a-f,A-F,0-9) or incorrect syntax
          - syntax:                    The source file contains a structure with incorrect syntax
          - semicolon:                 The source file is missing a semicolon

      --ast-json
          dump the abstract syntax tree as pretty printed json

      --ast
          dump the abstract syntax tree as rusts pretty printed debugging

      --lsp
          invoke sqleibniz as a language server

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print versio
```

### Configuration

Sqleibniz can be configured via a `leibniz.lua` file, this file has to be
accessible to sqleibniz by existing at the path sqleibniz is invoked at.
Consult [src/rules.rs](./src/rules.rs) for configuration documentation and
[leibniz.lua](./leibniz.lua) for said example:

````lua
-- this is an example configuration, consult: https://www.lua.org/manual/5.4/
-- or https://learnxinyminutes.com/docs/lua/ for syntax help and
-- src/rules.rs::Config for all available options
leibniz = {
    disabled_rules = {
        -- ignore sqleibniz specific diagnostics:
        "NoContent",               -- source file is empty
        "NoStatements",            -- source file contains no statements
        "Unimplemented",           -- construct is not implemented yet
        "BadSqleibnizInstruction", -- source file contains a bad sqleibniz instruction

        -- ignore sqlite specific diagnostics:

        -- "SqliteUnsupported", -- Source file uses sql features sqlite does not support
        -- "Quirk", -- Sqlite or SQL quirk: https://www.sqlite.org/quirks.html
        -- "UnknownKeyword", -- an unknown keyword was encountered
        -- "UnterminatedString", -- a not closed string was found
        -- "UnknownCharacter", -- an unknown character was found
        -- "InvalidNumericLiteral", -- an invalid numeric literal was found
        -- "InvalidBlob", -- an invalid blob literal was found (either bad hex data or incorrect syntax)
        -- "Syntax", -- a structure with incorrect syntax was found
        -- "Semicolon", -- a semicolon is missing
    },
    -- sqleibniz allows for writing custom rules with lua
    hooks = {
        {
            -- summarises the hooks content
            name = "idents should be lowercase",
            -- instructs sqleibniz which node to execute the `hook` for
            node = "literal",
            -- sqleibniz calls the hook function once it encounters a node name
            -- matching the hook.node content
            --
            -- The `node` argument holds the following fields:
            --
            --```
            --    node: {
            --     kind: string,
            --     content: string,
            --     children: node[],
            --    }
            --```
            --
            hook = function(node)
                if node.kind == "ident" then
                    if string.match(node.content, "%u") then
                        -- returing an error passes the diagnostic to sqleibniz,
                        -- thus a pretty message with the name of the hook, the
                        -- node it occurs and the message passed to error() is
                        -- generated
                        error("All idents should be lowercase")
                    end
                end
            end
        },
        {
            name = "idents shouldn't be longer than 12 characters",
            node = "literal",
            hook = function(node)
                local max_size = 12
                if node.kind == "ident" then
                    if string.len(node.content) >= max_size then
                        error("idents shouldn't be longer than " .. max_size .. " characters")
                    end
                end
            end
        }
    }
}
````

### sqleibniz instructions

A sqleibniz instrution is prefixed with `@sqleibniz::` and written inside of a
sql single line comment.

#### `expect`

In a similar fashion to ignoring diagnostics via the configuration in
`leibniz.toml`, sqleibniz allows the user to expect diagnostics in the source
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
warn: Ignoring the following diagnostics, as specified:
 -> NoContent
 -> NoStatements
 -> Unimplemented
 -> BadSqleibnizInstruction
======================== example/sqleibniz.sql =========================
error[Syntax]: Unexpected Literal
 -> /home/teo/programming/sqleibniz/example/sqleibniz.sql:13:20
 11 | -- will cause a diagnostic
 12 | -- incorrect, because EXPLAIN wants a sql stmt, not a literal
 13 | EXPLAIN QUERY PLAN 25;
    |                    ~~ error occurs here.
    |
    ~ note: Literal Number(25.0) can not start a statement
    ~ docs: https://www.sqlite.org/syntax/sql-stmt.html
 * Syntax: The source file contains a structure with incorrect syntax
=============================== Summary ================================
[-] example/sqleibniz.sql:
    1 Error(s) detected
    0 Error(s) ignored

=> 0/1 Files verified successfully, 1 verification failed.
```

Or syntax highlighted via [`highlight::highlight`](https://github.com/xNaCly/sqleibniz/blob/master/src/highlight/mod.rs#L50) for the terminal:

![rendered by the terminal](https://github.com/user-attachments/assets/dd349d59-1107-4421-82e4-f95549b43e85)

`@sqleibniz::expect` is implemented by inserting a token with the type
`Type::InstructionExpect`. The parser encounters this token and consumes all
token until a token with the type `Type::Semicolon` is found. Thus sqleibniz is
skipping the analysis of the statement directly after the sqleibniz
instruction. A statement is terminated via `;`. `@sqleibniz::expect` therefore
supports ignoring diagnostics for statements spanning either a single line or
multiple lines.

## Language Server Protocol (lsp)

Sqleibniz has an LSP provider included, with in-editor diagnostics, hover info and other dx helpers.

### Setup in Neovim

> requires systemwide installation beforehand via `make install`

As simple as adding the following to the neovim lua config:

```lua
vim.lsp.config.sqleibniz = {
    cmd = { '/usr/bin/sqleibniz', '--lsp' },
    filetypes = { "sql" },
    root_markers = { "leibniz.lua" }
}
vim.lsp.enable('sqleibniz')
```

## Contribution

Contributions are always welcome <3, but remember to test all features you contribute.

### Local Dev env

```shell
git clone git@github.com:xNaCly/sqleibniz.git
cargo run example/*
```

### Debugging the parser

Run sqleibniz via cargo with `--features trace` to enable the log of each
`Parser.<stmt_type>_stmt` function as well as the resulting ast nodes. This
allows for a deeper insight for deadlocks etc.

```sql
EXPLAIN VACUUM;
EXPLAIN QUERY PLAN VACUUM my_big_schema INTO 'repacked.db';
```

For instance, parsing the above SQL results in the generation and printing of a
parser callstack and the resulting AST:

```text
sqleibniz master M :: cargo run --features trace -- -i test.sql
============================== CALLSTACK ===============================
↳ parse | Keyword(EXPLAIN)
 ↳ sql_stmt_list | Keyword(EXPLAIN)
  ↳ sql_stmt_prefix | Keyword(EXPLAIN)
   ↳ sql_stmt | Keyword(VACUUM)
    ↳ vacuum_stmt | Keyword(VACUUM)
   ↳ sql_stmt_prefix | Keyword(EXPLAIN)
    ↳ sql_stmt | Keyword(VACUUM)
     ↳ vacuum_stmt | Keyword(VACUUM)
================================= AST ==================================
- Explain(Keyword(EXPLAIN)) [child=Vacuum { t: Token { ttype: Keyword(VACUUM), start: 8, end: 14, line: 0 }, schema_name: None, filename: None }]
- Explain(Keyword(EXPLAIN)) [child=Vacuum { t: Token { ttype: Keyword(VACUUM), start: 19, end: 25, line: 1 }, schema_name: Some(Token { ttype: Ident("my_big_schema"), start: 26, end: 39, li
ne: 1 }), filename: Some(Token { ttype: String("repacked.db"), start: 45, end: 58, line: 1 }) }]
took: [120.166µs]
=============================== Summary ================================
[+] test.sql:
    0 Error(s) detected
    0 Error(s) ignored

=> 1/1 Files verified successfully, 0 verification failed.
```

There is also `--ast` and `--ast-json`, both enabling ast introspection:

```json
[
  {
    "child": {
      "filename": null,
      "schema_name": null,
      "type": "Vacuum"
    },
    "type": "Explain"
  },
  {
    "child": {
      "filename": {
        "String": "repacked.db"
      },
      "schema_name": {
        "Ident": "my_big_schema"
      },
      "type": "Vacuum"
    },
    "type": "Explain"
  }
]
```
