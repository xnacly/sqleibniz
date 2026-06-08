mod error;
mod handlers;

use std::collections::HashMap;

use error::LspError;
use lsp_server::{
    Connection, ErrorCode, ExtractError, Message, Notification, Request, RequestId, Response,
    ResponseError,
};
use lsp_types::{
    DiagnosticOptions, InitializeParams, SaveOptions, ServerCapabilities, TextDocumentSyncKind,
    TextDocumentSyncOptions,
    notification::{DidChangeTextDocument, DidOpenTextDocument},
    request::{DocumentDiagnosticRequest, HoverRequest},
};

use crate::{
    lexer::Lexer,
    parser::{Parser, nodes::Node},
};

macro_rules! lsp_log {
    ($literal:literal) => {
        eprintln!("[sqleibniz]: {}", $literal)
    };
}

pub fn start() -> Result<(), LspError> {
    lsp_log!("starting language server");
    let (connection, threads) = Connection::stdio();
    let capabilities = serde_json::to_value(&ServerCapabilities {
        hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        diagnostic_provider: Some(lsp_types::DiagnosticServerCapabilities::Options(
            DiagnosticOptions {
                inter_file_dependencies: false,
                workspace_diagnostics: false,
                ..Default::default()
            },
        )),
        text_document_sync: Some(lsp_types::TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                save: Some(lsp_types::TextDocumentSyncSaveOptions::SaveOptions(
                    SaveOptions {
                        include_text: Some(true),
                    },
                )),
                ..Default::default()
            },
        )),
        ..Default::default()
    })
    .map_err(|_| "failed to serialize lsp_types::ServerCapabilities")?;

    let init_params = match connection.initialize(capabilities) {
        Ok(params) => params,
        Err(e) => {
            if e.channel_is_disconnected() {
                threads
                    .join()
                    .map_err(|_| "failed to wait on thread joining")?;
            }
            return Err(e.into());
        }
    };

    event_loop(connection, init_params)?;

    threads
        .join()
        .map_err(|_| "failed to wait on thread joining")?;

    lsp_log!("shutting down language server");
    Ok(())
}

#[derive(Default)]
struct DocumentState {
    ast: Vec<Box<dyn Node>>,
    errors: Vec<super::error::Error>,
}

fn analyze_document(text: &[u8], name: &str) -> DocumentState {
    let text = text.to_vec();
    let mut l = Lexer::new(&text, name);
    let tokens = l.run();
    let mut errors = l.errors;
    let mut p = Parser::new(tokens, name);
    let ast = p.parse();
    errors.append(&mut p.errors);

    DocumentState { ast, errors }
}

fn event_loop(connection: Connection, params: serde_json::Value) -> Result<(), LspError> {
    let _params: InitializeParams = serde_json::from_value(params)
        .map_err(|err| format!("failed to parse initialize params: {err}"))?;
    lsp_log!("starting event loop");
    let mut documents = HashMap::<String, DocumentState>::new();
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                match req.method.as_str() {
                    "textDocument/hover" => {
                        let id = req.id.clone();
                        match cast::<HoverRequest>(req) {
                            Ok((id, params)) => {
                                let state = documents.get(
                                    &params
                                        .text_document_position_params
                                        .text_document
                                        .uri
                                        .to_string(),
                                );
                                if let Err(e) = handlers::hover::handle(
                                    &connection,
                                    state.map(|s| s.ast.as_slice()),
                                    id,
                                    params,
                                ) {
                                    eprintln!("[sqleibniz]: err: {}", e);
                                }
                                continue;
                            }
                            Err(err) => send_request_error(&connection, id, err)?,
                        };
                    }
                    "textDocument/diagnostic" => {
                        let id = req.id.clone();
                        match cast::<DocumentDiagnosticRequest>(req) {
                            Ok((id, params)) => {
                                let state = documents.get(&params.text_document.uri.to_string());
                                if let Err(e) = handlers::diagnostic::handle(
                                    &connection,
                                    state.map(|s| s.errors.clone()).unwrap_or_default(),
                                    id,
                                    params,
                                ) {
                                    eprintln!("[sqleibniz]: err: {}", e);
                                }
                                continue;
                            }
                            Err(err) => send_request_error(&connection, id, err)?,
                        };
                    }
                    _ => send_error(
                        &connection,
                        req.id,
                        ErrorCode::MethodNotFound,
                        format!("unsupported method '{}'", req.method),
                    )?,
                }
            }
            Message::Response(_) => {}
            Message::Notification(not) => match not.method.as_str() {
                "textDocument/didChange" => {
                    match cast_noti::<DidChangeTextDocument>(not) {
                        Ok(params) => {
                            if let Some(change) = params.content_changes.first() {
                                let uri = params.text_document.uri.to_string();
                                let formatted_path = uri.replace("file://", "");
                                documents.insert(
                                    uri,
                                    analyze_document(change.text.as_bytes(), &formatted_path),
                                );
                            }
                        }
                        Err(err) => eprintln!("[sqleibniz]: failed to parse notification: {err}"),
                    };
                }
                "textDocument/didOpen" => {
                    match cast_noti::<DidOpenTextDocument>(not) {
                        Ok(params) => {
                            let uri = params.text_document.uri.to_string();
                            let formatted_path = uri.replace("file://", "");
                            documents.insert(
                                uri,
                                analyze_document(
                                    params.text_document.text.as_bytes(),
                                    &formatted_path,
                                ),
                            );
                        }
                        Err(err) => eprintln!("[sqleibniz]: failed to parse notification: {err}"),
                    };
                }
                _ => lsp_log!("unsupported method"),
            },
        }
    }
    Ok(())
}

fn send_request_error(
    connection: &Connection,
    id: RequestId,
    err: ExtractError<Request>,
) -> Result<(), LspError> {
    send_error(connection, id, ErrorCode::InvalidParams, err.to_string())
}

fn send_error(
    connection: &Connection,
    id: RequestId,
    code: ErrorCode,
    message: String,
) -> Result<(), LspError> {
    let resp = Response {
        id,
        result: None,
        error: Some(ResponseError {
            code: code as i32,
            message,
            data: None,
        }),
    };
    connection
        .sender
        .send(Message::Response(resp))
        .map_err(|_| "failed to send error response")?;
    Ok(())
}

fn cast<R>(req: Request) -> Result<(RequestId, R::Params), ExtractError<Request>>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}

fn cast_noti<N>(not: Notification) -> Result<N::Params, ExtractError<Notification>>
where
    N: lsp_types::notification::Notification,
    N::Params: serde::de::DeserializeOwned,
{
    not.extract(N::METHOD)
}
