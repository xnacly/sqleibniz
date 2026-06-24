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
        context.relation_count()
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

pub(crate) fn define_relation(name: &SchemaTableContainer, kind: RelationKind, columns: &[Column]) {
    ANALYSIS_DEPTH.with(|depth| {
        let indent = "  ".repeat(depth.get());
        eprintln!(
            "{indent}context: define relation name={name:?} kind={kind:?} columns={columns:?}"
        );
    });
}
