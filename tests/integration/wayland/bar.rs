//! The real `WaylandAdapter` on the headless sway, driven the way
//! `CrankyApp::run` drives it: flush, wait for events, dispatch.

use super::sway::{Rgb, Screenshot};
use crate::support::{Context, Outcome, within};
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
pub(crate) const OUTPUT: &str = "HEADLESS-1";

/// Width of that output in pixels.
const OUTPUT_WIDTH: u32 = 1280;

pub(crate) struct Bar {
    adapter: WaylandAdapter,
    manager: WaylandSurfaceManager,
    hub: Arc<SignalHub>,
    commands: mpsc::Receiver<DisplayCommand>,
    renders_requested: usize,
}

impl Bar {
    /// Connects through the environment, exactly as `main` does.
    pub(crate) fn connect() -> Outcome<Self> {
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .context("XDG_RUNTIME_DIR is not set")?;
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
    pub(crate) fn height() -> u32 {
        Config::default().root().height().value()
    }

    fn height_pixels() -> Outcome<usize> {
        usize::try_from(Self::height()).context("bar height")
    }

    /// How many times the adapter asked for a render (once per configure).
    pub(crate) const fn renders_requested(&self) -> usize {
        self.renders_requested
    }

    /// The scale the adapter published for the output, once it knows it.
    pub(crate) fn scale(&self) -> Option<Scale> {
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
    pub(crate) async fn drive_until(
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
    pub(crate) async fn until_configured(&mut self) -> Outcome {
        self.drive_until("the layer surface to be configured", |bar| {
            bar.renders_requested > 0
        })
        .await
    }

    /// Submits a bar-sized buffer of one colour, as the root module would.
    /// Only the notify channel tells the loop about it: no socket traffic.
    pub(crate) fn paint(&self, colour: Rgb) -> Outcome {
        let [red, green, blue] = colour;
        let size = Size::new(OUTPUT_WIDTH, Self::height());
        let pixels = size
            .width()
            .checked_mul(size.height())
            .and_then(|pixels| usize::try_from(pixels).ok())
            .context("the buffer size overflows")?;
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

    /// Runs the loop until a screenshot shows `colour` across the bar and
    /// not below it. A screenshot is a round trip through sway, not a sleep,
    /// and the loop keeps turning while each one is taken.
    pub(crate) async fn until_on_screen(&mut self, colour: Rgb) -> Outcome {
        within("the bar colour to be on screen", async {
            loop {
                let shot = std::pin::pin!(Screenshot::take());
                if self
                    .while_taking(shot)
                    .await?
                    .shows_bar(colour, Self::height_pixels()?)?
                {
                    return Ok(());
                }
            }
        })
        .await?
    }

    /// Turns the loop until the screenshot `shot` is taken.
    async fn while_taking(
        &mut self,
        mut shot: std::pin::Pin<&mut (impl Future<Output = Outcome<Screenshot>> + Send)>,
    ) -> Outcome<Screenshot> {
        loop {
            tokio::select! {
                taken = &mut shot => return taken,
                turned = self.turn() => turned?,
            }
        }
    }
}
