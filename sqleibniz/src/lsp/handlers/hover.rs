use lsp_server::{Connection, Message, RequestId, Response, ResponseKind};
use lsp_types::{HoverParams, Position};

use crate::{lsp::error::LspError, parser::nodes::Node};

pub fn handle(
    connection: &Connection,
    ast: Option<&[Box<dyn Node>]>,
    id: RequestId,
    params: HoverParams,
) -> Result<(), LspError> {
    let Position { line, character } = params.text_document_position_params.position;
    let text = match ast
        .unwrap_or_default()
        .iter()
        .filter(|n| {
            let location = n.location();
            location.line == line as usize && location.start <= character as usize
        })
        .next_back()
    {
        Some(node) => {
            format!("# {}\n\n{}", node.name(), node.doc(),)
        }
        None => "Unknown".into(),
    };
    let hover_result = lsp_types::Hover {
        contents: lsp_types::HoverContents::Markup(lsp_types::MarkupContent {
            kind: lsp_types::MarkupKind::Markdown,
            value: text,
        }),
        range: None,
    };
    let result = serde_json::to_value(&hover_result)
        .map_err(|err| format!("failed to serialize hover: {err}"))?;
    let resp = Response {
        id,
        response_kind: ResponseKind::Ok { result },
    };
    connection
        .sender
        .send(Message::Response(resp))
        .map_err(|_| "failed to send definition")?;
    Ok(())
}
