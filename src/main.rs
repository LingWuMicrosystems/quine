use std::env;
use std::{borrow::Cow, fs, path::PathBuf};

use quine_frontend::compile::compile_command;
use quine_frontend::compile::head2query::heads2query;
use quine_frontend::syntax::Command;
use quine_frontend::EngineContext;

use quine::pest_parser::{parse_file, parse_repl_commands};
use quine_frontend::prelude::register_prelude;

use quine_core::common::Set;
use quine_core::rule::VariableRecord;
use quine_core::table::Row;

use directories::ProjectDirs;
use reedline::{
    FileBackedHistory, Prompt, PromptEditMode, PromptHistorySearch, Reedline, Signal,
    ValidationResult, Validator,
};

#[derive(Clone)]
struct QuinePrompt;
impl Prompt for QuinePrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
    fn render_prompt_indicator(&self, _prompt_mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("> ")
    }
    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed(". ")
    }
    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        Cow::Owned(format!("(search: {})> ", history_search.term))
    }
}

fn get_history_path() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("", "", "quine") {
        let data_dir = proj_dirs.data_dir();
        if !data_dir.exists() {
            fs::create_dir_all(data_dir).unwrap();
        }
        data_dir.join("history.txt")
    } else {
        PathBuf::from("history.txt")
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut ctx = EngineContext::default();
    register_prelude(&mut ctx);

    if args.len() == 1 {
        run_repl(&mut ctx);
    } else if args.len() == 2 {
        run_file(&mut ctx, &args[1].clone().into()).unwrap();
    } else {
        eprintln!("invalid params size")
    }
}

fn run_file(ctx: &mut EngineContext, path: &PathBuf) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("{e}"))?;
    let cmds = parse_file(&content)?;
    execute_file_commands(ctx, cmds)
}

fn execute_repl_source(ctx: &mut EngineContext, source: &str) -> Result<(), String> {
    let trimmed = source.trim();
    if let Some(path) = trimmed.strip_prefix(":load ") {
        let path = path.trim().trim_matches('"');
        return run_file(ctx, &PathBuf::from(path));
    }
    let cmds = parse_repl_commands(source)?;
    execute_repl_commands(ctx, cmds)
}

fn execute_file_commands(ctx: &mut EngineContext, cmds: Vec<Command>) -> Result<(), String> {
    for cmd in cmds {
        execute_file_command(ctx, cmd)?;
    }
    Ok(())
}

fn execute_file_command(ctx: &mut EngineContext, cmd: Command) -> Result<(), String> {
    match &cmd {
        Command::Query(heads, vars) => {
            let query = heads2query(heads, &ctx.table_types, &ctx.data_types, &mut ctx.interner)
                .map_err(|e| format!("{:?}", e))?;
            let (var_record, rows) = ctx.query(&query, vars);
            print_query_result(&var_record, rows, ctx);
            return Ok(());
        }
        _ => {}
    }
    let unit = compile_command(
        &cmd,
        &mut ctx.data_types,
        &mut ctx.table_types,
        &mut ctx.interner,
        &ctx.native_names,
        &ctx.native_signatures,
    )
    .map_err(|e| format!("{:?}", e))?;
    ctx.apply(unit);
    Ok(())
}

fn execute_repl_commands(ctx: &mut EngineContext, cmds: Vec<Command>) -> Result<(), String> {
    for cmd in cmds {
        match cmd {
            Command::Query(heads, vars) => {
                let query =
                    heads2query(&heads, &ctx.table_types, &ctx.data_types, &mut ctx.interner)
                        .map_err(|e| format!("{:?}", e))?;
                let (var_record, rows) = ctx.query(&query, &vars);
                print_query_result(&var_record, rows, ctx);
            }
            _ => {
                let unit = compile_command(
                    &cmd,
                    &mut ctx.data_types,
                    &mut ctx.table_types,
                    &mut ctx.interner,
                    &ctx.native_names,
                    &ctx.native_signatures,
                )
                .map_err(|e| format!("{:?}", e))?;
                ctx.apply(unit);
            }
        }
    }
    Ok(())
}

fn print_query_result(var_record: &VariableRecord, rows: Set<Row>, ctx: &EngineContext) {
    for row in rows {
        for (name, offset) in &var_record.names_map {
            let ty = var_record.get_type(*offset).unwrap();
            let value = row.0.get(*offset).unwrap();
            let term = ctx.extract(*value, ty);
            print!("{name}: {term}");
        }
        println!();
    }
}

struct QuineValidator;
impl Validator for QuineValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed == "exit" || trimmed == "quit" {
            return ValidationResult::Complete;
        }
        match parse_repl_commands(line) {
            Ok(_) => ValidationResult::Complete,
            Err(_) => ValidationResult::Incomplete,
        }
    }
}

fn run_repl(ctx: &mut EngineContext) {
    let validator = Box::new(QuineValidator);
    let history_file = get_history_path();
    let history = Box::new(FileBackedHistory::with_file(1000, history_file).unwrap());
    let mut line_editor = Reedline::create()
        .with_validator(validator)
        .with_history(history);
    let prompt = QuinePrompt;

    println!("Quine 0.1.0");

    loop {
        let sig = line_editor.read_line(&prompt).unwrap();
        match sig {
            Signal::Success(buffer) => {
                let input = buffer.trim_start();
                if input.is_empty() {
                    continue;
                }

                if input == "exit" || input == "quit" {
                    break;
                }

                if let Err(e) = execute_repl_source(ctx, input) {
                    eprintln!("error: {e}");
                }
            }
            Signal::CtrlC | Signal::CtrlD => break,
            _ => {}
        }
    }
}
