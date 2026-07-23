use crate::{
    error::{Error, Location},
    parser::nodes::Node,
    types::{
        Keyword, Token, Type, config::Hook, ctx::HookContext, rules::Rule,
        storage::SqliteStorageClass,
    },
};
use std::{cell::RefCell, rc::Rc};

trait NodeHookContext {
    fn as_hook_context(&self) -> HookContext;
}

impl NodeHookContext for dyn Node {
    fn as_hook_context(&self) -> HookContext {
        let location = self.location();
        HookContext::node(self.name(), location.line, location.start, location.end)
    }
}

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

fn hook_value_text(value: mlua::Value) -> mlua::Result<String> {
    match value {
        mlua::Value::String(value) => Ok(value.to_str()?.to_string()),
        mlua::Value::Table(value) => value.get("content").or_else(|_| value.get("text")),
        mlua::Value::Nil => Ok(String::new()),
        value => Err(mlua::Error::FromLuaConversionError {
            from: value.type_name(),
            to: "string or hook node".to_string(),
            message: Some("expected a string or hook node table".to_string()),
        }),
    }
}

fn sqlite_type_name(value: &str) -> bool {
    SqliteStorageClass::from_str_strict(value.to_ascii_uppercase().as_str()).is_some()
}

fn sqleibniz_table(
    lua: &mlua::Lua,
    hook_file: String,
    hook_name: String,
    hook_errors: Rc<RefCell<Vec<Error>>>,
) -> mlua::Result<mlua::Table> {
    let table = lua.create_table()?;
    table.set(
        "diagnostic",
        lua.create_function(move |_, (node, note): (mlua::Table, String)| {
            hook_errors
                .borrow_mut()
                .push(hook_diagnostic(&hook_file, &hook_name, node, note)?);
            Ok(())
        })?,
    )?;
    table.set(
        "is_keyword",
        lua.create_function(|_, value: mlua::Value| {
            Ok(Keyword::from_str(&hook_value_text(value)?).is_some())
        })?,
    )?;
    table.set(
        "is_type_name",
        lua.create_function(|_, value: mlua::Value| {
            Ok(sqlite_type_name(&hook_value_text(value)?))
        })?,
    )?;
    Ok(table)
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
                let sqleibniz = sqleibniz_table(lua, hook_file, hook_name, hook_errors);

                match sqleibniz {
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

    let mut skip_expected_statement = false;
    for token in tokens {
        if skip_expected_statement {
            if token.ttype == Type::Semicolon {
                skip_expected_statement = false;
            }
            continue;
        }

        if token.ttype == Type::InstructionExpect {
            skip_expected_statement = true;
            continue;
        }

        run_context(lua, file, hooks, &HookContext::token(token), &mut errors);
    }

    errors
}
