use crate::features::styling::domain::StyleSheetName;
use crate::shared::primitives::ModuleName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemCommand {
    ReloadModule(ModuleName),
    ReloadStyle(StyleSheetName),
}

pub trait SystemCommandSender: Send + Sync {
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

pub struct ChannelSystemSender {
    pub tx: tokio::sync::mpsc::Sender<SystemCommand>,
}

impl ChannelSystemSender {
    #[must_use]
    pub const fn new(tx: tokio::sync::mpsc::Sender<SystemCommand>) -> Self {
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
        let cmd1 = SystemCommand::ReloadModule(ModuleName::new("hour"));
        let cmd2 = SystemCommand::ReloadModule(ModuleName::new("hour"));
        let cmd3 = SystemCommand::ReloadStyle(StyleSheetName::new("bar").unwrap());
        assert_eq!(cmd1, cmd2);
        assert_ne!(cmd1, cmd3);
    }

    #[tokio::test]
    async fn test_channel_system_sender() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(10);
        let sender = ChannelSystemSender::new(tx);
        sender.send_system_command(SystemCommand::ReloadModule(ModuleName::new("hour")));
        let received = rx.try_recv().unwrap();
        assert_eq!(
            received,
            SystemCommand::ReloadModule(ModuleName::new("hour"))
        );
    }
}
