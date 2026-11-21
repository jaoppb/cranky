//! The D-Bus MPRIS adapter: discovers `org.mpris.MediaPlayer2.*` names and follows each
//! player's properties.
mod player;
mod props;
pub mod watcher;

pub(crate) use watcher::ZbusMprisAdapter;
