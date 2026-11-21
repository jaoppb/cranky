//! The Wayland adapter (`src/shared/wayland/adapters/wayland/`) against a
//! headless sway: connect, bind, configure, submit, and the wait on the
//! display socket that #30 rewrites.

pub mod bar;
pub mod sway;

use crate::cases::{Case, case};
use crate::support::{Outcome, ensure, ensure_eq};
use bar::Bar;
use cranky::shared::primitives::geometry::Scale;
use sway::Rgb;

const RED: Rgb = [255, 0, 0];
const GREEN: Rgb = [0, 255, 0];
const BLUE: Rgb = [0, 0, 255];

/// Rounds of the socket-wait case: each needs a fresh read of the socket.
const ROUNDS: u8 = 50;

pub(crate) fn cases() -> Vec<Case> {
    vec![
        case!(wayland::binds_the_output),
        case!(wayland::reserves_its_exclusive_zone_once_configured),
        case!(wayland::shows_each_submitted_buffer),
        case!(wayland::keeps_reading_the_socket_across_many_wakes),
    ]
}

async fn binds_the_output() -> Outcome {
    // The sway is shared, and a case that failed half-way may have left it
    // at another scale.
    sway::set_scale(bar::OUTPUT, 1).await?;
    let mut bar = Bar::connect()?;
    bar.drive_until("the output to be announced", |bar| bar.scale().is_some())
        .await?;
    ensure_eq("output scale", &bar.scale(), &Some(Scale::new(1.0)))
}

async fn reserves_its_exclusive_zone_once_configured() -> Outcome {
    let mut bar = Bar::connect()?;
    bar.until_configured().await?;
    ensure(bar.renders_requested() > 0, || {
        "the configure did not ask for a render".to_string()
    })?;
    // sway maps a layer surface, and honours its zone, on its first buffer.
    bar.paint(RED)?;
    bar.until_on_screen(RED).await?;
    ensure_eq(
        "top of the workspace below the bar",
        &sway::workspace_top().await?,
        &i64::from(Bar::height()),
    )
}

/// Each buffer replaces the one before it, edge to edge and no further.
async fn shows_each_submitted_buffer() -> Outcome {
    let mut bar = Bar::connect()?;
    bar.until_configured().await?;
    for colour in [RED, GREEN, BLUE] {
        bar.paint(colour)?;
        bar.until_on_screen(colour).await?;
    }
    Ok(())
}

/// The regression #30 risks: a wait that reports the socket readable once
/// and never again. Every round needs a fresh read of the socket (sway
/// announces a new output scale), with buffers submitted in between as the
/// app would. The last buffer has a colour of its own, so seeing it on screen
/// shows the loop still takes a wake that comes with no socket traffic.
async fn keeps_reading_the_socket_across_many_wakes() -> Outcome {
    let mut bar = Bar::connect()?;
    bar.until_configured().await?;
    for round in 1..=ROUNDS {
        // Ends on scale 1, so the last screenshot is in output pixels.
        let scale = if round % 2 == 0 { 1 } else { 2 };
        let colour = if round % 2 == 0 { GREEN } else { RED };
        sway::set_scale(bar::OUTPUT, scale).await?;
        let expected = Some(Scale::new(f32::from(scale)));
        bar.drive_until("the new output scale to arrive", |bar| {
            bar.scale() == expected
        })
        .await?;
        bar.paint(colour)?;
    }
    ensure(bar.renders_requested() > 1, || {
        "the scale changes never reconfigured the bar".to_string()
    })?;
    bar.paint(BLUE)?;
    bar.until_on_screen(BLUE).await
}
