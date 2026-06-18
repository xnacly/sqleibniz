use crate::{
    parser::nodes::*,
    types::{Keyword, Token, Type, ctx::HookContext, storage::SqliteStorageClass},
};

/// impl FieldSerializable for $tt via serde_json::to_value(self).unwrap()
macro_rules! impl_field_serializable_with_serde_to_value {
    ($($tt:tt),*) => {
        $(
            impl FieldSerializable for $tt {
                fn field_as_serializable(&self) -> serde_json::Value {
                    serde_json::to_value(self).unwrap()
                }
            }
        )*
    };
}

pub trait FieldSerializable {
    fn field_as_serializable(&self) -> serde_json::Value;
}

pub trait FieldHookContexts {
    fn field_hook_contexts(&self) -> Vec<HookContext>;
}

pub trait FieldDiagnostics {
    fn field_diagnostics(
        &self,
        file: &str,
        context: &mut crate::analyse::AnalysisContext,
    ) -> Vec<crate::error::Error>;
}

macro_rules! impl_empty_field_hook_contexts {
    ($($tt:ty),*) => {
        $(
            impl FieldHookContexts for $tt {
                fn field_hook_contexts(&self) -> Vec<HookContext> {
                    vec![]
                }
            }
        )*
    };
}

macro_rules! impl_empty_field_diagnostics {
    ($($tt:ty),*) => {
        $(
            impl FieldDiagnostics for $tt {
                fn field_diagnostics(
                    &self,
                    file: &str,
                    context: &mut crate::analyse::AnalysisContext,
                ) -> Vec<crate::error::Error> {
                    let _ = file;
                    let _ = context;
                    vec![]
                }
            }
        )*
    };
}

impl_field_serializable_with_serde_to_value!(
    String,
    bool,
    Keyword,
    SqliteStorageClass,
    SchemaTableContainer,
    Type,
    PragmaInvocation,
    IndexedColumn,
    QualifiedTableIndex,
    SelectQuantifier,
    JoinOperator,
    TriggerTiming,
    TriggerEvent,
    TriggerBodyStmt
);

impl FieldSerializable for QualifiedTableName {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "alias": self.alias,
            "index": self.index,
        })
    }
}

impl FieldSerializable for ResultColumn {
    fn field_as_serializable(&self) -> serde_json::Value {
        match self {
            ResultColumn::Star => serde_json::json!({ "star": true }),
            ResultColumn::TableStar(table) => serde_json::json!({ "table_star": table }),
            ResultColumn::Expr { expr, alias } => serde_json::json!({
                "expr": expr.as_serializable(),
                "alias": alias,
            }),
        }
    }
}

impl FieldSerializable for SelectTable {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "alias": self.alias,
        })
    }
}

impl FieldSerializable for SelectSource {
    fn field_as_serializable(&self) -> serde_json::Value {
        match self {
            SelectSource::Table(table) => serde_json::json!({
                "table": table.field_as_serializable(),
            }),
            SelectSource::Join {
                left,
                operator,
                right,
                on,
            } => serde_json::json!({
                "join": {
                    "left": left.field_as_serializable(),
                    "operator": operator,
                    "right": right.field_as_serializable(),
                    "on": on.as_ref().map(|expr| expr.as_serializable()),
                }
            }),
        }
    }
}

impl FieldSerializable for OrderingTerm {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::json!({
            "expr": self.expr.as_serializable(),
            "order": self.order,
            "nulls": self.nulls,
        })
    }
}

impl FieldSerializable for LimitOffset {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::json!({
            "limit": self.limit.as_serializable(),
            "offset": self.offset.as_ref().map(|expr| expr.as_serializable()),
        })
    }
}

impl FieldSerializable for InsertSource {
    fn field_as_serializable(&self) -> serde_json::Value {
        match self {
            InsertSource::DefaultValues => serde_json::json!({ "default_values": true }),
            InsertSource::Values(rows) => serde_json::Value::Array(
                rows.iter()
                    .map(|row| {
                        serde_json::Value::Array(
                            row.iter().map(|expr| expr.as_serializable()).collect(),
                        )
                    })
                    .collect(),
            ),
            InsertSource::Select(select) => serde_json::json!({
                "select": select.as_serializable(),
            }),
        }
    }
}

impl FieldSerializable for CommonTableExpression {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "columns": self.columns,
            "materialized": self.materialized,
            "select": self.select.as_serializable(),
        })
    }
}

impl FieldSerializable for UpdateAssignment {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::json!({
            "columns": self.columns,
            "expr": self.expr.as_serializable(),
        })
    }
}

impl FieldHookContexts for InsertSource {
    fn field_hook_contexts(&self) -> Vec<HookContext> {
        match self {
            InsertSource::DefaultValues => vec![],
            InsertSource::Values(rows) => rows.field_hook_contexts(),
            InsertSource::Select(select) => vec![select.as_hook_context()],
        }
    }
}

impl FieldDiagnostics for InsertSource {
    fn field_diagnostics(
        &self,
        file: &str,
        context: &mut crate::analyse::AnalysisContext,
    ) -> Vec<crate::error::Error> {
        match self {
            InsertSource::DefaultValues | InsertSource::Values(_) => vec![],
            InsertSource::Select(select) => select.analyse(file, context),
        }
    }
}

impl FieldHookContexts for CommonTableExpression {
    fn field_hook_contexts(&self) -> Vec<HookContext> {
        vec![self.select.as_hook_context()]
    }
}

impl FieldDiagnostics for CommonTableExpression {
    fn field_diagnostics(
        &self,
        file: &str,
        context: &mut crate::analyse::AnalysisContext,
    ) -> Vec<crate::error::Error> {
        self.select.analyse(file, context)
    }
}

impl FieldHookContexts for UpdateAssignment {
    fn field_hook_contexts(&self) -> Vec<HookContext> {
        self.expr.field_hook_contexts()
    }
}

impl FieldSerializable for ColumnConstraint {
    fn field_as_serializable(&self) -> serde_json::Value {
        let name = match self {
            ColumnConstraint::PrimaryKey { .. } => "primary_key",
            ColumnConstraint::NotNull { .. } => "not_null",
            ColumnConstraint::Unique { .. } => "unique",
            ColumnConstraint::Check(_) => "check",
            ColumnConstraint::Default { .. } => "default",
            ColumnConstraint::Collate(_) => "collate",
            ColumnConstraint::Generated { .. } => "generated",
            ColumnConstraint::As { .. } => "as",
            ColumnConstraint::ForeignKey(_) => "foreign_key",
        };
        let inner = match self {
            ColumnConstraint::PrimaryKey {
                asc_desc,
                on_conflict,
                autoincrement,
            } => {
                serde_json::json!( {
                    "asc_desc": asc_desc,
                    "on_conflict": on_conflict,
                    "autoincrement": autoincrement
                })
            }
            ColumnConstraint::Unique { on_conflict }
            | ColumnConstraint::NotNull { on_conflict } => {
                serde_json::json!({
                   "on_conflict": on_conflict
                })
            }

            ColumnConstraint::ForeignKey(foreign_key_clause) => {
                serde_json::json!({
                   "foreign_key_clause": foreign_key_clause
                })
            }
            ColumnConstraint::Collate(str) => serde_json::json!(str),
            ColumnConstraint::Check(expr) => serde_json::json!({
                "expr": expr.as_serializable(),
            }),
            ColumnConstraint::Default { expr, literal } => {
                serde_json::json!({
                    "expr": match expr {
                        Some(e) => e.as_serializable(),
                        None => serde_json::Value::Null,
                    },
                    "literal": match literal {
                        Some(e) => e.as_serializable(),
                        None => serde_json::Value::Null,
                    },
                })
            }
            ColumnConstraint::Generated {
                expr,
                stored_virtual,
            }
            | ColumnConstraint::As {
                expr,
                stored_virtual,
            } => serde_json::json!({
                "expr": expr.as_serializable(),
                "stored_virtual": stored_virtual,
            }),
        };
        serde_json::json!({
            name: inner
        })
    }
}

impl FieldSerializable for TableConstraint {
    fn field_as_serializable(&self) -> serde_json::Value {
        match self {
            TableConstraint::PrimaryKey {
                columns,
                on_conflict,
            } => serde_json::json!({
                "primary_key": {
                    "columns": columns,
                    "on_conflict": on_conflict,
                }
            }),
            TableConstraint::Unique {
                columns,
                on_conflict,
            } => serde_json::json!({
                "unique": {
                    "columns": columns,
                    "on_conflict": on_conflict,
                }
            }),
            TableConstraint::Check(expr) => serde_json::json!({
                "check": {
                    "expr": expr.as_serializable(),
                }
            }),
            TableConstraint::ForeignKey {
                columns,
                foreign_key_clause,
            } => serde_json::json!({
                "foreign_key": {
                    "columns": columns,
                    "foreign_key_clause": foreign_key_clause,
                }
            }),
        }
    }
}

impl FieldSerializable for Token {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::to_value(&self.ttype).unwrap()
    }
}

impl FieldHookContexts for Token {
    fn field_hook_contexts(&self) -> Vec<HookContext> {
        vec![HookContext::token(self)]
    }
}

impl<T: Node + ?Sized> FieldSerializable for Box<T> {
    fn field_as_serializable(&self) -> serde_json::Value {
        self.as_serializable()
    }
}

impl<T: Node + ?Sized> FieldHookContexts for Box<T> {
    fn field_hook_contexts(&self) -> Vec<HookContext> {
        vec![self.as_hook_context()]
    }
}

impl<T: Node + ?Sized> FieldDiagnostics for Box<T> {
    fn field_diagnostics(
        &self,
        file: &str,
        context: &mut crate::analyse::AnalysisContext,
    ) -> Vec<crate::error::Error> {
        self.analyse(file, context)
    }
}

impl<T: FieldSerializable> FieldSerializable for Option<T> {
    fn field_as_serializable(&self) -> serde_json::Value {
        match self {
            Some(n) => n.field_as_serializable(),
            None => serde_json::Value::Null,
        }
    }
}

impl<T: FieldHookContexts> FieldHookContexts for Option<T> {
    fn field_hook_contexts(&self) -> Vec<HookContext> {
        match self {
            Some(n) => n.field_hook_contexts(),
            None => vec![],
        }
    }
}

impl<T: FieldDiagnostics> FieldDiagnostics for Option<T> {
    fn field_diagnostics(
        &self,
        file: &str,
        context: &mut crate::analyse::AnalysisContext,
    ) -> Vec<crate::error::Error> {
        match self {
            Some(n) => n.field_diagnostics(file, context),
            None => vec![],
        }
    }
}

impl<T: FieldSerializable> FieldSerializable for Vec<T> {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::Value::Array(self.iter().map(|n| n.field_as_serializable()).collect())
    }
}

impl<T: FieldHookContexts> FieldHookContexts for Vec<T> {
    fn field_hook_contexts(&self) -> Vec<HookContext> {
        self.iter().flat_map(|n| n.field_hook_contexts()).collect()
    }
}

impl<T: FieldDiagnostics> FieldDiagnostics for Vec<T> {
    fn field_diagnostics(
        &self,
        file: &str,
        context: &mut crate::analyse::AnalysisContext,
    ) -> Vec<crate::error::Error> {
        self.iter()
            .flat_map(|n| n.field_diagnostics(file, context))
            .collect()
    }
}

impl_empty_field_hook_contexts!(
    String,
    bool,
    Keyword,
    SqliteStorageClass,
    SchemaTableContainer,
    IndexedColumn,
    QualifiedTableIndex,
    SelectQuantifier,
    JoinOperator,
    SelectTable,
    QualifiedTableName,
    TriggerTiming,
    TriggerEvent,
    TriggerBodyStmt
);

impl_empty_field_diagnostics!(
    String,
    bool,
    Token,
    Keyword,
    SqliteStorageClass,
    BindParameter,
    Expr,
    SchemaTableContainer,
    QualifiedTableName,
    QualifiedTableIndex,
    IndexedColumn,
    ResultColumn,
    SelectQuantifier,
    SelectSource,
    SelectTable,
    JoinOperator,
    OrderingTerm,
    LimitOffset,
    UpdateAssignment,
    ColumnConstraint,
    TableConstraint,
    ForeignKeyClause,
    TriggerTiming,
    TriggerEvent,
    TriggerBodyStmt,
    PragmaInvocation
);

impl FieldHookContexts for ResultColumn {
    fn field_hook_contexts(&self) -> Vec<HookContext> {
        match self {
            ResultColumn::Expr { expr, .. } => expr.field_hook_contexts(),
            ResultColumn::Star | ResultColumn::TableStar(_) => vec![],
        }
    }
}

impl FieldHookContexts for SelectSource {
    fn field_hook_contexts(&self) -> Vec<HookContext> {
        match self {
            SelectSource::Table(_) => vec![],
            SelectSource::Join { left, on, .. } => {
                let mut contexts = left.field_hook_contexts();
                if let Some(on) = on {
                    contexts.extend(on.field_hook_contexts());
                }
                contexts
            }
        }
    }
}

impl FieldHookContexts for OrderingTerm {
    fn field_hook_contexts(&self) -> Vec<HookContext> {
        self.expr.field_hook_contexts()
    }
}

impl FieldHookContexts for LimitOffset {
    fn field_hook_contexts(&self) -> Vec<HookContext> {
        let mut contexts = self.limit.field_hook_contexts();
        if let Some(offset) = &self.offset {
            contexts.extend(offset.field_hook_contexts());
        }
        contexts
    }
}

impl FieldHookContexts for PragmaInvocation {
    fn field_hook_contexts(&self) -> Vec<HookContext> {
        match self {
            PragmaInvocation::Assign { value } | PragmaInvocation::Call { value } => {
                value.field_hook_contexts()
            }
            PragmaInvocation::Query => vec![],
        }
    }
}

impl FieldHookContexts for ColumnConstraint {
    fn field_hook_contexts(&self) -> Vec<HookContext> {
        match self {
            ColumnConstraint::Check(expr)
            | ColumnConstraint::Default {
                expr: Some(expr), ..
            }
            | ColumnConstraint::Generated { expr, .. }
            | ColumnConstraint::As { expr, .. } => expr.field_hook_contexts(),
            ColumnConstraint::Default {
                literal: Some(literal),
                ..
            } => literal.field_hook_contexts(),
            _ => vec![],
        }
    }
}

impl FieldHookContexts for TableConstraint {
    fn field_hook_contexts(&self) -> Vec<HookContext> {
        match self {
            TableConstraint::Check(expr) => expr.field_hook_contexts(),
            _ => vec![],
        }
    }
}
