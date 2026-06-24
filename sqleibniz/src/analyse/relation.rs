use crate::{
    analyse::{AnalysisContext, Relation, RelationKind},
    error::{Error, Location},
    parser::nodes::{
        Delete, Expr, Insert, InsertSource, Node, QualifiedTableName, ResultColumn,
        SchemaTableContainer, Select, SelectSource, SelectTable, Update, With,
    },
    types::rules::Rule,
};

pub fn select(file: &str, context: &mut AnalysisContext, select: &Select) -> Vec<Error> {
    let mut diagnostics = select
        .from
        .iter()
        .flat_map(|source| select_source_diagnostics(file, context, source, select.location()))
        .collect::<Vec<_>>();

    let scope = SelectScope::from_select(context, select);
    diagnostics.extend(select_column_diagnostics(file, &scope, select));

    diagnostics
}

pub fn insert(file: &str, context: &mut AnalysisContext, insert: &Insert) -> Vec<Error> {
    let mut diagnostics = relation_diagnostic(file, context, &insert.target, insert.location());
    diagnostics.extend(column_list_diagnostics(
        file,
        context,
        &insert.target,
        &insert.columns,
        insert.location(),
    ));

    if let InsertSource::Select(select) = &insert.source {
        diagnostics.extend(select.analyse(file, context));
    }

    diagnostics
}

pub fn update(file: &str, context: &mut AnalysisContext, update: &Update) -> Vec<Error> {
    let mut diagnostics =
        qualified_table_diagnostic(file, context, &update.target, update.location());
    diagnostics.extend(update.assignments.iter().flat_map(|assignment| {
        column_list_diagnostics(
            file,
            context,
            &update.target.name,
            &assignment.columns,
            update.location(),
        )
    }));
    diagnostics
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

fn column_list_diagnostics(
    file: &str,
    context: &AnalysisContext,
    relation_name: &SchemaTableContainer,
    columns: &[String],
    location: Location,
) -> Vec<Error> {
    let Some(relation) = context.relation(relation_name) else {
        return Vec::new();
    };

    if !can_validate_columns(relation) {
        return Vec::new();
    }

    columns
        .iter()
        .filter(|column| !relation_has_column(relation, column))
        .map(|column| unknown_column_diagnostic(file, relation_name, column, location))
        .collect()
}

fn select_column_diagnostics(file: &str, scope: &SelectScope, select: &Select) -> Vec<Error> {
    select
        .columns
        .iter()
        .flat_map(|column| result_column_diagnostics(file, scope, column))
        .chain(
            select
                .where_expr
                .iter()
                .flat_map(|expr| expr_column_diagnostics(file, scope, expr)),
        )
        .chain(
            select
                .group_by
                .iter()
                .flat_map(|expr| expr_column_diagnostics(file, scope, expr)),
        )
        .chain(
            select
                .having
                .iter()
                .flat_map(|expr| expr_column_diagnostics(file, scope, expr)),
        )
        .chain(
            select
                .order_by
                .iter()
                .flat_map(|term| expr_column_diagnostics(file, scope, &term.expr)),
        )
        .collect()
}

fn result_column_diagnostics(file: &str, scope: &SelectScope, column: &ResultColumn) -> Vec<Error> {
    match column {
        ResultColumn::Star => Vec::new(),
        ResultColumn::TableStar(_) => Vec::new(),
        ResultColumn::Expr { expr, .. } => expr_column_diagnostics(file, scope, expr),
    }
}

fn expr_column_diagnostics(file: &str, scope: &SelectScope, expr: &Expr) -> Vec<Error> {
    let mut diagnostics = expr
        .arguments
        .iter()
        .flat_map(|argument| expr_column_diagnostics(file, scope, argument))
        .collect::<Vec<_>>();

    let Some(column) = expr.column.as_deref() else {
        return diagnostics;
    };

    if let Some(table) = expr.table.as_deref() {
        if let Some(relation_name) = scope.relation_for_table_name(table) {
            if let Some(relation) = scope.relation_for_relation_name(relation_name) {
                if can_validate_columns(relation) && !relation_has_column(relation, column) {
                    diagnostics.push(unknown_column_diagnostic(
                        file,
                        relation_name,
                        column,
                        expr.location(),
                    ));
                }
            }
        }
        return diagnostics;
    }

    if let Some((relation_name, relation)) = scope.single_validated_relation() {
        if !relation_has_column(relation, column) {
            diagnostics.push(unknown_column_diagnostic(
                file,
                relation_name,
                column,
                expr.location(),
            ));
        }
    }

    diagnostics
}

#[derive(Default)]
struct SelectScope<'a> {
    sources: Vec<SelectScopeSource<'a>>,
}

struct SelectScopeSource<'a> {
    name: &'a SchemaTableContainer,
    alias: Option<&'a str>,
    relation: Option<&'a Relation>,
}

impl<'a> SelectScope<'a> {
    // SELECT column checks are deliberately conservative: validate qualified columns through a
    // matching source alias/table name, and validate unqualified columns only when exactly one
    // source has known columns from a CREATE TABLE statement.
    fn from_select(context: &'a AnalysisContext, select: &'a Select) -> Self {
        let mut scope = Self::default();
        for source in &select.from {
            scope.push_source(context, source);
        }
        scope
    }

    fn push_source(&mut self, context: &'a AnalysisContext, source: &'a SelectSource) {
        match source {
            SelectSource::Table(table) => self.push_table(context, table),
            SelectSource::Join { left, right, .. } => {
                self.push_source(context, left);
                self.push_table(context, right);
            }
        }
    }

    fn push_table(&mut self, context: &'a AnalysisContext, table: &'a SelectTable) {
        self.sources.push(SelectScopeSource {
            name: &table.name,
            alias: table.alias.as_deref(),
            relation: context.relation(&table.name),
        });
    }

    fn relation_for_table_name(&self, table: &str) -> Option<&'a SchemaTableContainer> {
        self.sources
            .iter()
            .find(|source| source.matches_name(table))
            .map(|source| source.name)
    }

    fn relation_for_relation_name(&self, name: &SchemaTableContainer) -> Option<&'a Relation> {
        self.sources
            .iter()
            .find(|source| source.name == name)
            .and_then(|source| source.relation)
    }

    fn single_validated_relation(&self) -> Option<(&'a SchemaTableContainer, &'a Relation)> {
        let mut relations = self
            .sources
            .iter()
            .filter_map(|source| source.relation.map(|relation| (source.name, relation)))
            .filter(|(_, relation)| can_validate_columns(relation));

        let relation = relations.next()?;
        if relations.next().is_some() {
            return None;
        }

        Some(relation)
    }
}

impl SelectScopeSource<'_> {
    fn matches_name(&self, table: &str) -> bool {
        self.alias
            .map(|alias| alias.eq_ignore_ascii_case(table))
            .unwrap_or(false)
            || match self.name {
                SchemaTableContainer::Table(name) => name.eq_ignore_ascii_case(table),
                SchemaTableContainer::SchemaAndTable { table: name, .. } => {
                    name.eq_ignore_ascii_case(table)
                }
            }
    }
}

fn can_validate_columns(relation: &Relation) -> bool {
    relation.kind == RelationKind::Table && !relation.columns.is_empty()
}

fn relation_has_column(relation: &Relation, column: &str) -> bool {
    relation
        .columns
        .iter()
        .any(|known| known.name.eq_ignore_ascii_case(column))
}

fn unknown_column_diagnostic(
    file: &str,
    relation: &SchemaTableContainer,
    column: &str,
    location: Location,
) -> Error {
    Error::new(
        file,
        location,
        Rule::UnknownColumn,
        format!(
            "Column `{column}` is not defined on relation `{}`",
            relation_display(relation)
        ),
        "sqleibniz only checks columns for tables defined earlier in this file with an explicit column list.",
    )
    .with_doc_url("https://www.sqlite.org/syntax/column-name.html")
}

fn relation_display(name: &SchemaTableContainer) -> String {
    match name {
        SchemaTableContainer::Table(table) => table.clone(),
        SchemaTableContainer::SchemaAndTable { schema, table } => format!("{schema}.{table}"),
    }
}
