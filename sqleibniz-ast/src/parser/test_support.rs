use crate::{
    parser::nodes::*,
    types::{Keyword, Token, Type, storage::SqliteStorageClass},
};

/// impl FieldSerializable for $tt via serde_json::json!(format!("{self:?}"))
macro_rules! impl_field_serializable_with_serde_to_value {
    ($($tt:tt),*) => {
        $(
            impl FieldSerializable for $tt {
                fn field_as_serializable(&self) -> serde_json::Value {
                    serde_json::json!(format!("{self:?}"))
                }
            }
        )*
    };
}

pub trait FieldSerializable {
    fn field_as_serializable(&self) -> serde_json::Value;
}

impl_field_serializable_with_serde_to_value!(
    String,
    bool,
    Keyword,
    SqliteStorageClass,
    SchemaTableContainer,
    Type,
    IndexedColumn,
    QualifiedTableIndex,
    SelectQuantifier,
    JoinOperator,
    TriggerTiming,
    TriggerEvent,
    TriggerBodyStmt
);

impl FieldSerializable for PragmaInvocation {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

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
                "expr": expr.as_test_serializable(),
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
                    "on": on.as_ref().map(|expr| expr.as_test_serializable()),
                }
            }),
        }
    }
}

impl FieldSerializable for OrderingTerm {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::json!({
            "expr": self.expr.as_test_serializable(),
            "order": self.order,
            "nulls": self.nulls,
        })
    }
}

impl FieldSerializable for LimitOffset {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::json!({
            "limit": self.limit.as_test_serializable(),
            "offset": self.offset.as_ref().map(|expr| expr.as_test_serializable()),
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
                            row.iter().map(|expr| expr.as_test_serializable()).collect(),
                        )
                    })
                    .collect(),
            ),
            InsertSource::Select(select) => serde_json::json!({
                "select": select.as_test_serializable(),
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
            "select": self.select.as_test_serializable(),
        })
    }
}

impl FieldSerializable for UpdateAssignment {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::json!({
            "columns": self.columns,
            "expr": self.expr.as_test_serializable(),
        })
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
                "expr": expr.as_test_serializable(),
            }),
            ColumnConstraint::Default { expr, literal } => {
                serde_json::json!({
                    "expr": match expr {
                        Some(e) => e.as_test_serializable(),
                        None => serde_json::Value::Null,
                    },
                    "literal": match literal {
                        Some(e) => e.as_test_serializable(),
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
                "expr": expr.as_test_serializable(),
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
                    "expr": expr.as_test_serializable(),
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
        serde_json::json!(format!("{:?}", self.ttype))
    }
}

impl<T: Node + ?Sized> FieldSerializable for Box<T> {
    fn field_as_serializable(&self) -> serde_json::Value {
        self.as_test_serializable()
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

impl<T: FieldSerializable> FieldSerializable for Vec<T> {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::Value::Array(self.iter().map(|n| n.field_as_serializable()).collect())
    }
}
