-- this is an example configuration, consult: https://www.lua.org/manual/5.4/
-- or https://learnxinyminutes.com/docs/lua/ for syntax help and
-- src/rules.rs::Config for all available options
leibniz = {
    disabled_rules = {
        -- ignore sqleibniz specific diagnostics:
        "NoContent",               -- source file is empty
        "NoStatements",            -- source file contains no statements
        "Unimplemented",           -- construct is not implemented yet
        "BadSqleibnizInstruction", -- source file contains a bad sqleibniz instruction

        -- ignore sqlite specific diagnostics:

        -- "SqliteUnsupported", -- Source file uses sql features sqlite does not support
        -- "Quirk", -- Sqlite or SQL quirk: https://www.sqlite.org/quirks.html
        -- "UnknownKeyword", -- an unknown keyword was encountered
        -- "UnterminatedString", -- a not closed string was found
        -- "UnknownCharacter", -- an unknown character was found
        -- "InvalidNumericLiteral", -- an invalid numeric literal was found
        -- "InvalidBlob", -- an invalid blob literal was found (either bad hex data or incorrect syntax)
        -- "Syntax", -- a structure with incorrect syntax was found
        -- "Semicolon", -- a semicolon is missing
    },
    -- sqleibniz allows for writing custom rules with lua
    hooks = {
        {
            -- summarises the hooks content
            name = "idents should be lowercase",
            -- instructs sqleibniz which node to execute the `hook` for
            node = "literal",
            -- sqleibniz calls the hook function once it encounters a node name
            -- matching the hook.node content
            --
            -- The `node` argument holds the following fields:
            --
            --```
            --    node: {
            --     kind: string,
            --     content: string,
            --     children: node[],
            --    }
            --```
            --
            hook = function(node)
                if node.kind == "ident" then
                    if string.match(node.content, "%u") then
                        -- returing an error passes the diagnostic to sqleibniz,
                        -- thus a pretty message with the name of the hook, the
                        -- node it occurs and the message passed to error() is
                        -- generated
                        error("All idents should be lowercase")
                    end
                end
            end
        },
        {
            name = "idents shouldn't be longer than 12 characters",
            node = "literal",
            hook = function(node)
                local max_size = 12
                if node.kind == "ident" then
                    if string.len(node.content) >= max_size then
                        error("idents shouldn't be longer than " .. max_size .. " characters")
                    end
                end
            end
        },
        {
            name = "hook test",
            hook = function(node)
                print(node.kind .. " " .. node.text .. " " .. #node.children)
            end
        }
    }
}
