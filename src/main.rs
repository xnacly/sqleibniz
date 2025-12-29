#![allow(dead_code)]
#[cfg(feature = "trace")]
use std::time::SystemTime;
use std::{fs, process::exit, vec};

use clap::Parser;
use error::{print_str_colored, warn};
use highlight::builder;
use lexer::Lexer;
use types::config::Config;
use types::rules::Rule;

use crate::parser::nodes::Node;

/// error does formatting and highlighting for errors
mod error;
/// highlight implements logic for highlighting tokens found in a string
mod highlight;
/// lev implements the levenshtein distance for all sql keywords, this is used to recommend a keyword based on a misspelled word or any
/// unknown keyword at an arbitrary location in the source statement - mainly used at the start of a new statement
mod lev;
/// lexer converts the input into a stream of token for the parser
mod lexer;
/// lsp implements the language server protocol to provide diagnostics, suggestions and snippets for sql based on the sqleibniz tooling
mod lsp;
/// parser converts the token stream into an abstract syntax tree
mod parser;
/// types holds all shared types between the above modules
mod types;

/// LSP and analysis cli for sql. Check for valid syntax, semantics and perform dynamic analysis.
#[derive(clap::Parser)]
#[command(about, version, long_about=None)]
struct Cli {
    /// instruct sqleibniz to ignore the configuration, if specified
    #[arg(short, long)]
    ignore_config: bool,

    /// files to analyse
    paths: Vec<String>,

    /// path to the configuration
    #[arg(short = 'c', long, default_value = "leibniz.lua")]
    config: String,

    /// disable stdout/stderr output
    #[arg(short = 's', long)]
    silent: bool,

    /// disable diagnostics by their rules, all are enabled by default - this may change in the
    /// future
    #[arg(short = 'D')]
    #[clap(value_enum)]
    disable: Option<Vec<Rule>>,

    /// dump the abstract syntax tree as pretty printed json
    #[arg(long)]
    ast_json: bool,
    /// dump the abstract syntax tree as rusts pretty printed debugging
    #[arg(long)]
    ast: bool,

    /// invoke sqleibniz as a language server
    #[arg(long)]
    lsp: bool,
}

fn configuration(lua: &mlua::Lua, file_name: &str) -> Result<Config, String> {
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
    let conf: Config = lua
        .unpack(raw_conf)
        .map_err(|err| format!("{}: {}", file_name, err))?;
    Ok(conf)
}

struct FileResult {
    name: String,
    errors: usize,
    ignored_errors: usize,
}

fn main() {
    let args = Cli::parse();

    if args.lsp {
        if let Err(e) = lsp::start() {
            panic!("fatal error in language server: {}", e);
        }
        return;
    }

    let mut error_string_builder = builder::Builder::default();

    if args.paths.is_empty() {
        if !args.silent {
            error::err(
                &mut error_string_builder,
                "no source file(s) provided, exiting",
            );
            print!("{}", error_string_builder.string())
        }
        exit(1);
    }

    let mut config = Config {
        disabled_rules: vec![],
        hooks: None,
    };

    if !args.ignore_config {
        // lua defined here because it would be dropped at the end of configuration(), in the
        // future this will probably need to be moved one scope up to life long enough for analysis
        let lua = mlua::Lua::new();
        match configuration(&lua, &args.config) {
            Ok(conf) => config = conf,
            Err(err) => {
                if !args.silent {
                    error::warn(&mut error_string_builder, &err.to_string());
                }
            }
        }
    }

    if let Some(rules) = args.disable {
        let mut p = rules.clone();
        config.disabled_rules.append(&mut p);
    }

    if !config.disabled_rules.is_empty() && !args.silent {
        let mut ignore_buffer = builder::Builder::default();
        warn(
            &mut ignore_buffer,
            "Ignoring the following diagnostics, as specified:",
        );
        for rule in &config.disabled_rules {
            print_str_colored(&mut ignore_buffer, " -> ", error::Color::Blue);
            ignore_buffer.write_str(rule.name());
            ignore_buffer.write_char('\n');
        }
        print!("{}", ignore_buffer.string())
    }

    let mut files = args
        .paths
        .into_iter()
        .map(|name| FileResult {
            name,
            errors: 0,
            ignored_errors: 0,
        })
        .collect::<Vec<FileResult>>();

    #[cfg(feature = "trace")]
    let start = SystemTime::now();

    for file in &mut files {
        let mut errors = vec![];
        let content = match fs::read(&file.name) {
            Ok(c) => c,
            Err(err) => {
                if !args.silent {
                    error::err(
                        &mut error_string_builder,
                        &format!("failed to read file '{}': {}", file.name, err),
                    );
                }
                print!("{}", error_string_builder.string());
                exit(1);
            }
        };
        let mut ignored_errors = 0;
        let mut lexer = Lexer::new(&content, file.name.as_str());
        let toks = lexer.run();
        errors.push(lexer.errors);

        if !toks.is_empty() {
            #[cfg(feature = "trace")]
            println!("{:=^72}", " CALLSTACK ");
            let mut parser = parser::Parser::new(toks.clone(), file.name.as_str());
            let ast = parser.parse();
            #[cfg(feature = "trace")]
            {
                println!("{:=^72}", " AST ");
                for node in &ast {
                    if let Some(node) = node {
                        node.display(0);
                    }
                }
            }

            if args.ast_json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &ast.iter()
                            .flat_map(|n| { n.as_ref().map(|n| n.as_serializable()) })
                            .collect::<Vec<_>>()
                    )
                    .unwrap_or_default()
                );
            }

            if args.ast {
                println!(
                    "{:#?}",
                    &ast.iter().flatten().collect::<Vec<&Box<dyn Node>>>()
                );
            }

            errors.push(parser.errors);
        }

        let processed_errors = errors
            .iter()
            .flatten()
            .filter(|e| {
                if config.disabled_rules.contains(&e.rule) {
                    ignored_errors += 1;
                    false
                } else {
                    true
                }
            })
            .collect::<Vec<&error::Error>>();

        if !processed_errors.is_empty() && !args.silent {
            error::print_str_colored(
                &mut error_string_builder,
                &format!("{:=^72}\n", format!(" {} ", file.name)),
                error::Color::Blue,
            );
            let error_count = processed_errors.len();
            for (i, e) in processed_errors.iter().enumerate() {
                (**e)
                    .clone()
                    .print(&mut error_string_builder, &content, &toks);

                if i + 1 != error_count {
                    error_string_builder.write_char('\n');
                }
            }
        }
        file.errors = processed_errors.len();
        file.ignored_errors = ignored_errors;
    }
    #[cfg(feature = "trace")]
    let took = SystemTime::now().duration_since(start).unwrap();

    if args.silent {
        let verified = files.iter().filter(|f| f.errors == 0).count();
        if verified != files.len() {
            exit(1);
        }
        return;
    }

    error::print_str_colored(
        &mut error_string_builder,
        &format!("{:=^72}\n", " Summary "),
        error::Color::Blue,
    );
    for file in &files {
        error::print_str_colored(
            &mut error_string_builder,
            &format!(
                "[{}]",
                match file.errors {
                    0 => '+',
                    _ => '-',
                }
            ),
            match file.errors {
                0 => error::Color::Green,
                _ => error::Color::Red,
            },
        );
        error_string_builder.write_char(' ');
        error_string_builder.write_str(&file.name);
        error_string_builder.write_char(':');
        error_string_builder.write_char('\n');
        error::print_str_colored(
            &mut error_string_builder,
            &format!("    {} Error(s) detected\n", file.errors),
            match file.errors {
                0 => error::Color::Green,
                _ => error::Color::Red,
            },
        );
        error::print_str_colored(
            &mut error_string_builder,
            &format!("    {} Error(s) ignored\n", file.ignored_errors),
            match file.ignored_errors {
                0 => error::Color::Green,
                _ => error::Color::Yellow,
            },
        )
    }
    error_string_builder.write_char('\n');
    print_str_colored(&mut error_string_builder, "=>", error::Color::Blue);
    let verified = files.iter().filter(|f| f.errors == 0).count();
    #[cfg(feature = "trace")]
    println!("took: [{:?}]", took);
    error_string_builder.write_string(format!(
        " {}/{} Files verified successfully, {} verification failed.\n",
        verified,
        files.len(),
        files.len() - verified
    ));

    if !args.silent {
        print!("{}", error_string_builder.string());
    }

    if verified != files.len() {
        exit(1);
    }
}
