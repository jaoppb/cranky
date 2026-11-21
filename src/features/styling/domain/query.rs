//! The element description that selectors are matched against.
use super::grid_types::PseudoClass;
use super::identifiers::{ClassName, ElementId};

/// What a selector can see of one element: tag, id, classes, active pseudo-classes, position
/// among its siblings, emptiness and a chain to its parent's query for combinators.
#[derive(Debug, Clone)]
pub struct ElementQuery<'a> {
    tag: &'a str,
    id: Option<&'a ElementId>,
    classes: &'a [ClassName],
    pseudo_classes: &'a [PseudoClass],
    child_index: usize,
    total_children: usize,
    is_empty: bool,
    parent: Option<&'a Self>,
}

impl<'a> ElementQuery<'a> {
    /// Describes an element as the first of one sibling and non-empty; the vdom bridge then sets
    /// the real position and emptiness with `with_structural_context`.
    #[must_use]
    pub const fn new(
        tag: &'a str,
        id: Option<&'a ElementId>,
        classes: &'a [ClassName],
        pseudo_classes: &'a [PseudoClass],
        parent: Option<&'a Self>,
    ) -> Self {
        Self {
            tag,
            id,
            classes,
            pseudo_classes,
            child_index: 0,
            total_children: 1,
            is_empty: false,
            parent,
        }
    }

    #[must_use]
    pub(crate) const fn with_structural_context(
        mut self,
        child_index: usize,
        total_children: usize,
        is_empty: bool,
    ) -> Self {
        self.child_index = child_index;
        self.total_children = total_children;
        self.is_empty = is_empty;
        self
    }

    #[must_use]
    pub(crate) const fn tag(&self) -> &'a str {
        self.tag
    }

    #[must_use]
    pub(crate) const fn id(&self) -> Option<&'a ElementId> {
        self.id
    }

    #[must_use]
    pub(crate) const fn classes(&self) -> &'a [ClassName] {
        self.classes
    }

    #[must_use]
    pub(crate) const fn pseudo_classes(&self) -> &'a [PseudoClass] {
        self.pseudo_classes
    }

    #[must_use]
    pub(crate) const fn child_index(&self) -> usize {
        self.child_index
    }

    #[must_use]
    pub(crate) const fn total_children(&self) -> usize {
        self.total_children
    }

    #[must_use]
    pub(crate) const fn is_empty(&self) -> bool {
        self.is_empty
    }

    #[must_use]
    pub(crate) const fn parent(&self) -> Option<&'a Self> {
        self.parent
    }
}
