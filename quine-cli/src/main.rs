use std::env;
use std::path::Path;
use std::{borrow::Cow, fs, path::PathBuf};

use quine::pest_parser::{parse_file, parse_repl_commands};
use quine_frontend::EngineContext;
use quine_frontend::compile::compile_command;
use quine_frontend::compile::head2query::heads2query;
use quine_frontend::syntax::Command;

use quine_frontend::prelude::register_prelude;

use quine_core::common::{Map, Set};
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
        // Pre-scan CWD for .quine modules so `import "name"` works in the REPL.
        let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if let Err(e) = pre_scan_modules(&mut ctx, &cwd) {
            eprintln!("warning: {e}");
        }
        run_repl(&mut ctx);
    } else if args.len() == 2 {
        let path: PathBuf = args[1].clone().into();
        // Pre-scan the directory containing the loaded file.
        let scan_dir = path
            .canonicalize()
            .unwrap_or_else(|_| path.clone())
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        if let Err(e) = pre_scan_modules(&mut ctx, &scan_dir) {
            eprintln!("warning: {e}");
        }
        if let Err(e) = run_file(&mut ctx, &path) {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    } else {
        eprintln!("invalid params size")
    }
}

/// Recursively scan `root_dir` for `.quine` files and populate
/// `ctx.module_map` with stem→canonical_path entries.  Duplicate stems
/// (e.g. `src/foo.quine` and `lib/foo.quine`) are a hard error.
fn pre_scan_modules(ctx: &mut EngineContext, root_dir: &Path) -> Result<(), String> {
    let mut seen: Map<String, String> = Map::default();

    fn walk(dir: &Path, seen: &mut Map<String, String>) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| format!("read_dir {:?}: {e}", dir))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("dir entry error: {e}"))?;
            let path = entry.path();
            if path.is_dir() {
                walk(&path, seen)?;
            } else if path.extension().map_or(false, |e| e == "quine") {
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                if stem.is_empty() {
                    continue;
                }
                let canonical = path
                    .canonicalize()
                    .map_err(|e| format!("canonicalize {:?}: {e}", &path))?;
                let path_str = canonical.to_string_lossy().to_string();
                if let Some(existing) = seen.get(&stem) {
                    return Err(format!(
                        "duplicate module name \"{stem}\": {existing} and {path_str}"
                    ));
                }
                seen.insert(stem, path_str);
            }
        }
        Ok(())
    }

    walk(root_dir, &mut seen)?;
    ctx.module_map = seen;
    Ok(())
}

fn run_file(ctx: &mut EngineContext, path: &PathBuf) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("{e}"))?;
    let cmds = parse_file(&content)?;
    // Resolve base directory for relative imports inside this file.
    let base_dir = path
        .canonicalize()
        .unwrap_or_else(|_| path.clone())
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    execute_file_commands(ctx, cmds, &base_dir)
}

/// Load a file via `import` statement. Deduplicates by canonical path:
/// each file is only loaded once. Returns Ok without doing anything if
/// the file was already loaded.
///
/// Resolution order:
/// 1. Bare module name (no `.` or `/` in the name) → look up in the
///    pre-scanned `module_map`.
/// 2. Otherwise → resolve as a file path relative to `base_dir`.
fn import_file(ctx: &mut EngineContext, import_path: &str, base_dir: &Path) -> Result<(), String> {
    // Check if this is a bare module name (no extension, no path separator).
    let is_module_name = !import_path.contains('.') && !import_path.contains('/');
    let canonical_str = if is_module_name {
        if let Some(path) = ctx.module_map.get(import_path) {
            path.clone()
        } else {
            // Module not found in pre-scan; fall through to path resolution
            // so the error message mentions the file path, not the module.
            String::new()
        }
    } else {
        String::new()
    };

    let resolved = if !canonical_str.is_empty() {
        PathBuf::from(&canonical_str)
    } else if Path::new(import_path).is_absolute() {
        PathBuf::from(import_path)
    } else {
        base_dir.join(import_path)
    };

    let canonical = resolved
        .canonicalize()
        .map_err(|e| format!("cannot resolve import \"{import_path}\": {e}"))?;
    let canonical_str = canonical.to_string_lossy().to_string();
    if ctx.loaded_files.contains(&canonical_str) {
        return Ok(()); // already loaded
    }
    ctx.loaded_files.insert(canonical_str);
    run_file(ctx, &canonical)
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

fn execute_file_commands(
    ctx: &mut EngineContext,
    cmds: Vec<Command>,
    base_dir: &Path,
) -> Result<(), String> {
    // Phase 0: process imports (before type registration, so imported types
    // are available for forward references and validation).
    let mut import_errors: Vec<String> = Vec::new();
    let mut after_imports: Vec<Command> = Vec::new();
    for cmd in cmds {
        if let Command::Import(path) = &cmd {
            if let Err(e) = import_file(ctx, path, base_dir) {
                import_errors.push(e);
            }
        } else {
            after_imports.push(cmd);
        }
    }
    if !import_errors.is_empty() {
        return Err(import_errors.join("\n"));
    }

    // Pre-register type names so forward references within the same file
    // are visible to check_type_defined during compilation.
    ctx.data_types.pending_names = after_imports
        .iter()
        .filter_map(|cmd| {
            if let Command::TypeDef(name, _) = cmd {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect();

    // Phase 1: compile all TypeDefs first, collecting ALL type errors rather
    // than stopping at the first one. This lets the user see every undefined
    // type in one run.
    let mut type_errors: Vec<String> = Vec::new();
    let mut remaining: Vec<Command> = Vec::new();
    for cmd in after_imports {
        if matches!(&cmd, Command::TypeDef(..)) {
            match compile_command(
                &cmd,
                &mut ctx.data_types,
                &mut ctx.table_types,
                &mut ctx.interner,
                &ctx.native_names,
                &ctx.native_signatures,
            ) {
                Ok(unit) => ctx.apply(unit),
                Err(e) => type_errors.push(format!("{:?}", e)),
            }
        } else {
            remaining.push(cmd);
        }
    }
    if !type_errors.is_empty() {
        return Err(type_errors.join("\n"));
    }

    // Phase 2: compile everything else (rules, facts, queries, etc.).
    for cmd in remaining {
        execute_file_command(ctx, cmd)?;
    }
    Ok(())
}

fn execute_file_command(ctx: &mut EngineContext, cmd: Command) -> Result<(), String> {
    if let Command::Import(_) = &cmd {
        return Ok(()); // already processed in Phase 0
    }
    if let Command::Query(heads, vars) = &cmd {
        let query = heads2query(heads, &ctx.table_types, &ctx.data_types, &mut ctx.interner)
            .map_err(|e| format!("{:?}", e))?;
        let (var_record, rows) = ctx.query(&query, vars);
        print_query_result(&var_record, rows, ctx);
        return Ok(());
    }
    if let Command::Extract(..) = &cmd {
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
        if let Some(ref warning) = ctx.last_extract_warning {
            eprintln!("{warning}");
        }
        if let Some(ref term) = ctx.last_extract {
            println!("{term}");
        }
        return Ok(());
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
    // Phase 0: process imports (base_dir = cwd for REPL).
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut import_errors: Vec<String> = Vec::new();
    let mut after_imports: Vec<Command> = Vec::new();
    for cmd in cmds {
        if let Command::Import(path) = &cmd {
            if let Err(e) = import_file(ctx, path, &cwd) {
                import_errors.push(e);
            }
        } else {
            after_imports.push(cmd);
        }
    }
    if !import_errors.is_empty() {
        return Err(import_errors.join("\n"));
    }

    // Pre-register type names for forward reference support (same as file loading).
    ctx.data_types.pending_names = after_imports
        .iter()
        .filter_map(|cmd| {
            if let Command::TypeDef(name, _) = cmd {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect();

    // Phase 1: compile all TypeDefs first, collecting all type errors.
    let mut type_errors: Vec<String> = Vec::new();
    let mut remaining: Vec<Command> = Vec::new();
    for cmd in after_imports {
        if matches!(&cmd, Command::TypeDef(..)) {
            match compile_command(
                &cmd,
                &mut ctx.data_types,
                &mut ctx.table_types,
                &mut ctx.interner,
                &ctx.native_names,
                &ctx.native_signatures,
            ) {
                Ok(unit) => ctx.apply(unit),
                Err(e) => type_errors.push(format!("{:?}", e)),
            }
        } else {
            remaining.push(cmd);
        }
    }
    if !type_errors.is_empty() {
        return Err(type_errors.join("\n"));
    }

    // Phase 2: compile everything else.
    for cmd in remaining {
        match cmd {
            Command::Query(heads, vars) => {
                let query =
                    heads2query(&heads, &ctx.table_types, &ctx.data_types, &mut ctx.interner)
                        .map_err(|e| format!("{:?}", e))?;
                let (var_record, rows) = ctx.query(&query, &vars);
                print_query_result(&var_record, rows, ctx);
            }
            Command::Extract(..) => {
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
                if let Some(ref warning) = ctx.last_extract_warning {
                    eprintln!("{warning}");
                }
                if let Some(ref term) = ctx.last_extract {
                    println!("{term}");
                }
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
