use crate::{
    analyse::AnalysisContext,
    error::Error,
    parser::nodes::*,
    types::{Keyword, Token, storage::SqliteStorageClass},
};

pub(crate) trait FieldDiagnostics {
    fn field_diagnostics(&self, file: &str, context: &mut AnalysisContext) -> Vec<Error>;
}

macro_rules! impl_empty_field_diagnostics {
    ($($tt:ty),*) => {
        $(
            impl FieldDiagnostics for $tt {
                fn field_diagnostics(
                    &self,
                    file: &str,
                    context: &mut AnalysisContext,
                ) -> Vec<Error> {
                    let _ = file;
                    let _ = context;
                    Vec::new()
                }
            }
        )*
    };
}

impl FieldDiagnostics for InsertSource {
    fn field_diagnostics(&self, file: &str, context: &mut AnalysisContext) -> Vec<Error> {
        match self {
            InsertSource::DefaultValues | InsertSource::Values(_) => Vec::new(),
            InsertSource::Select(select) => select.analyse(file, context),
        }
    }
}

impl FieldDiagnostics for CommonTableExpression {
    fn field_diagnostics(&self, file: &str, context: &mut AnalysisContext) -> Vec<Error> {
        self.select.analyse(file, context)
    }
}

impl<T: Node + ?Sized> FieldDiagnostics for Box<T> {
    fn field_diagnostics(&self, file: &str, context: &mut AnalysisContext) -> Vec<Error> {
        self.analyse(file, context)
    }
}

impl<T: FieldDiagnostics> FieldDiagnostics for Option<T> {
    fn field_diagnostics(&self, file: &str, context: &mut AnalysisContext) -> Vec<Error> {
        match self {
            Some(n) => n.field_diagnostics(file, context),
            None => Vec::new(),
        }
    }
}

impl<T: FieldDiagnostics> FieldDiagnostics for Vec<T> {
    fn field_diagnostics(&self, file: &str, context: &mut AnalysisContext) -> Vec<Error> {
        self.iter()
            .flat_map(|n| n.field_diagnostics(file, context))
            .collect()
    }
}

impl_empty_field_diagnostics!(
    String,
    bool,
    Token,
    Keyword,
    SqliteStorageClass,
    BindParameter,
    Expr,
    SchemaTableContainer,
    QualifiedTableName,
    QualifiedTableIndex,
    IndexedColumn,
    ResultColumn,
    SelectQuantifier,
    SelectSource,
    SelectTable,
    JoinOperator,
    OrderingTerm,
    LimitOffset,
    UpdateAssignment,
    ColumnConstraint,
    TableConstraint,
    ForeignKeyClause,
    TriggerTiming,
    TriggerEvent,
    TriggerBodyStmt,
    PragmaInvocation
);
