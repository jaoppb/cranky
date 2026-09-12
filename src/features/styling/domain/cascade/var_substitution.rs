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
/// uses for CSS-wide keywords and the gradient-border extension, so it
/// needs no new dependency on lightningcss's `substitute_variables`/
/// `into_owned` features or their `TokenList` ownership model. Uses
/// `str::get` throughout rather than range indexing — every boundary here
/// comes from `find`/`char_indices`, which only ever land on char
/// boundaries, but the indexing syntax can't express that guarantee.
#[must_use]
pub fn substitute_vars<S: BuildHasher>(
    value: &str,
    custom_properties: &HashMap<String, String, S>,
) -> String {
    substitute_vars_inner(value, custom_properties, 0)
}

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
        let Some(after) = rest.get(start.saturating_add(4)..) else {
            break;
        };
        let Some(end) = matching_close_paren(after) else {
            // Unbalanced parens: nothing sane to do, keep the rest verbatim.
            if let Some(tail) = rest.get(start..) {
                result.push_str(tail);
            }
            return result;
        };
        let Some(inner) = after.get(..end) else {
            break;
        };
        let (name, fallback) = inner
            .split_once(',')
            .map_or_else(|| (inner.trim(), None), |(n, f)| (n.trim(), Some(f.trim())));

        let resolved = custom_properties
            .get(name)
            .map(|v| substitute_vars_inner(v, custom_properties, depth.saturating_add(1)))
            .or_else(|| {
                fallback
                    .map(|f| substitute_vars_inner(f, custom_properties, depth.saturating_add(1)))
            });
        if let Some(resolved) = resolved {
            result.push_str(&resolved);
        }
        rest = after.get(end.saturating_add(1)..).unwrap_or("");
    }
    result.push_str(rest);
    result
}

/// Index of the `)` matching the implicit `(` already consumed before
/// `s`, respecting nested parens (a fallback value can itself contain a
/// function call, e.g. `var(--x, rgb(0, 0, 0))`).
fn matching_close_paren(s: &str) -> Option<usize> {
    let mut depth = 1i32;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth = depth.saturating_add(1),
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
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
}
