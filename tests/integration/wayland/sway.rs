//! What the headless sway itself reports, independently of the adapter.

use crate::support::{Context, Outcome, ensure};
use serde_json::Value;
use tokio::process::Command;

/// One pixel of a screenshot, as red, green, blue.
pub(crate) type Rgb = [u8; 3];

async fn output_of(program: &str, args: &[&str]) -> Outcome<Vec<u8>> {
    let output = Command::new(program)
        .args(args)
        .kill_on_drop(true)
        .output()
        .await
        .context(program)?;
    ensure(
        || output.status.success(),
        || {
            format!(
                "{program} {args:?} ended with {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            )
        },
    )?;
    Ok(output.stdout)
}

/// The top edge of the first workspace: how much room layer surfaces took.
pub(crate) async fn workspace_top() -> Outcome<i64> {
    let reply = output_of("swaymsg", &["-t", "get_workspaces", "-r"]).await?;
    let workspaces: Value = serde_json::from_slice(&reply).context("swaymsg JSON")?;
    workspaces
        .get(0)
        .and_then(|workspace| workspace.get("rect"))
        .and_then(|rect| rect.get("y"))
        .and_then(Value::as_i64)
        .context("swaymsg reported no workspace rect.y")
}

/// Sets the scale of `output`, which sway announces to every client.
pub(crate) async fn set_scale(output: &str, scale: u8) -> Outcome {
    let scale = scale.to_string();
    output_of("swaymsg", &["output", output, "scale", &scale])
        .await
        .map(|_| ())
}

/// A screenshot of the whole output, taken through sway by `grim`.
pub(crate) struct Screenshot {
    width: usize,
    pixels: Vec<u8>,
}

impl Screenshot {
    pub(crate) async fn take() -> Outcome<Self> {
        let ppm = output_of("grim", &["-t", "ppm", "-"]).await?;
        Self::parse(&ppm).context("grim did not print a binary PPM")
    }

    /// Binary PPM: `P6`, width, height, maximum value, then RGB bytes.
    fn parse(ppm: &[u8]) -> Option<Self> {
        let mut rest = ppm;
        let mut header = [0_usize; 3];
        ppm_token(&mut rest).filter(|magic| *magic == b"P6")?;
        for field in &mut header {
            *field = std::str::from_utf8(ppm_token(&mut rest)?)
                .ok()?
                .parse()
                .ok()?;
        }
        let [width, height, maximum] = header;
        let pixels = rest.get(1..)?.to_vec();
        let expected = width.checked_mul(height)?.checked_mul(3)?;
        (maximum == 255 && pixels.len() == expected).then_some(Self { width, pixels })
    }

    /// Whether the bar, `bar_height` pixels tall, is `colour` from edge to
    /// edge and the output below it is not.
    pub(crate) fn shows_bar(&self, colour: Rgb, bar_height: usize) -> Outcome<bool> {
        if let Some(result) = self.first_off_corner(colour, bar_height) {
            return result;
        }
        let below = self
            .pixel(0, bar_height)
            .context("the screenshot ends at the bar")?;
        ensure(
            || below != colour,
            || format!("{colour:?} also covers the output below the bar"),
        )?;
        Ok(true)
    }

    /// The outcome of the first corner of the bar that is not `colour`, or
    /// that the screenshot does not reach; none when all four are `colour`.
    fn first_off_corner(&self, colour: Rgb, bar_height: usize) -> Option<Outcome<bool>> {
        let last_row = bar_height.saturating_sub(1);
        let last_column = self.width.saturating_sub(1);
        let corners = [
            (0, 0),
            (last_column, 0),
            (0, last_row),
            (last_column, last_row),
        ];
        corners
            .into_iter()
            .map(|(x, y)| {
                let pixel = self.pixel(x, y);
                pixel
                    .context("the screenshot is smaller than the bar")
                    .map(|pixel| pixel == colour)
            })
            .find(|matched| !matches!(matched, Ok(true)))
    }

    fn pixel(&self, x: usize, y: usize) -> Option<Rgb> {
        let start = y.checked_mul(self.width)?.checked_add(x)?.checked_mul(3)?;
        let end = start.checked_add(3)?;
        self.pixels.get(start..end)?.try_into().ok()
    }
}

/// The next whitespace-separated token of a PPM header.
fn ppm_token<'a>(rest: &mut &'a [u8]) -> Option<&'a [u8]> {
    let start = rest.iter().position(|byte| !byte.is_ascii_whitespace())?;
    let tail = rest.get(start..)?;
    let len = tail.iter().position(u8::is_ascii_whitespace)?;
    let (token, after) = tail.split_at_checked(len)?;
    *rest = after;
    Some(token)
}
