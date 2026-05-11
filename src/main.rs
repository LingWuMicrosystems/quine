use std::{borrow::Cow, fs, path::PathBuf};

use quine::{
    engine::EngineContext,
    pest_parser::parse_commands,
    types::{BaseType, Type},
};

use directories::ProjectDirs;
use reedline::{
    DefaultValidator, FileBackedHistory, Prompt, PromptEditMode, PromptHistorySearch, Reedline,
    Signal,
};

#[derive(Clone)]
struct QuinePrompt;
impl Prompt for QuinePrompt {
    fn render_prompt_left<'a>(&'a self) -> Cow<'a, str> {
        Cow::Borrowed("")
    }
    fn render_prompt_right<'a>(&'a self) -> Cow<'a, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator<'a>(&'a self, _prompt_mode: PromptEditMode) -> Cow<'a, str> {
        Cow::Borrowed("> ")
    }
    fn render_prompt_multiline_indicator<'a>(&'a self) -> Cow<'a, str> {
        Cow::Borrowed(". ")
    }
    fn render_prompt_history_search_indicator<'a>(
        &'a self,
        history_search: PromptHistorySearch,
    ) -> Cow<'a, str> {
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
    let validator = Box::new(DefaultValidator);
    let history_file = get_history_path();
    let history = Box::new(FileBackedHistory::with_file(1000, history_file).unwrap());
    let mut line_editor = Reedline::create()
        .with_validator(validator)
        .with_history(history);
    let prompt = QuinePrompt;

    println!("Quine 0.1.0");

    let mut engine_context = EngineContext::default();

    loop {
        let sig = line_editor.read_line(&prompt).unwrap();
        match sig {
            Signal::Success(buffer) => {
                let input = buffer.trim_start();
                if input.is_empty() {
                    continue;
                }

                let cmds = parse_commands(input);
                let Ok(cmds) = cmds else {
                    eprintln!("error: {:?}", cmds.unwrap_err());
                    continue;
                };
                let cmds: Result<Vec<_>, _> = cmds
                    .into_iter()
                    .map(|cmd| engine_context.compile_env.check_and_compile_command(cmd))
                    .collect();
                let Ok(cmds) = cmds else {
                    eprintln!("error: {:?}", cmds.unwrap_err());
                    continue;
                };
                for cmd in cmds {
                    let result = engine_context.run_command(cmd);
                    let Ok(result) = result else {
                        eprintln!("error: {:?}", result.unwrap_err());
                        continue;
                    };
                    if let Some((var_record, rows)) = result {
                        for row in rows {
                            for (name, offset) in &var_record.names_map {
                                let ty = var_record.get_type(*offset).unwrap();
                                let value = row.0.get(*offset).unwrap();
                                let value = match ty {
                                    Type::Base(ty) => match ty {
                                        BaseType::Id => todo!(),
                                        BaseType::I1 => {
                                            if value.0 == 0 {
                                                "false".to_owned()
                                            } else {
                                                "true".to_owned()
                                            }
                                        }
                                        BaseType::I8 => (value.0 as i8).to_string(),
                                        BaseType::U8 => (value.0 as i8).to_string(),
                                        BaseType::I16 => (value.0 as i16).to_string(),
                                        BaseType::U16 => (value.0 as u16).to_string(),
                                        BaseType::I32 => (value.0 as i32).to_string(),
                                        BaseType::U32 => value.0.to_string(),
                                        BaseType::F32 => (value.0 as f32).to_string(),
                                        _ => unimplemented!(),
                                    },
                                    Type::Name(_) => todo!(),
                                };
                                print!("{name}: {value}\t");
                            }
                            println!();
                        }
                    }
                }
            }
            Signal::CtrlC | Signal::CtrlD => {
                break;
            }
            _ => {}
        }
    }
}
