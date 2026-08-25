pub mod adapters;
pub mod application;
pub mod domain;
pub mod ports;

#[cfg(test)]
pub mod test_support {
    use crate::app::commands::AppCommand;
    use crate::features::layout_engine::domain::TextMeasurer;
    use crate::features::module_runtime::ports::{AnyModulePort, CommandSender, ModuleInitError};
    use crate::features::vdom::domain::VNode;
    use crate::shared::config::domain::{Config, FontFamily, FontSize, ModuleConfig};
    use crate::shared::events::signals::{SignalHub, SignalKind};
    use crate::shared::primitives::geometry::{Position, Scale, Size};
    use crate::shared::primitives::render::RenderBuffer;
    use crate::shared::primitives::{FunctionName, ModuleId, MonitorId};
    use crate::shared::rendering::ports::canvas::{Canvas, CanvasFactory};

    pub struct MockCommandSender;

    impl CommandSender for MockCommandSender {
        fn send_command(&self, _cmd: AppCommand) {}
    }

    pub struct ChannelCommandSender {
        pub tx: std::sync::mpsc::Sender<AppCommand>,
    }

    impl ChannelCommandSender {
        pub fn new(tx: std::sync::mpsc::Sender<AppCommand>) -> Self {
            Self { tx }
        }
    }

    impl CommandSender for ChannelCommandSender {
        fn send_command(&self, cmd: AppCommand) {
            let _ = self.tx.send(cmd);
        }
    }

    pub struct MockSurfaceManager;

    #[async_trait::async_trait]
    impl crate::shared::wayland::ports::SurfaceManagerPort for MockSurfaceManager {
        async fn submit_buffer(
            &self,
            _mod_id: ModuleId,
            _mon_id: MonitorId,
            _pos: Position,
            _buf: RenderBuffer,
        ) {
        }
    }

    #[derive(Debug, Default, Clone)]
    pub struct MockCanvasFactory;

    impl CanvasFactory for MockCanvasFactory {
        fn create_canvas<'a>(
            &'a mut self,
            _data: &'a mut [u8],
            _size: Size,
            _scale: Scale,
            _font_family: FontFamily,
            _font_size: FontSize,
        ) -> impl Canvas + 'a {
            MockCanvas
        }

        fn create_text_measurer<'a>(
            &'a mut self,
            _scale: Scale,
            _font_family: FontFamily,
            _font_size: FontSize,
        ) -> impl TextMeasurer + 'a {
            MockMeasurer
        }
    }

    pub struct MockCanvas;

    impl Canvas for MockCanvas {
        fn draw_rect(
            &mut self,
            _x: crate::shared::primitives::geometry::LogicalPx,
            _y: crate::shared::primitives::geometry::LogicalPx,
            _w: crate::shared::primitives::geometry::LogicalPx,
            _h: crate::shared::primitives::geometry::LogicalPx,
            _color: crate::shared::primitives::color::DrawingColor,
            _radius: crate::shared::primitives::geometry::LogicalPx,
        ) {
        }

        fn draw_border(
            &mut self,
            _pos: Position,
            _size: Size,
            _color: crate::shared::primitives::color::DrawingColor,
            _radius: crate::shared::primitives::geometry::LogicalPx,
            _border_size: crate::shared::primitives::geometry::LogicalPx,
        ) {
        }

        fn draw_text(
            &mut self,
            _text: &str,
            _font_family: Option<&FontFamily>,
            _font_size: Option<FontSize>,
            _color: crate::shared::primitives::color::DrawingColor,
            _pos: Position,
        ) {
        }

        fn draw_image(
            &mut self,
            _image_data: &[u8],
            _pixel_size: Size,
            _logical_size: Size,
            _pos: Position,
        ) {
        }
    }

    pub struct MockMeasurer;

    impl TextMeasurer for MockMeasurer {
        fn measure(
            &mut self,
            _text: &str,
            _font_family: Option<&FontFamily>,
            _font_size: Option<FontSize>,
        ) -> Size {
            Size::new(10, 10)
        }
    }

    pub struct TestModulePort {
        pub node: VNode,
        pub subs: Vec<SignalKind>,
    }

    impl TestModulePort {
        pub fn new(node: VNode) -> Self {
            Self {
                node,
                subs: Vec::new(),
            }
        }

        pub fn with_subs(node: VNode, subs: Vec<SignalKind>) -> Self {
            Self { node, subs }
        }
    }

    impl AnyModulePort for TestModulePort {
        fn init(
            &mut self,
            _config: &ModuleConfig,
            _full_config: &Config,
        ) -> Result<(), ModuleInitError> {
            Ok(())
        }

        fn subscriptions(&self) -> &[SignalKind] {
            &self.subs
        }

        fn styles(&self) -> &[crate::features::styling::domain::StyleSheetName] {
            &[]
        }

        fn refresh(&mut self, _hub: &SignalHub, _signals: &[SignalKind]) {}

        fn render(&self, _monitor: &MonitorId) -> VNode {
            self.node.clone()
        }

        fn call_function(&mut self, _name: &FunctionName) -> Result<(), ModuleInitError> {
            Ok(())
        }
    }
}
