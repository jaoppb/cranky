use crate::features::styling::domain::StyleSheetName;
use crate::shared::primitives::ModuleName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SystemCommand {
    ReloadModule(ModuleName),
    ReloadStyle(StyleSheetName),
}

pub(super) trait SystemCommandSender: Send + Sync {
    fn send_system_command(&self, cmd: SystemCommand);
}

impl<F> SystemCommandSender for F
where
    F: Fn(SystemCommand) + Send + Sync,
{
    fn send_system_command(&self, cmd: SystemCommand) {
        self(cmd);
    }
}

pub(super) struct ChannelSystemSender {
    pub(crate) tx: tokio::sync::mpsc::Sender<SystemCommand>,
}

impl ChannelSystemSender {
    #[must_use]
    pub(crate) const fn new(tx: tokio::sync::mpsc::Sender<SystemCommand>) -> Self {
        Self { tx }
    }
}

impl SystemCommandSender for ChannelSystemSender {
    fn send_system_command(&self, cmd: SystemCommand) {
        let _ = self.tx.try_send(cmd);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_command_equality() {
        let cmd1 = SystemCommand::ReloadModule(ModuleName::new("clock"));
        let cmd2 = SystemCommand::ReloadModule(ModuleName::new("clock"));
        let cmd3 = SystemCommand::ReloadStyle(StyleSheetName::new("bar").unwrap());
        assert_eq!(cmd1, cmd2);
        assert_ne!(cmd1, cmd3);
    }

    #[test]
    fn test_channel_system_sender() {
        crate::test_utils::block_on(async {
            let (tx, mut rx) = tokio::sync::mpsc::channel(10);
            let sender = ChannelSystemSender::new(tx);
            sender.send_system_command(SystemCommand::ReloadModule(ModuleName::new("clock")));
            let received = rx.try_recv().unwrap();
            assert_eq!(
                received,
                SystemCommand::ReloadModule(ModuleName::new("clock"))
            );
        });
    }
}
