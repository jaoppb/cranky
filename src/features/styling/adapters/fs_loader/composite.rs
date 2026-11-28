//! Composes a module's parsed sheets into one resolver, the shared base sheet in the lower
//! cascade layer.

use super::super::lightningcss::LightningPropertyReparser;
use crate::features::styling::domain::{
    CascadeOrigin, ComputedStyle, ElementQuery, InheritedStyle, Layer, fold_matches,
};
use crate::features::styling::ports::{ParsedStyleSheetPort, StyleResolverPort};

pub(crate) struct CompositeStyleResolver {
    stylesheets: Vec<Box<dyn ParsedStyleSheetPort>>,
}

impl CompositeStyleResolver {
    #[must_use]
    pub(crate) fn new(stylesheets: Vec<Box<dyn ParsedStyleSheetPort>>) -> Self {
        Self { stylesheets }
    }
}

impl StyleResolverPort for CompositeStyleResolver {
    fn resolve_style(
        &self,
        query: &ElementQuery<'_>,
        inherited: &InheritedStyle,
    ) -> (ComputedStyle, InheritedStyle) {
        // "base" is the one shared origin sheet, always loaded first by the
        // registry; every other composed sheet is the module's own. Cascade
        // layer order beats specificity, so base stays overridable by
        // construction rather than by (previously accidental) load order.
        let matches = self
            .stylesheets
            .iter()
            .enumerate()
            .flat_map(|(sheet_index, sheet)| {
                let origin = origin_of(sheet.as_ref(), sheet_index);
                let rule_matches = sheet.matching_rules(query).into_iter();
                rule_matches.map(move |rule_match| rule_match.into_matched(origin))
            })
            .collect();
        fold_matches(matches).collapse(inherited, &LightningPropertyReparser)
    }
}

/// Where `sheet`, composed at `sheet_index`, sits in the cascade: the base layer for the shared
/// `base` sheet, the module layer for every other one.
fn origin_of(sheet: &dyn ParsedStyleSheetPort, sheet_index: usize) -> CascadeOrigin {
    let layer = if sheet.name().as_str() == "base" {
        Layer::BASE
    } else {
        Layer::MODULE
    };
    CascadeOrigin::new(layer, u32::try_from(sheet_index).unwrap_or(u32::MAX))
}
