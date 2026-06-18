use crate::analyse::AnalysisContext;
use crate::error::{Error, Location};
use crate::parser::debug::{FieldDiagnostics, FieldHookContexts, FieldSerializable};
use crate::types::{Keyword, Token, ctx::HookContext, storage::SqliteStorageClass};

macro_rules! field_hook_contexts {
    () => {
        Vec::new()
    };
    ($($field:expr),+ $(,)?) => {
        vec![
            $(
                $field.field_hook_contexts(),
            )+
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<HookContext>>()
    };
}

macro_rules! field_diagnostics {
    ($file:expr, $context:expr,) => {
        {
            let _ = $file;
            let _ = $context;
            Vec::new()
        }
    };
    ($file:expr, $context:expr, $($field:expr),+ $(,)?) => {
        vec![
            $(
                $field.field_diagnostics($file, $context),
            )+
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<Error>>()
    };
}

macro_rules! node_diagnostics {
    ($file:expr, $context:expr, [$($field:expr),*], $analyser:path, $node:expr) => {{
        $analyser($file, $context, $node)
    }};
    ($file:expr, $context:expr, [$($field:expr),*], $node:expr) => {
        field_diagnostics!($file, $context, $($field),*)
    };
}

macro_rules! node {
    ($node_name:ident,$documentation:literal,$($field_name:ident:$field_type:ty),* $(; analyse($analyser:path))?) => {
        #[derive(Debug)]
        #[doc = $documentation]
        pub struct $node_name {
            /// Source location for this AST node.
            pub location: Location,
            $(
                pub $field_name: $field_type,
            )*
        }

        impl Node for $node_name {
            fn location(&self) -> Location {
                self.location
            }

            #[cfg(feature = "trace")]
            fn display(&self, indent: usize) {
                print!("{}- {}({:?})", " ".repeat(indent), self.name(), self.location);
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

            fn as_hook_context(&self) -> HookContext {
                let children = field_hook_contexts!($(self.$field_name),*);

                HookContext {
                    node: stringify!($node_name).into(),
                    kind: stringify!($node_name).into(),
                    content: None,
                    line: self.location.line,
                    start: self.location.start,
                    finish: self.location.end,
                    children,
                }
            }

            fn doc(&self) -> &str {
                $documentation
            }

            fn analyse(&self, file: &str, context: &mut AnalysisContext) -> Vec<Error> {
                node_diagnostics!(file, context, [$(self.$field_name),*] $(, $analyser)?, self)
            }
        }

        #[cfg(test)]
        impl $node_name {
            pub fn new($($field_name: $field_type,)*) -> Self {
                Self {
                    location: Location::new(0, 0, 0),
                    $($field_name,)*
                }
            }
        }

        impl FieldSerializable for $node_name {
            fn field_as_serializable(&self) -> serde_json::Value {
                self.as_serializable()
            }
        }

        impl FieldHookContexts for $node_name {
            fn field_hook_contexts(&self) -> Vec<HookContext> {
                vec![self.as_hook_context()]
            }
        }
    };
}

pub trait Node: std::fmt::Debug {
    fn location(&self) -> Location;
    #[cfg(feature = "trace")]
    fn display(&self, indent: usize);
    fn name(&self) -> &str;
    /// serializes self as json
    fn as_serializable(&self) -> serde_json::Value;
    /// converts self into the data passed to Lua hooks
    fn as_hook_context(&self) -> HookContext;
    /// returns the documentation url for sefl
    fn doc(&self) -> &str;
    /// returns diagnostics found by analysing this node after parsing
    fn analyse(&self, file: &str, context: &mut AnalysisContext) -> Vec<Error> {
        let _ = file;
        let _ = context;
        Vec::new()
    }
}

node!(
    Literal,
    r"Literal value, see: https://www.sqlite.org/lang_expr.html#literal_values_constants_

A literal value represents a constant. Literal values may be integers, floating point numbers, strings, BLOBs, or NULLs.
",
    value: Token
);

node!(
    BindParameter,
    r"Bind parameter, see https://www.sqlite.org/lang_expr.html#parameters

A parameter specifies a placeholder in the expression for a value that is filled in at runtime.
These can take several forms:

- `?NNN`: A question mark followed by a number NNN holds a spot for the NNN-th parameter.
- `?`: A question mark that is not followed by a number creates a parameter with a number one
  greater than the largest parameter number already assigned.
- `:AAAA`: A colon followed by an identifier name holds a spot for a named parameter with the name
  `:AAAA`
- `@AAAA`: An 'at' sign works exactly like a colon, except that the name of the parameter created
  is @AAAA.
- `$AAAA`: A dollar-sign followed by an identifier name also holds a spot for a named parameter
  with the name $AAAA.
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
    column: Option<String>,
    function: Option<String>,
    operator: Option<String>,
    arguments: Vec<Expr>
);

node!(
    Explain,
   r"Explain stmt, see: https://www.sqlite.org/lang_explain.html

An SQL statement can be preceded by the keyword 'EXPLAIN' or by the phrase 'EXPLAIN QUERY PLAN'. Either modification causes the SQL statement to behave as a query and to return information about how the SQL statement would have operated if the EXPLAIN keyword or phrase had been omitted.

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

Transactions can be started manually using the BEGIN command. Such transactions usually persist until the next COMMIT or ROLLBACK command. But a transaction will also ROLLBACK if the database is closed or if an error occurs and the ROLLBACK conflict resolution algorithm is specified

Transactions can be DEFERRED, IMMEDIATE, or EXCLUSIVE. The default transaction behavior is DEFERRED. 

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

END TRANSACTION is an alias for COMMIT. Transactions created using BEGIN...COMMIT do not nest. For nested transactions, use the SAVEPOINT and RELEASE commands.

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

The ROLLBACK TO command reverts the state of the database back to what it was just after the corresponding SAVEPOINT. Note that unlike that plain ROLLBACK command (without the TO keyword) the ROLLBACK TO command does not cancel the transaction. Instead of cancelling the transaction, the ROLLBACK TO command restarts the transaction again at the beginning. All intervening SAVEPOINTs are canceled, however.

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

This statement detaches an additional database connection previously attached using the ATTACH statement. When not in shared cache mode, it is possible to have the same database file attached multiple times using different names, and detaching one connection to a file will leave the others intact.

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

The ANALYZE command gathers statistics about tables and indices and stores the collected information in internal tables of the database where the query optimizer can access the information and use it to help make better query planning choices. If no arguments are given, the main database and all attached databases are analyzed. If a schema name is given as the argument, then all tables and indices in that one database are analyzed. If the argument is a table name, then only that table and the indices associated with that table are analyzed. If the argument is an index name, then only that one index is analyzed.

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
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
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

The DROP INDEX statement removes an index added with the CREATE INDEX statement. The index is completely removed from the disk. The only way to recover the index is to reenter the appropriate CREATE INDEX command, see https://www.sqlite.org/lang_dropindex.html.

# Examples

```sql
DROP INDEX index_name;
DROP INDEX IF EXISTS schema_name.index_name;
```

## DROP TABLE

The DROP TABLE statement removes a table added with the CREATE TABLE statement. The name specified is the table name. The dropped table is completely removed from the database schema and the disk file. The table can not be recovered. All indices and triggers associated with the table are also deleted, see: https://www.sqlite.org/lang_droptable.html.

# Examples

```sql
DROP TABLE table_name;
DROP TABLE IF EXISTS schema_name.table_name;
```
                    
## DROP TRIGGER

The DROP TRIGGER statement removes a trigger created by the CREATE TRIGGER statement. Once removed, the trigger definition is no longer present in the sqlite_schema (or sqlite_temp_schema) table and is not fired by any subsequent INSERT, UPDATE or DELETE statements, see: https://www.sqlite.org/lang_droptrigger.html.

# Examples

```sql
DROP TRIGGER trigger_name;
DROP TRIGGER IF EXISTS schema_name.trigger_name;
```
                                                
## DROP VIEW

The DROP VIEW statement removes a view created by the CREATE VIEW statement. The view definition is removed from the database schema, but no actual data in the underlying base tables is modified, see: https://www.sqlite.org/lang_dropview.html.

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

#[derive(Debug, serde::Serialize)]
pub enum QualifiedTableIndex {
    IndexedBy(String),
    NotIndexed,
}

#[derive(Debug)]
pub struct QualifiedTableName {
    pub name: SchemaTableContainer,
    pub alias: Option<String>,
    pub index: Option<QualifiedTableIndex>,
}

#[derive(Debug)]
pub enum ResultColumn {
    Star,
    TableStar(String),
    Expr { expr: Expr, alias: Option<String> },
}

#[derive(Debug)]
pub struct OrderingTerm {
    pub expr: Expr,
    pub order: Option<Keyword>,
    pub nulls: Option<Keyword>,
}

#[derive(Debug)]
pub struct LimitOffset {
    pub limit: Expr,
    pub offset: Option<Expr>,
}

#[derive(Debug, serde::Serialize)]
pub enum SelectQuantifier {
    All,
    Distinct,
}

#[derive(Debug, serde::Serialize)]
pub enum JoinOperator {
    Inner,
    Left,
    LeftOuter,
    Cross,
}

#[derive(Debug)]
pub struct SelectTable {
    pub name: SchemaTableContainer,
    pub alias: Option<String>,
}

#[derive(Debug)]
pub enum SelectSource {
    Table(SelectTable),
    Join {
        left: Box<SelectSource>,
        operator: JoinOperator,
        right: SelectTable,
        on: Option<Expr>,
    },
}

node!(
    Select,
    r"SELECT statement, see: https://www.sqlite.org/lang_select.html

The SELECT command reads data from expressions and tables.

```sql
SELECT 1;
SELECT id, name FROM users WHERE active = true;
SELECT users.* FROM users ORDER BY id LIMIT 10;
```
",
    quantifier: Option<SelectQuantifier>,
    columns: Vec<ResultColumn>,
    from: Vec<SelectSource>,
    where_expr: Option<Expr>,
    group_by: Vec<Expr>,
    having: Option<Expr>,
    order_by: Vec<OrderingTerm>,
    limit: Option<LimitOffset>
);

#[derive(Debug)]
pub enum InsertSource {
    DefaultValues,
    Values(Vec<Vec<Expr>>),
    Select(Box<Select>),
}

#[derive(Debug)]
pub struct CommonTableExpression {
    pub name: String,
    pub columns: Vec<String>,
    pub materialized: Option<bool>,
    pub select: Box<Select>,
}

#[derive(Debug)]
pub struct UpdateAssignment {
    pub columns: Vec<String>,
    pub expr: Expr,
}

node!(
    Update,
    r"UPDATE statement, see: https://www.sqlite.org/lang_update.html

The UPDATE command changes existing rows in a table.

```sql
UPDATE table_name SET column_name = 'value';
UPDATE OR FAIL schema_name.table_name SET name = 'Ada' WHERE id = 1 RETURNING *;
UPDATE users SET (name, email) = user_defaults();
```
",
    conflict: Option<Keyword>,
    target: QualifiedTableName,
    assignments: Vec<UpdateAssignment>,
    where_expr: Option<Expr>,
    returning: Vec<ResultColumn>,
    order_by: Vec<OrderingTerm>,
    limit: Option<LimitOffset>
);

node!(
    Insert,
    r"INSERT statement, see: https://www.sqlite.org/lang_insert.html

The INSERT command creates new rows in a table.

```sql
INSERT INTO table_name DEFAULT VALUES;
INSERT INTO table_name (id, name) VALUES (1, 'Ada');
INSERT INTO table_name SELECT id, name FROM old_table;
INSERT OR IGNORE INTO schema_name.table_name VALUES (1) RETURNING *;
```
",
    conflict: Option<Keyword>,
    target: SchemaTableContainer,
    columns: Vec<String>,
    source: InsertSource,
    returning: Vec<ResultColumn>
);

node!(
    Delete,
    r"DELETE statement, see: https://www.sqlite.org/lang_delete.html

The DELETE command removes records from a table.

```sql
DELETE FROM table_name;
DELETE FROM schema_name.table_name WHERE id = 1;
DELETE FROM users AS u INDEXED BY idx_users_id WHERE u.id = 1 RETURNING *;
```
",
    target: QualifiedTableName,
    where_expr: Option<Expr>,
    returning: Vec<ResultColumn>,
    order_by: Vec<OrderingTerm>,
    limit: Option<LimitOffset>
);

node!(
    Savepoint,
    r"Savepoint stmt, see: https://www.sqlite.org/lang_savepoint.html

SAVEPOINTs are a method of creating transactions, similar to BEGIN and COMMIT, except that the SAVEPOINT and RELEASE commands are named and may be nested.

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

The RELEASE command is like a COMMIT for a SAVEPOINT. The RELEASE command causes all savepoints back to and including the most recent savepoint with a matching name to be removed from the transaction stack.

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

The ATTACH DATABASE statement adds another database file to the current database connection. Database files that were previously attached can be removed using the DETACH DATABASE command. 

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

The REINDEX command is used to delete and recreate indices from scratch. This is useful when the definition of a collation sequence has changed, or when there are indexes on expressions involving a function whose definition has changed.

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

SQLite supports a limited subset of ALTER TABLE: The ALTER TABLE command in SQLite allows these alterations of an existing table: a table can be renamed, a column can be renamed, a column can be added to a table or a column can be dropped from the table.

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
    ; analyse(crate::analyse::create::alter)
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

#[derive(Debug)]
/// https://www.sqlite.org/syntax/table-constraint.html
pub enum TableConstraint {
    PrimaryKey {
        columns: Vec<IndexedColumn>,
        on_conflict: Option<Keyword>,
    },
    Unique {
        columns: Vec<IndexedColumn>,
        on_conflict: Option<Keyword>,
    },
    Check(Expr),
    ForeignKey {
        columns: Vec<String>,
        foreign_key_clause: ForeignKeyClause,
    },
}

node!(
    ColumnDef,
    "Column definition, see: https://www.sqlite.org/syntax/column-def.html",
    name: String,
    // equivalent to type_name: https://www.sqlite.org/syntax/type-name.html
    type_name: Option<SqliteStorageClass>,
    constraints: Vec<ColumnConstraint>
    ; analyse(crate::analyse::column::column_def)
);

node!(
    CreateTable,
    r"CREATE TABLE statement, see: https://www.sqlite.org/lang_createtable.html

The CREATE TABLE command creates a new table in an SQLite database.

```sql
CREATE TABLE table_name (column_def, ...);
CREATE TEMP TABLE IF NOT EXISTS schema.table_name (column_def, ...);
CREATE TABLE table_name (column_def, ...) STRICT;
```
",
    temporary: bool,
    if_not_exists: bool,
    name: SchemaTableContainer,
    columns: Vec<ColumnDef>,
    table_constraints: Vec<TableConstraint>,
    strict: bool,
    without_rowid: bool
    ; analyse(crate::analyse::create::create_table)
);

node!(
    CreateTableAs,
    r"CREATE TABLE AS statement, see: https://www.sqlite.org/lang_createtable.html

The CREATE TABLE AS form creates a table from a SELECT statement.

```sql
CREATE TABLE table_name AS SELECT id FROM old_table;
CREATE TEMP TABLE IF NOT EXISTS schema.table_name AS SELECT id FROM old_table;
```
",
    temporary: bool,
    if_not_exists: bool,
    name: SchemaTableContainer,
    select: Box<Select>
    ; analyse(crate::analyse::create::create_table_as)
);

node!(
    CreateView,
    r"CREATE VIEW statement, see: https://www.sqlite.org/lang_createview.html

The CREATE VIEW command creates a named SELECT statement.

```sql
CREATE VIEW view_name AS SELECT id FROM table_name;
CREATE TEMP VIEW IF NOT EXISTS schema.view_name (id) AS SELECT id FROM table_name;
```
",
    temporary: bool,
    if_not_exists: bool,
    name: SchemaTableContainer,
    columns: Vec<String>,
    select: Box<Select>
    ; analyse(crate::analyse::create::create_view)
);

node!(
    CreateVirtualTable,
    r"CREATE VIRTUAL TABLE statement, see: https://www.sqlite.org/lang_createvtab.html

The CREATE VIRTUAL TABLE command creates a virtual table backed by a module.

```sql
CREATE VIRTUAL TABLE docs USING fts5(content);
CREATE VIRTUAL TABLE spatial_index USING rtree(id, min_x, max_x, min_y, max_y);
```
",
    temporary: bool,
    if_not_exists: bool,
    name: SchemaTableContainer,
    module: String,
    arguments: Vec<Token>
    ; analyse(crate::analyse::create::create_virtual_table)
);

node!(
    With,
    r"WITH statement, see: https://www.sqlite.org/lang_with.html

The WITH clause defines common table expressions for a following statement.

```sql
WITH rows AS (SELECT id FROM table_name) SELECT id FROM rows;
WITH RECURSIVE rows(id) AS NOT MATERIALIZED (SELECT 1) SELECT id FROM rows;
```
",
    recursive: bool,
    expressions: Vec<CommonTableExpression>,
    child: Box<dyn Node>
);

#[derive(Debug, serde::Serialize)]
pub struct IndexedColumn {
    pub name: String,
    pub collation: Option<String>,
    pub order: Option<Keyword>,
}

node!(
    CreateIndex,
    r"CREATE INDEX statement, see: https://www.sqlite.org/lang_createindex.html

The CREATE INDEX command creates a new index for a table.

```sql
CREATE INDEX index_name ON table_name (column_name);
CREATE UNIQUE INDEX IF NOT EXISTS schema.index_name ON table_name (column_name COLLATE collation_name DESC);
```
",
    unique: bool,
    if_not_exists: bool,
    name: SchemaTableContainer,
    table: String,
    columns: Vec<IndexedColumn>
);

#[derive(Debug, serde::Serialize)]
pub enum TriggerTiming {
    Before,
    After,
    InsteadOf,
}

#[derive(Debug, serde::Serialize)]
pub enum TriggerEvent {
    Delete,
    Insert,
    Update { columns: Vec<String> },
}

#[derive(Debug, serde::Serialize)]
pub enum TriggerBodyStmt {
    Delete,
    Insert,
    Select,
    Update,
}

node!(
    CreateTrigger,
    r"CREATE TRIGGER statement, see: https://www.sqlite.org/lang_createtrigger.html

The CREATE TRIGGER command creates a database trigger.

```sql
CREATE TRIGGER trigger_name AFTER INSERT ON table_name BEGIN SELECT 1; END;
CREATE TEMP TRIGGER IF NOT EXISTS schema.trigger_name INSTEAD OF UPDATE OF column_name ON table_name FOR EACH ROW BEGIN UPDATE table_name SET column_name = 1; END;
```
",
    temporary: bool,
    if_not_exists: bool,
    name: SchemaTableContainer,
    timing: Option<TriggerTiming>,
    event: TriggerEvent,
    table: String,
    for_each_row: bool,
    when: bool,
    body: Vec<TriggerBodyStmt>
);

#[derive(Debug, serde::Serialize)]
pub enum PragmaInvocation {
    Query,
    Assign { value: Token },
    Call { value: Token },
}

node!(
    Pragma,
    r"PRAGMA Statements, see: https://www.sqlite.org/pragma.html

The PRAGMA statement is an SQL extension specific to SQLite. PRAGMAs modify SQLite library
operation or query SQLite for internal, non-table data.

# Examples

```sql
PRAGMA schema.cache_size = 5; -- 5 pages
PRAGMA automatic_index = 1;
PRAGMA database_list;
```
",
    // since pragma names can be schema.pragma_name, we encode it like this in the ast
    name: SchemaTableContainer,
    invocation: PragmaInvocation
    ; analyse(crate::analyse::pragma::pragma)
);
