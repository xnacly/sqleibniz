#[derive(Clone, Copy)]
pub(super) struct Module {
    pub(super) doc: &'static str,
    pub(super) create_virtual_table: bool,
}

#[derive(Clone, Copy)]
struct ModuleEntry {
    name: &'static str,
    module: Module,
}

pub(super) fn module(name: &str) -> Option<Module> {
    MODULES
        .iter()
        .find(|module| module.name.eq_ignore_ascii_case(name))
        .map(|module| module.module)
}

// https://www.sqlite.org/vtablist.html
const MODULES: &[ModuleEntry] = &[
    ModuleEntry {
        name: "bytecode",
        module: Module {
            doc: "https://www.sqlite.org/bytecodevtab.html",
            create_virtual_table: false,
        },
    },
    ModuleEntry {
        name: "carray",
        module: Module {
            doc: "https://www.sqlite.org/carray.html",
            create_virtual_table: false,
        },
    },
    ModuleEntry {
        name: "closure",
        module: Module {
            doc: "https://sqlite.org/src/file/ext/misc/closure.c",
            create_virtual_table: true,
        },
    },
    ModuleEntry {
        name: "completion",
        module: Module {
            doc: "https://www.sqlite.org/completion.html",
            create_virtual_table: true,
        },
    },
    ModuleEntry {
        name: "csv",
        module: Module {
            doc: "https://www.sqlite.org/csv.html",
            create_virtual_table: true,
        },
    },
    ModuleEntry {
        name: "dbstat",
        module: Module {
            doc: "https://www.sqlite.org/dbstat.html",
            create_virtual_table: true,
        },
    },
    ModuleEntry {
        name: "files_of_checkin",
        module: Module {
            doc: "https://fossil-scm.org/fossil/file/src/foci.c",
            create_virtual_table: false,
        },
    },
    ModuleEntry {
        name: "fsdir",
        module: Module {
            doc: "https://sqlite.org/src/file/ext/misc/fileio.c",
            create_virtual_table: false,
        },
    },
    ModuleEntry {
        name: "fts3",
        module: Module {
            doc: "https://www.sqlite.org/fts3.html",
            create_virtual_table: true,
        },
    },
    ModuleEntry {
        name: "fts4",
        module: Module {
            doc: "https://www.sqlite.org/fts3.html",
            create_virtual_table: true,
        },
    },
    ModuleEntry {
        name: "fts5",
        module: Module {
            doc: "https://www.sqlite.org/fts5.html",
            create_virtual_table: true,
        },
    },
    ModuleEntry {
        name: "generate_series",
        module: Module {
            doc: "https://www.sqlite.org/series.html",
            create_virtual_table: false,
        },
    },
    ModuleEntry {
        name: "json_each",
        module: Module {
            doc: "https://www.sqlite.org/json1.html#jeach",
            create_virtual_table: false,
        },
    },
    ModuleEntry {
        name: "json_tree",
        module: Module {
            doc: "https://www.sqlite.org/json1.html#jtree",
            create_virtual_table: false,
        },
    },
    ModuleEntry {
        name: "pragma",
        module: Module {
            doc: "https://www.sqlite.org/pragma.html#pragfunc",
            create_virtual_table: false,
        },
    },
    ModuleEntry {
        name: "rtree",
        module: Module {
            doc: "https://www.sqlite.org/rtree.html",
            create_virtual_table: true,
        },
    },
    ModuleEntry {
        name: "spellfix1",
        module: Module {
            doc: "https://www.sqlite.org/spellfix1.html",
            create_virtual_table: true,
        },
    },
    ModuleEntry {
        name: "sqlite_btreeinfo",
        module: Module {
            doc: "https://sqlite.org/src/file/ext/misc/btreeinfo.c",
            create_virtual_table: false,
        },
    },
    ModuleEntry {
        name: "sqlite_dbpage",
        module: Module {
            doc: "https://www.sqlite.org/dbpage.html",
            create_virtual_table: true,
        },
    },
    ModuleEntry {
        name: "sqlite_memstat",
        module: Module {
            doc: "https://www.sqlite.org/memstat.html",
            create_virtual_table: false,
        },
    },
    ModuleEntry {
        name: "sqlite_stmt",
        module: Module {
            doc: "https://www.sqlite.org/stmt.html",
            create_virtual_table: false,
        },
    },
    ModuleEntry {
        name: "swarmvtab",
        module: Module {
            doc: "https://www.sqlite.org/swarmvtab.html#overview",
            create_virtual_table: true,
        },
    },
    ModuleEntry {
        name: "tables_used",
        module: Module {
            doc: "https://www.sqlite.org/bytecodevtab.html",
            create_virtual_table: false,
        },
    },
    ModuleEntry {
        name: "tclvar",
        module: Module {
            doc: "https://sqlite.org/src/file/src/test_tclvar.c",
            create_virtual_table: true,
        },
    },
    ModuleEntry {
        name: "templatevtab",
        module: Module {
            doc: "https://sqlite.org/src/file/ext/misc/templatevtab.c",
            create_virtual_table: true,
        },
    },
    ModuleEntry {
        name: "unionvtab",
        module: Module {
            doc: "https://www.sqlite.org/unionvtab.html",
            create_virtual_table: true,
        },
    },
    ModuleEntry {
        name: "vfsstat",
        module: Module {
            doc: "https://sqlite.org/src/file/ext/misc/vfsstat.c",
            create_virtual_table: true,
        },
    },
    ModuleEntry {
        name: "vtablog",
        module: Module {
            doc: "https://sqlite.org/src/file/ext/misc/vtablog.c",
            create_virtual_table: true,
        },
    },
    ModuleEntry {
        name: "wholenumber",
        module: Module {
            doc: "https://sqlite.org/src/file/ext/misc/wholenumber.c",
            create_virtual_table: true,
        },
    },
    ModuleEntry {
        name: "zipfile",
        module: Module {
            doc: "https://www.sqlite.org/zipfile.html",
            create_virtual_table: true,
        },
    },
];
