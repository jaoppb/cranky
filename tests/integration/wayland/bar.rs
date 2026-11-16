//! The real `WaylandAdapter` on the headless sway, driven the way
//! `CrankyApp::run` drives it: flush, wait for events, dispatch.

use super::sway::{Rgb, Screenshot};
use crate::support::{Context, Outcome, ensure, within};
use cranky::features::layout_engine::domain::DisplayCommand;
use cranky::shared::config::domain::Config;
use cranky::shared::env::domain::{AppEnvironment, HomeDir, RustLog, XdgCacheHome, XdgRuntimeDir};
use cranky::shared::events::signals::SignalHub;
use cranky::shared::primitives::geometry::{Position, Scale, Size};
use cranky::shared::primitives::render::RenderBuffer;
use cranky::shared::primitives::{ModuleId, MonitorId};
use cranky::shared::wayland::adapters::wayland::{WaylandAdapter, WaylandSurfaceManager};
use cranky::shared::wayland::ports::{DisplayServerPort, SurfaceManagerPort};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

/// The only output of the headless sway.
pub const OUTPUT: &str = "HEADLESS-1";

/// Width of that output in pixels.
const OUTPUT_WIDTH: u32 = 1280;

pub struct Bar {
    adapter: WaylandAdapter,
    manager: WaylandSurfaceManager,
    hub: Arc<SignalHub>,
    commands: mpsc::Receiver<DisplayCommand>,
    renders_requested: usize,
}

impl Bar {
    /// Connects through the environment, exactly as `main` does.
    pub fn connect() -> Outcome<Self> {
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .context("XDG_RUNTIME_DIR")?;
        let env = Arc::new(AppEnvironment::new(
            HomeDir::new(runtime_dir.clone()),
            XdgCacheHome::new(runtime_dir.clone()),
            XdgRuntimeDir::new(runtime_dir),
            RustLog::new(String::new()),
            None,
        ));
        let hub = Arc::new(SignalHub::new(Config::default()));
        let (sender, commands) = mpsc::channel(64);
        let (adapter, manager) =
            WaylandAdapter::new(Arc::clone(&hub), sender, env).context("connect to sway")?;
        Ok(Self {
            adapter,
            manager,
            hub,
            commands,
            renders_requested: 0,
        })
    }

    /// Height the bar asks for, from the default configuration.
    pub fn height() -> u32 {
        Config::default().root().height().value()
    }

    /// How many times the adapter asked for a render (once per configure).
    pub const fn renders_requested(&self) -> usize {
        self.renders_requested
    }

    /// The scale the adapter published for the output, once it knows it.
    pub fn scale(&self) -> Option<Scale> {
        self.hub
            .monitor_scales_rx()
            .borrow()
            .get(&MonitorId::new(OUTPUT))
            .copied()
    }

    /// One turn of the app's loop: flush, then either socket events or a
    /// command the adapter sent back.
    async fn turn(&mut self) -> Outcome {
        self.adapter.flush().context("flush")?;
        tokio::select! {
            waited = self.adapter.wait_for_events() => {
                waited.context("wait_for_events")?;
                self.adapter.dispatch_pending().context("dispatch_pending")?;
            }
            Some(command) = self.commands.recv() => {
                if command == DisplayCommand::RequestRender {
                    self.renders_requested = self.renders_requested.saturating_add(1);
                }
            }
        }
        Ok(())
    }

    /// Runs the loop until `done` holds, or fails at the deadline.
    pub async fn drive_until(
        &mut self,
        what: &str,
        done: impl Fn(&Self) -> bool + Send + Sync,
    ) -> Outcome {
        within(what, async {
            while !done(self) {
                self.turn().await?;
            }
            self.adapter.flush().context("flush")
        })
        .await?
    }

    /// Runs the loop until sway has configured the bar's layer surface.
    pub async fn until_configured(&mut self) -> Outcome {
        self.drive_until("the layer surface to be configured", |bar| {
            bar.renders_requested > 0
        })
        .await
    }

    /// Submits a bar-sized buffer of one colour, as the root module would.
    /// Only the notify channel tells the loop about it: no socket traffic.
    pub fn paint(&self, colour: Rgb) -> Outcome {
        let [red, green, blue] = colour;
        let size = Size::new(OUTPUT_WIDTH, Self::height());
        let pixels = size
            .width()
            .checked_mul(size.height())
            .and_then(|pixels| usize::try_from(pixels).ok())
            .context("buffer size")?;
        // Premultiplied ARGB8888, little endian.
        let buffer = RenderBuffer::new([blue, green, red, 255].repeat(pixels), size);
        self.manager.submit_buffer(
            ModuleId::new(1),
            MonitorId::new(OUTPUT),
            Position::new(0, 0),
            buffer,
        );
        Ok(())
    }

    /// Runs the loop while taking screenshots, until the bar shows `colour`
    /// and the area below it does not. A screenshot is a round trip through
    /// sway, not a sleep; one that a turn of the loop overtakes is retaken.
    pub async fn until_on_screen(&mut self, colour: Rgb) -> Outcome {
        let below = usize::try_from(Self::height().saturating_mul(4)).context("row")?;
        within("the bar colour to be on screen", async {
            loop {
                let shot = tokio::select! {
                    turned = self.turn() => { turned?; continue; }
                    shot = Screenshot::take() => shot?,
                };
                let bar = shot.pixel(10, 5).context("pixel inside the bar")?;
                let rest = shot.pixel(10, below).context("pixel below the bar")?;
                if bar == colour {
                    return ensure(rest != colour, || {
                        format!("{colour:?} also covers the area below the bar")
                    });
                }
            }
        })
        .await?
    }
}
