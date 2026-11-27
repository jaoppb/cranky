use super::reconciler::LayoutState;
use crate::features::styling::domain::CssLength;
use taffy::TaffyTree;
use taffy::style::Dimension;
use taffy::tree::NodeId;

/// Which style field a pending `calc()` needs to overwrite once its
/// containing block's size is known.
#[derive(Debug, Clone, Copy)]
enum CalcField {
    Width,
    Height,
    MinWidth,
    MinHeight,
    MaxWidth,
    MaxHeight,
    FlexBasis,
}

/// One node's unresolved `calc(<percent>% + <px>px)`, discovered from the
/// first layout pass's `LayoutState` (which already pairs every taffy
/// `NodeId` with the `StyledNode` — and so `ComputedStyle` — it came from).
struct PendingCalc {
    node_id: NodeId,
    field: CalcField,
    percent: f32,
    px: f32,
}

/// Walks `state` collecting every `CssLength::Calc` on the seven
/// length-bearing fields that can carry one, so `resolve_pending_calcs` can
/// patch them once real containing-block sizes exist.
fn collect_pending_calcs(state: &LayoutState, out: &mut Vec<PendingCalc>) {
    let computed = state.layout.style();
    let mut push = |field: CalcField, len: Option<CssLength>| {
        if let Some(CssLength::Calc { percent, px }) = len {
            out.push(PendingCalc {
                node_id: state.root_node,
                field,
                percent,
                px,
            });
        }
    };
    push(CalcField::Width, computed.width());
    push(CalcField::Height, computed.height());
    push(CalcField::MinWidth, computed.min_width());
    push(CalcField::MinHeight, computed.min_height());
    push(CalcField::MaxWidth, computed.max_width());
    push(CalcField::MaxHeight, computed.max_height());
    push(CalcField::FlexBasis, computed.flex_basis());

    for child in &state.children {
        collect_pending_calcs(child, out);
    }
}

/// Resolves every pending `calc()` against its parent's first-pass layout
/// size (the CSS percentage basis) and writes the concrete pixel value
/// into that node's taffy style. Returns whether anything was patched, so
/// the caller knows whether a second `compute_layout` is actually needed.
///
/// The parent's *border-box* size stands in for the CSS containing block —
/// this renderer doesn't track box-sizing distinctly, so it's the same
/// approximation every other percentage-based dimension here already makes.
pub(super) fn resolve_pending_calcs(taffy: &mut TaffyTree, root_state: &LayoutState) -> bool {
    let mut pending = Vec::new();
    collect_pending_calcs(root_state, &mut pending);
    if pending.is_empty() {
        return false;
    }

    let mut patched = false;
    for calc in &pending {
        if patch_calc(taffy, calc) {
            patched = true;
        }
    }
    patched
}

/// Writes `calc`'s value, resolved against its parent's first-pass size, into its node's style,
/// and tells whether the style was set; a node without a parent, a parent without a layout or
/// a node without a style is left alone.
fn patch_calc(taffy: &mut TaffyTree, calc: &PendingCalc) -> bool {
    let Some(parent_id) = taffy.parent(calc.node_id) else {
        return false;
    };
    let Ok(parent_layout) = taffy.layout(parent_id) else {
        return false;
    };
    let basis = calc.field.percentage_basis(parent_layout.size);
    let resolved = (calc.percent / 100.0).mul_add(basis, calc.px);
    let Ok(mut style) = taffy.style(calc.node_id).cloned() else {
        return false;
    };
    calc.field.set(&mut style, Dimension::length(resolved));
    taffy.set_style(calc.node_id, style).is_ok()
}

impl CalcField {
    /// The side of the parent's size a percentage in this field is taken of.
    const fn percentage_basis(self, parent: taffy::geometry::Size<f32>) -> f32 {
        match self {
            Self::Width | Self::MinWidth | Self::MaxWidth | Self::FlexBasis => parent.width,
            Self::Height | Self::MinHeight | Self::MaxHeight => parent.height,
        }
    }

    /// Sets this field of `style` to `value`.
    const fn set(self, style: &mut taffy::style::Style, value: Dimension) {
        match self {
            Self::Width => style.size.width = value,
            Self::Height => style.size.height = value,
            Self::MinWidth => style.min_size.width = value,
            Self::MinHeight => style.min_size.height = value,
            Self::MaxWidth => style.max_size.width = value,
            Self::MaxHeight => style.max_size.height = value,
            Self::FlexBasis => style.flex_basis = value,
        }
    }
}
