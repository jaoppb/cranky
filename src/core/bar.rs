use crate::config::Config;
use crate::core::shm::ShmBuffer;
use crate::core::{CrankyState, OutputInfo, WaylandGlobals};
use crate::render::RenderContext;
use crate::utils::parse_color;
use tiny_skia::PixmapMut;
use wayland_client::QueueHandle;
use wayland_client::protocol::{wl_buffer::WlBuffer, wl_shm::WlShm, wl_surface::WlSurface};
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::Layer,
    zwlr_layer_surface_v1::{Anchor, ZwlrLayerSurfaceV1},
};

pub struct Bar {
    surface: WlSurface,
    layer_surface: ZwlrLayerSurfaceV1,
    shm_buffer: ShmBuffer,
    buffer: Option<WlBuffer>,
    width: u32,
    height: u32,
    scale: i32,
    monitor_name: String,
    configured: bool,
}

fn create_rounded_rect_path(rect: tiny_skia::Rect, radius: f32) -> Option<tiny_skia::Path> {
    let mut pb = tiny_skia::PathBuilder::new();
    let x = rect.left();
    let y = rect.top();
    let w = rect.width();
    let h = rect.height();
    let r = radius.min(w / 2.0).min(h / 2.0);

    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

fn draw_rounded_rect(
    pixmap: &mut tiny_skia::PixmapMut,
    rect: tiny_skia::Rect,
    bg_color: tiny_skia::Color,
    border_size: f32,
    border_color: tiny_skia::Color,
    border_radius: f32,
) {
    let mut bg_paint = tiny_skia::Paint::default();
    bg_paint.set_color(bg_color);
    bg_paint.anti_alias = true;

    if border_radius > 0.0 {
        if let Some(path) = create_rounded_rect_path(rect, border_radius) {
            pixmap.fill_path(
                &path,
                &bg_paint,
                tiny_skia::FillRule::Winding,
                tiny_skia::Transform::identity(),
                None,
            );

            if border_size > 0.0 {
                let mut border_paint = tiny_skia::Paint::default();
                border_paint.set_color(border_color);
                border_paint.anti_alias = true;

                let stroke = tiny_skia::Stroke {
                    width: border_size,
                    ..Default::default()
                };
                pixmap.stroke_path(
                    &path,
                    &border_paint,
                    &stroke,
                    tiny_skia::Transform::identity(),
                    None,
                );
            }
        }
    } else {
        pixmap.fill_rect(rect, &bg_paint, tiny_skia::Transform::identity(), None);

        if border_size > 0.0 {
            let mut border_paint = tiny_skia::Paint::default();
            border_paint.set_color(border_color);
            border_paint.anti_alias = true;

            let stroke = tiny_skia::Stroke {
                width: border_size,
                ..Default::default()
            };
            let path = tiny_skia::PathBuilder::from_rect(rect);
            pixmap.stroke_path(
                &path,
                &border_paint,
                &stroke,
                tiny_skia::Transform::identity(),
                None,
            );
        }
    }
}

impl Bar {
    pub fn new(
        info: &OutputInfo,
        globals: &WaylandGlobals,
        config: &Config,
        qh: &QueueHandle<CrankyState>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let surface = globals.compositor().create_surface(qh, ());
        let layer_surface = globals.layer_shell().get_layer_surface(
            &surface,
            Some(info.output()),
            Layer::Top,
            "cranky-bar".to_string(),
            qh,
            (),
        );

        let height = config.bar().height();
        let scale = info.scale();
        let margin = config.bar().margin();

        layer_surface.set_size(0, height);
        layer_surface.set_anchor(Anchor::Top | Anchor::Left | Anchor::Right);
        layer_surface.set_margin(margin.top(), margin.right(), margin.bottom(), margin.left());
        layer_surface.set_exclusive_zone(height as i32 + margin.top() + margin.bottom());

        surface.set_buffer_scale(scale);
        surface.commit();

        let shm_buffer = ShmBuffer::new(globals.shm(), scale as u32, height * scale as u32, qh)?;

        Ok(Self {
            surface,
            layer_surface,
            shm_buffer,
            buffer: None,
            width: 1,
            height,
            scale,
            monitor_name: info.name().to_string(),
            configured: false,
        })
    }

    pub fn render(
        &mut self,
        config: &Config,
        registry: &crate::modules::ModuleRegistry,
        context: &mut RenderContext,
        qh: &QueueHandle<CrankyState>,
    ) {
        if !self.configured {
            return;
        }

        let scaled_width = self.scaled_width();
        let scaled_height = self.scaled_height();

        log::info!(
            "Rendering bar for monitor '{}': width={}, height={}, scale={}, scaled_width={}, scaled_height={}",
            self.monitor_name,
            self.width,
            self.height,
            self.scale,
            scaled_width,
            scaled_height
        );

        let mut pixmap =
            PixmapMut::from_bytes(self.shm_buffer.mmap_mut(), scaled_width, scaled_height).unwrap();

        // Clear pixmap for transparency
        pixmap.fill(tiny_skia::Color::TRANSPARENT);

        let border = config.bar().border();
        let border_size = border.size() * self.scale as f32;
        let border_radius = border.radius() * self.scale as f32;

        let rect = tiny_skia::Rect::from_xywh(
            border_size / 2.0,
            border_size / 2.0,
            scaled_width as f32 - border_size,
            scaled_height as f32 - border_size,
        )
        .unwrap();

        draw_rounded_rect(
            &mut pixmap,
            rect,
            parse_color(config.bar().background()),
            border_size,
            parse_color(border.color()),
            border_radius,
        );

        context.set_vertical_alignment(config.bar().vertical_alignment());
        context.set_scale(self.scale as f32);

        // Render modules
        let area =
            tiny_skia::Rect::from_xywh(0.0, 0.0, self.width as f32, self.height as f32).unwrap();

        registry.view(&mut pixmap, area, context, &self.monitor_name);

        // Recreate buffer only if needed
        if self.buffer.is_none() {
            let buffer = self.shm_buffer.pool().create_buffer(
                0,
                scaled_width as i32,
                scaled_height as i32,
                (scaled_width * 4) as i32,
                wayland_client::protocol::wl_shm::Format::Abgr8888,
                qh,
                (),
            );
            self.buffer = Some(buffer);
        }

        if let Some(buffer) = &self.buffer {
            self.surface.attach(Some(buffer), 0, 0);
            self.surface
                .damage(0, 0, self.width as i32, self.height as i32);
            self.surface.commit();
        }
    }

    pub fn layer_surface(&self) -> &ZwlrLayerSurfaceV1 {
        &self.layer_surface
    }

    pub fn monitor_name(&self) -> &str {
        &self.monitor_name
    }

    pub fn set_configured(&mut self) {
        self.configured = true;
    }

    fn scaled_width(&self) -> u32 {
        self.width * self.scale as u32
    }

    fn scaled_height(&self) -> u32 {
        self.height * self.scale as u32
    }

    pub fn set_width(&mut self, shm: &WlShm, width: u32, qh: &QueueHandle<CrankyState>) {
        if self.width != width {
            self.width = width;
            let scaled_width = self.scaled_width();
            let scaled_height = self.scaled_height();
            let required_size = (scaled_width * scaled_height * 4) as usize;

            if required_size > self.shm_buffer.size() {
                // Recreate ShmBuffer with the new larger size
                if let Ok(new_shm) = ShmBuffer::new(shm, scaled_width, scaled_height, qh) {
                    self.shm_buffer = new_shm;
                }
            }
            self.buffer = None; // Invalidate buffer on resize
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tiny_skia::*;
    use crate::assert_pixmap_has_color;
    use memmap2::MmapMut;
    use wayland_client::protocol::wl_shm::WlShm;
    use wayland_client::protocol::wl_shm_pool::WlShmPool;
    use wayland_client::QueueHandle;
    use crate::core::CrankyState;

    #[test]
    fn test_bar_methods() {
        let mut bar = unsafe { std::mem::MaybeUninit::<Bar>::uninit().assume_init() };
        // Initialize fields manually to avoid dropping uninitialized ones
        unsafe {
            std::ptr::write(&mut bar.width, 100);
            std::ptr::write(&mut bar.height, 30);
            std::ptr::write(&mut bar.scale, 2);
            std::ptr::write(&mut bar.configured, false);
        }

        assert_eq!(bar.scaled_width(), 200);
        assert_eq!(bar.scaled_height(), 60);

        bar.set_configured();
        assert!(bar.configured);
        
        unsafe {
            std::ptr::write(&mut bar.monitor_name, "test-monitor".to_string());
        }
        assert_eq!(bar.monitor_name(), "test-monitor");
        
        // Test layer_surface getter (just calling it, not using the result)
        let _ = bar.layer_surface();
        
        std::mem::forget(bar);
    }

    #[test]
    fn test_bar_set_width_no_change() {
        let mut bar = unsafe { std::mem::MaybeUninit::<Bar>::uninit().assume_init() };
        unsafe {
            std::ptr::write(&mut bar.width, 100);
            std::ptr::write(&mut bar.scale, 1);
        }
        
        let shm = unsafe { std::mem::MaybeUninit::<WlShm>::uninit().assume_init() };
        let qh = unsafe { std::mem::MaybeUninit::<QueueHandle<CrankyState>>::uninit().assume_init() };
        
        // width is already 100
        bar.set_width(&shm, 100, &qh);
        assert_eq!(bar.width, 100);
        
        std::mem::forget(bar);
        std::mem::forget(shm);
        std::mem::forget(qh);
    }

    #[test]
    fn test_bar_set_width_small_change() {
        let mut bar = unsafe { std::mem::MaybeUninit::<Bar>::uninit().assume_init() };
        
        unsafe {
            std::ptr::write(&mut bar.width, 100);
            std::ptr::write(&mut bar.height, 30);
            std::ptr::write(&mut bar.scale, 1);
            
            let mmap = MmapMut::map_anon(1000 * 1000).unwrap();
            let pool = std::mem::MaybeUninit::<WlShmPool>::uninit().assume_init();
            let shm_buffer = ShmBuffer::test_new(mmap, pool);
            
            std::ptr::write(&mut bar.shm_buffer, shm_buffer);
            std::ptr::write(&mut bar.buffer, None);
        }
        
        let shm = unsafe { std::mem::MaybeUninit::<WlShm>::uninit().assume_init() };
        let qh = unsafe { std::mem::MaybeUninit::<QueueHandle<CrankyState>>::uninit().assume_init() };
        
        // Change width to 110, required size = 110*30*4 = 13200 < 1000000
        bar.set_width(&shm, 110, &qh);
        assert_eq!(bar.width, 110);
        
        std::mem::forget(bar);
        std::mem::forget(shm);
        std::mem::forget(qh);
    }

    #[test]
    fn test_create_rounded_rect_path() {
        let rect = Rect::from_xywh(0.0, 0.0, 100.0, 50.0).unwrap();
        let path = create_rounded_rect_path(rect, 10.0);
        assert!(path.is_some());
        
        let path = create_rounded_rect_path(rect, 0.0);
        assert!(path.is_some());

        let path = create_rounded_rect_path(rect, 100.0); // radius > w/2 or h/2
        assert!(path.is_some());
    }

    #[test]
    fn test_draw_rounded_rect_no_border() {
        let mut pixmap_data = vec![0; 100 * 100 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 100, 100).unwrap();
        pixmap.fill(Color::TRANSPARENT);

        let rect = Rect::from_xywh(10.0, 10.0, 80.0, 80.0).unwrap();
        let bg_color = Color::from_rgba8(255, 0, 0, 255);
        
        draw_rounded_rect(
            &mut pixmap,
            rect,
            bg_color,
            0.0,
            Color::TRANSPARENT,
            0.0,
        );

        assert_pixmap_has_color!(pixmap, bg_color);
    }

    #[test]
    fn test_draw_rounded_rect_with_border() {
        let mut pixmap_data = vec![0; 100 * 100 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 100, 100).unwrap();
        pixmap.fill(Color::TRANSPARENT);

        let rect = Rect::from_xywh(10.0, 10.0, 80.0, 80.0).unwrap();
        let bg_color = Color::from_rgba8(255, 0, 0, 255);
        let border_color = Color::from_rgba8(0, 255, 0, 255);
        
        draw_rounded_rect(
            &mut pixmap,
            rect,
            bg_color,
            2.0,
            border_color,
            0.0,
        );

        assert_pixmap_has_color!(pixmap, bg_color);
        assert_pixmap_has_color!(pixmap, border_color);
    }

    #[test]
    fn test_draw_rounded_rect_radius() {
        let mut pixmap_data = vec![0; 100 * 100 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 100, 100).unwrap();
        pixmap.fill(Color::TRANSPARENT);

        let rect = Rect::from_xywh(10.0, 10.0, 80.0, 80.0).unwrap();
        let bg_color = Color::from_rgba8(255, 0, 0, 255);
        
        draw_rounded_rect(
            &mut pixmap,
            rect,
            bg_color,
            0.0,
            Color::TRANSPARENT,
            10.0,
        );

        assert_pixmap_has_color!(pixmap, bg_color);
    }
}
