use quine_core::common::Set;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ModuleTree {
    pub name: String,
    pub source: String,
    pub sub_modules: Vec<ModuleTree>,
}

#[derive(Debug)]
pub enum LoadError {
    ModuleNotFound(String),
    CircularDependency(String),
    Io(std::io::Error),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::ModuleNotFound(p) => write!(f, "module not found: {p}"),
            LoadError::CircularDependency(cycle) => write!(f, "circular dependency: {cycle}"),
            LoadError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl From<std::io::Error> for LoadError {
    fn from(e: std::io::Error) -> Self {
        LoadError::Io(e)
    }
}

pub struct ModuleLoader {
    root_dirs: Vec<PathBuf>,
    extension: String,
    loaded: Set<String>,
    loading_stack: Vec<String>,
}

impl ModuleLoader {
    pub fn new(root_dirs: Vec<PathBuf>, extension: &str) -> Self {
        Self {
            root_dirs,
            extension: extension.to_string(),
            loaded: Set::default(),
            loading_stack: Vec::new(),
        }
    }

    pub fn load(&mut self, module_path: &str) -> Result<ModuleTree, LoadError> {
        if self.loaded.contains(module_path) {
            return Err(LoadError::ModuleNotFound(module_path.to_string()));
        }

        if let Some(pos) = self.loading_stack.iter().position(|p| p == module_path) {
            let cycle: Vec<_> = self.loading_stack[pos..]
                .iter()
                .chain(std::iter::once(&module_path.to_string()))
                .cloned()
                .collect();
            return Err(LoadError::CircularDependency(cycle.join(" -> ")));
        }

        self.loading_stack.push(module_path.to_string());

        let file_path = self.resolve_file(module_path)?;
        let source = std::fs::read_to_string(&file_path)?;
        let name = module_name_from_path(&file_path);
        let sub_decls = scan_mod_declarations(&source);

        let mut sub_modules = Vec::new();
        for sub_path in sub_decls {
            let sub = self.load(&sub_path)?;
            sub_modules.push(sub);
        }

        self.loading_stack.pop();
        self.loaded.insert(module_path.to_string());

        Ok(ModuleTree {
            name,
            source,
            sub_modules,
        })
    }

    fn resolve_file(&self, module_path: &str) -> Result<PathBuf, LoadError> {
        let relative = module_path.replace("::", "/");
        for root in &self.root_dirs {
            let file = root.join(format!("{}.{}", relative, self.extension));
            if file.exists() {
                return Ok(file);
            }
            let dir_file = root
                .join(&relative)
                .join(format!("mod.{}", self.extension));
            if dir_file.exists() {
                return Ok(dir_file);
            }
        }
        Err(LoadError::ModuleNotFound(module_path.to_string()))
    }
}

fn module_name_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_default()
}

fn scan_mod_declarations(_source: &str) -> Vec<String> {
    // TODO: parse mod declarations from source
    Vec::new()
}
