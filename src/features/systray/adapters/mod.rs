//! The D-Bus side of the tray: claims or joins the `StatusNotifierWatcher`, follows each item's
//! properties and signals, and routes click and scroll actions back to the item.
mod action_invoker;
mod cleaner;
mod fetcher;
pub mod icon_resolver;
mod signal_loop;
mod sni_adapter;
mod sni_event;
mod sni_proxy;
mod watcher;

#[cfg(test)]
mod tests;

pub use sni_adapter::SniAdapter;
