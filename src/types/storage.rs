use std::fmt::Display;

use serde::Serialize;

/// see: https://sqlite.org/datatype3.html#storage_classes_and_datatypes
#[derive(Debug, Serialize)]
pub enum SqliteStorageClass {
    Null,
    Integer,
    Real,
    Text,
    Blob,
}

trait StrExtension {
    /// returns if s contains any of the elements of v
    fn contains_any(self, v: Vec<&str>) -> bool;
}

impl StrExtension for &str {
    fn contains_any(self, v: Vec<&str>) -> bool {
        for e in v {
            if self.contains(e) {
                return true;
            }
        }
        false
    }
}

impl Display for SqliteStorageClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Self::Null => write!(f, "SqliteStorageClass::Null"),
            Self::Real => write!(f, "SqliteStorageClass::Real"),
            Self::Text => write!(f, "SqliteStorageClass::Text"),
            Self::Blob => write!(f, "SqliteStorageClass::Blob"),
            Self::Integer => write!(f, "SqliteStorageClass::Integer"),
        }
    }
}

impl SqliteStorageClass {
    /// https://sqlite.org/datatype3.html#determination_of_column_affinity
    pub fn from_str(s: &str) -> Self {
        if s.contains_any(vec!["VARCHAR", "CLOB", "TEXT"]) {
            Self::Text
        } else if s.is_empty() || s.contains("BLOB") {
            Self::Blob
        } else if s.contains_any(vec!["REAL", "FLOA", "DOUB"]) {
            Self::Real
        } else if s.contains("INT") {
            Self::Integer
        } else {
            // includes TRUE, FALSE and anything else
            Self::Integer
        }
    }

    pub fn from_str_strict(s: &str) -> Option<Self> {
        Some(match s {
            "TEXT" => Self::Text,
            "BLOB" => Self::Blob,
            "REAL" => Self::Real,
            "INT" | "INTEGER" => Self::Integer,
            _ => {
                if s.contains("VARCHAR") {
                    Self::Text
                } else {
                    return None;
                }
            }
        })
    }
}
