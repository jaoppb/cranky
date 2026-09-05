use super::reconciler::{LayoutState, Patch};
use super::style_builder::node_to_style;
use crate::features::layout_engine::domain::{StyledNode, TextMeasurer};

#[allow(clippy::too_many_lines, clippy::if_not_else, clippy::indexing_slicing)]
pub fn diff<'a>(
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
        )
        | (
            StyledNode::Grid {
                style: old_style, ..
            },
            StyledNode::Grid {
                style: new_style,
                children: new_children,
                ..
            },
        ) => {
            let style = if old_style == new_style {
                None
            } else {
                Some(node_to_style(new_layout, measurer))
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
            let style = if old_text == new_text && old_style == new_style {
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
            let style = if old_val == new_val && old_orient == new_orient && old_style == new_style
            {
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
            let style = if old_size == new_size && old_style == new_style {
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
