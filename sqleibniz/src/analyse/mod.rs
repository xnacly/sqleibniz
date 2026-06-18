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

#[cfg(feature = "trace-analysis")]
pub(crate) mod trace {
    use std::cell::Cell;

    use crate::{
        analyse::{AnalysisContext, Column, RelationKind},
        error::{Error, Location},
        parser::nodes::SchemaTableContainer,
    };

    thread_local! {
        static ANALYSIS_DEPTH: Cell<usize> = const { Cell::new(0) };
    }

    pub(crate) fn run_start(file: &str, nodes: usize) {
        eprintln!("{:=^72}", " ANALYSIS ");
        eprintln!("analysis: file={file:?} nodes={nodes}");
    }

    pub(crate) fn run_end(diagnostics: usize, context: &AnalysisContext) {
        eprintln!(
            "analysis: complete diagnostics={} relations={}",
            diagnostics,
            context.relations.len()
        );
    }

    pub(crate) fn enter_node() {
        ANALYSIS_DEPTH.with(|depth| {
            depth.set(depth.get() + 1);
        });
    }

    pub(crate) fn exit_node(name: &str, location: Location, diagnostics: &[Error]) {
        ANALYSIS_DEPTH.with(|depth| {
            let next = depth.get().saturating_sub(1);
            depth.set(next);

            if !diagnostics.is_empty() {
                let indent = "  ".repeat(next);
                eprintln!(
                    "{indent}diagnostic: node={name} location={location:?} count={}",
                    diagnostics.len()
                );
                for diagnostic in diagnostics {
                    eprintln!(
                        "{indent}  -> {}: {}",
                        diagnostic.rule.name(),
                        diagnostic.msg
                    );
                }
            }
        });
    }

    pub(crate) fn define_relation(
        name: &SchemaTableContainer,
        kind: RelationKind,
        columns: &[Column],
    ) {
        ANALYSIS_DEPTH.with(|depth| {
            let indent = "  ".repeat(depth.get());
            eprintln!(
                "{indent}context: define relation name={name:?} kind={kind:?} columns={columns:?}"
            );
        });
    }
}

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
        #[cfg(feature = "trace-analysis")]
        trace::define_relation(name, kind, &columns);

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

    #[cfg(feature = "trace-analysis")]
    trace::run_start(file, ast.len());

    let diagnostics: Vec<Error> = ast
        .iter()
        .flat_map(|node| node.analyse(file, &mut context))
        .collect();

    #[cfg(feature = "trace-analysis")]
    trace::run_end(diagnostics.len(), &context);

    diagnostics
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
