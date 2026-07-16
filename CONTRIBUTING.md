# Contributing

Contributions are always welcome, remember to test all features you contribute.

## Local Dev env

```shell
git clone git@github.com:xNaCly/sqleibniz.git
cargo run example/*
```

To run all example files with sqleibniz-specific diagnostics disabled:

```shell
cargo run -- \
  --ignore-config \
  -Dfile/no-statements \
  -Dfile/no-content \
  -Dsqleibniz/unimplemented \
  -Dsqleibniz/bad-instruction \
  example/*.sql
```

## Debugging the parser

Run sqleibniz via cargo with `--features trace` to enable the log of each
`Parser.<stmt_type>_stmt` function as well as the resulting ast nodes. This
allows for a deeper insight for deadlocks etc.

```sql
EXPLAIN VACUUM;
EXPLAIN QUERY PLAN VACUUM my_big_schema INTO 'repacked.db';
```

For instance, run the checked-in trace example:

```text
cargo run --features trace -- -i example/trace_example.sql
```

That prints the parser callstack and resulting AST before the normal diagnostic
summary:

```text
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
- Explain(...) [child=Vacuum { ... }]
- Explain(...) [child=Vacuum { ... }]
took: [...]
=============================== Summary ================================
[+] example/trace_example.sql:
    0 Diagnostic(s) detected
    0 Diagnostic(s) ignored

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
