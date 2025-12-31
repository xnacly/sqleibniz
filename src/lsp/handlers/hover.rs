use lsp_server::{Connection, Message, RequestId, Response};
use lsp_types::{HoverParams, Position};

use crate::{lsp::error::LspError, parser::nodes::Node};

pub fn handle(
    connection: &Connection,
    ast: &[Box<dyn Node>],
    id: RequestId,
    params: HoverParams,
) -> Result<(), LspError> {
    eprintln!("got hover request #{id}");
    let Position { line, character } = params.text_document_position_params.position;
    let text = match ast
        .iter()
        .filter(|n| {
            let tok = n.token();
            tok.line == line as usize && tok.start <= character as usize
        })
        .next_back()
    {
        Some(node) => {
            format!("#\n{}\n{}", node.name(), node.doc(),)
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
    let result = serde_json::to_value(&hover_result).unwrap();
    let resp = Response {
        id,
        result: Some(result),
        error: None,
    };
    connection
        .sender
        .send(Message::Response(resp))
        .map_err(|_| "failed to send definition")?;
    Ok(())
}
