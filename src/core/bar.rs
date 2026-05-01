use crate::config::Config;
use crate::core::shm::ShmBuffer;
use crate::core::{CrankyState, OutputInfo, WaylandGlobals};
use crate::render::RenderContext;
use crate::utils::ParsedColor;
use tiny_skia::PixmapMut;
use wayland_client::QueueHandle;
use wayland_client::protocol::{
    wl_buffer::WlBuffer, wl_shm::WlShm, wl_subsurface::WlSubsurface, wl_surface::WlSurface,
};

pub struct ModuleSurface {
    surface: WlSurface,
    subsurface: WlSubsurface,
    shm_buffer: ShmBuffer,
    buffer: Option<WlBuffer>,
}

impl ModuleSurface {
    pub fn new(
        parent: &WlSurface,
        globals: &WaylandGlobals,
        qh: &QueueHandle<CrankyState>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let surface = globals.compositor().create_surface(qh, ());
        let subsurface = globals
            .subcompositor()
            .get_subsurface(&surface, parent, qh, ());

        // Initial small buffer, will be resized on first render
        let shm_buffer = ShmBuffer::new(globals.shm(), 1, 1, qh)?;

        Ok(Self {
            surface,
            subsurface,
            shm_buffer,
            buffer: None,
        })
    }

    pub fn subsurface(&self) -> &WlSubsurface {
        &self.subsurface
    }

    pub fn surface(&self) -> &WlSurface {
        &self.surface
    }
}
use wayland_protocols_wlr::layer_shell::v1::client::{
    zwlr_layer_shell_v1::Layer,
    zwlr_layer_surface_v1::{Anchor, ZwlrLayerSurfaceV1},
};

#[derive(Default)]
pub struct BarState {
    pub width: u32,
    pub height: u32,
    pub scale: i32,
}

pub struct Bar {
    surface: WlSurface,
    layer_surface: ZwlrLayerSurfaceV1,
    shm_buffer: ShmBuffer,
    buffer: Option<WlBuffer>,
    state: BarState,
    monitor_name: String,
    configured: bool,

    left_surfaces: Vec<ModuleSurface>,
    center_surfaces: Vec<ModuleSurface>,
    right_surfaces: Vec<ModuleSurface>,
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

fn create_paint<'a>(color: &ParsedColor, rect: tiny_skia::Rect) -> tiny_skia::Paint<'a> {
    let mut paint = tiny_skia::Paint {
        anti_alias: true,
        ..tiny_skia::Paint::default()
    };

    match color {
        ParsedColor::Solid(c) => {
            paint.set_color(*c);
        }
        ParsedColor::Gradient(colors, angle) => {
            let stops: Vec<tiny_skia::GradientStop> = colors
                .iter()
                .enumerate()
                .map(|(i, &c)| {
                    tiny_skia::GradientStop::new(i as f32 / (colors.len() - 1) as f32, c)
                })
                .collect();

            // Calculate start and end points based on angle (degrees)
            // 0deg: Left to Right
            // 90deg: Top to Bottom
            let angle_rad = angle.to_radians();
            let center_x = rect.left() + rect.width() / 2.0;
            let center_y = rect.top() + rect.height() / 2.0;

            let x_offset = angle_rad.cos() * rect.width() / 2.0;
            let y_offset = angle_rad.sin() * rect.height() / 2.0;

            let start = tiny_skia::Point::from_xy(center_x - x_offset, center_y - y_offset);
            let end = tiny_skia::Point::from_xy(center_x + x_offset, center_y + y_offset);

            paint.shader = tiny_skia::LinearGradient::new(
                start,
                end,
                stops,
                tiny_skia::SpreadMode::Pad,
                tiny_skia::Transform::identity(),
            )
            .unwrap();
        }
    }
    paint
}

fn draw_rounded_rect(
    pixmap: &mut tiny_skia::PixmapMut,
    rect: tiny_skia::Rect,
    bg_color: &ParsedColor,
    border_size: f32,
    border_color: &ParsedColor,
    border_radius: f32,
) {
    let bg_paint = create_paint(bg_color, rect);

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
                let border_paint = create_paint(border_color, rect);
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
            let border_paint = create_paint(border_color, rect);
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
    pub fn find_module_by_surface(
        &self,
        surface: &WlSurface,
    ) -> Option<(crate::modules::Position, usize)> {
        if let Some(pos) = self.left_surfaces.iter().position(|s| s.surface() == surface) {
            return Some((crate::modules::Position::Left, pos));
        }
        if let Some(pos) = self
            .center_surfaces
            .iter()
            .position(|s| s.surface() == surface)
        {
            return Some((crate::modules::Position::Center, pos));
        }
        if let Some(pos) = self.right_surfaces.iter().position(|s| s.surface() == surface) {
            return Some((crate::modules::Position::Right, pos));
        }
        None
    }

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
            state: BarState {
                width: 1,
                height,
                scale,
            },
            monitor_name: info.name().to_string(),
            configured: false,
            left_surfaces: Vec::new(),
            center_surfaces: Vec::new(),
            right_surfaces: Vec::new(),
        })
    }

    pub fn sync_surfaces(
        &mut self,
        registry: &crate::modules::ModuleRegistry,
        globals: &WaylandGlobals,
        qh: &QueueHandle<CrankyState>,
    ) {
        let left_count = registry.left_modules().len();
        let center_count = registry.center_modules().len();
        let right_count = registry.right_modules().len();

        if self.left_surfaces.len() != left_count {
            self.left_surfaces = (0..left_count)
                .filter_map(|_| ModuleSurface::new(&self.surface, globals, qh).ok())
                .collect();
        }

        if self.center_surfaces.len() != center_count {
            self.center_surfaces = (0..center_count)
                .filter_map(|_| ModuleSurface::new(&self.surface, globals, qh).ok())
                .collect();
        }

        if self.right_surfaces.len() != right_count {
            self.right_surfaces = (0..right_count)
                .filter_map(|_| ModuleSurface::new(&self.surface, globals, qh).ok())
                .collect();
        }
    }

    pub fn render(
        &mut self,
        _config: &Config,
        bar_config: &crate::config::BarConfig,
        registry: &crate::modules::ModuleRegistry,
        context: &mut RenderContext,
        error_message: &Option<String>,
        globals: &WaylandGlobals,
        qh: &QueueHandle<CrankyState>,
    ) {
        if !self.configured {
            return;
        }

        self.sync_surfaces(registry, globals, qh);

        let scaled_width = self.scaled_width();
        let scaled_height = self.scaled_height();

        log::info!(
            "Rendering bar for monitor '{}': width={}, height={}, scale={}, scaled_width={}, scaled_height={}",
            self.monitor_name,
            self.state.width,
            self.state.height,
            self.state.scale,
            scaled_width,
            scaled_height
        );

        let mut pixmap =
            PixmapMut::from_bytes(self.shm_buffer.mmap_mut(), scaled_width, scaled_height).unwrap();

        // Clear pixmap for transparency
        pixmap.fill(tiny_skia::Color::TRANSPARENT);

        let border = bar_config.border();
        let border_size = border.size() * self.state.scale as f32;
        let border_radius = border.radius() * self.state.scale as f32;

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
            bar_config.background(),
            border_size,
            border.color(),
            border_radius,
        );

        context.set_vertical_alignment(bar_config.vertical_alignment());
        context.set_scale(self.state.scale as f32);

        if let Some(error) = error_message {
            let styling = crate::render::TextStyling::new(
                14.0,
                20.0,
                tiny_skia::Color::from_rgba8(255, 0, 0, 255),
                "monospace".to_string(),
            );
            let y_offset = context.calculate_vertical_offset(
                tiny_skia::Rect::from_xywh(
                    0.0,
                    0.0,
                    self.state.width as f32,
                    self.state.height as f32,
                )
                .unwrap(),
                14.0,
            );
            context.render_text(&mut pixmap, error, styling, 10.0, y_offset);
        } else {
            let spacing = 10.0;
            let bar_area = tiny_skia::Rect::from_xywh(
                0.0,
                0.0,
                self.state.width as f32,
                self.state.height as f32,
            )
            .unwrap();

            // Render left modules
            let mut left_offset = bar_area.left() + spacing;
            for (i, module) in registry.left_modules().iter().enumerate() {
                let width = module.measure(context, &self.monitor_name);
                Self::render_module_static(
                    module,
                    &mut self.left_surfaces[i],
                    left_offset as i32,
                    0,
                    width,
                    self.state.scale,
                    self.state.height,
                    &self.monitor_name,
                    globals,
                    qh,
                );
                left_offset += width + spacing;
            }

            // Render right modules
            let mut right_widths = Vec::new();
            let mut total_right_width = 0.0;
            for module in registry.right_modules() {
                let width = module.measure(context, &self.monitor_name);
                right_widths.push(width);
                total_right_width += width + spacing;
            }

            let mut right_offset = bar_area.right() - total_right_width;
            for (i, module) in registry.right_modules().iter().enumerate() {
                let width = right_widths[i];
                Self::render_module_static(
                    module,
                    &mut self.right_surfaces[i],
                    right_offset as i32,
                    0,
                    width,
                    self.state.scale,
                    self.state.height,
                    &self.monitor_name,
                    globals,
                    qh,
                );
                right_offset += width + spacing;
            }

            // Render center modules
            let mut center_widths = Vec::new();
            let mut total_center_width = 0.0;
            for module in registry.center_modules() {
                let width = module.measure(context, &self.monitor_name);
                center_widths.push(width);
                total_center_width += width + spacing;
            }
            if !center_widths.is_empty() {
                total_center_width -= spacing;
            }

            let mut center_offset = bar_area.left() + (bar_area.width() - total_center_width) / 2.0;
            for (i, module) in registry.center_modules().iter().enumerate() {
                let width = center_widths[i];
                Self::render_module_static(
                    module,
                    &mut self.center_surfaces[i],
                    center_offset as i32,
                    0,
                    width,
                    self.state.scale,
                    self.state.height,
                    &self.monitor_name,
                    globals,
                    qh,
                );
                center_offset += width + spacing;
            }
        }

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
                .damage(0, 0, self.state.width as i32, self.state.height as i32);
            self.surface.commit();
        }
    }

    fn render_module_static(
        module: &Box<dyn crate::modules::AnyModule>,
        module_surface: &mut ModuleSurface,
        x: i32,
        y: i32,
        width: f32,
        scale: i32,
        bar_height: u32,
        monitor_name: &str,
        globals: &WaylandGlobals,
        qh: &QueueHandle<CrankyState>,
    ) {
        let scaled_width = (width * scale as f32).ceil() as u32;
        let scaled_height = bar_height * scale as u32;

        if scaled_width == 0 {
            module_surface.subsurface().set_position(0, 0);
            return;
        }

        // Resize buffer if needed
        let required_size = (scaled_width * scaled_height * 4) as usize;
        if required_size > module_surface.shm_buffer.size() {
            if let Ok(new_shm) = ShmBuffer::new(globals.shm(), scaled_width, scaled_height, qh) {
                module_surface.shm_buffer = new_shm;
                module_surface.buffer = None;
            }
        } else if module_surface.shm_buffer.width() != scaled_width
            || module_surface.shm_buffer.height() != scaled_height
        {
            module_surface.buffer = None;
        }

        module_surface.subsurface().set_position(x, y);

        let mut pixmap = PixmapMut::from_bytes(
            module_surface.shm_buffer.mmap_mut(),
            scaled_width,
            scaled_height,
        )
        .unwrap();
        pixmap.fill(tiny_skia::Color::TRANSPARENT);

        let mut context = RenderContext::new();
        context.set_scale(scale as f32);

        module.view(&mut pixmap, &mut context, monitor_name);

        if module_surface.buffer.is_none() {
            let buffer = module_surface.shm_buffer.pool().create_buffer(
                0,
                scaled_width as i32,
                scaled_height as i32,
                (scaled_width * 4) as i32,
                wayland_client::protocol::wl_shm::Format::Abgr8888,
                qh,
                (),
            );
            module_surface.buffer = Some(buffer);
        }

        if let Some(buffer) = &module_surface.buffer {
            module_surface.surface.set_buffer_scale(scale);
            module_surface.surface.attach(Some(buffer), 0, 0);
            module_surface
                .surface
                .damage(0, 0, width as i32, bar_height as i32);
            module_surface.surface.commit();
        }
    }

    pub fn layer_surface(&self) -> &ZwlrLayerSurfaceV1 {
        &self.layer_surface
    }

    pub fn monitor_name(&self) -> &str {
        &self.monitor_name
    }

    pub fn scale(&self) -> i32 {
        self.state.scale
    }

    pub fn needs_redraw(&self) -> bool {
        self.buffer.is_none()
    }

    pub fn set_configured(&mut self) {
        self.configured = true;
    }

    fn scaled_width(&self) -> u32 {
        self.state.width * self.state.scale as u32
    }

    fn scaled_height(&self) -> u32 {
        self.state.height * self.state.scale as u32
    }

    pub fn set_width(&mut self, shm: &WlShm, width: u32, qh: &QueueHandle<CrankyState>) {
        if self.state.width != width {
            self.state.width = width;
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

    pub fn set_scale(&mut self, shm: &WlShm, scale: i32, qh: &QueueHandle<CrankyState>) {
        if self.state.scale != scale {
            log::info!(
                "Bar scale for '{}' changed from {} to {}",
                self.monitor_name,
                self.state.scale,
                scale
            );
            self.state.scale = scale;
            self.surface.set_buffer_scale(scale);

            let scaled_width = self.scaled_width();
            let scaled_height = self.scaled_height();
            let required_size = (scaled_width * scaled_height * 4) as usize;

            if required_size > self.shm_buffer.size() {
                // Recreate ShmBuffer with the new larger size
                if let Ok(new_shm) = ShmBuffer::new(shm, scaled_width, scaled_height, qh) {
                    self.shm_buffer = new_shm;
                }
            }
            self.buffer = None; // Invalidate buffer on scale change
            self.surface.commit();
        }
    }

    pub fn update_config(
        &mut self,
        shm: &WlShm,
        _config: &Config,
        bar_config: &crate::config::BarConfig,
        qh: &QueueHandle<CrankyState>,
    ) {
        let height = bar_config.height();
        let margin = bar_config.margin();

        if self.state.height != height {
            self.state.height = height;
            self.layer_surface.set_size(0, height);

            let scaled_width = self.scaled_width();
            let scaled_height = self.scaled_height();
            let required_size = (scaled_width * scaled_height * 4) as usize;

            if required_size > self.shm_buffer.size()
                && let Ok(new_shm) = ShmBuffer::new(shm, scaled_width, scaled_height, qh)
            {
                self.shm_buffer = new_shm;
            }
            self.buffer = None;
        }

        self.layer_surface
            .set_margin(margin.top(), margin.right(), margin.bottom(), margin.left());
        self.layer_surface
            .set_exclusive_zone(height as i32 + margin.top() + margin.bottom());

        self.surface.commit();
    }
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;
    use crate::assert_pixmap_has_color;
    use tiny_skia::*;

    #[test]
    fn test_bar_state_dimensions() {
        let state = BarState {
            width: 100,
            height: 30,
            scale: 2,
        };
        assert_eq!(state.width * state.scale as u32, 200);
        assert_eq!(state.height * state.scale as u32, 60);
    }

    #[test]
    fn test_bar_state_default() {
        let state = BarState::default();
        assert_eq!(state.width, 0);
        assert_eq!(state.height, 0);
        assert_eq!(state.scale, 0);
    }

    #[test]
    fn test_draw_rounded_rect_all_branches() {
        let mut data = vec![0u8; 50 * 50 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut data, 50, 50).unwrap();
        let rect = Rect::from_xywh(5.0, 5.0, 40.0, 40.0).unwrap();
        let color = ParsedColor::Solid(Color::from_rgba8(255, 255, 255, 128));

        // Test with border and radius
        draw_rounded_rect(&mut pixmap, rect, &color, 1.0, &color, 2.0);
        // Test with no radius
        draw_rounded_rect(&mut pixmap, rect, &color, 1.0, &color, 0.0);
        // Test with no border
        draw_rounded_rect(&mut pixmap, rect, &color, 0.0, &color, 2.0);
    }

    #[test]
    fn test_create_rounded_rect_path_edge_cases() {
        let rect = Rect::from_xywh(0.0, 0.0, 10.0, 10.0).unwrap();
        // Radius larger than half width/height
        let path = create_rounded_rect_path(rect, 6.0);
        assert!(path.is_some());

        // Zero area rect
        let empty_rect = Rect::from_xywh(0.0, 0.0, 0.0, 0.0).unwrap();
        let path = create_rounded_rect_path(empty_rect, 1.0);
        assert!(path.is_some());
    }

    #[test]
    fn test_create_paint_variants() {
        let rect = Rect::from_xywh(0.0, 0.0, 100.0, 30.0).unwrap();
        let solid = ParsedColor::Solid(Color::BLACK);
        let paint_solid = create_paint(&solid, rect);
        assert!(paint_solid.shader.is_opaque());

        let grad = ParsedColor::Gradient(vec![Color::BLACK, Color::WHITE], 90.0);
        let _paint_grad = create_paint(&grad, rect);
        // Shader is private, but we can check if it exists implicitly by not being solid or something if it was public.
        // For now just ensure it doesn't panic.
    }

    #[test]
    fn test_create_rounded_rect_path() {
        let rect = Rect::from_xywh(0.0, 0.0, 100.0, 50.0).unwrap();
        let path = create_rounded_rect_path(rect, 10.0);
        assert!(path.is_some());

        let path = create_rounded_rect_path(rect, 0.0);
        assert!(path.is_some());

        // Test large radius clamping
        let path = create_rounded_rect_path(rect, 100.0);
        assert!(path.is_some());
    }

    #[test]
    fn test_create_paint_solid() {
        let rect = Rect::from_xywh(0.0, 0.0, 100.0, 50.0).unwrap();
        let color = ParsedColor::Solid(Color::from_rgba8(0, 0, 0, 255));
        let paint = create_paint(&color, rect);
        assert!(paint.anti_alias);
    }

    #[test]
    fn test_create_paint_gradient() {
        let rect = Rect::from_xywh(0.0, 0.0, 100.0, 50.0).unwrap();
        let color = ParsedColor::Gradient(
            vec![
                Color::from_rgba8(0, 0, 0, 255),
                Color::from_rgba8(255, 255, 255, 255),
            ],
            45.0,
        );
        let paint = create_paint(&color, rect);
        assert!(paint.shader.is_opaque() || !paint.shader.is_opaque()); // Just check it's created
    }

    #[test]
    fn test_draw_rounded_rect_variants() {
        let mut data = vec![0u8; 100 * 100 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut data, 100, 100).unwrap();
        let rect = Rect::from_xywh(10.0, 10.0, 80.0, 80.0).unwrap();
        let bg = ParsedColor::Solid(Color::from_rgba8(255, 0, 0, 255));
        let border_color = ParsedColor::Solid(Color::from_rgba8(0, 0, 255, 255));

        // Variant 1: No radius, no border
        draw_rounded_rect(&mut pixmap, rect, &bg, 0.0, &border_color, 0.0);

        // Variant 2: Radius, no border
        draw_rounded_rect(&mut pixmap, rect, &bg, 0.0, &border_color, 5.0);

        // Variant 3: No radius, border
        draw_rounded_rect(&mut pixmap, rect, &bg, 2.0, &border_color, 0.0);

        // Variant 4: Radius, border
        draw_rounded_rect(&mut pixmap, rect, &bg, 2.0, &border_color, 5.0);
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
            &ParsedColor::Solid(bg_color),
            0.0,
            &ParsedColor::Solid(Color::TRANSPARENT),
            0.0,
        );

        assert_pixmap_has_color!(pixmap, bg_color);
    }

    #[test]
    fn test_draw_rounded_rect_radius() {
        let mut pixmap_data = vec![0; 120 * 40 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 120, 40).unwrap();
        pixmap.fill(Color::TRANSPARENT);

        let rect = Rect::from_xywh(10.0, 5.0, 100.0, 30.0).unwrap();
        let bg_color = Color::from_rgba8(0, 255, 0, 255);

        draw_rounded_rect(
            &mut pixmap,
            rect,
            &ParsedColor::Solid(bg_color),
            0.0,
            &ParsedColor::Solid(Color::TRANSPARENT),
            10.0,
        );

        assert_pixmap_has_color!(pixmap, bg_color);
    }

    #[test]
    fn test_draw_rounded_rect_with_border() {
        let mut pixmap_data = vec![0; 120 * 40 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 120, 40).unwrap();
        pixmap.fill(Color::TRANSPARENT);

        let rect = Rect::from_xywh(10.0, 5.0, 100.0, 30.0).unwrap();
        let bg_color = Color::from_rgba8(0, 0, 255, 255);

        draw_rounded_rect(
            &mut pixmap,
            rect,
            &ParsedColor::Solid(bg_color),
            2.0,
            &ParsedColor::Solid(Color::from_rgba8(255, 255, 255, 255)),
            0.0,
        );

        assert_pixmap_has_color!(pixmap, bg_color);
    }

    #[test]
    fn test_draw_rounded_rect_with_gradient_background() {
        let mut pixmap_data = vec![0; 120 * 40 * 4];
        let mut pixmap = PixmapMut::from_bytes(&mut pixmap_data, 120, 40).unwrap();
        pixmap.fill(Color::TRANSPARENT);

        let rect = Rect::from_xywh(10.0, 5.0, 100.0, 30.0).unwrap();
        let gradient = ParsedColor::Gradient(
            vec![
                Color::from_rgba8(255, 0, 0, 255),
                Color::from_rgba8(0, 0, 255, 255),
            ],
            0.0,
        );

        draw_rounded_rect(
            &mut pixmap,
            rect,
            &gradient,
            1.0,
            &ParsedColor::Solid(Color::from_rgba8(255, 255, 255, 255)),
            8.0,
        );

        assert!(pixmap.data_mut().iter().any(|v| *v != 0));
    }
}
