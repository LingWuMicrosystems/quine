use std::env;
use std::{borrow::Cow, fs, path::PathBuf};

use quine::{
    engine::EngineContext,
    engine::compile::Compiler,
    engine::compile::head2query::heads2query,
    syntax::{Command, ReplCommand},
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

// ── prompt ──────────────────────────────────────────────────────────

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

// ── main ────────────────────────────────────────────────────────────

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut ctx = EngineContext::default();

    let mut module_paths: Vec<PathBuf> = Vec::new();
    let mut script_path: Option<PathBuf> = None;
    let mut force_repl = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-m" => {
                i += 1;
                if let Some(path) = args.get(i) {
                    module_paths.push(PathBuf::from(path));
                }
            }
            "-e" | "--eval" => {
                i += 1;
                if let Some(source) = args.get(i) {
                    load_modules(&mut ctx, &module_paths);
                    if let Err(e) = eval_source(&mut ctx, source) {
                        eprintln!("error: {e}");
                    }
                    if !force_repl {
                        return;
                    }
                }
            }
            "--repl" => {
                force_repl = true;
            }
            positional => {
                script_path = Some(PathBuf::from(positional));
            }
        }
        i += 1;
    }

    load_modules(&mut ctx, &module_paths);

    if let Some(script) = script_path {
        if let Err(e) = run_script(&mut ctx, &script) {
            eprintln!("error: {e}");
        }
        if !force_repl {
            return;
        }
    }

    run_repl(&mut ctx);
}

// ── module loading ──────────────────────────────────────────────────

fn load_modules(ctx: &mut EngineContext, paths: &[PathBuf]) {
    for path in paths {
        if let Err(e) = load_module(ctx, path) {
            eprintln!("error: {e}");
        }
    }
}

fn load_module(ctx: &mut EngineContext, path: &PathBuf) -> Result<(), String> {
    if path.is_dir() {
        let mut files: Vec<PathBuf> = Vec::new();
        collect_ql_files(path, &mut files);
        files.sort();
        for file in &files {
            run_file(ctx, file)?;
        }
    } else {
        run_file(ctx, path)?;
    }
    Ok(())
}

fn collect_ql_files(dir: &PathBuf, out: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_ql_files(&path, out);
            } else if path.extension().map_or(false, |e| e == "ql") {
                out.push(path);
            }
        }
    }
}

fn run_file(ctx: &mut EngineContext, path: &PathBuf) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("{e}"))?;
    let cmds = parse_file(&content)?;
    execute_file_commands(ctx, cmds)
}

// ── script / eval ───────────────────────────────────────────────────

fn run_script(ctx: &mut EngineContext, path: &PathBuf) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("{e}"))?;
    execute_repl_source(ctx, &content)
}

fn eval_source(ctx: &mut EngineContext, source: &str) -> Result<(), String> {
    execute_repl_source(ctx, source)
}

fn execute_repl_source(ctx: &mut EngineContext, source: &str) -> Result<(), String> {
    let cmds = parse_repl_commands(source)?;
    execute_repl_commands(ctx, cmds)
}

// ── command execution ───────────────────────────────────────────────

fn execute_file_commands(ctx: &mut EngineContext, cmds: Vec<Command>) -> Result<(), String> {
    for cmd in cmds {
        execute_file_command(ctx, cmd)?;
    }
    Ok(())
}

fn execute_file_command(ctx: &mut EngineContext, cmd: Command) -> Result<(), String> {
    if let Command::Load(path) = &cmd {
        return load_module(ctx, &PathBuf::from(path));
    }
    let unit = Compiler::compile_command(
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

fn execute_repl_commands(ctx: &mut EngineContext, cmds: Vec<ReplCommand>) -> Result<(), String> {
    for cmd in cmds {
        match cmd {
            ReplCommand::Cmd(cmd) => execute_file_command(ctx, cmd)?,
            ReplCommand::Fact(fact) => {
                let unit = Compiler::compile_fact(
                    &fact,
                    &ctx.table_types,
                    &mut ctx.interner,
                    &ctx.native_names,
                    &ctx.native_signatures,
                )
                .map_err(|e| format!("{:?}", e))?;
                ctx.apply(unit);
            }
            ReplCommand::Query(heads, vars) => {
                let query = heads2query(
                    &heads,
                    &ctx.table_types,
                    &ctx.data_types,
                    &mut ctx.interner,
                )
                .map_err(|e| format!("{:?}", e))?;
                let (var_record, rows) = ctx.run_query(&query, &vars);
                print_query_result(&var_record, rows, ctx);
            }
            ReplCommand::Run => {
                ctx.run();
            }
            ReplCommand::Extract(name) => {
                eprintln!("extract: not yet implemented for '{name}'");
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

// ── REPL ────────────────────────────────────────────────────────────

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
