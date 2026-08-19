use std::collections::HashMap;

use crate::{
    parser::nodes::{ColumnDef, SchemaTableContainer},
    types::storage::SqliteStorageClass,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    Table,
    View,
    VirtualTable,
    CommonTableExpression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnKnowledge {
    Complete,
    Partial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaMutationError {
    UnknownRelation,
    NotTable,
    DuplicateColumn,
    UnknownColumn,
    DuplicateRelation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Relation {
    pub name: SchemaTableContainer,
    pub kind: RelationKind,
    pub columns: Vec<Column>,
    pub column_knowledge: ColumnKnowledge,
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
        self.define_relation_with_column_knowledge(
            name,
            kind,
            Vec::new(),
            ColumnKnowledge::Partial,
        );
    }

    pub fn contains_relation(&self, name: &SchemaTableContainer) -> bool {
        self.relation(name).is_some()
    }

    pub fn define_relation_with_columns(
        &mut self,
        name: &SchemaTableContainer,
        kind: RelationKind,
        columns: Vec<Column>,
    ) {
        self.define_relation_with_column_knowledge(name, kind, columns, ColumnKnowledge::Complete);
    }

    fn define_relation_with_column_knowledge(
        &mut self,
        name: &SchemaTableContainer,
        kind: RelationKind,
        columns: Vec<Column>,
        column_knowledge: ColumnKnowledge,
    ) {
        #[cfg(feature = "trace-analysis")]
        crate::analyse::trace::define_relation(name, kind, &columns);

        self.relations.insert(
            relation_key(name),
            Relation {
                name: name.clone(),
                kind,
                columns,
                column_knowledge,
            },
        );
    }

    pub fn relation(&self, name: &SchemaTableContainer) -> Option<&Relation> {
        self.relations.get(&relation_key(name))
    }

    pub fn relation_count(&self) -> usize {
        self.relations
            .values()
            .filter(|relation| relation.kind != RelationKind::CommonTableExpression)
            .count()
    }

    pub fn relations(&self) -> impl Iterator<Item = &Relation> {
        self.relations.values()
    }

    pub fn add_column(
        &mut self,
        name: &SchemaTableContainer,
        column: Column,
    ) -> Result<(), SchemaMutationError> {
        let Some(relation) = self.relations.get_mut(&relation_key(name)) else {
            return Err(SchemaMutationError::UnknownRelation);
        };

        if relation.kind != RelationKind::Table {
            return Err(SchemaMutationError::NotTable);
        }

        if relation
            .columns
            .iter()
            .any(|known| known.name.eq_ignore_ascii_case(&column.name))
        {
            return Err(SchemaMutationError::DuplicateColumn);
        }

        relation.columns.push(column);
        Ok(())
    }

    pub fn rename_column(
        &mut self,
        name: &SchemaTableContainer,
        old_name: &str,
        new_name: &str,
    ) -> Result<(), SchemaMutationError> {
        let Some(relation) = self.relations.get_mut(&relation_key(name)) else {
            return Err(SchemaMutationError::UnknownRelation);
        };

        let Some(index) = relation
            .columns
            .iter()
            .position(|column| column.name.eq_ignore_ascii_case(old_name))
        else {
            return Err(SchemaMutationError::UnknownColumn);
        };

        if relation
            .columns
            .iter()
            .any(|known| known.name.eq_ignore_ascii_case(new_name))
        {
            return Err(SchemaMutationError::DuplicateColumn);
        }

        relation.columns[index].name = new_name.into();
        Ok(())
    }

    pub fn drop_column(
        &mut self,
        name: &SchemaTableContainer,
        column_name: &str,
    ) -> Result<(), SchemaMutationError> {
        let Some(relation) = self.relations.get_mut(&relation_key(name)) else {
            return Err(SchemaMutationError::UnknownRelation);
        };

        let Some(index) = relation
            .columns
            .iter()
            .position(|column| column.name.eq_ignore_ascii_case(column_name))
        else {
            return Err(SchemaMutationError::UnknownColumn);
        };

        relation.columns.remove(index);
        Ok(())
    }

    pub fn rename_relation(
        &mut self,
        old_name: &SchemaTableContainer,
        new_name: &SchemaTableContainer,
    ) -> Result<(), SchemaMutationError> {
        let old_key = relation_key(old_name);
        let new_key = relation_key(new_name);
        if old_key != new_key && self.relations.contains_key(&new_key) {
            return Err(SchemaMutationError::DuplicateRelation);
        }

        let Some(mut relation) = self.relations.remove(&old_key) else {
            return Err(SchemaMutationError::UnknownRelation);
        };
        relation.name = new_name.clone();
        self.relations.insert(new_key, relation);
        Ok(())
    }

    pub(crate) fn with_scoped_relation<T>(
        &mut self,
        name: &SchemaTableContainer,
        kind: RelationKind,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let key = relation_key(name);
        let previous = self.relations.insert(
            key.clone(),
            Relation {
                name: name.clone(),
                kind,
                columns: Vec::new(),
                column_knowledge: ColumnKnowledge::Partial,
            },
        );

        let result = f(self);

        match previous {
            Some(relation) => self.relations.insert(key, relation),
            None => self.relations.remove(&key),
        };

        result
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

#[cfg(test)]
mod tests {
    use crate::{
        analyse::{AnalysisContext, Column, RelationKind},
        parser::nodes::SchemaTableContainer,
        types::storage::SqliteStorageClass,
    };

    #[test]
    fn relation_lookup_is_case_insensitive() {
        let mut context = AnalysisContext::default();
        context.define_relation(
            &SchemaTableContainer::Table("Users".into()),
            RelationKind::Table,
        );

        assert_eq!(
            context
                .relation(&SchemaTableContainer::Table("users".into()))
                .unwrap()
                .kind,
            RelationKind::Table
        );
    }

    #[test]
    fn relation_lookup_keeps_schema_qualified_names_separate() {
        let mut context = AnalysisContext::default();
        context.define_relation(
            &SchemaTableContainer::Table("users".into()),
            RelationKind::Table,
        );
        context.define_relation_with_columns(
            &SchemaTableContainer::SchemaAndTable {
                schema: "main".into(),
                table: "users".into(),
            },
            RelationKind::View,
            vec![Column {
                name: "id".into(),
                type_name: Some(SqliteStorageClass::Integer),
            }],
        );

        assert_eq!(context.relation_count(), 2);
        assert_eq!(
            context
                .relation(&SchemaTableContainer::Table("users".into()))
                .unwrap()
                .kind,
            RelationKind::Table
        );
        assert_eq!(
            context
                .relation(&SchemaTableContainer::SchemaAndTable {
                    schema: "MAIN".into(),
                    table: "USERS".into(),
                })
                .unwrap()
                .columns,
            vec![Column {
                name: "id".into(),
                type_name: Some(SqliteStorageClass::Integer),
            }]
        );
    }

    #[test]
    fn scoped_relation_is_removed_after_callback() {
        let mut context = AnalysisContext::default();
        let name = SchemaTableContainer::Table("rows".into());

        context.with_scoped_relation(&name, RelationKind::CommonTableExpression, |context| {
            assert!(context.contains_relation(&name));
            assert_eq!(context.relation_count(), 0);
        });

        assert!(!context.contains_relation(&name));
    }

    #[test]
    fn scoped_relation_restores_shadowed_relation() {
        let mut context = AnalysisContext::default();
        let name = SchemaTableContainer::Table("rows".into());
        context.define_relation(&name, RelationKind::Table);

        context.with_scoped_relation(&name, RelationKind::CommonTableExpression, |context| {
            assert_eq!(
                context.relation(&name).unwrap().kind,
                RelationKind::CommonTableExpression
            );
        });

        assert_eq!(context.relation(&name).unwrap().kind, RelationKind::Table);
    }
}
