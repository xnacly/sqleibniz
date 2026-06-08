use lsp_server::{Connection, Message, RequestId, Response};
use lsp_types::{Diagnostic, DiagnosticSeverity, DocumentDiagnosticParams, Position, Range};

use crate::{error::Error, lsp::error::LspError};

fn error_range(error: &Error) -> Range {
    Range::new(
        Position {
            line: error.line as u32,
            character: error.start as u32,
        },
        Position {
            line: error.line as u32,
            character: usize::max(error.end, error.start + 1) as u32,
        },
    )
}

impl From<Error> for Diagnostic {
    fn from(value: Error) -> Self {
        Self {
            range: error_range(&value),
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(lsp_types::NumberOrString::String(
                value.rule.name().to_string(),
            )),
            code_description: None,
            source: Some("sqleibniz".into()),
            message: format!("{}: {}", value.msg, value.note),
            related_information: None,
            tags: None,
            data: None,
        }
    }
}

pub fn handle(
    connection: &Connection,
    errors: Vec<Error>,
    id: RequestId,
    _: DocumentDiagnosticParams,
) -> Result<(), LspError> {
    eprintln!("got diagnostic request #{id}");
    let diagnostics = lsp_types::FullDocumentDiagnosticReport {
        result_id: None,
        items: errors.into_iter().map(Error::into).collect(),
    };
    let result = serde_json::to_value(&diagnostics).unwrap();
    let resp = Response {
        id,
        result: Some(result),
        error: None,
    };
    connection
        .sender
        .send(Message::Response(resp))
        .map_err(|_| "failed to send diagnostics")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{error::Error, lsp::handlers::diagnostic::error_range, types::rules::Rule};

    fn error(start: usize, end: usize) -> Error {
        Error {
            file: "test.sql".into(),
            line: 2,
            rule: Rule::Syntax,
            note: "note".into(),
            msg: "msg".into(),
            start,
            end,
            improved_line: None,
            doc_url: None,
        }
    }

    #[test]
    fn range_preserves_zero_based_positions() {
        let range = error_range(&error(3, 8));

        assert_eq!(range.start.line, 2);
        assert_eq!(range.start.character, 3);
        assert_eq!(range.end.line, 2);
        assert_eq!(range.end.character, 8);
    }

    #[test]
    fn range_is_never_empty() {
        let range = error_range(&error(3, 3));

        assert_eq!(range.start.character, 3);
        assert_eq!(range.end.character, 4);
    }
}
