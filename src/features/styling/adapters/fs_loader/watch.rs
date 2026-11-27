//! The style watcher's steps: which directories it watches and which reloads a file system
//! event asks for.

use crate::features::styling::domain::StyleSheetName;
use crate::features::styling::ports::StyleReloadSender;
use notify::{Event, RecursiveMode, Watcher};
use std::path::Path;

/// Watches `dir` for hot reload if it exists; a failure to watch it is ignored.
pub(super) fn watch_if_present(watcher: &mut impl Watcher, dir: &Path) {
    if dir.exists() {
        let _ = watcher.watch(dir, RecursiveMode::NonRecursive);
    }
}

/// Asks for a reload of every sheet a modify event touched; errors and other events are ignored.
pub(super) fn reload_modified_sheets(
    result: notify::Result<Event>,
    command_tx: &dyn StyleReloadSender,
) {
    if let Ok(event) = result
        && event.kind.is_modify()
    {
        for path in event.paths {
            reload_sheet_at(&path, command_tx);
        }
    }
}

/// Asks for a reload of the sheet stored at `path`, if its file stem is a valid sheet name.
fn reload_sheet_at(path: &Path, command_tx: &dyn StyleReloadSender) {
    if let Some(stem) = path.file_stem().and_then(|file_stem| file_stem.to_str()) {
        super::log_sheet_modified(path);
        if let Ok(sheet_name) = StyleSheetName::new(stem) {
            command_tx.reload_style(sheet_name);
        }
    }
}
