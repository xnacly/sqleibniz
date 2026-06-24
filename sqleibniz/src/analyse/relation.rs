use crate::{
    analyse::{AnalysisContext, RelationKind},
    error::{Error, Location},
    parser::nodes::{
        Delete, Insert, InsertSource, Node, QualifiedTableName, SchemaTableContainer, Select,
        SelectSource, Update, With,
    },
    types::rules::Rule,
};

pub fn select(file: &str, context: &mut AnalysisContext, select: &Select) -> Vec<Error> {
    select
        .from
        .iter()
        .flat_map(|source| select_source_diagnostics(file, context, source, select.location()))
        .collect()
}

pub fn insert(file: &str, context: &mut AnalysisContext, insert: &Insert) -> Vec<Error> {
    let mut diagnostics = relation_diagnostic(file, context, &insert.target, insert.location());

    if let InsertSource::Select(select) = &insert.source {
        diagnostics.extend(select.analyse(file, context));
    }

    diagnostics
}

pub fn update(file: &str, context: &mut AnalysisContext, update: &Update) -> Vec<Error> {
    qualified_table_diagnostic(file, context, &update.target, update.location())
}

pub fn delete(file: &str, context: &mut AnalysisContext, delete: &Delete) -> Vec<Error> {
    qualified_table_diagnostic(file, context, &delete.target, delete.location())
}

pub fn with(file: &str, context: &mut AnalysisContext, with: &With) -> Vec<Error> {
    let mut diagnostics = with
        .expressions
        .iter()
        .flat_map(|expression| expression.select.analyse(file, context))
        .collect::<Vec<_>>();

    diagnostics.extend(analyse_with_child(file, context, with, 0));

    diagnostics
}

fn analyse_with_child(
    file: &str,
    context: &mut AnalysisContext,
    with: &With,
    index: usize,
) -> Vec<Error> {
    let Some(expression) = with.expressions.get(index) else {
        return with.child.analyse(file, context);
    };

    context.with_scoped_relation(
        &SchemaTableContainer::Table(expression.name.clone()),
        RelationKind::CommonTableExpression,
        |context| analyse_with_child(file, context, with, index + 1),
    )
}

fn select_source_diagnostics(
    file: &str,
    context: &AnalysisContext,
    source: &SelectSource,
    location: Location,
) -> Vec<Error> {
    match source {
        SelectSource::Table(table) => relation_diagnostic(file, context, &table.name, location),
        SelectSource::Join { left, right, .. } => {
            let mut diagnostics = select_source_diagnostics(file, context, left, location);
            diagnostics.extend(relation_diagnostic(file, context, &right.name, location));
            diagnostics
        }
    }
}

fn qualified_table_diagnostic(
    file: &str,
    context: &AnalysisContext,
    table: &QualifiedTableName,
    location: Location,
) -> Vec<Error> {
    relation_diagnostic(file, context, &table.name, location)
}

fn relation_diagnostic(
    file: &str,
    context: &AnalysisContext,
    relation: &SchemaTableContainer,
    location: Location,
) -> Vec<Error> {
    if context.relation_count() == 0 || context.contains_relation(relation) {
        return Vec::new();
    }

    vec![
        Error::new(
            file,
            location,
            Rule::UnknownRelation,
            format!("Relation `{}` is not defined", relation_display(relation)),
            "sqleibniz only checks relation names after this file defines at least one table, view, or virtual table.",
        )
        .with_doc_url("https://www.sqlite.org/syntax/table-or-subquery.html"),
    ]
}

fn relation_display(name: &SchemaTableContainer) -> String {
    match name {
        SchemaTableContainer::Table(table) => table.clone(),
        SchemaTableContainer::SchemaAndTable { schema, table } => format!("{schema}.{table}"),
    }
}
