use crate::{
    parser::nodes::*,
    types::{Keyword, Token, storage::SqliteStorageClass},
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
    SchemaTableContainer
);

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
