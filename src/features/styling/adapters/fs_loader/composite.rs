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
                let layer = if sheet.name().as_str() == "base" {
                    Layer::BASE
                } else {
                    Layer::MODULE
                };
                let sheet_index = u32::try_from(sheet_index).unwrap_or(u32::MAX);
                sheet
                    .matching_rules(query)
                    .into_iter()
                    .map(move |rule_match| {
                        rule_match.into_matched(CascadeOrigin::new(layer, sheet_index))
                    })
            })
            .collect();
        fold_matches(matches).collapse(inherited, &LightningPropertyReparser)
    }
}
