use std::fmt::Display;

use serde::Serialize;

/// see: https://sqlite.org/datatype3.html#storage_classes_and_datatypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SqliteStorageClass {
    Null,
    Integer,
    Real,
    Text,
    Blob,
    Any,
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
            Self::Null => write!(f, "NULL"),
            Self::Real => write!(f, "REAL"),
            Self::Text => write!(f, "TEXT"),
            Self::Blob => write!(f, "BLOB"),
            Self::Any => write!(f, "ANY"),
            Self::Integer => write!(f, "INTEGER"),
        }
    }
}

impl SqliteStorageClass {
    /// https://sqlite.org/datatype3.html#determination_of_column_affinity
    pub fn from_str(s: &str) -> Self {
        if s.contains_any(vec!["VARCHAR", "CLOB", "TEXT"]) {
            Self::Text
        } else if s == "ANY" {
            Self::Any
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
            "ANY" => Self::Any,
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
