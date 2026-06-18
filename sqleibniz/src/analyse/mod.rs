/// Column definition analysis rules.
pub mod column;
/// CREATE statement analysis rules.
pub mod create;
/// PRAGMA statement analysis rules.
pub mod pragma;

use std::collections::HashMap;

use crate::{
    error::Error,
    parser::nodes::{ColumnDef, Node, SchemaTableContainer},
    types::storage::SqliteStorageClass,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    Table,
    View,
    VirtualTable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relation {
    pub name: SchemaTableContainer,
    pub kind: RelationKind,
    pub columns: Vec<Column>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub name: String,
    pub type_name: Option<SqliteStorageClass>,
}

#[derive(Debug, Default)]
pub struct AnalysisContext {
    relations: HashMap<String, Relation>,
}

impl AnalysisContext {
    pub fn define_relation(&mut self, name: &SchemaTableContainer, kind: RelationKind) {
        self.define_relation_with_columns(name, kind, Vec::new());
    }

    pub fn define_relation_with_columns(
        &mut self,
        name: &SchemaTableContainer,
        kind: RelationKind,
        columns: Vec<Column>,
    ) {
        self.relations.insert(
            relation_key(name),
            Relation {
                name: name.clone(),
                kind,
                columns,
            },
        );
    }

    pub fn relation(&self, name: &SchemaTableContainer) -> Option<&Relation> {
        self.relations.get(&relation_key(name))
    }

    pub fn relations(&self) -> impl Iterator<Item = &Relation> {
        self.relations.values()
    }
}

impl From<&ColumnDef> for Column {
    fn from(column: &ColumnDef) -> Self {
        Self {
            name: column.name.clone(),
            type_name: column.type_name,
        }
    }
}

pub fn run(file: &str, ast: &[Box<dyn Node>]) -> Vec<Error> {
    let mut context = AnalysisContext::default();

    ast.iter()
        .flat_map(|node| node.analyse(file, &mut context))
        .collect()
}

fn relation_key(name: &SchemaTableContainer) -> String {
    match name {
        SchemaTableContainer::Table(table) => table.to_ascii_lowercase(),
        SchemaTableContainer::SchemaAndTable { schema, table } => {
            format!(
                "{}.{}",
                schema.to_ascii_lowercase(),
                table.to_ascii_lowercase()
            )
        }
    }
}
