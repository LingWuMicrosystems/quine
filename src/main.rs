use std::env;
use std::{borrow::Cow, fs, path::PathBuf};

use quine::{
    engine::EngineContext,
    syntax::Command,
    syntax::pest_parser::{parse_file, parse_repl_commands},
};

use directories::ProjectDirs;
use quine_core::common::Set;
use quine_core::rule::VariableRecord;
use quine_core::table::Row;
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
    let mut i = 1;
    let mut has_file = false;
    let mut force_repl = false;

    while i < args.len() {
        match args[i].as_str() {
            "-e" | "--eval" => {
                i += 1;
                if let Some(source) = args.get(i) {
                    match execute_source(&mut ctx, source) {
                        Ok(()) => {}
                        Err(e) => eprintln!("error: {e}"),
                    }
                }
            }
            "--repl" => {
                force_repl = true;
            }
            file => {
                has_file = true;
                match run_file(&mut ctx, file) {
                    Ok(()) => {}
                    Err(e) => eprintln!("error: {e}"),
                }
            }
        }
        i += 1;
    }

    let enter_repl = !has_file || force_repl;

    if enter_repl {
        run_repl(&mut ctx);
    }
}

fn execute_source(ctx: &mut EngineContext, source: &str) -> Result<(), String> {
    let cmds = parse_file(source)?;
    execute_commands(ctx, cmds)
}

fn run_file(ctx: &mut EngineContext, path: &str) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("{e}"))?;
    let cmds = parse_file(&content)?;
    execute_commands(ctx, cmds)
}

struct QuineValidator;
impl Validator for QuineValidator {
    fn validate(&self, line: &str) -> ValidationResult {
        let trimmed = line.trim_start();
        if trimmed.is_empty()
            || trimmed == "exit"
            || trimmed == "quit"
            || trimmed.starts_with("load \"")
        {
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

                // REPL meta-commands
                match input {
                    "exit" | "quit" => break,
                    cmd if cmd.starts_with("load \"") => {
                        let path = &cmd[6..cmd.len() - 1];
                        match run_file(ctx, path) {
                            Ok(()) => {}
                            Err(e) => eprintln!("error: {e}"),
                        }
                        continue;
                    }
                    _ => {}
                }

                let cmds = parse_repl_commands(input);
                let Ok(cmds) = cmds else {
                    eprintln!("error: {:?}", cmds.unwrap_err());
                    continue;
                };
                if let Err(e) = execute_commands(ctx, cmds) {
                    eprintln!("error: {e:?}");
                }
            }
            Signal::CtrlC | Signal::CtrlD => break,
            _ => {}
        }
    }
}

fn execute_commands(ctx: &mut EngineContext, cmds: Vec<Command>) -> Result<(), String> {
    let cmds: Result<Vec<_>, _> = cmds
        .into_iter()
        .map(|cmd| ctx.check_and_compile_command(cmd))
        .collect();
    let Ok(cmds) = cmds else {
        return Err(format!("{:?}", cmds.unwrap_err()));
    };
    for cmd in cmds {
        if let Some((var_record, rows)) = ctx.run_command(cmd) {
            print_query_result(&var_record, rows, ctx);
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
