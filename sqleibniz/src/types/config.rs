use std::fs;

use mlua::{FromLua, Function, Table, UserData};

use super::{ctx::HookContext, rules::Rule};

#[derive(Debug)]
/// Configuration is expected to be at ./leibniz.lua - its existence is not required for the program invocation
pub struct Config {
    /// holds the rules that the user wants to not see errors for.
    pub disabled_rules: Vec<Rule>,
    /// holds the hooks the user wants to execute
    pub hooks: Option<Vec<Hook>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            disabled_rules: vec![],
            hooks: None,
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
        Ok(Self {
            disabled_rules: table.get("disabled_rules").unwrap_or_else(|_| vec![]),
            hooks: None,
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
        let disabled_rules: Vec<Rule> = table.get("disabled_rules").unwrap_or_else(|_| vec![]);
        let hooks: Option<Vec<Hook>> = table.get("hooks").ok();
        Ok(Self {
            disabled_rules,
            hooks,
        })
    }
}

#[derive(Debug)]
/// sqleibniz allows for writing custom rules with lua
pub struct Hook {
    pub name: String,
    /// node is optional, because omitting it executes the hook for every encountered node
    pub node: Option<String>,
    /// hook can be executed via [Function::exec]`(arg)`, where args is [HookContext]
    pub hook: Option<Function>,
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
        let node = table.get("node").ok();
        let hook: Option<Function> = table.get("hook").ok();
        // INFO: lua function call example
        //
        // if let Some(h) = &hook {
        //     let () = dbg!(h.call(HookContext {
        //         kind: "ident".into(),
        //         text: Some("UPPERCASE".into()),
        //         children: vec![],
        //     }))?;
        // }
        Ok(Self { name, node, hook })
    }
}

impl UserData for Config {}
impl UserData for Rule {}
