use crate::{
    parser::nodes::*,
    types::{Keyword, Token, storage::SqliteStorageClass},
};

pub trait FieldSerializable {
    fn field_as_serializable(&self) -> serde_json::Value;
}

impl FieldSerializable for Box<dyn Node> {
    fn field_as_serializable(&self) -> serde_json::Value {
        self.as_serializable()
    }
}

impl FieldSerializable for String {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

impl FieldSerializable for bool {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

impl FieldSerializable for Token {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::to_value(&self.ttype).unwrap()
    }
}

impl FieldSerializable for Keyword {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

impl FieldSerializable for SqliteStorageClass {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
}

impl FieldSerializable for SchemaTableContainer {
    fn field_as_serializable(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
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
