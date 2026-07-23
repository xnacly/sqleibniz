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

pub use context::{AnalysisContext, Column, Relation, RelationKind};

use crate::{
    error::Error,
    parser::nodes::{
        Alter, ColumnDef, CreateTable, CreateTableAs, CreateView, CreateVirtualTable, Delete,
        Insert, Node, Pragma, Select, Update, With,
    },
};

pub(crate) trait Analyse {
    fn analyse(&self, file: &str, context: &mut AnalysisContext) -> Vec<Error>;
}

pub(crate) fn analyse_ast_node(
    node: &dyn Node,
    file: &str,
    context: &mut AnalysisContext,
) -> Vec<Error> {
    node.analyse(file, context)
}

macro_rules! impl_analyse {
    ($node:ty, $handler:path) => {
        impl Analyse for $node {
            fn analyse(&self, file: &str, context: &mut AnalysisContext) -> Vec<Error> {
                analyse_node(self, file, context, |file, context| {
                    $handler(file, context, self)
                })
            }
        }
    };
}

impl_analyse!(Select, relation::select);
impl_analyse!(Update, relation::update);
impl_analyse!(Insert, relation::insert);
impl_analyse!(Delete, relation::delete);
impl_analyse!(With, relation::with);
impl_analyse!(Alter, create::alter);
impl_analyse!(ColumnDef, column::column_def);
impl_analyse!(CreateTable, create::create_table);
impl_analyse!(CreateTableAs, create::create_table_as);
impl_analyse!(CreateView, create::create_view);
impl_analyse!(CreateVirtualTable, create::create_virtual_table);
impl_analyse!(Pragma, pragma::pragma);

impl<T: Analyse + ?Sized> Analyse for Box<T> {
    fn analyse(&self, file: &str, context: &mut AnalysisContext) -> Vec<Error> {
        self.as_ref().analyse(file, context)
    }
}

impl Analyse for dyn Node {
    fn analyse(&self, file: &str, context: &mut AnalysisContext) -> Vec<Error> {
        macro_rules! dispatch {
            ($($node:ty),* $(,)?) => {
                $(
                    if let Some(node) = self.as_any().downcast_ref::<$node>() {
                        return node.analyse(file, context);
                    }
                )*
            };
        }

        dispatch!(
            Select,
            Update,
            Insert,
            Delete,
            With,
            Alter,
            ColumnDef,
            CreateTable,
            CreateTableAs,
            CreateView,
            CreateVirtualTable,
            Pragma,
        );
        Vec::new()
    }
}

pub fn run(file: &str, ast: &[Box<dyn Node>]) -> Vec<Error> {
    let mut context = AnalysisContext::default();

    #[cfg(feature = "trace-analysis")]
    trace::run_start(file, ast.len());

    let diagnostics: Vec<Error> = ast
        .iter()
        .flat_map(|node| analyse_ast_node(node.as_ref(), file, &mut context))
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
