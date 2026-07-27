use std::{fs, time::Duration};

use mlua::{FromLua, Function, Table, UserData};

use super::{ctx::HookContext, rules::Rule};

pub const DEFAULT_MAX_HOOK_RUNTIME: Duration = Duration::from_millis(10);

pub fn parse_max_hook_runtime(value: &str) -> Result<Duration, String> {
    let milliseconds = value
        .strip_suffix("ms")
        .unwrap_or(value)
        .parse::<u64>()
        .map_err(|_| {
            format!(
                "invalid hook runtime '{value}'; expected a number of milliseconds such as 10ms"
            )
        })?;
    Ok(Duration::from_millis(milliseconds))
}

fn max_hook_runtime(value: mlua::Value) -> mlua::Result<Duration> {
    let milliseconds = match value {
        mlua::Value::Nil => return Ok(DEFAULT_MAX_HOOK_RUNTIME),
        mlua::Value::Integer(value) if value >= 0 => value as u64,
        mlua::Value::Number(value)
            if value.is_finite()
                && value >= 0.0
                && value.fract() == 0.0
                && value <= u64::MAX as f64 =>
        {
            value as u64
        }
        mlua::Value::String(value) => {
            return parse_max_hook_runtime(value.to_str()?.as_ref()).map_err(mlua::Error::runtime);
        }
        value => {
            return Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "hook runtime in milliseconds".into(),
                message: Some("expected a non-negative integer or a string such as '10ms'".into()),
            });
        }
    };
    Ok(Duration::from_millis(milliseconds))
}

#[derive(Debug)]
/// Configuration is expected to be at ./leibniz.lua - its existence is not required for the program invocation
pub struct Config {
    /// holds the rules that the user wants to not see errors for.
    pub disabled_rules: Vec<Rule>,
    /// holds the hooks the user wants to execute
    pub hooks: Option<Vec<Hook>>,
    /// maximum time a single Lua hook invocation may execute for
    pub max_hook_runtime: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            disabled_rules: vec![],
            hooks: None,
            max_hook_runtime: DEFAULT_MAX_HOOK_RUNTIME,
        }
    }
}

impl Config {
    pub fn from_lua_file(lua: &mlua::Lua, file_name: &str) -> Result<Self, String> {
        let raw_conf = Self::raw_lua_config(lua, file_name)?;
        lua.unpack(raw_conf)
            .map_err(|err| format!("{}: {}", file_name, err))
    }

    pub fn rules_from_lua_file(lua: &mlua::Lua, file_name: &str) -> Result<Self, String> {
        let raw_conf = Self::raw_lua_config(lua, file_name)?;
        let table: Table = lua
            .unpack(raw_conf)
            .map_err(|err| format!("{}: {}", file_name, err))?;
        let disabled_rules = table
            .get::<Vec<String>>("disabled_rules")
            .unwrap_or_else(|_| vec![])
            .into_iter()
            .map(|name| {
                Rule::from_name(&name)
                    .ok_or_else(|| format!("{}: Unknown rule name '{name}'", file_name))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            disabled_rules,
            hooks: None,
            max_hook_runtime: DEFAULT_MAX_HOOK_RUNTIME,
        })
    }

    fn raw_lua_config(lua: &mlua::Lua, file_name: &str) -> Result<mlua::Value, String> {
        let conf_str = fs::read_to_string(file_name).map_err(|err| {
            format!(
                "Issue trying to read configuration from '{}': [{}], falling back to default configuration",
                file_name, err
            )
        })?;
        let globals = lua.globals();
        lua.load(conf_str)
            .set_name(file_name)
            .exec()
            .map_err(|err| format!("{}: {}", file_name, err))?;
        let raw_conf = globals
            .get::<mlua::Value>("leibniz")
            .map_err(|err| format!("{}: {}", file_name, err))?;
        if raw_conf.is_nil() {
            return Err(format!(
                "{}: leibniz table is missing from configuration",
                file_name
            ));
        }
        Ok(raw_conf)
    }
}

impl FromLua for Config {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let table: Table = lua.unpack(value)?;
        let disabled_rules = table
            .get::<Vec<String>>("disabled_rules")
            .unwrap_or_else(|_| vec![])
            .into_iter()
            .map(|name| {
                Rule::from_name(&name).ok_or_else(|| mlua::Error::FromLuaConversionError {
                    from: "string",
                    to: "sqleibniz rule".into(),
                    message: Some(format!("Unknown rule name '{name}'")),
                })
            })
            .collect::<mlua::Result<Vec<_>>>()?;
        let hooks: Option<Vec<Hook>> = table.get("hooks").ok();
        let max_hook_runtime = max_hook_runtime(table.get("max_hook_runtime")?)?;
        Ok(Self {
            disabled_rules,
            hooks,
            max_hook_runtime,
        })
    }
}

#[derive(Debug)]
/// sqleibniz allows for writing custom rules with lua
pub struct Hook {
    pub name: String,
    /// matcher limits which hook contexts execute the hook.
    /// Omitting it executes the hook for every encountered context.
    pub matcher: Option<HookMatch>,
    /// hook can be executed via `Function::call(arg)`, where args is [HookContext]
    pub hook: Option<Function>,
}

#[derive(Debug, Default)]
pub struct HookMatch {
    pub node: Option<String>,
    pub kind: Option<String>,
    pub content: Option<String>,
}

impl Hook {
    pub fn exec(&self, arg: HookContext) -> mlua::Result<()> {
        if let Some(hook) = &self.hook {
            hook.call(arg)?
        }
        Ok(())
    }
}

impl FromLua for Hook {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let table: Table = lua.unpack(value)?;
        let name = table.get("name")?;
        let matcher = table.get("match").ok();
        let hook: Option<Function> = table.get("hook").ok();
        Ok(Self {
            name,
            matcher,
            hook,
        })
    }
}

impl FromLua for HookMatch {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let table: Table = lua.unpack(value)?;
        Ok(Self {
            node: table.get("node").ok(),
            kind: table.get("kind").ok(),
            content: table.get("content").ok(),
        })
    }
}

impl UserData for Config {}
