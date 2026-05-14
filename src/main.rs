use std::env;
use std::{borrow::Cow, fs, path::PathBuf};

use quine::{
    engine::{frontend::syntax::Command, EngineContext},
    pest_parser::{parse_commands, parse_file},
    regraph::{
        common::Set,
        rule::VariableRecord,
        table::Row,
    },
};

use directories::ProjectDirs;
use reedline::{
    DefaultValidator, FileBackedHistory, Prompt, PromptEditMode, PromptHistorySearch, Reedline,
    Signal,
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
    let mut enter_repl = true;

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
            file => {
                enter_repl = false;
                match run_file(&mut ctx, file) {
                    Ok(()) => {}
                    Err(e) => eprintln!("error: {e}"),
                }
            }
        }
        i += 1;
    }

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

fn run_repl(ctx: &mut EngineContext) {
    let validator = Box::new(DefaultValidator);
    let history_file = get_history_path();
    let history = Box::new(FileBackedHistory::with_file(1000, history_file).unwrap());
    let mut line_editor = Reedline::create()
        .with_validator(validator)
        .with_history(history);
    let prompt = QuinePrompt;

    println!("Quine 0.1.0");
    println!("Type :exit to quit, :load <file> to load a file");

    loop {
        let sig = line_editor.read_line(&prompt).unwrap();
        match sig {
            Signal::Success(buffer) => {
                let input = buffer.trim_start();
                if input.is_empty() {
                    continue;
                }

                // REPL meta-commands (:load, :exit)
                if let Some(rest) = input.strip_prefix(':') {
                    match rest.trim() {
                        "exit" | "quit" | "q" => break,
                        cmd if cmd.starts_with("load ") => {
                            let path = cmd.strip_prefix("load ").unwrap().trim().trim_matches('"');
                            match run_file(ctx, path) {
                                Ok(()) => {}
                                Err(e) => eprintln!("error: {e}"),
                            }
                        }
                        _ => eprintln!("unknown command: {input}"),
                    }
                    continue;
                }

                let cmds = parse_commands(input);
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
        let result = ctx.run_command(cmd);
        let Some((var_record, rows)) = result else {
            continue;
        };
        print_query_result(&var_record, rows, ctx);
    }
    Ok(())
}

fn print_query_result(var_record: &VariableRecord, rows: Set<Row>, ctx: &EngineContext) {
    for row in rows {
        for (name, offset) in &var_record.names_map {
            let ty = var_record.get_type(*offset).unwrap();
            let value = row.0.get(*offset).unwrap();
            let term = ctx.extract(*value, ty);
            println!("{name}: {term}");
        }
    }
}
