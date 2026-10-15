use super::target::FloatingKind;
use std::cmp::Reverse;
use std::collections::HashSet;
use thiserror::Error;

/// A floating surface's parent link forms a cycle — nesting is unrestricted
/// (decision 7), but a chain can never loop back on itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("popup nesting cycle detected")]
pub struct PopupCycle;

/// Walks `start`'s parent chain via `parent_of`, returning every ancestor
/// nearest-first.
///
/// `parent_of` is the direct-parent link a `FloatingKind` was opened with.
/// The result never includes `start` itself.
///
/// # Errors
///
/// Returns `PopupCycle` if the walk revisits a kind already seen — the same
/// "walk the chain, reject a repeat" shape `AppState::would_cycle` already
/// uses for lazy module spawns, just over `FloatingKind` parentage instead
/// of `ModuleId` parentage. No depth cap (decision 12/non-goals): only a
/// genuine cycle is rejected.
pub fn ancestors(
    start: &FloatingKind,
    parent_of: impl Fn(&FloatingKind) -> Option<FloatingKind>,
) -> Result<Vec<FloatingKind>, PopupCycle> {
    let mut seen: HashSet<FloatingKind> = HashSet::from([start.clone()]);
    let mut result = Vec::new();
    let mut current = start.clone();
    while let Some(parent) = parent_of(&current) {
        if !seen.insert(parent.clone()) {
            return Err(PopupCycle);
        }
        result.push(parent.clone());
        current = parent;
    }
    Ok(result)
}

/// Every descendant of `root` among `links`, ordered deepest-first.
///
/// Each pair in `links` is a floating surface's own kind and the direct
/// parent it was opened with. Deepest-first is the order a nested chain
/// must be torn down in, since destroying a non-topmost `xdg_popup` is a
/// protocol error. The result never includes `root` itself; the caller
/// destroys that last.
#[must_use]
pub fn descendants_deepest_first(
    root: &FloatingKind,
    links: &[(FloatingKind, Option<FloatingKind>)],
) -> Vec<FloatingKind> {
    let mut by_depth: Vec<(usize, FloatingKind)> = Vec::new();
    let mut frontier = vec![root.clone()];
    let mut depth = 0usize;
    while !frontier.is_empty() {
        depth = depth.saturating_add(1);
        let mut next = Vec::new();
        for (kind, parent) in links {
            if parent.as_ref().is_some_and(|p| frontier.contains(p)) {
                by_depth.push((depth, kind.clone()));
                next.push(kind.clone());
            }
        }
        frontier = next;
    }
    by_depth.sort_by_key(|(depth, _)| Reverse(*depth));
    by_depth.into_iter().map(|(_, kind)| kind).collect()
}
