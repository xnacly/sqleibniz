use clap::ValueEnum;

use crate::{parser::nodes, types::rules::Rule};

pub struct Explanation {
    pub kind: ExplanationKind,
    pub name: &'static str,
    pub description: &'static str,
    pub documentation: Option<&'static str>,
    pub details: Option<String>,
}

pub enum ExplanationKind {
    Rule,
    SqlStatement,
}

struct SqlStatement {
    name: &'static str,
    aliases: &'static [&'static str],
    node_doc: &'static str,
}

const SQL_STATEMENTS: &[SqlStatement] = &[
    SqlStatement {
        name: "explain-stmt",
        aliases: &["explain", "Explain"],
        node_doc: nodes::Explain::DOC,
    },
    SqlStatement {
        name: "alter-table-stmt",
        aliases: &["alter", "alter-stmt", "Alter"],
        node_doc: nodes::Alter::DOC,
    },
    SqlStatement {
        name: "analyze-stmt",
        aliases: &["analyze", "Analyze"],
        node_doc: nodes::Analyze::DOC,
    },
    SqlStatement {
        name: "attach-stmt",
        aliases: &["attach", "Attach"],
        node_doc: nodes::Attach::DOC,
    },
    SqlStatement {
        name: "begin-stmt",
        aliases: &["begin", "Begin"],
        node_doc: nodes::Begin::DOC,
    },
    SqlStatement {
        name: "commit-stmt",
        aliases: &["commit", "end", "Commit"],
        node_doc: nodes::Commit::DOC,
    },
    SqlStatement {
        name: "create-index-stmt",
        aliases: &["create-index", "CreateIndex"],
        node_doc: nodes::CreateIndex::DOC,
    },
    SqlStatement {
        name: "create-table-stmt",
        aliases: &["create-table", "CreateTable", "CreateTableAs"],
        node_doc: nodes::CreateTable::DOC,
    },
    SqlStatement {
        name: "create-trigger-stmt",
        aliases: &["create-trigger", "CreateTrigger"],
        node_doc: nodes::CreateTrigger::DOC,
    },
    SqlStatement {
        name: "create-view-stmt",
        aliases: &["create-view", "CreateView"],
        node_doc: nodes::CreateView::DOC,
    },
    SqlStatement {
        name: "create-virtual-table-stmt",
        aliases: &["create-virtual-table", "CreateVirtualTable"],
        node_doc: nodes::CreateVirtualTable::DOC,
    },
    SqlStatement {
        name: "delete-stmt",
        aliases: &["delete", "Delete"],
        node_doc: nodes::Delete::DOC,
    },
    SqlStatement {
        name: "detach-stmt",
        aliases: &["detach", "Detach"],
        node_doc: nodes::Detach::DOC,
    },
    SqlStatement {
        name: "drop-stmt",
        aliases: &[
            "drop",
            "drop-index",
            "drop-table",
            "drop-trigger",
            "drop-view",
            "Drop",
        ],
        node_doc: nodes::Drop::DOC,
    },
    SqlStatement {
        name: "insert-stmt",
        aliases: &["insert", "Insert"],
        node_doc: nodes::Insert::DOC,
    },
    SqlStatement {
        name: "pragma-stmt",
        aliases: &["pragma", "Pragma"],
        node_doc: nodes::Pragma::DOC,
    },
    SqlStatement {
        name: "reindex-stmt",
        aliases: &["reindex", "Reindex"],
        node_doc: nodes::Reindex::DOC,
    },
    SqlStatement {
        name: "release-stmt",
        aliases: &["release", "Release"],
        node_doc: nodes::Release::DOC,
    },
    SqlStatement {
        name: "rollback-stmt",
        aliases: &["rollback", "Rollback"],
        node_doc: nodes::Rollback::DOC,
    },
    SqlStatement {
        name: "savepoint-stmt",
        aliases: &["savepoint", "Savepoint"],
        node_doc: nodes::Savepoint::DOC,
    },
    SqlStatement {
        name: "select-stmt",
        aliases: &["select", "Select"],
        node_doc: nodes::Select::DOC,
    },
    SqlStatement {
        name: "update-stmt",
        aliases: &["update", "Update"],
        node_doc: nodes::Update::DOC,
    },
    SqlStatement {
        name: "vacuum-stmt",
        aliases: &["vacuum", "Vacuum"],
        node_doc: nodes::Vacuum::DOC,
    },
    SqlStatement {
        name: "with-stmt",
        aliases: &["with", "With"],
        node_doc: nodes::With::DOC,
    },
];

pub fn lookup(name: &str) -> Option<Explanation> {
    Rule::value_variants()
        .iter()
        .find(|rule| rule.name() == name)
        .map(|rule| Explanation {
            kind: ExplanationKind::Rule,
            name: rule.name(),
            description: rule.description(),
            documentation: None,
            details: Some(rule_examples(rule)),
        })
        .or_else(|| lookup_sql_statement(name))
}

pub fn rules() -> Vec<Explanation> {
    Rule::value_variants()
        .iter()
        .map(|rule| Explanation {
            kind: ExplanationKind::Rule,
            name: rule.name(),
            description: rule.description(),
            documentation: None,
            details: None,
        })
        .collect()
}

pub fn sql_statements() -> Vec<Explanation> {
    SQL_STATEMENTS
        .iter()
        .map(|stmt| Explanation {
            kind: ExplanationKind::SqlStatement,
            name: stmt.name,
            description: statement_description(stmt.node_doc),
            documentation: statement_documentation(stmt.node_doc),
            details: None,
        })
        .collect()
}

fn lookup_sql_statement(name: &str) -> Option<Explanation> {
    let canonical = normalize(name);
    SQL_STATEMENTS
        .iter()
        .find(|stmt| {
            normalize(stmt.name) == canonical
                || stmt
                    .aliases
                    .iter()
                    .any(|alias| normalize(alias) == canonical)
        })
        .map(|stmt| Explanation {
            kind: ExplanationKind::SqlStatement,
            name: stmt.name,
            description: statement_description(stmt.node_doc),
            documentation: statement_documentation(stmt.node_doc),
            details: Some(stmt.node_doc.to_string()),
        })
}

fn rule_examples(rule: &Rule) -> String {
    let mut details = String::from("# Examples");
    for example in rule.examples() {
        details.push_str("\n\n");
        details.push_str(example.explanation);
        details.push_str("\n\n```sql\n");
        details.push_str(example.sql);
        details.push_str("\n```");
    }
    details
}

fn statement_description(doc: &'static str) -> &'static str {
    doc.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.contains("see:"))
        .unwrap_or(doc)
}

fn statement_documentation(doc: &'static str) -> Option<&'static str> {
    doc.split_whitespace()
        .find(|part| part.starts_with("https://"))
        .map(|url| url.trim_end_matches(['.', ',', ')']))
}

fn normalize(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    for c in name.chars() {
        match c {
            '_' | '-' | ' ' => {}
            _ => normalized.extend(c.to_lowercase()),
        }
    }
    normalized
}
