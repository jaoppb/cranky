use crate::features::module_runtime::ports::AnyModulePort;
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum BuiltinError {
    #[error("HOME environment variable not set: {0}")]
    Env(String),
    #[error("IO error: {0}")]
    InputOutput(String),
    #[error("Unsupported engine '{engine}' for module '{module_name}' (expected 'rhai' or 'lua')")]
    UnsupportedEngine {
        engine: String,
        module_name: crate::shared::primitives::ModuleName,
    },
    #[error("Module '{module_name}' not found{engine_suffix}")]
    ModuleNotFound {
        module_name: crate::shared::primitives::ModuleName,
        engine_suffix: String,
    },
}

pub(super) struct BuiltinModules;

impl BuiltinModules {
    const BUILTINS: &[(&'static str, &'static str)] = &[
        ("bar.lua", include_str!("../../../assets/widgets/bar.lua")),
        ("bar.rhai", include_str!("../../../assets/widgets/bar.rhai")),
        (
            "clock.lua",
            include_str!("../../../assets/widgets/clock.lua"),
        ),
        (
            "clock.rhai",
            include_str!("../../../assets/widgets/clock.rhai"),
        ),
        (
            "calendar.lua",
            include_str!("../../../assets/widgets/calendar.lua"),
        ),
        (
            "calendar.rhai",
            include_str!("../../../assets/widgets/calendar.rhai"),
        ),
        (
            "workspace.lua",
            include_str!("../../../assets/widgets/workspace.lua"),
        ),
        (
            "workspace.rhai",
            include_str!("../../../assets/widgets/workspace.rhai"),
        ),
        (
            "systray.lua",
            include_str!("../../../assets/widgets/systray.lua"),
        ),
        (
            "systray.rhai",
            include_str!("../../../assets/widgets/systray.rhai"),
        ),
        (
            "metrics.lua",
            include_str!("../../../assets/widgets/metrics.lua"),
        ),
        (
            "metrics.rhai",
            include_str!("../../../assets/widgets/metrics.rhai"),
        ),
        (
            "mpris.lua",
            include_str!("../../../assets/widgets/mpris.lua"),
        ),
        (
            "mpris.rhai",
            include_str!("../../../assets/widgets/mpris.rhai"),
        ),
    ];

    /// Ensures that all builtin module files exist in the local share directory.
    ///
    /// # Errors
    ///
    /// Returns [`BuiltinError::InputOutput`] if creating directories or reading/writing files fails.
    pub(crate) fn ensure_builtins(
        app_env: &crate::shared::env::domain::AppEnvironment,
    ) -> Result<PathBuf, BuiltinError> {
        let home = app_env.home().as_path();
        let dir = home.join(".local/share/cranky/modules");

        fs::create_dir_all(&dir).map_err(|error| BuiltinError::InputOutput(error.to_string()))?;

        for (filename, content) in Self::BUILTINS {
            write_if_stale(&dir.join(filename), content);
        }

        Ok(dir)
    }

    fn registered_engines() -> Vec<Box<dyn crate::shared::scripting::ports::ScriptEnginePort>> {
        vec![
            Box::new(crate::shared::scripting::adapters::LuaEngineAdapter),
            Box::new(crate::shared::scripting::adapters::RhaiEngineAdapter),
        ]
    }

    /// Finds and instantiates a module matching the given name and engine selection.
    ///
    /// # Errors
    ///
    /// Returns [`BuiltinError::UnsupportedEngine`] if the explicit engine is unknown,
    /// [`BuiltinError::ModuleNotFound`] if no script matching the name and engine could be found or loaded,
    /// or [`BuiltinError::InputOutput`] on filesystem error.
    pub(crate) fn find_module(
        name: &crate::shared::primitives::ModuleName,
        selection: &crate::shared::config::domain::EngineSelection,
        app_env: &crate::shared::env::domain::AppEnvironment,
    ) -> Result<Box<dyn AnyModulePort>, BuiltinError> {
        let _ = Self::ensure_builtins(app_env)?;

        let dirs = module_dirs(app_env);
        let engines = Self::registered_engines();
        let target_engines = select_engines(&engines, selection, name)?;

        load_first_found(&dirs, &target_engines, name).ok_or_else(|| not_found(name, selection))
    }

    /// Watches user and system module directories for changes and triggers reloads.
    ///
    /// # Errors
    ///
    /// Returns [`BuiltinError::InputOutput`] if creating the file watcher fails.
    pub(crate) fn watch_scripts(
        command_tx: std::sync::Arc<dyn crate::app::commands::SystemCommandSender>,
        app_env: &crate::shared::env::domain::AppEnvironment,
    ) -> Result<Box<dyn notify::Watcher>, BuiltinError> {
        let [user_dir, shadow_dir] = module_dirs(app_env);

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            reload_modified_scripts(res, command_tx.as_ref());
        })
        .map_err(|error| BuiltinError::InputOutput(format!("Failed to create watcher: {error}")))?;

        watch_if_present(&mut watcher, &user_dir);
        watch_if_present(&mut watcher, &shadow_dir);

        Ok(Box::new(watcher))
    }
}

/// The user module directory, then the directory the builtins are deployed to, under the home
/// of `app_env`: the order modules are searched in.
fn module_dirs(app_env: &crate::shared::env::domain::AppEnvironment) -> [PathBuf; 2] {
    let home = app_env.home().as_path();
    [
        home.join(".config/cranky/modules"),
        home.join(".local/share/cranky/modules"),
    ]
}

/// The engines of `engines` that `selection` allows, in registration order.
///
/// # Errors
///
/// Returns [`BuiltinError::UnsupportedEngine`] if `selection` names an engine that is not
/// registered.
fn select_engines<'engines>(
    engines: &'engines [Box<dyn crate::shared::scripting::ports::ScriptEnginePort>],
    selection: &crate::shared::config::domain::EngineSelection,
    name: &crate::shared::primitives::ModuleName,
) -> Result<Vec<&'engines dyn crate::shared::scripting::ports::ScriptEnginePort>, BuiltinError> {
    let registered = engines.iter().map(AsRef::as_ref);
    match selection {
        crate::shared::config::domain::EngineSelection::Auto => Ok(registered.collect()),
        crate::shared::config::domain::EngineSelection::Explicit(id) => {
            let matching: Vec<_> = registered.filter(|engine| &engine.id() == id).collect();
            if matching.is_empty() {
                return Err(BuiltinError::UnsupportedEngine {
                    engine: id.as_str().to_string(),
                    module_name: name.clone(),
                });
            }
            Ok(matching)
        }
    }
}

/// Writes `content` to `path` unless the file already holds exactly it; a failed write is
/// ignored.
fn write_if_stale(path: &Path, content: &str) {
    if path.exists() && fs::read_to_string(path).ok().as_deref() == Some(content) {
        return;
    }
    let _ = fs::write(path, content);
}

/// The first module named `name` that loads, searching each of `dirs` in order with every one
/// of `engines` in order; a script that is missing or fails to load is skipped.
fn load_first_found(
    dirs: &[PathBuf],
    engines: &[&dyn crate::shared::scripting::ports::ScriptEnginePort],
    name: &crate::shared::primitives::ModuleName,
) -> Option<Box<dyn AnyModulePort>> {
    dirs.iter()
        .flat_map(|dir| engines.iter().map(move |engine| (dir, *engine)))
        .find_map(|(dir, engine)| load_from(dir, engine, name))
}

/// The module named `name` that `engine` loads from its script in `dir`, if the script exists
/// and loads.
fn load_from(
    dir: &Path,
    engine: &dyn crate::shared::scripting::ports::ScriptEnginePort,
    name: &crate::shared::primitives::ModuleName,
) -> Option<Box<dyn AnyModulePort>> {
    let path = dir.join(format!("{name}.{}", engine.file_extension().as_str()));
    let source = fs::read_to_string(&path).ok()?;
    engine.load_module(name.as_str(), &source).ok()
}

/// The error for a module `name` that no allowed engine found, naming the engine when
/// `selection` pins one.
fn not_found(
    name: &crate::shared::primitives::ModuleName,
    selection: &crate::shared::config::domain::EngineSelection,
) -> BuiltinError {
    let engine_suffix = selection
        .as_explicit()
        .map_or_else(String::new, |id| format!(" (engine: {})", id.as_str()));

    BuiltinError::ModuleNotFound {
        module_name: name.clone(),
        engine_suffix,
    }
}

/// Asks for a reload of every module whose script the watcher event `res` reports modified.
fn reload_modified_scripts(
    res: notify::Result<notify::Event>,
    command_tx: &dyn crate::app::commands::SystemCommandSender,
) {
    if let Ok(event) = res
        && event.kind.is_modify()
    {
        for path in event.paths {
            reload_script(&path, command_tx);
        }
    }
}

/// Asks for a reload of the module the script at `path` defines, named after its file stem; a
/// path with no readable stem is ignored.
fn reload_script(path: &Path, command_tx: &dyn crate::app::commands::SystemCommandSender) {
    if let Some(stem) = path.file_stem().and_then(|file_stem| file_stem.to_str()) {
        log_script_modified(path);
        command_tx.send_system_command(crate::app::commands::SystemCommand::ReloadModule(
            crate::shared::primitives::ModuleName::new(stem),
        ));
    }
}

/// Logs that the script at `path` changed on disk.
///
/// Kept out of line to keep its caller under `cognitive_complexity`.
fn log_script_modified(path: &Path) {
    tracing::debug!("Script modified: {path:?}");
}

/// Watches `dir`, without recursion, when it exists; a directory that cannot be watched is
/// ignored.
fn watch_if_present(watcher: &mut impl notify::Watcher, dir: &Path) {
    if dir.exists() {
        let _ = watcher.watch(dir, notify::RecursiveMode::NonRecursive);
    }
}

#[cfg(test)]
mod tests;
