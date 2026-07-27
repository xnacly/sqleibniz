use crate::{
    error::Location,
    types::{Keyword, Token, storage::SqliteStorageClass},
};

macro_rules! node {
    ($node_name:ident,$documentation:literal,$($(#[doc = $field_documentation:literal])* $field_name:ident:$field_type:ty),* $(; analyse($analyser:path))?) => {
        #[derive(Debug)]
        #[doc = $documentation]
        pub struct $node_name {
            /// Source location for this AST node.
            pub location: Location,
            $(
                $(#[doc = $field_documentation])*
                pub $field_name: $field_type,
            )*
        }

        impl $node_name {
            /// DOC is the documentation of this node type, [Node::doc] returns it for a
            /// `dyn Node`
            pub const DOC: &'static str = $documentation;
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

            #[cfg(test)]
            fn as_test_serializable(&self) -> serde_json::Value {
                let mut map = serde_json::Map::new();
                map.insert("type".to_string(), serde_json::Value::String(stringify!($node_name).to_string()));
                $(
                    map.insert(
                        stringify!($field_name).to_string(),
                        crate::parser::test_support::FieldSerializable::field_as_serializable(&self.$field_name),
                    );
                )*
                serde_json::Value::Object(map)
            }

            fn doc(&self) -> &str {
                Self::DOC
            }

            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }

        impl $node_name {
            /// new creates the node at line 0, offset 0, the parser overwrites the location with
            /// the one of the tokens the node was parsed from
            pub fn new($($field_name: $field_type,)*) -> Self {
                Self {
                    location: Location::new(0, 0, 0),
                    $($field_name,)*
                }
            }
        }

        #[cfg(test)]
        impl crate::parser::test_support::FieldSerializable for $node_name {
            fn field_as_serializable(&self) -> serde_json::Value {
                self.as_test_serializable()
            }
        }

        #[cfg(feature = "serde")]
        impl serde::Serialize for $node_name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                use serde::ser::SerializeMap;

                let mut map = serializer.serialize_map(None)?;
                map.serialize_entry("type", stringify!($node_name))?;
                $(
                    map.serialize_entry(stringify!($field_name), &self.$field_name)?;
                )*
                map.end()
            }
        }

    };
}

/// Node is implemented by every ast node, statements are stored as `Box<dyn Node>`
///
/// The trait itself only holds what all nodes share, everything node specific lives on the
/// concrete types in this module. To get from a node to its fields, ask [Node::as_any] for the
/// concrete type:
///
/// ```
/// use sqleibniz_ast::Node;
/// use sqleibniz_ast::parser::nodes::Select;
///
/// let parsed = sqleibniz_ast::parse("SELECT id FROM users;", "query.sql");
/// let node = parsed.ast.first().unwrap().as_ref();
///
/// assert_eq!(node.name(), "Select");
///
/// let select = node.as_any().downcast_ref::<Select>().unwrap();
/// assert_eq!(select.columns.len(), 1);
/// ```
///
/// Nodes that wrap another statement store it as `Box<dyn Node>` too, see [Explain::child] and
/// [With::child], every other child is stored as its concrete type, such as [Select::columns].
pub trait Node: std::fmt::Debug + std::any::Any {
    /// location returns the position in the source file self was parsed from
    fn location(&self) -> Location;
    /// display prints self and its fields, indented by indent spaces
    #[cfg(feature = "trace")]
    fn display(&self, indent: usize);
    /// name returns the name of the node type, such as `Select`
    fn name(&self) -> &str;
    #[cfg(test)]
    fn as_test_serializable(&self) -> serde_json::Value;
    /// doc returns the documentation for the sql construct self was parsed from, this is the same
    /// documentation the concrete node type carries
    fn doc(&self) -> &str;
    /// as_any returns self as `Any`, downcast it to a concrete node type to access its fields
    fn as_any(&self) -> &dyn std::any::Any;
}

node!(
    Literal,
    r"Literal value, see: <https://www.sqlite.org/lang_expr.html#literal_values_constants_>

A literal value represents a constant. Literal values may be integers, floating point numbers, strings, BLOBs, or NULLs.
",
    /// the token the literal was lexed from, its [crate::Type] holds the value
    value: Token
);

node!(
    BindParameter,
    r"Bind parameter, see <https://www.sqlite.org/lang_expr.html#parameters>

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
    /// the NNN of `?NNN`, holds a [Literal]
    counter: Option<Box<dyn Node>>,
    /// the AAAA of `:AAAA`, `@AAAA` and `$AAAA`, without the leading symbol
    name: Option<String>
);

node!(
    Expr,
    "Expr expression, see: <https://www.sqlite.org/lang_expr.html>",
    /// set for a literal value, such as `1` or `'text'`
    literal: Option<Token>,
    /// set for a bind parameter, such as `:id`
    bind: Option<BindParameter>,
    /// schema of a column reference, set for `schema_name.table_name.column_name`
    schema: Option<String>,
    /// table of a column reference, set for `table_name.column_name`
    table: Option<String>,
    /// set for a column reference, such as `id` or `users.id`
    column: Option<String>,
    /// name of the called function, its arguments are in [Expr::arguments]
    function: Option<String>,
    /// the operator, such as `+` or `NOT`, its operands are in [Expr::arguments]
    operator: Option<String>,
    /// arguments of [Expr::function] or operands of [Expr::operator], empty for every other
    /// expression
    arguments: Vec<Expr>
);

node!(
    Explain,
   r"Explain stmt, see: <https://www.sqlite.org/lang_explain.html>

An SQL statement can be preceded by the keyword 'EXPLAIN' or by the phrase 'EXPLAIN QUERY PLAN'. Either modification causes the SQL statement to behave as a query and to return information about how the SQL statement would have operated if the EXPLAIN keyword or phrase had been omitted.

In depth guide for `EXPLAIN QUERY PLAN`: <https://www.sqlite.org/eqp.html>

# Examples

```sql
EXPLAIN VACUUM;
EXPLAIN QUERY PLAN VACUUM;
```
",
    /// the statement the plan is explained for
    child: Box<dyn Node>
);

node!(
    Vacuum,
    r"Vacuum stmt, see: <https://www.sqlite.org/lang_vacuum.html>

The VACUUM command rebuilds the database file, repacking it into a minimal amount of disk space. 

# Examples

```sql
VACUUM;
VACUUM schema_name;
VACUUM INTO 'filename';
VACUUM schema_name INTO 'filename';
```
",

    /// schema to rebuild, all attached databases are rebuilt when this is `None`
    schema_name: Option<Token>,
    /// INTO 'filename', the file the rebuilt database is written to
    filename: Option<Token>
);

node!(
    Begin,
    r"Begin stmt, see: <https://www.sqlite.org/lang_transaction.html>

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
    /// either DEFERRED, IMMEDIATE or EXCLUSIVE, sqlite defaults to DEFERRED
    transaction_kind: Option<Keyword>
);

node!(
    Commit,
    r"Commit stmt, see: <https://www.sqlite.org/lang_transaction.html>

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
    r"Rollback stmt, see:  <https://www.sqlite.org/lang_savepoint.html>

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
    /// TO save_point, the savepoint the transaction is restarted at
    save_point: Option<String>
);

node!(
    Detach,
    r"Detach stmt, see: <https://www.sqlite.org/lang_detach.html>

This statement detaches an additional database connection previously attached using the ATTACH statement. When not in shared cache mode, it is possible to have the same database file attached multiple times using different names, and detaching one connection to a file will leave the others intact.

# Examples

```sql
DETACH schema_name;
DETACH DATABASE schema_name;
```
",
    /// the schema to detach, as attached by an ATTACH statement
    schema_name: String
);

node!(
    Analyze,
    r"Analyze stmt, see: <https://www.sqlite.org/lang_analyze.html>

The ANALYZE command gathers statistics about tables and indices and stores the collected information in internal tables of the database where the query optimizer can access the information and use it to help make better query planning choices. If no arguments are given, the main database and all attached databases are analyzed. If a schema name is given as the argument, then all tables and indices in that one database are analyzed. If the argument is a table name, then only that table and the indices associated with that table are analyzed. If the argument is an index name, then only that one index is analyzed.

# Examples

```sql
ANALYZE;
ANALYZE schema_name;
ANALYZE index_or_table_name.index_or_table_name;
ANALYZE schema_name.index_or_table_name;
```
    ",
    /// the analysed schema, table or index, all attached databases are analysed when this is
    /// `None`
    target: Option<SchemaTableContainer>
);

/// SchemaTableContainer contains either schema_name.table_name or table_name
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(any(test, feature = "serde"), derive(serde::Serialize))]
pub enum SchemaTableContainer {
    /// schema_name.table_name
    SchemaAndTable {
        /// the schema the object lives in
        schema: String,
        /// name of the object
        table: String,
    },
    /// table_name
    Table(String),
}

node!(
    Drop,
    r"Drop stmt

## DROP INDEX

The DROP INDEX statement removes an index added with the CREATE INDEX statement. The index is completely removed from the disk. The only way to recover the index is to reenter the appropriate CREATE INDEX command, see <https://www.sqlite.org/lang_dropindex.html>.

# Examples

```sql
DROP INDEX index_name;
DROP INDEX IF EXISTS schema_name.index_name;
```

## DROP TABLE

The DROP TABLE statement removes a table added with the CREATE TABLE statement. The name specified is the table name. The dropped table is completely removed from the database schema and the disk file. The table can not be recovered. All indices and triggers associated with the table are also deleted, see: <https://www.sqlite.org/lang_droptable.html>.

# Examples

```sql
DROP TABLE table_name;
DROP TABLE IF EXISTS schema_name.table_name;
```
                    
## DROP TRIGGER

The DROP TRIGGER statement removes a trigger created by the CREATE TRIGGER statement. Once removed, the trigger definition is no longer present in the sqlite_schema (or sqlite_temp_schema) table and is not fired by any subsequent INSERT, UPDATE or DELETE statements, see: <https://www.sqlite.org/lang_droptrigger.html>.

# Examples

```sql
DROP TRIGGER trigger_name;
DROP TRIGGER IF EXISTS schema_name.trigger_name;
```
                                                
## DROP VIEW

The DROP VIEW statement removes a view created by the CREATE VIEW statement. The view definition is removed from the database schema, but no actual data in the underlying base tables is modified, see: <https://www.sqlite.org/lang_dropview.html>.

# Examples

```sql
DROP VIEW view_name;
DROP VIEW IF EXISTS schema_name.view_name;
```
",
    /// whether IF EXISTS is specified
    if_exists: bool,
    /// what is dropped, either INDEX, TABLE, TRIGGER or VIEW
    ttype: Keyword,
    /// the dropped object, either name or schema_name.name
    argument: SchemaTableContainer
);

#[derive(Debug)]
#[cfg_attr(any(test, feature = "serde"), derive(serde::Serialize))]
/// QualifiedTableIndex is the index a statement forces or forbids for the table it operates on
///
/// see: <https://www.sqlite.org/syntax/qualified-table-name.html>
pub enum QualifiedTableIndex {
    /// INDEXED BY index_name
    IndexedBy(String),
    /// NOT INDEXED
    NotIndexed,
}

#[derive(Debug)]
#[cfg_attr(any(test, feature = "serde"), derive(serde::Serialize))]
/// QualifiedTableName is the table an UPDATE or DELETE operates on
///
/// see: <https://www.sqlite.org/syntax/qualified-table-name.html>
pub struct QualifiedTableName {
    /// table_name or schema_name.table_name
    pub name: SchemaTableContainer,
    /// AS alias
    pub alias: Option<String>,
    /// INDEXED BY index_name or NOT INDEXED
    pub index: Option<QualifiedTableIndex>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
/// ResultColumn is a column a SELECT returns or a RETURNING clause reports
///
/// see: <https://www.sqlite.org/syntax/result-column.html>
pub enum ResultColumn {
    /// `*`
    Star,
    /// table_name.`*`
    TableStar(String),
    /// an expression, optionally aliased with AS
    Expr {
        /// the expression that produces the column
        expr: Expr,
        /// AS alias
        alias: Option<String>,
    },
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
/// OrderingTerm is a single term of an ORDER BY clause
///
/// see: <https://www.sqlite.org/syntax/ordering-term.html>
pub struct OrderingTerm {
    /// the expression rows are ordered by
    pub expr: Expr,
    /// either ASC or DESC
    pub order: Option<Keyword>,
    /// NULLS FIRST or NULLS LAST, holds either FIRST or LAST
    pub nulls: Option<Keyword>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
/// LimitOffset is the LIMIT clause of a statement, including its optional offset
pub struct LimitOffset {
    /// maximum amount of rows
    pub limit: Expr,
    /// amount of rows to skip, either OFFSET expr or the `LIMIT expr, expr` spelling
    pub offset: Option<Expr>,
}

#[derive(Debug)]
#[cfg_attr(any(test, feature = "serde"), derive(serde::Serialize))]
/// SelectQuantifier is the ALL or DISTINCT a SELECT is prefixed with
pub enum SelectQuantifier {
    /// ALL, keeps duplicate rows, this is the default
    All,
    /// DISTINCT, removes duplicate rows
    Distinct,
}

#[derive(Debug)]
#[cfg_attr(any(test, feature = "serde"), derive(serde::Serialize))]
/// JoinOperator is the kind of join two tables are joined with
///
/// see: <https://www.sqlite.org/syntax/join-operator.html>
pub enum JoinOperator {
    /// JOIN or INNER JOIN
    Inner,
    /// LEFT JOIN
    Left,
    /// LEFT OUTER JOIN
    LeftOuter,
    /// CROSS JOIN
    Cross,
}

#[derive(Debug)]
#[cfg_attr(any(test, feature = "serde"), derive(serde::Serialize))]
/// SelectTable is a table a SELECT reads from
pub struct SelectTable {
    /// table_name or schema_name.table_name
    pub name: SchemaTableContainer,
    /// AS alias
    pub alias: Option<String>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
/// SelectSource is the FROM clause of a SELECT, joins nest to the left
///
/// see: <https://www.sqlite.org/syntax/table-or-subquery.html>
pub enum SelectSource {
    /// a single table
    Table(SelectTable),
    /// two sources joined together
    Join {
        /// everything joined so far
        left: Box<SelectSource>,
        /// the kind of join
        operator: JoinOperator,
        /// the table joined onto left
        right: SelectTable,
        /// ON expr, joins without a constraint hold `None`
        on: Option<Expr>,
    },
}

node!(
    Select,
    r"SELECT statement, see: <https://www.sqlite.org/lang_select.html>

The SELECT command reads data from expressions and tables.

```sql
SELECT 1;
SELECT id, name FROM users WHERE active = true;
SELECT users.* FROM users ORDER BY id LIMIT 10;
```
",
    /// either ALL or DISTINCT
    quantifier: Option<SelectQuantifier>,
    /// the returned columns
    columns: Vec<ResultColumn>,
    /// the FROM clause, empty for a SELECT without one, such as `SELECT 1`
    from: Vec<SelectSource>,
    /// the WHERE clause
    where_expr: Option<Expr>,
    /// the expressions of the GROUP BY clause
    group_by: Vec<Expr>,
    /// the HAVING clause, filters the groups of [Select::group_by]
    having: Option<Expr>,
    /// the terms of the ORDER BY clause
    order_by: Vec<OrderingTerm>,
    /// the LIMIT clause and its offset
    limit: Option<LimitOffset>
    ; analyse(crate::analyse::relation::select)
);

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
/// InsertSource is where an INSERT takes its rows from
pub enum InsertSource {
    /// DEFAULT VALUES
    DefaultValues,
    /// VALUES (...), (...), the outer Vec holds the rows, the inner one the values of a row
    Values(Vec<Vec<Expr>>),
    /// the rows a SELECT returns
    Select(Box<Select>),
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
/// CommonTableExpression is a single named SELECT of a WITH clause
///
/// see: <https://www.sqlite.org/syntax/common-table-expression.html>
pub struct CommonTableExpression {
    /// name the expression is referenced by
    pub name: String,
    /// the column names the expression exposes, empty when it does not rename them
    pub columns: Vec<String>,
    /// `Some(true)` for MATERIALIZED, `Some(false)` for NOT MATERIALIZED, `None` when neither is
    /// specified
    pub materialized: Option<bool>,
    /// the SELECT the name refers to
    pub select: Box<Select>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
/// UpdateAssignment is a single assignment of an UPDATE SET clause
pub struct UpdateAssignment {
    /// the assigned columns, holds more than one for the `SET (a, b) = expr` spelling
    pub columns: Vec<String>,
    /// the expression assigned to the columns
    pub expr: Expr,
}

node!(
    Update,
    r"UPDATE statement, see: <https://www.sqlite.org/lang_update.html>

The UPDATE command changes existing rows in a table.

```sql
UPDATE table_name SET column_name = 'value';
UPDATE OR FAIL schema_name.table_name SET name = 'Ada' WHERE id = 1 RETURNING *;
UPDATE users SET (name, email) = user_defaults();
```
",
    /// the resolution of `UPDATE OR <conflict>`, such as ROLLBACK or FAIL
    conflict: Option<Keyword>,
    /// the updated table
    target: QualifiedTableName,
    /// the assignments of the SET clause
    assignments: Vec<UpdateAssignment>,
    /// the WHERE clause
    where_expr: Option<Expr>,
    /// the columns of the RETURNING clause
    returning: Vec<ResultColumn>,
    /// the terms of the ORDER BY clause
    order_by: Vec<OrderingTerm>,
    /// the LIMIT clause and its offset
    limit: Option<LimitOffset>
    ; analyse(crate::analyse::relation::update)
);

node!(
    Insert,
    r"INSERT statement, see: <https://www.sqlite.org/lang_insert.html>

The INSERT command creates new rows in a table.

```sql
INSERT INTO table_name DEFAULT VALUES;
INSERT INTO table_name (id, name) VALUES (1, 'Ada');
INSERT INTO table_name SELECT id, name FROM old_table;
INSERT OR IGNORE INTO schema_name.table_name VALUES (1) RETURNING *;
```
",
    /// the resolution of `INSERT OR <conflict>`, such as IGNORE or REPLACE
    conflict: Option<Keyword>,
    /// the table rows are inserted into
    target: SchemaTableContainer,
    /// the named columns, empty when the statement names none
    columns: Vec<String>,
    /// where the inserted rows come from
    source: InsertSource,
    /// the columns of the RETURNING clause
    returning: Vec<ResultColumn>
    ; analyse(crate::analyse::relation::insert)
);

node!(
    Delete,
    r"DELETE statement, see: <https://www.sqlite.org/lang_delete.html>

The DELETE command removes records from a table.

```sql
DELETE FROM table_name;
DELETE FROM schema_name.table_name WHERE id = 1;
DELETE FROM users AS u INDEXED BY idx_users_id WHERE u.id = 1 RETURNING *;
```
",
    /// the table rows are deleted from
    target: QualifiedTableName,
    /// the WHERE clause
    where_expr: Option<Expr>,
    /// the columns of the RETURNING clause
    returning: Vec<ResultColumn>,
    /// the terms of the ORDER BY clause
    order_by: Vec<OrderingTerm>,
    /// the LIMIT clause and its offset
    limit: Option<LimitOffset>
    ; analyse(crate::analyse::relation::delete)
);

node!(
    Savepoint,
    r"Savepoint stmt, see: <https://www.sqlite.org/lang_savepoint.html>

SAVEPOINTs are a method of creating transactions, similar to BEGIN and COMMIT, except that the SAVEPOINT and RELEASE commands are named and may be nested.

# Examples

```sql
SAVEPOINT savepoint_name;
```
",
    /// name of the created savepoint
    savepoint_name: String
);

node!(
    Release,
    r"Release stmt, see: <https://www.sqlite.org/lang_savepoint.html>

The RELEASE command is like a COMMIT for a SAVEPOINT. The RELEASE command causes all savepoints back to and including the most recent savepoint with a matching name to be removed from the transaction stack.

# Examples

```sql
RELEASE savepoint_name;
RELEASE SAVEPOINT savepoint_name;
```
",
    /// the savepoint to release, together with every savepoint created after it
    savepoint_name: String
);

node!(
    Attach,
    "Attach stmt, see: <https://www.sqlite.org/lang_attach.html>

The ATTACH DATABASE statement adds another database file to the current database connection. Database files that were previously attached can be removed using the DETACH DATABASE command. 

# Examples

```sql
ATTACH DATABASE 'users.db' AS users;
ATTACH 'users.db' AS users;
```
",
    /// the name the database is attached as
    schema_name: String,
    /// the attached database file
    expr: Expr
);

node!(
    Reindex,
    r"Reindex stmt, see: <https://www.sqlite.org/lang_reindex.html>

The REINDEX command is used to delete and recreate indices from scratch. This is useful when the definition of a collation sequence has changed, or when there are indexes on expressions involving a function whose definition has changed.

# Examples

```sql
REINDEX;
REINDEX collation_name;
REINDEX schema_name.table_name;
```
",
    /// the collation, table or index to reindex, everything is reindexed when this is `None`
    target: Option<SchemaTableContainer>
);

node!(
    Alter,
    r"Alter stmt, see: <https://www.sqlite.org/lang_altertable.html>

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
    /// the altered table
    target: SchemaTableContainer,
    /// RENAME TO, the new table name
    rename_to: Option<String>,
    /// the renamed column of `RENAME [COLUMN] <target> TO <new_column_name>`
    rename_column_target: Option<String>,
    /// the new name of [Alter::rename_column_target]
    new_column_name: Option<String>,
    /// `ADD [COLUMN]`, the added column definition
    add_column: Option<ColumnDef>,
    /// `DROP [COLUMN]`, the dropped column
    drop_column: Option<String>
    ; analyse(crate::analyse::create::alter)
);

#[derive(Debug)]
#[cfg_attr(any(test, feature = "serde"), derive(serde::Serialize))]
/// ForeignKeyAction is what happens to a referencing row when the referenced row changes, used as
/// `ON DELETE ForeignKeyAction` and `ON UPDATE ForeignKeyAction`
///
/// see: <https://www.sqlite.org/syntax/foreign-key-clause.html>
pub enum ForeignKeyAction {
    /// CASCADE
    Cascade,
    /// RESTRICT
    Restrict,
    /// NO ACTION
    NoAction,
    /// SET NULL
    SetNull,
    /// SET DEFAULT
    SetDefault,
}

#[derive(Debug)]
#[cfg_attr(any(test, feature = "serde"), derive(serde::Serialize))]
/// SQLite parses MATCH clauses (i.e. does not report a syntax error if you specify one), but does
/// not enforce them. All foreign key constraints in SQLite are handled as if MATCH SIMPLE were
/// specified, see <https://sqlite.org/foreignkeys.html#fk_unsupported>
pub enum ForeignKeyMatch {
    /// MATCH SIMPLE, the behaviour sqlite always applies
    Simple,
    /// MATCH FULL
    Full,
    /// MATCH PARTIAL
    Partial,
}

#[derive(Debug)]
#[cfg_attr(any(test, feature = "serde"), derive(serde::Serialize))]
/// ForeignKeyClause is the REFERENCES clause of a column or table constraint
///
/// see: <https://www.sqlite.org/syntax/foreign-key-clause.html>
pub struct ForeignKeyClause {
    /// the referenced table
    pub foreign_table: String,
    /// the referenced columns, empty when the clause references the primary key
    pub references_columns: Vec<String>,
    /// ON DELETE action
    pub on_delete: Option<ForeignKeyAction>,
    /// ON UPDATE action
    pub on_update: Option<ForeignKeyAction>,
    /// MATCH type, parsed but not enforced by sqlite, see [ForeignKeyMatch]
    pub match_type: Option<ForeignKeyMatch>,
    /// whether the constraint is DEFERRABLE
    pub deferrable: bool,
    /// whether the constraint is INITIALLY DEFERRED
    pub initially_deferred: bool,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
/// ColumnConstraint is a constraint attached to a single column definition
///
/// see: <https://www.sqlite.org/syntax/column-constraint.html>
pub enum ColumnConstraint {
    /// PRIMARY KEY
    PrimaryKey {
        /// either ASC or DESC
        asc_desc: Option<Keyword>,
        /// ON CONFLICT resolution, such as ROLLBACK or IGNORE
        on_conflict: Option<Keyword>,
        /// whether AUTOINCREMENT is specified
        autoincrement: bool,
    },
    /// NOT NULL
    NotNull {
        /// ON CONFLICT resolution, such as ROLLBACK or IGNORE
        on_conflict: Option<Keyword>,
    },
    /// UNIQUE
    Unique {
        /// ON CONFLICT resolution, such as ROLLBACK or IGNORE
        on_conflict: Option<Keyword>,
    },
    /// CHECK (expr)
    Check(Expr),
    /// DEFAULT, either an expression in parentheses or a literal
    Default {
        /// DEFAULT (expr)
        expr: Option<Expr>,
        /// DEFAULT literal
        literal: Option<Literal>,
    },
    /// COLLATE collation_name
    Collate(String),
    /// GENERATED ALWAYS AS (expr)
    Generated {
        /// the expression the column value is computed from
        expr: Expr,
        /// either STORED or VIRTUAL
        stored_virtual: Option<Keyword>,
    },
    /// AS (expr), the GENERATED ALWAYS keywords are optional
    As {
        /// the expression the column value is computed from
        expr: Expr,
        /// either STORED or VIRTUAL
        stored_virtual: Option<Keyword>,
    },
    /// REFERENCES foreign_table
    ForeignKey(ForeignKeyClause),
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
/// TableConstraint is a constraint attached to a table instead of a single column
///
/// see: <https://www.sqlite.org/syntax/table-constraint.html>
pub enum TableConstraint {
    /// PRIMARY KEY (columns)
    PrimaryKey {
        /// the columns forming the primary key
        columns: Vec<IndexedColumn>,
        /// ON CONFLICT resolution, such as ROLLBACK or IGNORE
        on_conflict: Option<Keyword>,
    },
    /// UNIQUE (columns)
    Unique {
        /// the columns that have to be unique together
        columns: Vec<IndexedColumn>,
        /// ON CONFLICT resolution, such as ROLLBACK or IGNORE
        on_conflict: Option<Keyword>,
    },
    /// CHECK (expr)
    Check(Expr),
    /// FOREIGN KEY (columns) REFERENCES foreign_table
    ForeignKey {
        /// the columns of this table the clause applies to
        columns: Vec<String>,
        /// the referenced table and its actions
        foreign_key_clause: ForeignKeyClause,
    },
}

node!(
    ColumnDef,
    "Column definition, see: <https://www.sqlite.org/syntax/column-def.html>",
    /// name of the column
    name: String,
    /// the storage class the declared type maps to, `None` for a column without a type
    ///
    /// see: <https://www.sqlite.org/syntax/type-name.html>
    type_name: Option<SqliteStorageClass>,
    /// the constraints of the column
    constraints: Vec<ColumnConstraint>
    ; analyse(crate::analyse::column::column_def)
);

node!(
    CreateTable,
    r"CREATE TABLE statement, see: <https://www.sqlite.org/lang_createtable.html>

The CREATE TABLE command creates a new table in an SQLite database.

```sql
CREATE TABLE table_name (column_def, ...);
CREATE TEMP TABLE IF NOT EXISTS schema.table_name (column_def, ...);
CREATE TABLE table_name (column_def, ...) STRICT;
```
",
    /// whether TEMP or TEMPORARY is specified
    temporary: bool,
    /// whether IF NOT EXISTS is specified
    if_not_exists: bool,
    /// the created table
    name: SchemaTableContainer,
    /// the column definitions
    columns: Vec<ColumnDef>,
    /// the constraints defined on the table instead of a single column
    table_constraints: Vec<TableConstraint>,
    /// whether the STRICT table option is specified
    strict: bool,
    /// whether the WITHOUT ROWID table option is specified
    without_rowid: bool
    ; analyse(crate::analyse::create::create_table)
);

node!(
    CreateTableAs,
    r"CREATE TABLE AS statement, see: <https://www.sqlite.org/lang_createtable.html>

The CREATE TABLE AS form creates a table from a SELECT statement.

```sql
CREATE TABLE table_name AS SELECT id FROM old_table;
CREATE TEMP TABLE IF NOT EXISTS schema.table_name AS SELECT id FROM old_table;
```
",
    /// whether TEMP or TEMPORARY is specified
    temporary: bool,
    /// whether IF NOT EXISTS is specified
    if_not_exists: bool,
    /// the created table
    name: SchemaTableContainer,
    /// the SELECT the table is filled from, it also defines the columns
    select: Box<Select>
    ; analyse(crate::analyse::create::create_table_as)
);

node!(
    CreateView,
    r"CREATE VIEW statement, see: <https://www.sqlite.org/lang_createview.html>

The CREATE VIEW command creates a named SELECT statement.

```sql
CREATE VIEW view_name AS SELECT id FROM table_name;
CREATE TEMP VIEW IF NOT EXISTS schema.view_name (id) AS SELECT id FROM table_name;
```
",
    /// whether TEMP or TEMPORARY is specified
    temporary: bool,
    /// whether IF NOT EXISTS is specified
    if_not_exists: bool,
    /// the created view
    name: SchemaTableContainer,
    /// the column names the view exposes, empty when it does not rename them
    columns: Vec<String>,
    /// the SELECT the view stands for
    select: Box<Select>
    ; analyse(crate::analyse::create::create_view)
);

node!(
    CreateVirtualTable,
    r"CREATE VIRTUAL TABLE statement, see: <https://www.sqlite.org/lang_createvtab.html>

The CREATE VIRTUAL TABLE command creates a virtual table backed by a module.

```sql
CREATE VIRTUAL TABLE docs USING fts5(content);
CREATE VIRTUAL TABLE spatial_index USING rtree(id, min_x, max_x, min_y, max_y);
```
",
    /// whether TEMP or TEMPORARY is specified
    temporary: bool,
    /// whether IF NOT EXISTS is specified
    if_not_exists: bool,
    /// the created virtual table
    name: SchemaTableContainer,
    /// the module named by USING, such as `fts5`
    module: String,
    /// the arguments passed to the module, these are module defined
    arguments: Vec<Token>
    ; analyse(crate::analyse::create::create_virtual_table)
);

node!(
    With,
    r"WITH statement, see: <https://www.sqlite.org/lang_with.html>

The WITH clause defines common table expressions for a following statement.

```sql
WITH rows AS (SELECT id FROM table_name) SELECT id FROM rows;
WITH RECURSIVE rows(id) AS NOT MATERIALIZED (SELECT 1) SELECT id FROM rows;
```
",
    /// whether RECURSIVE is specified
    recursive: bool,
    /// the common table expressions the statement below can reference
    expressions: Vec<CommonTableExpression>,
    /// the statement the expressions apply to
    child: Box<dyn Node>
    ; analyse(crate::analyse::relation::with)
);

#[derive(Debug)]
#[cfg_attr(any(test, feature = "serde"), derive(serde::Serialize))]
/// IndexedColumn is a column an index or a table constraint is defined on
///
/// see: <https://www.sqlite.org/syntax/indexed-column.html>
pub struct IndexedColumn {
    /// name of the column
    pub name: String,
    /// COLLATE collation_name
    pub collation: Option<String>,
    /// either ASC or DESC
    pub order: Option<Keyword>,
}

node!(
    CreateIndex,
    r"CREATE INDEX statement, see: <https://www.sqlite.org/lang_createindex.html>

The CREATE INDEX command creates a new index for a table.

```sql
CREATE INDEX index_name ON table_name (column_name);
CREATE UNIQUE INDEX IF NOT EXISTS schema.index_name ON table_name (column_name COLLATE collation_name DESC);
```
",
    /// whether UNIQUE is specified
    unique: bool,
    /// whether IF NOT EXISTS is specified
    if_not_exists: bool,
    /// the created index
    name: SchemaTableContainer,
    /// the indexed table
    table: String,
    /// the indexed columns
    columns: Vec<IndexedColumn>
);

#[derive(Debug)]
#[cfg_attr(any(test, feature = "serde"), derive(serde::Serialize))]
/// TriggerTiming is when a trigger fires relative to the event it watches
pub enum TriggerTiming {
    /// BEFORE
    Before,
    /// AFTER
    After,
    /// INSTEAD OF
    InsteadOf,
}

#[derive(Debug)]
#[cfg_attr(any(test, feature = "serde"), derive(serde::Serialize))]
/// TriggerEvent is the event a trigger fires on
pub enum TriggerEvent {
    /// DELETE
    Delete,
    /// INSERT
    Insert,
    /// UPDATE, optionally restricted to columns via UPDATE OF
    Update {
        /// the columns of UPDATE OF, empty for a plain UPDATE
        columns: Vec<String>,
    },
}

#[derive(Debug)]
#[cfg_attr(any(test, feature = "serde"), derive(serde::Serialize))]
/// TriggerBodyStmt is the kind of a statement in a trigger body, the statements themselves are not
/// parsed yet
pub enum TriggerBodyStmt {
    /// DELETE
    Delete,
    /// INSERT
    Insert,
    /// SELECT
    Select,
    /// UPDATE
    Update,
}

node!(
    CreateTrigger,
    r"CREATE TRIGGER statement, see: <https://www.sqlite.org/lang_createtrigger.html>

The CREATE TRIGGER command creates a database trigger.

```sql
CREATE TRIGGER trigger_name AFTER INSERT ON table_name BEGIN SELECT 1; END;
CREATE TEMP TRIGGER IF NOT EXISTS schema.trigger_name INSTEAD OF UPDATE OF column_name ON table_name FOR EACH ROW BEGIN UPDATE table_name SET column_name = 1; END;
```
",
    /// whether TEMP or TEMPORARY is specified
    temporary: bool,
    /// whether IF NOT EXISTS is specified
    if_not_exists: bool,
    /// the created trigger
    name: SchemaTableContainer,
    /// when the trigger fires, such as BEFORE or AFTER
    timing: Option<TriggerTiming>,
    /// the event the trigger fires on
    event: TriggerEvent,
    /// the table the event is watched on
    table: String,
    /// whether FOR EACH ROW is specified
    for_each_row: bool,
    /// whether a WHEN clause is specified, its expression is not kept
    when: bool,
    /// the kinds of the statements in the trigger body, their payloads are not parsed yet
    body: Vec<TriggerBodyStmt>
);

#[derive(Debug)]
#[cfg_attr(any(test, feature = "serde"), derive(serde::Serialize))]
/// PragmaInvocation is how a PRAGMA is invoked
pub enum PragmaInvocation {
    /// `PRAGMA name`, queries the current value
    Query,
    /// `PRAGMA name = value`
    Assign {
        /// the assigned value
        value: Token,
    },
    /// `PRAGMA name(value)`
    Call {
        /// the passed value
        value: Token,
    },
}

node!(
    Pragma,
    r"PRAGMA Statements, see: <https://www.sqlite.org/pragma.html>

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
    /// the pragma, encoded as a [SchemaTableContainer] since pragma names can be
    /// schema_name.pragma_name
    name: SchemaTableContainer,
    /// how the pragma is invoked: queried, assigned or called
    invocation: PragmaInvocation
    ; analyse(crate::analyse::pragma::pragma)
);

#[cfg(feature = "serde")]
impl serde::Serialize for dyn Node {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        macro_rules! serialize_node {
            ($($node:ty),* $(,)?) => {
                $(
                    if let Some(node) = self.as_any().downcast_ref::<$node>() {
                        return serde::Serialize::serialize(node, serializer);
                    }
                )*
            };
        }

        serialize_node!(
            Literal,
            BindParameter,
            Expr,
            Explain,
            Vacuum,
            Begin,
            Commit,
            Rollback,
            Detach,
            Analyze,
            Drop,
            Select,
            Update,
            Insert,
            Delete,
            Savepoint,
            Release,
            Attach,
            Reindex,
            Alter,
            ColumnDef,
            CreateTable,
            CreateTableAs,
            CreateView,
            CreateVirtualTable,
            With,
            CreateIndex,
            CreateTrigger,
            Pragma,
        );

        Err(serde::ser::Error::custom(format!(
            "cannot serialize unknown AST node {}",
            self.name()
        )))
    }
}
