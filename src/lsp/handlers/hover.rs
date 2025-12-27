use lsp_server::{Connection, Message, RequestId, Response};
use lsp_types::{HoverParams, Position};

use crate::{lsp::error::LspError, types::Token};

pub fn handle(
    connection: &Connection,
    tokens: &[Token],
    id: RequestId,
    params: HoverParams,
) -> Result<(), LspError> {
    eprintln!("got hover request #{id}");
    let Position { line, character } = params.text_document_position_params.position;
    // TODO: build a better string with more information
    let text = match tokens
        .iter()
        .filter(|tok| {
            tok.line == line as usize
                && tok.start <= character as usize
                && tok.end >= character as usize
        })
        .next_back()
    {
        Some(tok) => format!("sqleibniz: {:?}", tok.ttype),
        None => "sqleibniz: Unknown".into(),
    };
    let hover_result = lsp_types::Hover {
        contents: lsp_types::HoverContents::Scalar(lsp_types::MarkedString::String(text)),
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
