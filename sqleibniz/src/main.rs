#![allow(dead_code)]

#[cfg(feature = "trace")]
use std::time::SystemTime;
use std::{fs, process::exit, vec};

use clap::{Parser, Subcommand};
use sqleibniz::error::{self, Error, print_str_colored, warn};
use sqleibniz::highlight::builder;
use sqleibniz::lexer::Lexer;
use sqleibniz::parser;
use sqleibniz::types::config::Config;
use sqleibniz::types::rules::Rule;

#[derive(Subcommand)]
enum Command {
    /// explain a diagnostic rule or supported SQL statement
    #[command(
        after_help = "Examples:\n  sqleibniz explain sql/syntax\n  sqleibniz explain select-stmt\n  sqleibniz explain CreateTable\n  sqleibniz explain --list-rules\n  sqleibniz explain --list-stmts"
    )]
    Explain {
        /// diagnostic rule or SQL statement name to explain
        ///
        /// Rules use their full id, for example sql/syntax or sqlite/unknown-column.
        /// Statements accept canonical names like select-stmt plus aliases like select,
        /// create-table, or CreateTable.
        name: Option<String>,

        /// list diagnostic rule ids accepted by explain and -D
        #[arg(long, conflicts_with = "name")]
        list_rules: bool,

        /// list supported SQL statement names accepted by explain
        #[arg(long, conflicts_with = "name")]
        list_stmts: bool,
    },
}

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
    #[arg(short = 's', long, conflicts_with = "sarif")]
    silent: bool,

    /// keep it simple, stupid :^): make all stdoutput small and summarizing
    #[arg(short = 'k', long, conflicts_with = "sarif")]
    kiss: bool,

    /// disable diagnostics by their rules, all are enabled by default
    ///
    /// Defaults may change in the future
    #[arg(short = 'D', value_parser = parse_rule)]
    disable: Option<Vec<Rule>>,

    /// skip AST analysis diagnostics after parsing
    #[arg(long)]
    no_analyse: bool,

    /// dump the abstract syntax tree as pretty printed json
    #[arg(long, conflicts_with = "sarif")]
    ast_json: bool,
    /// dump the abstract syntax tree as rusts pretty printed debugging
    #[arg(long, conflicts_with = "sarif")]
    ast: bool,

    /// emit SARIF 2.1.0 JSON to stdout
    #[arg(long, conflicts_with_all = ["silent", "kiss", "ast_json", "ast", "lsp"])]
    sarif: bool,

    /// invoke sqleibniz as a language server
    #[arg(long, conflicts_with = "sarif")]
    lsp: bool,
    /// execute configured Lua hooks in language server diagnostics
    #[arg(long, requires = "lsp")]
    lsp_enable_hooks: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

fn parse_rule(value: &str) -> Result<Rule, String> {
    Rule::from_name(value).ok_or_else(|| format!("unknown diagnostic rule `{value}`"))
}

struct FileResult {
    name: String,
    errors: usize,
    ignored_errors: usize,
    ignored_rules: Vec<(Rule, usize)>,
    diagnostics: Vec<Error>,
}

fn record_ignored_rule(ignored_rules: &mut Vec<(Rule, usize)>, rule: Rule) {
    if let Some((_, count)) = ignored_rules
        .iter_mut()
        .find(|(ignored_rule, _)| *ignored_rule == rule)
    {
        *count += 1;
    } else {
        ignored_rules.push((rule, 1));
    }
}

fn main() {
    let args = Cli::parse();

    if let Some(Command::Explain {
        name,
        list_rules,
        list_stmts,
    }) = args.command
    {
        if list_rules {
            print_explanation_list("Diagnostic rules", sqleibniz::explain::rules());
        }
        if list_stmts {
            print_explanation_list("SQL statements", sqleibniz::explain::sql_statements());
        }
        if list_rules || list_stmts {
            return;
        }

        let Some(name) = name else {
            eprintln!(
                "error: explain requires a name, --list-rules, or --list-stmts\n\nTry 'sqleibniz explain --help' for examples."
            );
            exit(2);
        };

        match sqleibniz::explain::lookup(&name) {
            Some(explanation) => {
                print_explanation(&explanation);
            }
            None => {
                eprintln!("error: unknown rule or SQL statement '{name}'");
                exit(1);
            }
        }
        return;
    }

    if args.lsp {
        if let Err(e) = sqleibniz::lsp::start(args.lsp_enable_hooks) {
            panic!("fatal error in language server: {}", e);
        }
        return;
    }

    let mut error_string_builder = builder::Builder::default();

    if args.paths.is_empty() {
        if !args.silent {
            if args.sarif {
                eprintln!("error: no source file(s) provided, exiting");
            } else {
                error::err(
                    &mut error_string_builder,
                    "no source file(s) provided, exiting",
                );
                print!("{}", error_string_builder.string())
            }
        }
        exit(1);
    }

    let mut config = Config::default();
    let lua = mlua::Lua::new();

    if !args.ignore_config {
        match Config::from_lua_file(&lua, &args.config) {
            Ok(conf) => config = conf,
            Err(err) => {
                if !args.silent && !args.sarif {
                    error::warn(&mut error_string_builder, &err.to_string());
                }
            }
        }
    }

    if let Some(rules) = args.disable {
        let mut p = rules.clone();
        config.disabled_rules.append(&mut p);
    }

    if !config.disabled_rules.is_empty() && !args.silent && !args.kiss && !args.sarif {
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
            ignored_rules: vec![],
            diagnostics: vec![],
        })
        .collect::<Vec<FileResult>>();

    #[cfg(feature = "trace")]
    let start = SystemTime::now();

    for file in &mut files {
        let mut errors: Vec<Error> = vec![];
        let content = match fs::read(&file.name) {
            Ok(c) => c,
            Err(err) => {
                if args.sarif {
                    eprintln!("error: failed to read file '{}': {}", file.name, err);
                } else if !args.silent {
                    error::err(
                        &mut error_string_builder,
                        &format!("failed to read file '{}': {}", file.name, err),
                    );
                    print!("{}", error_string_builder.string());
                }
                exit(1);
            }
        };
        let mut ignored_errors = 0;
        let mut ignored_rules = Vec::new();
        let mut lexer = Lexer::new(&content, file.name.as_str());
        let toks = lexer.run();
        errors.append(&mut lexer.errors);

        if !toks.is_empty() {
            #[cfg(feature = "trace")]
            println!("{:=^72}", " CALLSTACK ");
            let mut parser = parser::Parser::new(toks.clone(), file.name.as_str());
            let ast = parser.parse();
            #[cfg(feature = "trace")]
            {
                println!("{:=^72}", " AST ");
                for node in &ast {
                    node.display(0);
                }
            }

            if args.ast_json {
                println!("{}", serde_json::to_string_pretty(&ast).unwrap_or_default());
            }

            if args.ast {
                println!("{:#?}", &ast);
            }

            errors.append(&mut parser.errors);
            if !args.no_analyse {
                errors.append(&mut sqleibniz::analyse::run(&file.name, &ast));
            }

            if let Some(hooks) = config.hooks.as_deref() {
                errors.append(&mut sqleibniz::hooks::run(
                    &lua, &file.name, hooks, &ast, &toks,
                ));
            }
        }

        let mut processed_errors = errors
            .into_iter()
            .filter(|e| {
                if config.disabled_rules.contains(&e.rule) {
                    ignored_errors += 1;
                    record_ignored_rule(&mut ignored_rules, e.rule);
                    false
                } else {
                    true
                }
            })
            .collect::<Vec<error::Error>>();

        if !processed_errors.is_empty() && !args.silent && !args.sarif {
            if !args.kiss {
                error::print_str_colored(
                    &mut error_string_builder,
                    &format!("{:=^72}\n", format!(" {} ", file.name)),
                    error::Color::Blue,
                );
            }
            let error_count = processed_errors.len();
            for (i, e) in processed_errors.iter_mut().enumerate() {
                if args.kiss {
                    println!(
                        "{}: {}, {} at l:{}:{}-{}",
                        e.rule.name(),
                        e.msg,
                        e.note,
                        e.location.line,
                        e.location.start,
                        e.location.end
                    );
                } else {
                    e.print(&mut error_string_builder, &content, &toks);
                }

                if i + 1 != error_count {
                    error_string_builder.write_char('\n');
                }
            }
        }

        file.errors = processed_errors.len();
        file.ignored_errors = ignored_errors;
        file.ignored_rules = ignored_rules;
        file.diagnostics = processed_errors;
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

    if args.sarif {
        let diagnostics = files
            .iter()
            .flat_map(|file| file.diagnostics.iter().cloned())
            .collect::<Vec<_>>();
        println!(
            "{}",
            serde_json::to_string_pretty(&sqleibniz::sarif::log(&diagnostics))
                .expect("SARIF log serialization must succeed")
        );
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
            &format!("    {} Diagnostic(s) detected\n", file.errors),
            match file.errors {
                0 => error::Color::Green,
                _ => error::Color::Red,
            },
        );
        error::print_str_colored(
            &mut error_string_builder,
            &format!("    {} Diagnostic(s) ignored\n", file.ignored_errors),
            match file.ignored_errors {
                0 => error::Color::Green,
                _ => error::Color::Yellow,
            },
        );

        for (rule, count) in &file.ignored_rules {
            error::print_str_colored(&mut error_string_builder, "      -> ", error::Color::Blue);
            error_string_builder.write_str(rule.name());
            error_string_builder.write_string(format!(" {count}x\n"));
        }
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

    if !args.silent && !args.kiss {
        print!("{}", error_string_builder.string());
    }

    if verified != files.len() {
        exit(1);
    }
}

fn print_explanation(explanation: &sqleibniz::explain::Explanation) {
    let kind = match explanation.kind {
        sqleibniz::explain::ExplanationKind::Rule => "rule",
        sqleibniz::explain::ExplanationKind::SqlStatement => "sql statement",
    };
    println!("{} ({})", explanation.name, kind);
    if explanation.details.is_none()
        || matches!(explanation.kind, sqleibniz::explain::ExplanationKind::Rule)
    {
        println!("{}", explanation.description);
    }
    if let Some(documentation) = explanation.documentation {
        println!("docs: {}", documentation);
    }
    if let Some(details) = explanation.details.as_deref() {
        println!();
        println!("{details}");
    }
}

fn print_explanation_list(title: &str, explanations: Vec<sqleibniz::explain::Explanation>) {
    println!("{title}:");
    for explanation in explanations {
        println!("  {:<32} {}", explanation.name, explanation.description);
    }
}
