use crate::{error::Error, parser::nodes::ColumnDef, types::rules::Rule};

pub fn column_def(file: &str, column: &ColumnDef) -> Vec<Error> {
    if column.type_name.is_some() {
        return Vec::new();
    }

    vec![
        Error::new(
            file,
            column.location,
            Rule::Quirk,
            format!("Column `{}` has no declared type", column.name),
            "SQLite allows columns without a declared type. Such columns use dynamic typing and type affinity is not enforced. Consider adding TEXT, BLOB, REAL, or INTEGER if this is unintended.",
        )
        .with_doc_url("https://www.sqlite.org/quirks.html#the_datatype_is_optional"),
    ]
}

#[cfg(test)]
mod tests {
    use crate::{
        analyse::column::column_def,
        parser::nodes::ColumnDef,
        types::{rules::Rule, storage::SqliteStorageClass},
    };

    #[test]
    fn reports_columns_without_declared_type() {
        let column = ColumnDef::new("name".into(), None, vec![]);

        let diagnostics = column_def("test.sql", &column);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].file, "test.sql");
        assert_eq!(diagnostics[0].rule, Rule::Quirk);
        assert_eq!(diagnostics[0].msg, "Column `name` has no declared type");
        assert!(
            diagnostics[0]
                .note
                .contains("SQLite allows columns without a declared type")
        );
        assert_eq!(
            diagnostics[0].doc_url,
            Some("https://www.sqlite.org/quirks.html#the_datatype_is_optional")
        );
    }

    #[test]
    fn accepts_columns_with_declared_type() {
        let column = ColumnDef::new("name".into(), Some(SqliteStorageClass::Text), vec![]);

        let diagnostics = column_def("test.sql", &column);

        assert!(diagnostics.is_empty());
    }
}
