mod error;
mod handlers;

use std::{collections::HashMap, path::PathBuf};

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
    error::Error,
    hooks,
    lexer::Lexer,
    parser::{Parser, nodes::Node},
    types::{
        config::{Config, Hook},
        rules::Rule,
    },
};

macro_rules! lsp_log {
    ($literal:literal) => {
        eprintln!("[sqleibniz]: {}", $literal)
    };
}

pub fn start(enable_hooks: bool) -> Result<(), LspError> {
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

    event_loop(connection, init_params, enable_hooks)?;

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

fn analyze_document(
    text: &[u8],
    name: &str,
    hooks: Option<&[Hook]>,
    lua: Option<&mlua::Lua>,
) -> DocumentState {
    let text = text.to_vec();
    let mut l = Lexer::new(&text, name);
    let tokens = l.run();
    let mut errors = l.errors;
    let mut p = Parser::new(tokens.clone(), name);
    let ast = p.parse();
    errors.append(&mut p.errors);
    if let (Some(hooks), Some(lua)) = (hooks, lua) {
        errors.append(&mut hooks::run(lua, name, hooks, &ast, &tokens));
    }

    DocumentState { ast, errors }
}

struct LspConfig {
    config: Config,
    _lua: Option<mlua::Lua>,
}

fn event_loop(
    connection: Connection,
    params: serde_json::Value,
    enable_hooks: bool,
) -> Result<(), LspError> {
    let params: InitializeParams = serde_json::from_value(params)
        .map_err(|err| format!("failed to parse initialize params: {err}"))?;
    lsp_log!("starting event loop");
    let config = load_config(&params, enable_hooks);
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
                                    state
                                        .map(|s| {
                                            filter_errors(&s.errors, &config.config.disabled_rules)
                                        })
                                        .unwrap_or_default(),
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
                                    analyze_document(
                                        change.text.as_bytes(),
                                        &formatted_path,
                                        config.config.hooks.as_deref(),
                                        config._lua.as_ref(),
                                    ),
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
                                    config.config.hooks.as_deref(),
                                    config._lua.as_ref(),
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

fn load_config(params: &InitializeParams, enable_hooks: bool) -> LspConfig {
    let path = config_path(params);
    let lua = mlua::Lua::new();
    let loaded_config = if enable_hooks {
        Config::from_lua_file(&lua, &path.to_string_lossy())
    } else {
        Config::rules_from_lua_file(&lua, &path.to_string_lossy())
    };

    match loaded_config {
        Ok(config) => {
            eprintln!("[sqleibniz]: loaded config from {}", path.display());
            LspConfig {
                config,
                _lua: enable_hooks.then_some(lua),
            }
        }
        Err(err) => {
            eprintln!("[sqleibniz]: {err}");
            LspConfig {
                config: Config::default(),
                _lua: None,
            }
        }
    }
}

fn config_path(params: &InitializeParams) -> PathBuf {
    if let Some(workspace) = params
        .workspace_folders
        .as_ref()
        .and_then(|workspaces| workspaces.first())
        .and_then(|workspace| uri_file_path(&workspace.uri))
    {
        return workspace.join("leibniz.lua");
    }

    #[allow(deprecated)]
    let root_uri = params.root_uri.as_ref();
    if let Some(root) = root_uri.and_then(uri_file_path) {
        return root.join("leibniz.lua");
    }

    PathBuf::from("leibniz.lua")
}

fn uri_file_path(uri: &lsp_types::Uri) -> Option<PathBuf> {
    uri.to_string()
        .strip_prefix("file://")
        .map(|path| PathBuf::from(path.replace("%20", " ")))
}

fn filter_errors(errors: &[Error], disabled_rules: &[Rule]) -> Vec<Error> {
    errors
        .iter()
        .filter(|error| !disabled_rules.contains(&error.rule))
        .cloned()
        .collect()
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

#[cfg(test)]
mod tests {
    use crate::{error::Error, lsp::filter_errors, types::rules::Rule};
    use crate::{
        lsp::analyze_document,
        types::config::{Hook, HookMatch},
    };

    fn error(rule: Rule) -> Error {
        Error {
            file: "test.sql".into(),
            line: 0,
            rule,
            note: "note".into(),
            msg: "msg".into(),
            start: 0,
            end: 1,
            improved_line: None,
            doc_url: None,
        }
    }

    #[test]
    fn disabled_rules_are_filtered_from_lsp_diagnostics() {
        let errors = vec![error(Rule::Syntax), error(Rule::Hook)];

        let filtered = filter_errors(&errors, &[Rule::Hook]);

        assert_eq!(filtered, vec![error(Rule::Syntax)]);
    }

    #[test]
    fn hooks_run_only_when_supplied_to_lsp_analysis() {
        let lua = mlua::Lua::new();
        let hook_fn = lua
            .load(
                r#"
                return function(node)
                    if string.match(node.content, "%u") then
                        sqleibniz.diagnostic(node, "ident should be lowercase")
                    end
                end
            "#,
            )
            .eval()
            .unwrap();
        let hooks = vec![Hook {
            name: "idents should be lowercase".into(),
            matcher: Some(HookMatch {
                node: Some("Token".into()),
                kind: Some("Ident".into()),
                content: None,
            }),
            hook: Some(hook_fn),
        }];

        let without_hooks = analyze_document(b"VACUUM UpperName;", "test.sql", None, None);
        let with_hooks =
            analyze_document(b"VACUUM UpperName;", "test.sql", Some(&hooks), Some(&lua));

        assert!(
            !without_hooks
                .errors
                .iter()
                .any(|error| error.rule == Rule::Hook)
        );
        assert!(
            with_hooks
                .errors
                .iter()
                .any(|error| error.rule == Rule::Hook)
        );
    }
}
