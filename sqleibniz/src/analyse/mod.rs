/// Column definition analysis rules.
pub mod column;
mod context;
/// CREATE statement analysis rules.
pub mod create;
pub(crate) mod fields;
/// PRAGMA statement analysis rules.
pub mod pragma;
/// Relation reference analysis rules.
pub mod relation;
#[cfg(feature = "trace-analysis")]
pub(crate) mod trace;

pub use context::{AnalysisContext, Column, Relation, RelationKind};

use crate::{error::Error, parser::nodes::Node};

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

pub(crate) fn analyse_node(
    node: &dyn Node,
    file: &str,
    context: &mut AnalysisContext,
    analyse: impl FnOnce(&str, &mut AnalysisContext) -> Vec<Error>,
) -> Vec<Error> {
    #[cfg(not(feature = "trace-analysis"))]
    let _ = node;

    #[cfg(feature = "trace-analysis")]
    trace::enter_node();

    let diagnostics = analyse(file, context);

    #[cfg(feature = "trace-analysis")]
    trace::exit_node(node.name(), node.location(), &diagnostics);

    diagnostics
}
