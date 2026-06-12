use crate::{
    error::{Error, Location},
    parser::nodes::Node,
    types::{Token, config::Hook, ctx::HookContext, rules::Rule},
};
use std::{cell::RefCell, rc::Rc};

fn hook_error_note(err: mlua::Error) -> String {
    match err {
        mlua::Error::RuntimeError(msg) => msg,
        mlua::Error::CallbackError { cause, .. } => cause.to_string(),
        err => err.to_string(),
    }
}

fn hook_error(file: &str, hook: &Hook, ctx: &HookContext, err: mlua::Error) -> Error {
    Error::new(
        file,
        Location {
            line: ctx.line,
            start: ctx.start,
            end: ctx.finish,
        },
        Rule::Hook,
        hook.name.clone(),
        hook_error_note(err),
    )
}

fn hook_diagnostic(
    file: &str,
    hook_name: &str,
    node: mlua::Table,
    note: String,
) -> mlua::Result<Error> {
    Ok(Error::new(
        file,
        Location {
            line: node.get("line")?,
            start: node.get("start")?,
            end: node.get("finish")?,
        },
        Rule::Hook,
        hook_name,
        note,
    ))
}

fn hook_matches(hook: &Hook, ctx: &HookContext) -> bool {
    let Some(matcher) = &hook.matcher else {
        return true;
    };

    matcher
        .node
        .as_deref()
        .is_none_or(|node| node == ctx.node.as_str())
        && matcher
            .kind
            .as_deref()
            .is_none_or(|kind| kind == ctx.kind.as_str())
        && matcher
            .content
            .as_deref()
            .is_none_or(|content| ctx.content.as_deref() == Some(content))
}

fn run_context(
    lua: &mlua::Lua,
    file: &str,
    hooks: &[Hook],
    ctx: &HookContext,
    errors: &mut Vec<Error>,
) {
    for hook in hooks {
        if hook_matches(hook, ctx) {
            let reported_errors = Rc::new(RefCell::new(Vec::new()));
            if let Some(hook_fn) = &hook.hook {
                let hook_errors = Rc::clone(&reported_errors);
                let hook_file = file.to_string();
                let hook_name = hook.name.clone();
                let sqleibniz = lua.create_table();
                let sqleibniz_diagnostic = sqleibniz.and_then(|table| {
                    table.set(
                        "diagnostic",
                        lua.create_function(move |_, (node, note): (mlua::Table, String)| {
                            hook_errors
                                .borrow_mut()
                                .push(hook_diagnostic(&hook_file, &hook_name, node, note)?);
                            Ok(())
                        })?,
                    )?;
                    Ok(table)
                });

                match sqleibniz_diagnostic {
                    Ok(table) => {
                        if let Err(err) = lua.globals().set("sqleibniz", table) {
                            errors.push(hook_error(file, hook, ctx, err));
                            continue;
                        }
                    }
                    Err(err) => {
                        errors.push(hook_error(file, hook, ctx, err));
                        continue;
                    }
                }

                if let Err(err) = hook_fn.call::<()>(ctx.clone()) {
                    errors.push(hook_error(file, hook, ctx, err));
                }
            }

            if let Ok(mut reported_errors) = reported_errors.try_borrow_mut() {
                errors.append(&mut reported_errors);
            }
        }
    }

    for child in &ctx.children {
        run_context(lua, file, hooks, child, errors);
    }
}

pub fn run(
    lua: &mlua::Lua,
    file: &str,
    hooks: &[Hook],
    ast: &[Box<dyn Node>],
    tokens: &[Token],
) -> Vec<Error> {
    let mut errors = vec![];

    for node in ast {
        run_context(lua, file, hooks, &node.as_hook_context(), &mut errors);
    }

    for token in tokens {
        run_context(lua, file, hooks, &HookContext::token(token), &mut errors);
    }

    errors
}
