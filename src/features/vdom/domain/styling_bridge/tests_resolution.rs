//! Pins what the styling pass hands the resolver for each node (its tag, classes, parent query,
//! sibling position, emptiness and inherited style), the order it asks in, and how the matched
//! style is laid over the kind's default.
#[cfg(test)]
mod tests {
    use crate::features::styling::domain::{
        ClassNameList, ComputedStyle, DisplayMode, ElementId, ElementQuery, GridAutoFlow,
        InheritedStyle,
    };
    use crate::features::styling::ports::StyleResolverPort;
    use crate::features::vdom::domain::{
        ElementSelectors, Interactions, PanelSpec, PopupSpec, TextContent, VNode,
    };
    use crate::shared::primitives::DrawingColor;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// What the resolver saw of one element: its id, tag, classes, parent's id, sibling position
    /// and the color it inherited.
    #[derive(Debug, Clone, PartialEq)]
    struct Seen {
        id: String,
        tag: String,
        classes: Vec<String>,
        parent: Option<String>,
        position: (usize, usize),
        inherited_color: Option<DrawingColor>,
    }

    /// Records every element it resolves, in call order, and answers each id with the style the
    /// test configured for it; what that element's children inherit is derived from that style.
    struct ProbeResolver {
        matched: HashMap<String, ComputedStyle>,
        seen: Mutex<Vec<Seen>>,
        empty: Mutex<Vec<String>>,
    }

    impl ProbeResolver {
        fn new(matched: HashMap<String, ComputedStyle>) -> Self {
            Self {
                matched,
                seen: Mutex::new(Vec::new()),
                empty: Mutex::new(Vec::new()),
            }
        }

        fn seen(&self) -> Vec<Seen> {
            self.seen.lock().unwrap().clone()
        }

        /// The `(id, parent id)` of every element resolved, in call order.
        fn ids_and_parents(&self) -> Vec<(String, Option<String>)> {
            let seen = self.seen();
            seen.into_iter()
                .map(|probe| (probe.id, probe.parent))
                .collect()
        }
    }

    /// What `query` shows of its element.
    fn seen_of(query: &ElementQuery<'_>, inherited: &InheritedStyle) -> Seen {
        let id_of = |query: &ElementQuery<'_>| query.id().map(|id| id.as_str().to_string());
        Seen {
            id: id_of(query).unwrap_or_default(),
            tag: query.tag().to_string(),
            classes: query
                .classes()
                .iter()
                .map(|class| class.as_str().to_string())
                .collect(),
            parent: query.parent().and_then(id_of),
            position: (query.child_index(), query.total_children()),
            inherited_color: inherited.color().cloned(),
        }
    }

    impl StyleResolverPort for ProbeResolver {
        fn resolve_style(
            &self,
            query: &ElementQuery<'_>,
            inherited: &InheritedStyle,
        ) -> (ComputedStyle, InheritedStyle) {
            let seen = seen_of(query, inherited);
            let empty_id = query.is_empty().then(|| seen.id.clone());
            self.empty.lock().unwrap().extend(empty_id);
            let style = self.matched.get(&seen.id).cloned().unwrap_or_default();
            self.seen.lock().unwrap().push(seen);
            let child_inherited = inherited.descend(&style, Arc::new(HashMap::new()));
            (style, child_inherited)
        }
    }

    fn selectors(id: &str) -> ElementSelectors {
        ElementSelectors::new(None, Some(ElementId::new(id).unwrap()))
    }

    fn text(id: &str, content: &str) -> VNode {
        let content = TextContent::new(content.to_string());
        VNode::new_text(content, selectors(id), Interactions::default())
    }

    fn color(hex: &str) -> DrawingColor {
        DrawingColor::parse(hex).unwrap()
    }

    /// A style that only sets the color to `hex`.
    fn colored(hex: &str) -> ComputedStyle {
        let mut style = ComputedStyle::default();
        style.set_color(color(hex));
        style
    }

    /// `#root.bar` (red) with a `#tip` tooltip, a `#pop` popup and a `#pan` panel, over three
    /// children: `#first` (blue, holding `#grand`), the blank `#second` and the `#third` rect.
    fn fixture() -> VNode {
        let first = VNode::new_flex(
            vec![text("grand", "g")],
            selectors("first"),
            Interactions::default(),
        );
        let third = VNode::new_rect(selectors("third"), Interactions::default());
        let root_selectors = ElementSelectors::new(
            Some(ClassNameList::parse("bar").unwrap()),
            Some(ElementId::new("root").unwrap()),
        );
        let tooltip = Some(Box::new(text("tip", "t")));
        VNode::new_flex(
            vec![first, text("second", "  "), third],
            root_selectors,
            Interactions::new(None, None, tooltip),
        )
        .with_popup(PopupSpec::new(Box::new(text("pop", "p"))))
        .with_panel(PanelSpec::new(Box::new(text("pan", "q"))))
    }

    /// Resolves `fixture()` with `#root` red and `#first` blue.
    fn resolve_fixture() -> ProbeResolver {
        let matched = HashMap::from([
            ("root".to_string(), colored("#ff0000")),
            ("first".to_string(), colored("#0000ff")),
        ]);
        let resolver = ProbeResolver::new(matched);
        let _ = fixture().resolve_styles(&resolver, None);
        resolver
    }

    /// The `(id, value)` pairs of `field` for every element resolved, in call order.
    fn project<T>(resolver: &ProbeResolver, field: impl Fn(&Seen) -> T) -> Vec<(String, T)> {
        let seen = resolver.seen();
        seen.iter()
            .map(|probe| (probe.id.clone(), field(probe)))
            .collect()
    }

    fn owned(pairs: &[(&str, Option<&str>)]) -> Vec<(String, Option<String>)> {
        let own =
            |(id, parent): &(&str, Option<&str>)| (id.to_string(), parent.map(str::to_string));
        pairs.iter().map(own).collect()
    }

    #[test]
    fn test_the_owner_resolves_before_its_floating_contents_and_then_its_children() {
        let resolver = resolve_fixture();
        let expected = owned(&[
            ("root", None),
            ("tip", None),
            ("pop", Some("root")),
            ("pan", Some("root")),
            ("first", Some("root")),
            ("grand", Some("first")),
            ("second", Some("root")),
            ("third", Some("root")),
        ]);
        assert_eq!(resolver.ids_and_parents(), expected);
    }

    #[test]
    fn test_each_child_sees_its_index_among_its_siblings_and_each_root_is_an_only_child() {
        let positions = project(&resolve_fixture(), |probe| probe.position);
        let expected: Vec<(String, (usize, usize))> = [
            ("root", (0, 1)),
            ("tip", (0, 1)),
            ("pop", (0, 1)),
            ("pan", (0, 1)),
            ("first", (0, 3)),
            ("grand", (0, 1)),
            ("second", (1, 3)),
            ("third", (2, 3)),
        ]
        .map(|(id, position)| (id.to_string(), position))
        .into();
        assert_eq!(positions, expected);
    }

    #[test]
    fn test_children_inherit_from_their_parent_and_floating_roots_inherit_nothing() {
        let inherited = project(&resolve_fixture(), |probe| probe.inherited_color.clone());
        let (red, blue) = (Some(color("#ff0000")), Some(color("#0000ff")));
        let expected: Vec<(String, Option<DrawingColor>)> = [
            ("root", None),
            ("tip", None),
            ("pop", None),
            ("pan", None),
            ("first", red.clone()),
            ("grand", blue),
            ("second", red.clone()),
            ("third", red),
        ]
        .map(|(id, inherited)| (id.to_string(), inherited))
        .into();
        assert_eq!(inherited, expected);
    }

    #[test]
    fn test_floating_contents_of_a_child_with_an_inherited_style_inherit_nothing() {
        let tooltip = Some(Box::new(text("tip", "t")));
        let owner = VNode::new_text(
            TextContent::new("o".to_string()),
            selectors("owner"),
            Interactions::new(None, None, tooltip),
        )
        .with_popup(PopupSpec::new(Box::new(text("pop", "p"))))
        .with_panel(PanelSpec::new(Box::new(text("pan", "q"))));
        let root = VNode::new_flex(vec![owner], selectors("root"), Interactions::default());
        let resolver =
            ProbeResolver::new(HashMap::from([("root".to_string(), colored("#ff0000"))]));
        let _ = root.resolve_styles(&resolver, None);
        let inherited = project(&resolver, |probe| probe.inherited_color.clone());
        let expected: Vec<(String, Option<DrawingColor>)> = [
            ("root", None),
            ("owner", Some(color("#ff0000"))),
            ("tip", None),
            ("pop", None),
            ("pan", None),
        ]
        .map(|(id, inherited)| (id.to_string(), inherited))
        .into();
        assert_eq!(inherited, expected);
    }

    #[test]
    fn test_the_query_carries_the_tag_classes_and_emptiness_of_the_node() {
        let resolver = resolve_fixture();
        let tags = project(&resolver, |probe| {
            (probe.tag.clone(), probe.classes.clone())
        });
        let tag = |id: &str| {
            tags.iter()
                .find(|(seen, _)| seen == id)
                .map(|(_, tag)| tag.clone())
        };
        assert_eq!(
            tag("root"),
            Some(("flex".to_string(), vec!["bar".to_string()]))
        );
        assert_eq!(tag("grand"), Some(("text".to_string(), Vec::new())));
        assert_eq!(tag("third"), Some(("rect".to_string(), Vec::new())));
        assert_eq!(*resolver.empty.lock().unwrap(), vec!["second".to_string()]);
    }

    #[test]
    fn test_the_matched_style_is_laid_over_the_default_of_the_node_kind() {
        let mut grid_override = colored("#ff0000");
        grid_override.set_display(DisplayMode::Flex);
        let matched = HashMap::from([
            ("root".to_string(), colored("#ff0000")),
            ("grid".to_string(), grid_override),
        ]);
        let resolver = ProbeResolver::new(matched);
        let grid = VNode::new_grid(Vec::new(), selectors("grid"), Interactions::default());
        let root = VNode::new_flex(vec![grid], selectors("root"), Interactions::default());
        let styled = root.resolve_styles(&resolver, None);

        let mut expected_root = ComputedStyle::default_for_flex();
        expected_root.set_color(color("#ff0000"));
        assert_eq!(styled.style(), &expected_root);
        let grid_style = styled.children()[0].style();
        let layout = (
            grid_style.display(),
            grid_style.grid_auto_flow(),
            grid_style.color(),
        );
        let expected = (
            Some(DisplayMode::Flex),
            Some(GridAutoFlow::Row),
            Some(&color("#ff0000")),
        );
        assert_eq!(layout, expected);
    }
}
