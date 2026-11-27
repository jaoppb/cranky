//! The file watchers that reload module scripts and style sheets while the bar runs.

use super::BootError;
use super::{log_style_deploy_failed, log_style_watch_failed};
use crate::app::commands::{ChannelSystemSender, SystemCommand};
use crate::features::styling::adapters::fs_loader::FsStyleLoader;
use crate::features::styling::domain::StyleSheetName;
use crate::features::styling::ports::{StyleLoaderPort, StyleReloadSender};
use crate::shared::env::domain::AppEnvironment;
use std::sync::Arc;
use tokio::sync::mpsc;

/// The watchers of the style sheets and the module scripts, kept alive while the bar runs.
///
/// Fields drop in declaration order: the scripts watcher first, then the styles one, as the
/// locals they replace did.
pub(super) struct FileWatchers {
    _scripts: Box<dyn notify::Watcher>,
    _styles: Option<Box<dyn notify::Watcher>>,
}

/// Deploys and watches the style sheets, then watches the module scripts, every change asking
/// `system_tx` for a reload.
///
/// # Errors
///
/// Returns a [`BootError`] when the module script watcher cannot be created; the style steps
/// only log their failures.
pub(super) fn start_file_watchers(
    app_env: &Arc<AppEnvironment>,
    system_tx: mpsc::Sender<SystemCommand>,
) -> Result<FileWatchers, BootError> {
    let styles = watch_builtin_styles(app_env, system_tx.clone());
    let scripts = crate::app::builtins::BuiltinModules::watch_scripts(
        Arc::new(ChannelSystemSender::new(system_tx)),
        app_env,
    )?;
    Ok(FileWatchers {
        _scripts: scripts,
        _styles: styles,
    })
}

/// Deploys the builtin style sheets and watches the style directories, asking `system_tx` to
/// reload a modified sheet; either step failing is logged, and a failed watch gives no watcher.
fn watch_builtin_styles(
    app_env: &Arc<AppEnvironment>,
    system_tx: mpsc::Sender<SystemCommand>,
) -> Option<Box<dyn notify::Watcher>> {
    let style_loader = FsStyleLoader::new(app_env.clone());
    if let Err(error) = style_loader.ensure_builtin_styles() {
        log_style_deploy_failed(&error);
    }

    style_loader
        .watch_styles(style_reload_sender(system_tx))
        .inspect_err(log_style_watch_failed)
        .ok()
}

/// Forwards each modified sheet to `system_tx` as a style reload, dropped when the channel is
/// full or closed.
fn style_reload_sender(system_tx: mpsc::Sender<SystemCommand>) -> Arc<dyn StyleReloadSender> {
    Arc::new(move |sheet: StyleSheetName| {
        let _ = system_tx.try_send(SystemCommand::ReloadStyle(sheet));
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    /// Every command `rx` receives until none arrives for two seconds.
    fn received(rx: &mut mpsc::Receiver<SystemCommand>) -> Vec<SystemCommand> {
        let mut commands = Vec::new();
        let mut quiet_since = Instant::now();
        while quiet_since.elapsed() < Duration::from_secs(2) {
            if let Ok(command) = rx.try_recv() {
                commands.push(command);
                quiet_since = Instant::now();
            } else {
                std::thread::sleep(Duration::from_millis(20));
            }
        }
        commands
    }

    #[test]
    fn test_a_modified_sheet_becomes_a_style_reload() {
        let (system_tx, mut system_rx) = mpsc::channel(4);
        let sheet = StyleSheetName::new("clock").unwrap();

        style_reload_sender(system_tx).reload_style(sheet.clone());

        assert_eq!(
            system_rx.try_recv().unwrap(),
            SystemCommand::ReloadStyle(sheet)
        );
    }

    /// The watchers deploy the builtin sheets and turn an edited sheet into a style reload and an
    /// edited script into a module reload.
    #[test]
    fn test_file_watchers_reload_edited_sheets_and_scripts() {
        let home = crate::test_utils::TestHome::new("bootstrap-watchers");
        let modules = crate::app::builtins::BuiltinModules::ensure_builtins(&home).unwrap();
        let (system_tx, mut system_rx) = mpsc::channel(100);

        let _watchers = start_file_watchers(&home.env(), system_tx).unwrap();
        let styles = home.home().as_path().join(".local/share/cranky/styles");
        assert!(styles.join("base.css").exists());
        std::fs::write(styles.join("clock.css"), "/* edited */").unwrap();
        std::fs::write(modules.join("clock.lua"), "-- edited").unwrap();

        let commands = received(&mut system_rx);
        let clock = StyleSheetName::new("clock").unwrap();
        assert!(
            commands.contains(&SystemCommand::ReloadStyle(clock)),
            "{commands:?}"
        );
        assert!(
            commands.contains(&SystemCommand::ReloadModule("clock".into())),
            "{commands:?}"
        );
    }
}
