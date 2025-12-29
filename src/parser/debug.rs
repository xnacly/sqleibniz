use crate::{
    parser::nodes::*,
    types::{Keyword, Token, Type, storage::SqliteStorageClass},
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

impl_field_serializable_with_serde_to_value!(
    String,
    bool,
    Keyword,
    SqliteStorageClass,
    SchemaTableContainer,
    Type
);

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

impl FieldSerializable for Token {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::to_value(&self.ttype).unwrap()
    }
}

impl FieldSerializable for Box<dyn Node> {
    fn field_as_serializable(&self) -> serde_json::Value {
        self.as_serializable()
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
