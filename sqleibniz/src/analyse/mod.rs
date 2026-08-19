/// Column definition analysis rules.
pub mod column;
mod context;
/// CREATE statement analysis rules.
pub mod create;
/// PRAGMA statement analysis rules.
pub mod pragma;
/// Relation reference analysis rules.
pub mod relation;
#[cfg(feature = "trace-analysis")]
pub(crate) mod trace;

pub use context::{
    AnalysisContext, Column, ColumnKnowledge, Relation, RelationKind, SchemaMutationError,
};

use std::marker::PhantomData;

use crate::{
    error::Error,
    parser::nodes::{
        Alter, CreateTable, CreateTableAs, CreateView, CreateVirtualTable, Delete, Insert, Node,
        Pragma, Select, Update, With,
    },
};

trait AnalysisPass {
    fn analyse(&self, file: &str, node: &dyn Node, context: &mut AnalysisContext) -> Vec<Error>;
}

#[derive(Default)]
pub(crate) struct AnalysisPipeline {
    passes: Vec<Box<dyn AnalysisPass>>,
}

impl AnalysisPipeline {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn on<N, F>(mut self, handler: F) -> Self
    where
        N: Node + 'static,
        F: Fn(&str, &mut AnalysisContext, &N) -> Vec<Error> + 'static,
    {
        self.passes.push(Box::new(NodePass {
            handler,
            node: PhantomData,
        }));
        self
    }
}

struct NodePass<N: Node, F> {
    handler: F,
    node: PhantomData<fn(&N)>,
}

impl<N, F> AnalysisPass for NodePass<N, F>
where
    N: Node + 'static,
    F: Fn(&str, &mut AnalysisContext, &N) -> Vec<Error>,
{
    fn analyse(&self, file: &str, node: &dyn Node, context: &mut AnalysisContext) -> Vec<Error> {
        node.as_any()
            .downcast_ref::<N>()
            .map(|node| (self.handler)(file, context, node))
            .unwrap_or_default()
    }
}

pub fn run(file: &str, ast: &[Box<dyn Node>]) -> Vec<Error> {
    run_with_pipeline(file, ast, default_pipeline())
}

pub(crate) fn run_with_pipeline(
    file: &str,
    ast: &[Box<dyn Node>],
    pipeline: AnalysisPipeline,
) -> Vec<Error> {
    let mut context = AnalysisContext::default();

    #[cfg(feature = "trace-analysis")]
    trace::run_start(file, ast.len());

    let mut diagnostics = Vec::new();
    for node in ast {
        for pass in &pipeline.passes {
            #[cfg(feature = "trace-analysis")]
            trace::enter_node();

            let pass_diagnostics = pass.analyse(file, node.as_ref(), &mut context);

            #[cfg(feature = "trace-analysis")]
            trace::exit_node(node.name(), node.location(), &pass_diagnostics);

            diagnostics.extend(pass_diagnostics);
        }
    }

    #[cfg(feature = "trace-analysis")]
    trace::run_end(diagnostics.len(), &context);

    diagnostics
}

pub(crate) fn default_pipeline() -> AnalysisPipeline {
    AnalysisPipeline::new()
        .on::<Select, _>(relation::select)
        .on::<Update, _>(relation::update)
        .on::<Insert, _>(relation::insert)
        .on::<Delete, _>(relation::delete)
        .on::<With, _>(relation::with)
        .on::<Alter, _>(create::alter)
        .on::<CreateTable, _>(create::create_table)
        .on::<CreateTableAs, _>(create::create_table_as)
        .on::<CreateView, _>(create::create_view)
        .on::<CreateVirtualTable, _>(create::create_virtual_table)
        .on::<Pragma, _>(pragma::pragma)
}
