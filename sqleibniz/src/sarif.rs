use std::borrow::Cow;

use serde_json::{Value, json};

use crate::{error::Error, types::rules::Rule};

const SARIF_SCHEMA: &str =
    "https://docs.oasis-open.org/sarif/sarif/v2.1.0/os/schemas/sarif-schema-2.1.0.json";

pub fn log(errors: &[Error]) -> Value {
    let rules = Rule::all().iter().map(rule_descriptor).collect::<Vec<_>>();

    json!({
        "$schema": SARIF_SCHEMA,
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "sqleibniz",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": env!("CARGO_PKG_REPOSITORY"),
                    "rules": rules,
                }
            },
            "results": errors.iter().map(result).collect::<Vec<_>>(),
        }]
    })
}

fn rule_descriptor(rule: &Rule) -> Value {
    json!({
        "id": rule.name(),
        "name": rule.name(),
        "shortDescription": {
            "text": rule.description(),
        },
        "fullDescription": {
            "text": rule.description(),
        },
    })
}

fn result(error: &Error) -> Value {
    let end_column = usize::max(error.location.end + 1, error.location.start + 2);
    let message = if error.note.is_empty() {
        Cow::Borrowed(error.msg.as_str())
    } else {
        Cow::Owned(format!("{}: {}", error.msg, error.note))
    };

    let mut value = json!({
        "ruleId": error.rule.name(),
        "level": "error",
        "message": {
            "text": message,
        },
        "locations": [{
            "physicalLocation": {
                "artifactLocation": {
                    "uri": error.file.as_str(),
                },
                "region": {
                    "startLine": error.location.line + 1,
                    "startColumn": error.location.start + 1,
                    "endColumn": end_column,
                }
            }
        }]
    });

    if let Some(doc_url) = error.doc_url {
        value["properties"] = json!({
            "docUrl": doc_url,
        });
    }

    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Location;

    #[test]
    fn emits_sarif_log_with_result_location() {
        let error = Error::new(
            "example.sql",
            Location {
                line: 2,
                start: 4,
                end: 4,
            },
            Rule::Syntax,
            "Unexpected Literal",
            "expected semicolon",
        )
        .with_doc_url("https://sqlite.org/lang.html");

        let log = log(&[error]);

        assert_eq!(log["version"], "2.1.0");
        assert_eq!(log["runs"][0]["tool"]["driver"]["name"], "sqleibniz");
        assert_eq!(log["runs"][0]["results"][0]["ruleId"], "sql/syntax");
        assert_eq!(
            log["runs"][0]["results"][0]["message"]["text"],
            "Unexpected Literal: expected semicolon"
        );
        assert_eq!(
            log["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            "example.sql"
        );
        assert_eq!(
            log["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"]["startLine"],
            3
        );
        assert_eq!(
            log["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"]["startColumn"],
            5
        );
        assert_eq!(
            log["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"]["endColumn"],
            6
        );
        assert_eq!(
            log["runs"][0]["results"][0]["properties"]["docUrl"],
            "https://sqlite.org/lang.html"
        );
    }
}
