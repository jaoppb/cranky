//! Icon files on disk for the tests that resolve icons by name and theme path.
use crate::features::systray::domain::IconImage;

/// A fresh directory of icon files for one test, removed when the test ends.
pub(in crate::features::systray::adapters) struct TempIconDir(std::path::PathBuf);

impl TempIconDir {
    /// Creates an empty directory whose name carries `tag` and the process id.
    pub(in crate::features::systray::adapters) fn new(tag: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("cranky-systray-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    /// The directory, as the theme path an item reports.
    pub(in crate::features::systray::adapters) fn theme_path(&self) -> String {
        self.0.to_str().unwrap().to_string()
    }

    /// Writes `name.svg`, a square filled with `color` (`#rrggbb`), and returns its path.
    pub(in crate::features::systray::adapters) fn svg(&self, name: &str, color: &str) -> String {
        let path = self.0.join(format!("{name}.svg"));
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><rect width="4" height="4" fill="{color}"/></svg>"#
        );
        std::fs::write(&path, svg).unwrap();
        path.to_str().unwrap().to_string()
    }

    /// Writes `name.png`, a square filled with the opaque `rgb` colour.
    pub(in crate::features::systray::adapters) fn png(&self, name: &str, rgb: [u8; 3]) {
        let [red, green, blue] = rgb;
        image::RgbaImage::from_pixel(4, 4, image::Rgba([red, green, blue, 255]))
            .save(self.0.join(format!("{name}.png")))
            .unwrap();
    }
}

impl Drop for TempIconDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The first RGBA pixel of a resolved icon.
pub(in crate::features::systray::adapters) fn first_pixel(image: &IconImage) -> &[u8] {
    &image.data()[..4]
}
