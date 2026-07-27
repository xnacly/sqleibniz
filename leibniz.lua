-- Example sqleibniz configuration.

---@class SqleibnizHookNode
---@field node string
---@field kind string
---@field content string
---@field text string
---@field line integer
---@field start integer
---@field finish integer
---@field children SqleibnizHookNode[]

---@class SqleibnizApi
---@field diagnostic fun(node: SqleibnizHookNode, note: string)
---@field is_keyword fun(value: string|SqleibnizHookNode): boolean
---@field is_type_name fun(value: string|SqleibnizHookNode): boolean

---@type SqleibnizApi
sqleibniz = sqleibniz

leibniz = {
    -- Maximum execution time for one custom hook invocation. Accepts milliseconds
    -- as a number or a string such as "10ms". The default is 10ms.
    max_hook_runtime = "10ms",

    disabled_rules = {
        -- Ignore project-level diagnostics by default.
        "file/no-content",           -- source file is empty
        "file/no-statements",        -- source file contains no statements
        "sqleibniz/unimplemented",   -- construct is not implemented yet
        "sqleibniz/bad-instruction", -- source file contains a bad sqleibniz instruction
        -- "sqleibniz/hook",             -- a user-defined Lua hook reported a diagnostic

        -- Uncomment sqlite diagnostics to ignore them.
        -- "sqlite/unsupported", -- Source file uses sql features sqlite does not support
        -- "sqlite/unknown-pragma", -- Source file uses a PRAGMA not documented by SQLite
        -- "sqlite/quirk", -- Sqlite or SQL quirk: https://www.sqlite.org/quirks.html
        -- "sql/unknown-keyword", -- an unknown keyword was encountered
        -- "sql/unterminated-string", -- a not closed string was found
        -- "sql/unknown-character", -- an unknown character was found
        -- "sql/invalid-numeric-literal", -- an invalid numeric literal was found
        -- "sql/invalid-blob", -- an invalid blob literal was found (either bad hex data or incorrect syntax)
        -- "sql/syntax", -- a structure with incorrect syntax was found
        -- "sql/missing-semicolon", -- a semicolon is missing
    },

    -- Custom diagnostics written in Lua.
    hooks = {
        {
            name = "idents should be lowercase",
            match = { node = "Token", kind = "Ident" },
            hook = function(node)
                if string.match(node.content, "%u") and not sqleibniz.is_type_name(node) then
                    sqleibniz.diagnostic(node, "All idents should be lowercase")
                end
            end
        },
        {
            name = "idents shouldn't be longer than 52 characters",
            match = { node = "Token", kind = "Ident" },
            hook = function(node)
                local max_size = 52
                if string.len(node.content) >= max_size then
                    sqleibniz.diagnostic(
                        node,
                        "idents shouldn't be longer than " .. max_size .. " characters"
                    )
                end
            end
        }
    }
}
