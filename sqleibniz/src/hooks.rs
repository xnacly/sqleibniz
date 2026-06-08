use crate::{
    error::Error,
    parser::nodes::Node,
    types::{Token, config::Hook, ctx::HookContext, rules::Rule},
};

fn hook_error_note(err: mlua::Error) -> String {
    match err {
        mlua::Error::RuntimeError(msg) => msg,
        mlua::Error::CallbackError { cause, .. } => cause.to_string(),
        err => err.to_string(),
    }
}

fn hook_error(file: &str, hook: &Hook, ctx: &HookContext, err: mlua::Error) -> Error {
    Error {
        file: file.into(),
        line: ctx.line,
        rule: Rule::Hook,
        note: hook_error_note(err),
        msg: hook.name.clone(),
        start: ctx.start,
        end: ctx.finish,
        improved_line: None,
        doc_url: None,
    }
}

fn hook_matches(hook: &Hook, ctx: &HookContext) -> bool {
    hook.node
        .as_deref()
        .is_none_or(|node| node == ctx.node.as_str())
}

fn run_context(file: &str, hooks: &[Hook], ctx: &HookContext, errors: &mut Vec<Error>) {
    for hook in hooks {
        if hook_matches(hook, ctx) {
            if let Err(err) = hook.exec(ctx.clone()) {
                errors.push(hook_error(file, hook, ctx, err));
            }
        }
    }

    for child in &ctx.children {
        run_context(file, hooks, child, errors);
    }
}

pub fn run(file: &str, hooks: &[Hook], ast: &[Box<dyn Node>], tokens: &[Token]) -> Vec<Error> {
    let mut errors = vec![];

    for node in ast {
        run_context(file, hooks, &node.as_hook_context(), &mut errors);
    }

    for token in tokens {
        run_context(file, hooks, &HookContext::token(token), &mut errors);
    }

    errors
}
