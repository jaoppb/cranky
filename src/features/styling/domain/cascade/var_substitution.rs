//! Substitutes `var()` references in a declaration's value from the element's custom
//! properties, with a depth limit against reference cycles.
use std::collections::HashMap;
use std::hash::BuildHasher;

/// How many nested `var()` lookups to follow before giving up — guards
/// against a custom property that (directly or through a chain) references
/// itself, without needing true cycle detection.
const MAX_SUBSTITUTION_DEPTH: u8 = 8;

/// Substitutes `var(--name)` / `var(--name, fallback)` references in `value`
/// using `custom_properties`, resolving `var()` inside both the looked-up
/// value and any fallback.
///
/// A hand-written scanner rather than lightningcss's own token-level
/// substitution: it works on the same serialized text this adapter already
/// uses for CSS-wide keywords, so it needs no new dependency on
/// lightningcss's `substitute_variables`/
/// `into_owned` features or their `TokenList` ownership model. Uses
/// `str::get` throughout rather than range indexing — every boundary here
/// comes from `find`/`char_indices`, which only ever land on char
/// boundaries, but the indexing syntax can't express that guarantee.
#[must_use]
pub(crate) fn substitute_vars<S: BuildHasher>(
    value: &str,
    custom_properties: &HashMap<String, String, S>,
) -> String {
    substitute_vars_inner(value, custom_properties, 0)
}

/// One `var(` occurrence, scanned from the text that starts with it.
enum Occurrence<'text> {
    /// A complete `var(...)`: what sits between its parentheses, and the text after its `)`.
    Reference {
        /// The name and optional fallback, `--name, fallback`, untrimmed.
        inner: &'text str,
        /// The text after the closing `)`, still to scan.
        remainder: &'text str,
    },
    /// A `var(` with no matching `)`: the text from it to the end, kept verbatim.
    Unbalanced(&'text str),
    /// A boundary that would not slice, which `find` never produces: the scan stops there.
    Unsliceable,
}

/// `substitute_vars` at nesting `depth`: past `MAX_SUBSTITUTION_DEPTH` the value is kept
/// verbatim, otherwise every `var()` in it is replaced by its resolved value, or by nothing
/// when neither the property nor a fallback exists.
fn substitute_vars_inner<S: BuildHasher>(
    value: &str,
    custom_properties: &HashMap<String, String, S>,
    depth: u8,
) -> String {
    if depth >= MAX_SUBSTITUTION_DEPTH {
        return value.to_string();
    }
    let mut result = String::new();
    let mut rest = value;
    while let Some(start) = rest.find("var(") {
        let Some(before) = rest.get(..start) else {
            break;
        };
        result.push_str(before);
        match scan_occurrence(rest, start) {
            Occurrence::Reference { inner, remainder } => {
                let resolved = resolve_reference(inner, custom_properties, depth);
                result.push_str(&resolved.unwrap_or_default());
                rest = remainder;
            }
            Occurrence::Unbalanced(tail) => {
                result.push_str(tail);
                return result;
            }
            Occurrence::Unsliceable => break,
        }
    }
    result.push_str(rest);
    result
}

/// Splits the `var(` that starts at `start` in `rest` into its contents and what follows it.
fn scan_occurrence(rest: &str, start: usize) -> Occurrence<'_> {
    let Some(after) = rest.get(start.saturating_add(4)..) else {
        return Occurrence::Unsliceable;
    };
    let Some(end) = matching_close_paren(after) else {
        // Unbalanced parens: nothing sane to do, keep the rest verbatim.
        return Occurrence::Unbalanced(rest.get(start..).unwrap_or(""));
    };
    let Some(inner) = after.get(..end) else {
        return Occurrence::Unsliceable;
    };
    Occurrence::Reference {
        inner,
        remainder: after.get(end.saturating_add(1)..).unwrap_or(""),
    }
}

/// The value of the `var()` whose contents are `inner`: the named custom property, else the
/// fallback, each with its own `var()` references substituted one level deeper.
fn resolve_reference<S: BuildHasher>(
    inner: &str,
    custom_properties: &HashMap<String, String, S>,
    depth: u8,
) -> Option<String> {
    let (name, fallback) = inner.split_once(',').map_or_else(
        || (inner.trim(), None),
        |(var_name, fallback_value)| (var_name.trim(), Some(fallback_value.trim())),
    );
    let deeper = depth.saturating_add(1);
    custom_properties
        .get(name)
        .map(|property_value| substitute_vars_inner(property_value, custom_properties, deeper))
        .or_else(|| {
            fallback.map(|fallback_value| {
                substitute_vars_inner(fallback_value, custom_properties, deeper)
            })
        })
}

/// Index of the `)` matching the implicit `(` already consumed before
/// `text`, respecting nested parens (a fallback value can itself contain a
/// function call, e.g. `var(--x, rgb(0, 0, 0))`).
fn matching_close_paren(text: &str) -> Option<usize> {
    let mut depth = 1i32;
    text.char_indices().find_map(|(index, character)| {
        depth = match character {
            '(' => depth.saturating_add(1),
            ')' => depth.saturating_sub(1),
            _ => depth,
        };
        // Only a `)` lowers the depth, and it starts at 1, so it reaches 0 only on a `)`.
        (depth == 0).then_some(index)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
            .collect()
    }

    #[test]
    fn test_matching_close_paren_skips_nested_pairs() {
        assert_eq!(matching_close_paren("a)b)"), Some(1));
        assert_eq!(matching_close_paren("a(b)c)d"), Some(5));
        assert_eq!(matching_close_paren("((x))) "), Some(5));
        assert_eq!(matching_close_paren("a(b)c"), None);
        assert_eq!(matching_close_paren(""), None);
    }

    #[test]
    fn test_substitutes_simple_var() {
        let custom = vars(&[("--bg", "#1a1b26")]);
        assert_eq!(substitute_vars("var(--bg)", &custom), "#1a1b26");
    }

    #[test]
    fn test_falls_back_when_undefined() {
        let custom = vars(&[]);
        assert_eq!(substitute_vars("var(--bg, #000000)", &custom), "#000000");
    }

    #[test]
    fn test_fallback_with_nested_function_parens() {
        let custom = vars(&[]);
        assert_eq!(
            substitute_vars("var(--bg, rgb(0, 0, 0))", &custom),
            "rgb(0, 0, 0)"
        );
    }

    #[test]
    fn test_resolves_var_referencing_another_var() {
        let custom = vars(&[("--accent", "var(--bg)"), ("--bg", "#7aa2f7")]);
        assert_eq!(substitute_vars("var(--accent)", &custom), "#7aa2f7");
    }

    #[test]
    fn test_undefined_with_no_fallback_yields_nothing() {
        let custom = vars(&[]);
        assert_eq!(substitute_vars("var(--missing)", &custom), "");
    }

    #[test]
    fn test_surrounding_text_preserved() {
        let custom = vars(&[("--gap", "8px")]);
        assert_eq!(
            substitute_vars("0 var(--gap) 0 var(--gap)", &custom),
            "0 8px 0 8px"
        );
    }

    #[test]
    fn test_self_reference_terminates() {
        let custom = vars(&[("--loop", "var(--loop)")]);
        // Must not hang or overflow the stack; exact output isn't load-bearing.
        let _ = substitute_vars("var(--loop)", &custom);
    }

    #[test]
    fn test_unbalanced_reference_keeps_the_rest_verbatim() {
        let custom = vars(&[("--gap", "8px")]);
        assert_eq!(
            substitute_vars("var(--gap) 0 var(--gap 1", &custom),
            "8px 0 var(--gap 1"
        );
    }

    #[test]
    fn test_defined_value_wins_over_the_fallback() {
        let custom = vars(&[("--bg", "#111111")]);
        assert_eq!(substitute_vars("var(--bg, #000000)", &custom), "#111111");
    }

    #[test]
    fn test_name_and_fallback_are_trimmed() {
        let custom = vars(&[("--bg", "#111111")]);
        assert_eq!(substitute_vars("var( --bg )", &custom), "#111111");
        assert_eq!(substitute_vars("var(--none,  red )", &custom), "red");
    }

    #[test]
    fn test_var_inside_a_fallback_is_substituted() {
        let custom = vars(&[("--bg", "#111111")]);
        assert_eq!(
            substitute_vars("var(--missing, var(--bg))", &custom),
            "#111111"
        );
    }

    #[test]
    fn test_text_after_the_last_reference_is_kept() {
        let custom = vars(&[("--gap", "8px")]);
        assert_eq!(substitute_vars("a var(--gap) b", &custom), "a 8px b");
    }

    #[test]
    fn test_chain_stops_after_eight_lookups() {
        let names: Vec<String> = (0..=8).map(|index| format!("--v{index}")).collect();
        let mut custom = HashMap::new();
        for pair in names.windows(2) {
            custom.insert(pair[0].clone(), format!("var({})", pair[1]));
        }
        custom.insert("--v8".to_string(), "end".to_string());
        // `--v7`'s value is read at depth 8, where substitution stops and keeps it verbatim.
        assert_eq!(substitute_vars("var(--v0)", &custom), "var(--v8)");
        // One link shorter, the chain resolves to the end.
        assert_eq!(substitute_vars("var(--v1)", &custom), "end");
    }
}
