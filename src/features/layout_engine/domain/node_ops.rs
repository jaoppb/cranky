use super::popup::{ActivePanel, AnchoredPopup};
use super::render_node::RenderNode;
use crate::shared::primitives::geometry::Position;
use crate::shared::primitives::ChildModuleLayout;

impl RenderNode {
    #[must_use]
    pub fn collect_module_layouts(&self) -> Vec<ChildModuleLayout> {
        let mut layouts = Vec::new();
        self.collect_module_layouts_recursive(&mut layouts);
        layouts
    }

    fn collect_module_layouts_recursive(&self, out: &mut Vec<ChildModuleLayout>) {
        match self {
            Self::Flex { children, .. } | Self::Grid { children, .. } => {
                for child in children {
                    child.collect_module_layouts_recursive(out);
                }
            }
            Self::Module { rect, key, .. } => {
                out.push(ChildModuleLayout::new(key.clone(), *rect));
            }
            _ => {}
        }
    }

    #[must_use]
    pub fn hit_test(&self, pos: Position) -> Vec<&Self> {
        let mut path = Vec::new();
        self.hit_test_internal(pos, &mut path);
        path
    }

    fn hit_test_internal<'a>(&'a self, pos: Position, path: &mut Vec<&'a Self>) {
        let r = self.rect();
        let width_i32 = i32::try_from(r.width()).unwrap_or(i32::MAX);
        let height_i32 = i32::try_from(r.height()).unwrap_or(i32::MAX);
        let max_x = r.x().saturating_add(width_i32);
        let max_y = r.y().saturating_add(height_i32);
        if pos.x() >= r.x() && pos.x() < max_x && pos.y() >= r.y() && pos.y() < max_y {
            path.push(self);
            if let Self::Flex { children, .. } | Self::Grid { children, .. } = self {
                for child in children {
                    child.hit_test_internal(pos, path);
                }
            }
        }
    }

    #[must_use]
    pub fn find_popup_with_anchor(&self) -> Option<AnchoredPopup<'_>> {
        if let Some(popup) = self.popup() {
            return Some(AnchoredPopup::new(self.rect(), popup));
        }
        if let Self::Flex { children, .. } | Self::Grid { children, .. } = self {
            for child in children {
                if let Some(found) = child.find_popup_with_anchor() {
                    return Some(found);
                }
            }
        }
        None
    }

    #[must_use]
    pub fn find_panel(&self) -> Option<ActivePanel<'_>> {
        if let Some(panel) = self.panel() {
            return Some(ActivePanel::new(panel));
        }
        if let Self::Flex { children, .. } | Self::Grid { children, .. } = self {
            for child in children {
                if let Some(found) = child.find_panel() {
                    return Some(found);
                }
            }
        }
        None
    }
}
