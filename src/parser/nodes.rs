use crate::parser::debug::FieldSerializable;
use crate::types::{Keyword, Token, Type, storage::SqliteStorageClass};

/// Generates a Node from the given input:
///
///```
///node!(
///    Literal,
///    "holds all literal types, such as strings, numbers, etc.",
///);
///#[derive(Debug)]
///#[doc = "holds all literal types, such as strings, numbers, etc."]
///pub struct Literal {
///    #[doc = r" predefined for all structures defined with the node! macro, holds the token of the ast node"]
///    pub t:Token,pub children:Option<Vec<Box<dyn Node>>>,
///}
///impl Node for Literal {
///    fn token(&self) ->  &Token {
///        &self.t
///    }
///    #[cfg(feature = "trace")]
///    fn display(&self,indent:usize){
///        print!("{}- {}"," ".repeat(indent),self.name());
///        println!();
///        if let Some(children) =  &self.children {
///            for child in children {
///                child.display(indent+1)
///            }
///        }
///    }
///    fn name(&self) ->  &str {
///        stringify!(Literal)
///    }
///}
///```
macro_rules! node {
    ($node_name:ident,$documentation:literal,$($field_name:ident:$field_type:ty),*) => {
        #[derive(Debug)]
        #[doc = $documentation]
        pub struct $node_name {
            /// predefined for all structures defined with the node! macro, holds the token of the ast node
            pub t: Token,
            $(
                pub $field_name: $field_type,
            )*
        }

        impl Node for $node_name {
            fn token(&self) -> &Token {
                &self.t
            }

            #[cfg(feature = "trace")]
            fn display(&self, indent: usize) {
                print!("{}- {}({:?})", " ".repeat(indent), self.name(), self.t.ttype);
                $(
                    print!(" [{}={:?}] ", stringify!($field_name), self.$field_name);
                )*
                println!();
            }

            fn name(&self) -> &str {
                stringify!($node_name)
            }

            fn as_serializable(&self) -> serde_json::Value {
                let mut map = serde_json::Map::new();
                map.insert("type".to_string(), serde_json::Value::String(stringify!($node_name).to_string()));

                $(
                    map.insert(stringify!($field_name).to_string(), self.$field_name.field_as_serializable());
                )*

                serde_json::Value::Object(map)
            }

            fn doc(&self) -> &str {
                $documentation
            }
        }

        impl $node_name {
            // #[cfg(test)]
            pub fn new($($field_name: $field_type,)*) -> Self {
                Self {
                    // Type::InstructionExpect is always used in tests
                    t: Token::new(Type::InstructionExpect),
                    $($field_name,)*
                }
            }
        }

        impl FieldSerializable for $node_name {
            fn field_as_serializable(&self) -> serde_json::Value {
                self.as_serializable()
            }
        }
    };
}

pub trait Node: std::fmt::Debug {
    fn token(&self) -> &Token;
    #[cfg(feature = "trace")]
    fn display(&self, indent: usize);
    fn name(&self) -> &str;
    /// serializes self as json
    fn as_serializable(&self) -> serde_json::Value;
    /// returns the documentation url for sefl
    fn doc(&self) -> &str;

    // TODO: every node should analyse its own contents after the ast was build, to do so the Node
    // trait should enforce a analyse(&self, ctx &types::ctx::Context) -> Vec<Error> function.
}

node!(
    Literal,
    r"Literal value, see: https://www.sqlite.org/lang_expr.html#literal_values_constants_

A literal value represents a constant. Literal values may be integers, floating point numbers, strings, BLOBs, or NULLs.",
);

node!(
    BindParameter,
    r"Bind parameter, see https://www.sqlite.org/lang_expr.html#parameters

A parameter specifies a placeholder in the expression for a value that is filled in at runtime.
These can take several forms:

- `?NNN`: A question mark followed by a number NNN holds a spot for the NNN-th parameter.
- `?`: A question mark that is not followed by a number creates a parameter with a number one
  greater than the largest parameter number already assigned. (Discouraged - in sqleibniz, this format
  produces an Error)
- `:AAAA`: A colon followed by an identifier name holds a spot for a named parameter with the name
  `:AAAA`
- `@AAAA`: An 'at' sign works exactly like a colon, except that the name of the parameter created
  is @AAAA.
- `$AAAA`: A dollar-sign followed by an identifier name also holds a spot for a named parameter
  with the name $AAAA. Sqlite allows everything to follow after '$()', sqleibniz forbids this via
  Rule::Quirks errors
",
    counter: Option<Box<dyn Node>>,
    name: Option<String>
);

node!(
    Expr,
    "Expr expression, see: https://www.sqlite.org/lang_expr.html",
    literal: Option<Token>,
    bind: Option<BindParameter>,
    schema: Option<String>,
    table: Option<String>,
    column: Option<String>
);

node!(
    Explain,
   r"Explain stmt, see: https://www.sqlite.org/lang_explain.html

An SQL statement can be preceded by the keyword 'EXPLAIN' or by the phrase 'EXPLAIN QUERY PLAN'.
Either modification causes the SQL statement to behave as a query and to return information about
how the SQL statement would have operated if the EXPLAIN keyword or phrase had been omitted.

In depth guide for `EXPLAIN QUERY PLAN`: https://www.sqlite.org/eqp.html

# Examples

```sql
EXPLAIN VACUUM;
EXPLAIN QUERY PLAN VACUUM;
```
",
    child: Box<dyn Node>
);

node!(
    Vacuum,
    r"Vacuum stmt, see: https://www.sqlite.org/lang_vacuum.html

The VACUUM command rebuilds the database file, repacking it into a minimal amount of disk space. 

# Examples

```sql
VACUUM;
VACUUM schema_name;
VACUUM INTO 'filename';
VACUUM schema_name INTO 'filename';
```
",

    schema_name: Option<Token>,
    filename: Option<Token>
);

node!(
    Begin,
    r"Begin stmt, see: https://www.sqlite.org/lang_transaction.html

Transactions can be started manually using the BEGIN command. Such transactions usually persist
until the next COMMIT or ROLLBACK command. But a transaction will also ROLLBACK if the database is
closed or if an error occurs and the ROLLBACK conflict resolution algorithm is specified

Transactions can be DEFERRED, IMMEDIATE, or EXCLUSIVE. The default transaction behavior is
DEFERRED. 

# Examples

```sql
BEGIN;
BEGIN TRANSACTION;
BEGIN DEFERRED;
BEGIN IMMEDIATE;
BEGIN EXCLUSIVE;
BEGIN DEFERRED TRANSACTION;
BEGIN IMMEDIATE TRANSACTION;
BEGIN EXCLUSIVE TRANSACTION;
```
",
    transaction_kind: Option<Keyword>
);

node!(
    Commit,
    r"Commit stmt, see: https://www.sqlite.org/lang_transaction.html

END TRANSACTION is an alias for COMMIT. Transactions created using BEGIN...COMMIT do not nest. For
nested transactions, use the SAVEPOINT and RELEASE commands.

# Examples

```sql
COMMIT;
END;
COMMIT TRANSACTION;
END TRANSACTION;
```
",
);

node!(
    Rollback,
    r"Rollback stmt, see:  https://www.sqlite.org/lang_savepoint.html

The ROLLBACK TO command reverts the state of the database back to what it was just after the
corresponding SAVEPOINT. Note that unlike that plain ROLLBACK command (without the TO keyword) the
ROLLBACK TO command does not cancel the transaction. Instead of cancelling the transaction, the
ROLLBACK TO command restarts the transaction again at the beginning. All intervening SAVEPOINTs are
canceled, however.

# Examples

```sql
ROLLBACK;
ROLLBACK TO save_point;
ROLLBACK TO SAVEPOINT save_point;
ROLLBACK TRANSACTION;
ROLLBACK TRANSACTION TO save_point;
ROLLBACK TRANSACTION TO SAVEPOINT save_point;
```
",
    save_point: Option<String>
);

node!(
    Detach,
    r"Detach stmt, see: https://www.sqlite.org/lang_detach.html

This statement detaches an additional database connection previously attached using the ATTACH
statement. When not in shared cache mode, it is possible to have the same database file attached
multiple times using different names, and detaching one connection to a file will leave the others
intact.

# Examples

```sql
DETACH schema_name;
DETACH DATABASE schema_name;
```
",
    schema_name: String
);

node!(
    Analyze,
    r"Analyze stmt, see: https://www.sqlite.org/lang_analyze.html

The ANALYZE command gathers statistics about tables and indices and stores the collected
information in internal tables of the database where the query optimizer can access the
information and use it to help make better query planning choices. If no arguments are given, the
main database and all attached databases are analyzed. If a schema name is given as the argument,
then all tables and indices in that one database are analyzed. If the argument is a table name,
then only that table and the indices associated with that table are analyzed. If the argument is
an index name, then only that one index is analyzed.

# Examples

```sql
ANALYZE;
ANALYZE schema_name;
ANALYZE index_or_table_name.index_or_table_name;
ANALYZE schema_name.index_or_table_name;
```
    ",
    target: Option<SchemaTableContainer>
);

/// SchemaTableContainer contains either schema_name.table_name or table_name
#[derive(Debug, PartialEq, Eq, serde::Serialize)]
pub enum SchemaTableContainer {
    /// schema_name.table_name
    SchemaAndTable { schema: String, table: String },
    /// table_name
    Table(String),
}

node!(
    Drop,
    r"Drop stmt

## DROP INDEX

The DROP INDEX statement removes an index added with the CREATE INDEX statement. The index is
completely removed from the disk. The only way to recover the index is to reenter the appropriate
CREATE INDEX command, see https://www.sqlite.org/lang_dropindex.html.

# Examples

```sql
DROP INDEX index_name;
DROP INDEX IF EXISTS schema_name.index_name;
```

## DROP TABLE

The DROP TABLE statement removes a table added with the CREATE TABLE statement. The name specified
is the table name. The dropped table is completely removed from the database schema and the disk
file. The table can not be recovered. All indices and triggers associated with the table are also
deleted, see: https://www.sqlite.org/lang_droptable.html.

# Examples

```sql
DROP TABLE table_name;
DROP TABLE IF EXISTS schema_name.table_name;
```
                    
## DROP TRIGGER

The DROP TRIGGER statement removes a trigger created by the CREATE TRIGGER statement. Once removed, the trigger definition 
is no longer present in the sqlite_schema (or sqlite_temp_schema) table and is not fired by any subsequent
INSERT, UPDATE or DELETE statements, see: https://www.sqlite.org/lang_droptrigger.html.

# Examples

```sql
DROP TRIGGER trigger_name;
DROP TRIGGER IF EXISTS schema_name.trigger_name;
```
                                                
## DROP VIEW

The DROP VIEW statement removes a view created by the CREATE VIEW statement. 
The view definition is removed from the database schema,
but no actual data in the underlying base tables is modified,
see: https://www.sqlite.org/lang_dropview.html.

# Examples

```sql
DROP VIEW view_name;
DROP VIEW IF EXISTS schema_name.view_name;
```
",
    if_exists: bool,
    ttype: Keyword,
    argument: SchemaTableContainer
);

node!(
    Savepoint,
    r"Savepoint stmt, see: https://www.sqlite.org/lang_savepoint.html

SAVEPOINTs are a method of creating transactions, similar to BEGIN and COMMIT, except that the
SAVEPOINT and RELEASE commands are named and may be nested.

# Examples

```sql
SAVEPOINT savepoint_name;
```
",
    savepoint_name: String
);

node!(
    Release,
    r"Release stmt, see: https://www.sqlite.org/lang_savepoint.html

The RELEASE command is like a COMMIT for a SAVEPOINT. The RELEASE command causes all savepoints
back to and including the most recent savepoint with a matching name to be removed from the
transaction stack.

# Examples

```sql
RELEASE savepoint_name;
RELEASE SAVEPOINT savepoint_name;
```
",
    savepoint_name: String
);

node!(
    Attach,
    "Attach stmt, see: https://www.sqlite.org/lang_attach.html

The ATTACH DATABASE statement adds another database file to the current database connection.
Database files that were previously attached can be removed using the DETACH DATABASE command. 

# Examples

```sql
ATTACH DATABASE 'users.db' AS users;
ATTACH 'users.db' AS users;
```
",
    schema_name: String,
    expr: Expr
);

node!(
    Reindex,
    r"Reindex stmt, see: https://www.sqlite.org/lang_reindex.html

The REINDEX command is used to delete and recreate indices from scratch. This is useful when the
definition of a collation sequence has changed, or when there are indexes on expressions involving
a function whose definition has changed.

# Examples

```sql
REINDEX;
REINDEX collation_name;
REINDEX schema_name.table_name;
```
",
    target: Option<SchemaTableContainer>
);

node!(
    Alter,
    r"Alter stmt, see: https://www.sqlite.org/lang_altertable.html

SQLite supports a limited subset of ALTER TABLE:
The ALTER TABLE command in SQLite allows these alterations of an existing table: a table can be
renamed, a column can be renamed, a column can be added to a table or a column can be dropped
from the table.

# Examples

```sql
ALTER TABLE schema.table_name RENAME TO new_table;
ALTER TABLE schema.table_name RENAME old_column_name TO new_column_name;
ALTER TABLE schema.table_name RENAME COLUMN old_column_name TO new_column_name;

ALTER TABLE schema.table_name ADD new_column_name TEXT;
ALTER TABLE schema.table_name ADD COLUMN new_column_name TEXT;

ALTER TABLE schema.table_name DROP column_name;
ALTER TABLE schema.table_name DROP COLUMN column_name;
```
",
    target: SchemaTableContainer,
    rename_to: Option<String>,
    rename_column_target: Option<String>,
    new_column_name: Option<String>,
    add_column: Option<ColumnDef>,
    drop_column: Option<String>
);

#[derive(Debug, serde::Serialize)]
/// https://www.sqlite.org/syntax/foreign-key-clause.html ON [DELETE|UPDATE] ForeignKeyAction
pub enum ForeignKeyAction {
    Cascade,
    Restrict,
    NoAction,
    SetNull,
    SetDefault,
}

#[derive(Debug, serde::Serialize)]
/// SQLite parses MATCH clauses (i.e. does not report a syntax error if you specify one), but does
/// not enforce them. All foreign key constraints in SQLite are handled as if MATCH SIMPLE were
/// specified, see https://sqlite.org/foreignkeys.html#fk_unsupported
pub enum ForeignKeyMatch {
    Simple,
    Full,
    Partial,
}

#[derive(Debug, serde::Serialize)]
/// https://www.sqlite.org/syntax/foreign-key-clause.html
pub struct ForeignKeyClause {
    pub foreign_table: String,
    pub references_columns: Vec<String>,
    pub on_delete: Option<ForeignKeyAction>,
    pub on_update: Option<ForeignKeyAction>,
    pub match_type: Option<ForeignKeyMatch>,
    pub deferrable: bool,
    pub initially_deferred: bool,
}

#[derive(Debug)]
/// https://www.sqlite.org/syntax/column-constraint.html
pub enum ColumnConstraint {
    PrimaryKey {
        // either ASC or DESC
        asc_desc: Option<Keyword>,
        on_conflict: Option<Keyword>,
        autoincrement: bool,
    },
    NotNull {
        on_conflict: Option<Keyword>,
    },
    Unique {
        on_conflict: Option<Keyword>,
    },
    Check(Expr),
    Default {
        expr: Option<Expr>,
        literal: Option<Literal>,
    },
    Collate(String),
    Generated {
        expr: Expr,
        /// either STORED or VIRTUAL
        stored_virtual: Option<Keyword>,
    },
    As {
        expr: Expr,
        /// either STORED or VIRTUAL
        stored_virtual: Option<Keyword>,
    },
    ForeignKey(ForeignKeyClause),
}

node!(
    ColumnDef,
    "Column definition, see: https://www.sqlite.org/syntax/column-def.html",
    name: String,
    // equivalent to type_name: https://www.sqlite.org/syntax/type-name.html
    type_name: Option<SqliteStorageClass>,
    constraints: Vec<ColumnConstraint>
);
