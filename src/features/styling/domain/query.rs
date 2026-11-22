//! The element description that selectors are matched against.
use super::grid_types::PseudoClass;
use super::identifiers::{ClassName, ElementId};

/// Whether an element has no children, which the `:empty` pseudo-class matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Emptiness {
    /// The element has no children.
    Empty,
    /// The element has children.
    NonEmpty,
}

impl From<bool> for Emptiness {
    /// `true` is [`Emptiness::Empty`].
    fn from(is_empty: bool) -> Self {
        if is_empty {
            Self::Empty
        } else {
            Self::NonEmpty
        }
    }
}

/// What an element is to the type, id and class selectors: its tag, its id and its classes,
/// as the markup gave them, apart from any state or position.
#[derive(Debug, Clone, Copy)]
pub struct ElementIdentity<'a> {
    tag: &'a str,
    id: Option<&'a ElementId>,
    classes: &'a [ClassName],
}

impl<'a> ElementIdentity<'a> {
    /// An element of `tag`, with `id` if it has one, carrying `classes`.
    #[must_use]
    pub const fn new(tag: &'a str, id: Option<&'a ElementId>, classes: &'a [ClassName]) -> Self {
        Self { tag, id, classes }
    }
}

/// Where an element sits among its parent's children: its zero-based index and how many
/// siblings there are, itself included; read by the structural pseudo-classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SiblingPosition {
    index: usize,
    total: usize,
}

impl SiblingPosition {
    /// The only child: what a root element, which has no siblings, is treated as.
    #[must_use]
    pub(crate) const fn root() -> Self {
        Self { index: 0, total: 1 }
    }

    /// Child `index` of `total` children.
    #[must_use]
    pub(crate) const fn new(index: usize, total: usize) -> Self {
        Self { index, total }
    }
}

/// What a selector can see of one element: tag, id, classes, active pseudo-classes, position
/// among its siblings, emptiness and a chain to its parent's query for combinators.
#[derive(Debug, Clone)]
pub struct ElementQuery<'a> {
    identity: ElementIdentity<'a>,
    pseudo_classes: &'a [PseudoClass],
    position: SiblingPosition,
    emptiness: Emptiness,
    parent: Option<&'a Self>,
}

impl<'a> ElementQuery<'a> {
    /// Describes an element as the first of one sibling and non-empty; the vdom bridge then sets
    /// the real position and emptiness with `with_structural_context`.
    #[must_use]
    pub const fn new(
        identity: ElementIdentity<'a>,
        pseudo_classes: &'a [PseudoClass],
        parent: Option<&'a Self>,
    ) -> Self {
        Self {
            identity,
            pseudo_classes,
            position: SiblingPosition::root(),
            emptiness: Emptiness::NonEmpty,
            parent,
        }
    }

    #[must_use]
    pub(crate) const fn with_structural_context(
        mut self,
        position: SiblingPosition,
        emptiness: Emptiness,
    ) -> Self {
        self.position = position;
        self.emptiness = emptiness;
        self
    }

    #[must_use]
    pub(crate) const fn tag(&self) -> &'a str {
        self.identity.tag
    }

    #[must_use]
    pub(crate) const fn id(&self) -> Option<&'a ElementId> {
        self.identity.id
    }

    #[must_use]
    pub(crate) const fn classes(&self) -> &'a [ClassName] {
        self.identity.classes
    }

    #[must_use]
    pub(crate) const fn pseudo_classes(&self) -> &'a [PseudoClass] {
        self.pseudo_classes
    }

    #[must_use]
    pub(crate) const fn child_index(&self) -> usize {
        self.position.index
    }

    #[must_use]
    pub(crate) const fn total_children(&self) -> usize {
        self.position.total
    }

    #[must_use]
    pub(crate) const fn is_empty(&self) -> bool {
        matches!(self.emptiness, Emptiness::Empty)
    }

    #[must_use]
    pub(crate) const fn parent(&self) -> Option<&'a Self> {
        self.parent
    }
}
