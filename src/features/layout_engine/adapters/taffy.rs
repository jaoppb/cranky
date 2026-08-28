use crate::features::layout_engine::domain::LayoutError;
use crate::features::layout_engine::domain::{
    AlignItems, FlexDirection, JustifyContent, PositionType, RenderNode, StyledNode, TextMeasurer,
};
use crate::features::layout_engine::ports::LayoutEnginePort;
use crate::features::styling::domain::Orientation;
use crate::shared::primitives::geometry::{Position, Rect, Size};
use taffy::prelude::TaffyMaxContent;
use taffy::{
    TaffyTree,
    geometry::Size as TaffySize,
    style::Dimension,
    style::LengthPercentage,
    style::{
        AlignItems as TaffyAlignItems, FlexDirection as TaffyFlexDirection,
        JustifyContent as TaffyJustifyContent, Style,
    },
    tree::NodeId,
};

#[derive(Clone)]
struct LayoutState {
    root_node: NodeId,
    layout: StyledNode,
    children: Vec<Self>,
}

enum Patch<'a> {
    Keep(&'a LayoutState),
    Update {
        old_state: &'a LayoutState,
        new_layout: &'a StyledNode,
        style: Box<Option<Style>>,
        children: Option<Vec<Self>>,
    },
    Replace {
        old_state: &'a LayoutState,
        new_layout: &'a StyledNode,
    },
    Create {
        new_layout: &'a StyledNode,
    },
}

pub struct TaffyLayoutAdapter {
    taffy: TaffyTree,
    state: Option<LayoutState>,
}

impl Default for TaffyLayoutAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl TaffyLayoutAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            taffy: TaffyTree::new(),
            state: None,
        }
    }
}

impl From<FlexDirection> for TaffyFlexDirection {
    fn from(dir: FlexDirection) -> Self {
        match dir {
            FlexDirection::Row => Self::Row,
            FlexDirection::Column => Self::Column,
        }
    }
}

impl From<JustifyContent> for TaffyJustifyContent {
    fn from(jc: JustifyContent) -> Self {
        match jc {
            JustifyContent::Start => Self::FLEX_START,
            JustifyContent::End => Self::FLEX_END,
            JustifyContent::Center => Self::CENTER,
            JustifyContent::SpaceBetween => Self::SPACE_BETWEEN,
            JustifyContent::SpaceAround => Self::SPACE_AROUND,
            JustifyContent::SpaceEvenly => Self::SPACE_EVENLY,
        }
    }
}

impl From<AlignItems> for TaffyAlignItems {
    fn from(ai: AlignItems) -> Self {
        match ai {
            AlignItems::Start => Self::FLEX_START,
            AlignItems::End => Self::FLEX_END,
            AlignItems::Center => Self::CENTER,
            AlignItems::Stretch => Self::STRETCH,
        }
    }
}

impl From<PositionType> for taffy::style::Position {
    fn from(pt: PositionType) -> Self {
        match pt {
            PositionType::Relative => Self::Relative,
            PositionType::Absolute => Self::Absolute,
        }
    }
}

#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
impl From<&crate::features::layout_engine::domain::BoxMargin>
    for taffy::geometry::Rect<LengthPercentage>
{
    fn from(padding: &crate::features::layout_engine::domain::BoxMargin) -> Self {
        Self {
            left: LengthPercentage::length(padding.left() as f32),
            right: LengthPercentage::length(padding.right() as f32),
            top: LengthPercentage::length(padding.top() as f32),
            bottom: LengthPercentage::length(padding.bottom() as f32),
        }
    }
}

#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
impl From<&crate::features::layout_engine::domain::BoxMargin>
    for taffy::geometry::Rect<taffy::style::LengthPercentageAuto>
{
    fn from(margin: &crate::features::layout_engine::domain::BoxMargin) -> Self {
        Self {
            left: LengthPercentage::length(margin.left() as f32).into(),
            right: LengthPercentage::length(margin.right() as f32).into(),
            top: LengthPercentage::length(margin.top() as f32).into(),
            bottom: LengthPercentage::length(margin.bottom() as f32).into(),
        }
    }
}

#[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
impl From<&crate::features::layout_engine::domain::Gap> for TaffySize<LengthPercentage> {
    fn from(gap: &crate::features::layout_engine::domain::Gap) -> Self {
        Self {
            width: LengthPercentage::length(gap.value() as f32),
            height: LengthPercentage::length(gap.value() as f32),
        }
    }
}

impl From<crate::features::styling::domain::CssLength> for Dimension {
    fn from(l: crate::features::styling::domain::CssLength) -> Self {
        match l {
            crate::features::styling::domain::CssLength::Px(v) => Self::length(v),
            crate::features::styling::domain::CssLength::Percent(v) => Self::percent(v / 100.0),
            crate::features::styling::domain::CssLength::Auto => Self::auto(),
        }
    }
}

#[allow(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::too_many_lines
)]
fn node_to_style(node: &StyledNode, measurer: &mut dyn TextMeasurer) -> Style {
    let computed = node.style();
    let mut style = Style {
        flex_direction: computed.flex_direction().unwrap_or_default().into(),
        justify_content: computed.justify_content().map(Into::into),
        align_items: computed.align_items().map(Into::into),
        position: computed.position().unwrap_or_default().into(),
        padding: computed
            .padding()
            .map_or_else(taffy::geometry::Rect::zero, Into::into),
        margin: computed
            .margin()
            .map_or_else(taffy::geometry::Rect::zero, Into::into),
        gap: computed.gap().map_or_else(TaffySize::zero, Into::into),
        ..Default::default()
    };

    if let Some(w) = computed.width() {
        style.size.width = w.into();
    }
    if let Some(h) = computed.height() {
        style.size.height = h.into();
    }
    if let Some(mw) = computed.min_width() {
        style.min_size.width = mw.into();
    }
    if let Some(mw) = computed.max_width() {
        style.max_size.width = mw.into();
    }
    if let Some(mh) = computed.min_height() {
        style.min_size.height = mh.into();
    }
    if let Some(mh) = computed.max_height() {
        style.max_size.height = mh.into();
    }
    if let Some(fg) = computed.flex_grow() {
        style.flex_grow = fg.value();
    }
    if let Some(fs) = computed.flex_shrink() {
        style.flex_shrink = fs.value();
    }
    if let Some(fb) = computed.flex_basis() {
        style.flex_basis = fb.into();
    }
    if let Some(as_) = computed.align_self() {
        style.align_self = Some(as_.into());
    }

    match node {
        StyledNode::Flex { .. } => style,
        StyledNode::Text { text, style: s, .. } => {
            let text_size = measurer.measure(text.as_str(), s.font_family(), s.font_size());
            if computed.width().is_none() {
                style.size.width = Dimension::length(text_size.width() as f32);
            }
            if computed.height().is_none() {
                style.size.height = Dimension::length(text_size.height() as f32);
            }
            style
        }
        StyledNode::Progress { orientation, .. } => {
            let default_size = match orientation {
                Orientation::Horizontal => Size::new(40, 8),
                Orientation::Vertical => Size::new(8, 40),
            };
            if computed.width().is_none() {
                style.size.width = Dimension::length(default_size.width() as f32);
            }
            if computed.height().is_none() {
                style.size.height = Dimension::length(default_size.height() as f32);
            }
            style
        }
        StyledNode::Rect { .. } => {
            if computed.width().is_none() {
                style.size.width = Dimension::length(10.0);
            }
            if computed.height().is_none() {
                style.size.height = Dimension::length(10.0);
            }
            style
        }
        StyledNode::Image { .. } => {
            if computed.width().is_none() {
                style.size.width = Dimension::length(24.0);
            }
            if computed.height().is_none() {
                style.size.height = Dimension::length(24.0);
            }
            style
        }
        StyledNode::Module { key, .. } => {
            if let Some(size) = measurer.measure_module(key) {
                if computed.width().is_none() {
                    style.size.width = Dimension::length(size.width() as f32);
                }
                if computed.height().is_none() {
                    style.size.height = Dimension::length(size.height() as f32);
                }
            }
            style
        }
    }
}

struct TaffyTreeBuilder<'a> {
    taffy: &'a mut TaffyTree,
}

impl<'a> TaffyTreeBuilder<'a> {
    const fn new(taffy: &'a mut TaffyTree) -> Self {
        Self { taffy }
    }

    fn add_leaf(&mut self, style: Style) -> Result<NodeId, LayoutError> {
        self.taffy
            .new_leaf(style)
            .map_err(|e| LayoutError::EngineError(e.to_string()))
    }

    fn add_node(&mut self, style: Style, children: &[NodeId]) -> Result<NodeId, LayoutError> {
        self.taffy
            .new_with_children(style, children)
            .map_err(|e| LayoutError::EngineError(e.to_string()))
    }

    fn set_style(&mut self, node_id: NodeId, style: Style) -> Result<(), LayoutError> {
        self.taffy
            .set_style(node_id, style)
            .map_err(|e| LayoutError::EngineError(e.to_string()))
    }

    fn set_children(&mut self, node_id: NodeId, children: &[NodeId]) -> Result<(), LayoutError> {
        self.taffy
            .set_children(node_id, children)
            .map_err(|e| LayoutError::EngineError(e.to_string()))
    }

    fn remove_recursive(&mut self, node_id: NodeId) {
        if let Ok(children) = self.taffy.children(node_id) {
            for child in children {
                self.remove_recursive(child);
            }
        }
        let _ = self.taffy.remove(node_id);
    }
}

impl LayoutEnginePort for TaffyLayoutAdapter {
    #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
    fn calculate_layout_with_constraints(
        &mut self,
        node: StyledNode,
        measurer: &mut dyn TextMeasurer,
        start_pos: Position,
        available_size: Option<Size>,
    ) -> Result<RenderNode, LayoutError> {
        let mut builder = TaffyTreeBuilder::new(&mut self.taffy);
        let new_state = if let Some(state) = &self.state {
            let patch = diff(state, &node, measurer);
            apply_patch(&mut builder, patch, measurer)?
        } else {
            build_layout_state(&mut builder, &node, measurer)?
        };

        let root_node_id = new_state.root_node;

        let available_space = available_size.map_or(taffy::geometry::Size::MAX_CONTENT, |size| {
            taffy::geometry::Size {
                width: taffy::style::AvailableSpace::Definite(size.width() as f32),
                height: taffy::style::AvailableSpace::Definite(size.height() as f32),
            }
        });

        // Compute layout
        self.taffy
            .compute_layout(root_node_id, available_space)
            .map_err(|e| LayoutError::EngineError(e.to_string()))?;

        // Build RenderNode tree
        let render_tree = build_render_tree(&self.taffy, root_node_id, &node, start_pos)?;

        tracing::trace!(
            rect = ?render_tree.rect(),
            "Calculated layout render tree"
        );

        self.state = Some(new_state);

        Ok(render_tree)
    }
}

fn build_layout_state(
    builder: &mut TaffyTreeBuilder,
    node: &StyledNode,
    measurer: &mut dyn TextMeasurer,
) -> Result<LayoutState, LayoutError> {
    let style = node_to_style(node, measurer);

    if let StyledNode::Flex { children, .. } = node {
        let mut state_children = Vec::new();
        let mut child_ids = Vec::new();
        for child in children {
            let state_child = build_layout_state(builder, child, measurer)?;
            child_ids.push(state_child.root_node);
            state_children.push(state_child);
        }

        let node_id = builder.add_node(style, &child_ids)?;

        Ok(LayoutState {
            root_node: node_id,
            layout: node.clone(),
            children: state_children,
        })
    } else {
        let node_id = builder.add_leaf(style)?;
        Ok(LayoutState {
            root_node: node_id,
            layout: node.clone(),
            children: Vec::new(),
        })
    }
}

#[allow(clippy::too_many_lines, clippy::if_not_else, clippy::indexing_slicing)]
fn diff<'a>(
    old_state: &'a LayoutState,
    new_layout: &'a StyledNode,
    measurer: &mut dyn TextMeasurer,
) -> Patch<'a> {
    if std::mem::discriminant(&old_state.layout) != std::mem::discriminant(new_layout) {
        return Patch::Replace {
            old_state,
            new_layout,
        };
    }

    match (&old_state.layout, new_layout) {
        (
            StyledNode::Flex {
                style: old_style, ..
            },
            StyledNode::Flex {
                style: new_style,
                children: new_children,
                ..
            },
        ) => {
            let style = if old_style != new_style {
                Some(node_to_style(new_layout, measurer))
            } else {
                None
            };

            let mut child_patches = Vec::new();
            for (i, new_child) in new_children.iter().enumerate() {
                if i < old_state.children.len() {
                    child_patches.push(diff(&old_state.children[i], new_child, measurer));
                } else {
                    child_patches.push(Patch::Create {
                        new_layout: new_child,
                    });
                }
            }

            Patch::Update {
                old_state,
                new_layout,
                style: Box::new(style),
                children: Some(child_patches),
            }
        }
        (
            StyledNode::Text {
                text: old_text,
                style: old_style,
                ..
            },
            StyledNode::Text {
                text: new_text,
                style: new_style,
                ..
            },
        ) => {
            let style = if old_text != new_text || old_style != new_style {
                Some(node_to_style(new_layout, measurer))
            } else {
                None
            };

            if style.is_some() {
                Patch::Update {
                    old_state,
                    new_layout,
                    style: Box::new(style),
                    children: None,
                }
            } else {
                Patch::Keep(old_state)
            }
        }
        (
            StyledNode::Progress {
                value: old_val,
                orientation: old_orient,
                style: old_style,
                ..
            },
            StyledNode::Progress {
                value: new_val,
                orientation: new_orient,
                style: new_style,
                ..
            },
        ) => {
            let style = if old_val != new_val || old_orient != new_orient || old_style != new_style
            {
                Some(node_to_style(new_layout, measurer))
            } else {
                None
            };

            if style.is_some() {
                Patch::Update {
                    old_state,
                    new_layout,
                    style: Box::new(style),
                    children: None,
                }
            } else {
                Patch::Keep(old_state)
            }
        }
        (
            StyledNode::Rect {
                style: old_style, ..
            },
            StyledNode::Rect {
                style: new_style, ..
            },
        ) => {
            let style = if old_style == new_style {
                None
            } else {
                Some(node_to_style(new_layout, measurer))
            };

            if style.is_some() {
                Patch::Update {
                    old_state,
                    new_layout,
                    style: Box::new(style),
                    children: None,
                }
            } else {
                Patch::Keep(old_state)
            }
        }
        (
            StyledNode::Image {
                pixel_size: old_size,
                style: old_style,
                ..
            },
            StyledNode::Image {
                pixel_size: new_size,
                style: new_style,
                ..
            },
        ) => {
            let style = if old_size != new_size || old_style != new_style {
                Some(node_to_style(new_layout, measurer))
            } else {
                None
            };

            if style.is_some() {
                Patch::Update {
                    old_state,
                    new_layout,
                    style: Box::new(style),
                    children: None,
                }
            } else {
                Patch::Keep(old_state)
            }
        }
        (StyledNode::Module { .. }, StyledNode::Module { .. }) => {
            let style = node_to_style(new_layout, measurer);
            Patch::Update {
                old_state,
                new_layout,
                style: Box::new(Some(style)),
                children: None,
            }
        }
        _ => Patch::Replace {
            old_state,
            new_layout,
        },
    }
}

fn apply_patch(
    builder: &mut TaffyTreeBuilder,
    patch: Patch,
    measurer: &mut dyn TextMeasurer,
) -> Result<LayoutState, LayoutError> {
    match patch {
        Patch::Keep(state) => Ok(state.clone()),
        Patch::Update {
            old_state,
            new_layout,
            style,
            children,
        } => {
            let node_id = old_state.root_node;

            if let Some(s) = *style {
                builder.set_style(node_id, s)?;
            }

            let mut new_state_children = Vec::new();
            if let Some(child_patches) = children {
                let mut new_child_ids = Vec::new();
                for cp in child_patches {
                    let child_state = apply_patch(builder, cp, measurer)?;
                    new_child_ids.push(child_state.root_node);
                    new_state_children.push(child_state);
                }

                if old_state.children.len() > new_state_children.len() {
                    for child in old_state.children.iter().skip(new_state_children.len()) {
                        builder.remove_recursive(child.root_node);
                    }
                }

                builder.set_children(node_id, &new_child_ids)?;
            }

            Ok(LayoutState {
                root_node: node_id,
                layout: new_layout.clone(),
                children: if new_state_children.is_empty() && !old_state.children.is_empty() {
                    old_state.children.clone()
                } else {
                    new_state_children
                },
            })
        }
        Patch::Replace {
            old_state,
            new_layout,
        } => {
            let new_state = build_layout_state(builder, new_layout, measurer)?;
            builder.remove_recursive(old_state.root_node);
            Ok(new_state)
        }
        Patch::Create { new_layout } => build_layout_state(builder, new_layout, measurer),
    }
}

#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]
fn build_render_tree(
    taffy: &TaffyTree,
    node_id: NodeId,
    node: &StyledNode,
    offset: Position,
) -> Result<RenderNode, LayoutError> {
    let layout = taffy
        .layout(node_id)
        .map_err(|e| LayoutError::EngineError(e.to_string()))?;

    let abs_x = offset.x().saturating_add(layout.location.x as i32);
    let abs_y = offset.y().saturating_add(layout.location.y as i32);
    let rect = Rect::new(
        Position::new(abs_x, abs_y),
        Size::new(layout.size.width as u32, layout.size.height as u32),
    );

    match node {
        StyledNode::Flex {
            children,
            style,
            on_click,
            on_hover,
            tooltip,
        } => {
            let child_ids = taffy
                .children(node_id)
                .map_err(|e| LayoutError::EngineError(e.to_string()))?;
            let mut render_children = Vec::new();

            for (child, &child_id) in children.iter().zip(child_ids.iter()) {
                render_children.push(build_render_tree(
                    taffy,
                    child_id,
                    child,
                    Position::new(abs_x, abs_y),
                )?);
            }

            Ok(RenderNode::Flex {
                rect,
                children: render_children,
                style: style.clone(),
                on_click: on_click.clone(),
                on_hover: on_hover.clone(),
                tooltip: tooltip.clone(),
            })
        }
        StyledNode::Text {
            text,
            style,
            on_click,
            on_hover,
            tooltip,
        } => Ok(RenderNode::Text {
            rect,
            text: text.clone(),
            style: style.clone(),
            on_click: on_click.clone(),
            on_hover: on_hover.clone(),
            tooltip: tooltip.clone(),
        }),
        StyledNode::Progress {
            value,
            orientation,
            style,
            on_click,
            on_hover,
            tooltip,
        } => Ok(RenderNode::Progress {
            rect,
            value: *value,
            orientation: *orientation,
            style: style.clone(),
            on_click: on_click.clone(),
            on_hover: on_hover.clone(),
            tooltip: tooltip.clone(),
        }),
        StyledNode::Rect {
            style,
            on_click,
            on_hover,
            tooltip,
        } => Ok(RenderNode::Rect {
            rect,
            style: style.clone(),
            on_click: on_click.clone(),
            on_hover: on_hover.clone(),
            tooltip: tooltip.clone(),
        }),
        StyledNode::Image {
            data,
            pixel_size,
            tooltip,
            ..
        } => Ok(RenderNode::Image {
            rect,
            data: data.clone(),
            pixel_size: *pixel_size,
            tooltip: tooltip.clone(),
        }),
        StyledNode::Module {
            key,
            style,
            on_click,
            on_hover,
            tooltip,
            ..
        } => Ok(RenderNode::Module {
            rect,
            key: key.clone(),
            style: style.clone(),
            on_click: on_click.clone(),
            on_hover: on_hover.clone(),
            tooltip: tooltip.clone(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::layout_engine::domain::{StyledNode, TextContent, TextMeasurer};
    use crate::features::styling::domain::ComputedStyle;
    use crate::shared::config::domain::{FontFamily, FontSize};
    use crate::shared::primitives::geometry::{Position, Size};
    use crate::shared::primitives::{ModuleKey, ModuleName, ModuleOptions};

    struct MockMeasurer;
    impl TextMeasurer for MockMeasurer {
        fn measure(
            &mut self,
            text: &str,
            _font: Option<&FontFamily>,
            _size: Option<FontSize>,
        ) -> Size {
            let len = u32::try_from(text.len()).unwrap_or(0);
            Size::new(len.saturating_mul(10), 20)
        }

        fn measure_module(&self, key: &ModuleKey) -> Option<Size> {
            if key.name() == "workspace" {
                Some(Size::new(150, 28))
            } else {
                None
            }
        }
    }

    #[test]
    fn test_calculate_layout_styled_text() {
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;

        let node = StyledNode::Text {
            text: TextContent::new("hello".to_string()),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
        };

        let render_tree = adapter
            .calculate_layout(node, &mut measurer, Position::new(0, 0))
            .unwrap();
        assert_eq!(render_tree.rect().width(), 50);
        assert_eq!(render_tree.rect().height(), 20);
    }

    #[test]
    fn test_calculate_layout_styled_module_with_custom_size() {
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;

        let mut style = ComputedStyle::default();
        style.set_width(crate::features::styling::domain::CssLength::Px(120.0));
        style.set_height(crate::features::styling::domain::CssLength::Px(30.0));

        let node = StyledNode::Module {
            key: ModuleKey::from_name(ModuleName::new("custom_mod")),
            options: ModuleOptions::default(),
            style,
            on_click: None,
            on_hover: None,
            tooltip: None,
        };

        let render_tree = adapter
            .calculate_layout(node, &mut measurer, Position::new(10, 5))
            .unwrap();
        assert_eq!(render_tree.rect().x(), 10);
        assert_eq!(render_tree.rect().y(), 5);
        assert_eq!(render_tree.rect().width(), 120);
        assert_eq!(render_tree.rect().height(), 30);
    }

    #[test]
    fn test_calculate_layout_styled_module_with_measured_size() {
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;

        let node = StyledNode::Module {
            key: ModuleKey::from_name(ModuleName::new("workspace")),
            options: ModuleOptions::default(),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
        };

        let render_tree = adapter
            .calculate_layout(node, &mut measurer, Position::new(0, 0))
            .unwrap();
        assert_eq!(render_tree.rect().width(), 150);
        assert_eq!(render_tree.rect().height(), 28);
    }

    #[test]
    fn test_nested_container_module_collect_layouts() {
        let mut adapter = TaffyLayoutAdapter::new();
        let mut measurer = MockMeasurer;

        let child1 = StyledNode::Module {
            key: ModuleKey::from_name(ModuleName::new("workspace")),
            options: ModuleOptions::default(),
            style: ComputedStyle::default(),
            on_click: None,
            on_hover: None,
            tooltip: None,
        };

        let mut root_style = ComputedStyle::default();
        root_style.set_padding(crate::features::layout_engine::domain::BoxMargin::new(
            0.0, 0.0, 16.0, 16.0,
        ));
        let root = StyledNode::Flex {
            children: vec![child1],
            style: root_style,
            on_click: None,
            on_hover: None,
            tooltip: None,
        };

        let render_tree = adapter
            .calculate_layout(root, &mut measurer, Position::new(0, 0))
            .unwrap();

        let layouts = render_tree.collect_module_layouts();
        assert_eq!(layouts.len(), 1);
        assert_eq!(layouts[0].key().name().as_str(), "workspace");
        assert_eq!(layouts[0].bounds().x(), 16);
    }
}
