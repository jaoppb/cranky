#[cfg(test)]
mod tests {
    use crate::features::vdom::domain::UiAction;
    use crate::features::vdom::domain::{NodeTag, VNode};
    use crate::shared::events::core::PointerButton;
    use crate::shared::scripting::adapters::lua::{register_cranky_api, value_to_vnode};
    use mlua::Lua;

    /// The tags of the children of `node`, in order.
    fn child_tags(node: &VNode) -> Vec<NodeTag> {
        node.children().iter().map(VNode::tag).collect()
    }

    const CONSTRUCTORS_SCRIPT: &str = r#"
        local txt = ui.text({ text = "hello", class = "greeting" })
        local rect = ui.rect({ class = "box" })
        local prog = ui.progress({ value = 0.75, orientation = "vertical" })
        local flex = ui.flex({
            class = "root",
            children = { txt, rect, prog }
        })
        return flex
    "#;

    #[test]
    fn test_vdom_dsl_constructors() {
        let lua = Lua::new();
        register_cranky_api(&lua).expect("DSL registration failed");

        let val = lua
            .load(CONSTRUCTORS_SCRIPT)
            .eval::<mlua::Value>()
            .expect("Eval failed");
        let vnode = value_to_vnode(&lua, val).expect("Conversion failed");
        assert_eq!(vnode.tag(), crate::features::vdom::domain::NodeTag::Flex);
        assert_eq!(
            child_tags(&vnode),
            [NodeTag::Text, NodeTag::Rect, NodeTag::Progress]
        );
    }

    const SINGLE_CLICK_SCRIPT: &str = r#"
        return ui.text({
            text = "click me",
            on_click = { Exec = "echo single" }
        })
    "#;

    const MULTI_CLICK_SCRIPT: &str = r#"
        return ui.text({
            text = "multi click",
            on_click = {
                left = { Exec = "echo left" },
                right = { Exec = "echo right" },
                side = { Exec = "echo side" },
                [276] = { Exec = "echo extra" }
            }
        })
    "#;

    #[test]
    fn test_lua_vnode_single_click_handler_binds_every_button() {
        let lua = Lua::new();
        register_cranky_api(&lua).expect("DSL registration failed");

        let single_val = lua.load(SINGLE_CLICK_SCRIPT).eval::<mlua::Value>().unwrap();
        let single_node = value_to_vnode(&lua, single_val).unwrap();
        let single_handlers = single_node.on_click().expect("on_click expected");
        assert_eq!(
            single_handlers.get(PointerButton::Left),
            Some(&UiAction::Exec("echo single".into()))
        );
        assert_eq!(
            single_handlers.get(PointerButton::Right),
            Some(&UiAction::Exec("echo single".into()))
        );
        assert_eq!(
            single_handlers.get(PointerButton::Middle),
            Some(&UiAction::Exec("echo single".into()))
        );
    }

    #[test]
    fn test_lua_vnode_click_handlers_per_button() {
        let lua = Lua::new();
        register_cranky_api(&lua).expect("DSL registration failed");

        let multi_val = lua.load(MULTI_CLICK_SCRIPT).eval::<mlua::Value>().unwrap();
        let multi_node = value_to_vnode(&lua, multi_val).unwrap();
        let multi_handlers = multi_node.on_click().expect("on_click expected");
        assert_eq!(
            multi_handlers.get(PointerButton::Left),
            Some(&UiAction::Exec("echo left".into()))
        );
        assert_eq!(
            multi_handlers.get(PointerButton::Right),
            Some(&UiAction::Exec("echo right".into()))
        );
        assert_eq!(
            multi_handlers.get(PointerButton::Side),
            Some(&UiAction::Exec("echo side".into()))
        );
        assert_eq!(
            multi_handlers.get(PointerButton::Extra),
            Some(&UiAction::Exec("echo extra".into()))
        );
        assert_eq!(multi_handlers.get(PointerButton::Middle), None);
    }

    const POPUP_SCRIPT: &str = r#"
        return ui.flex({
            children = {
                ui.text({
                    text = "Show Popup",
                    popup = ui.popup({
                        anchor = "bottom",
                        offset = { x = 0, y = 8 },
                        content = ui.flex({
                            class = "popup-menu",
                            children = {
                                ui.text({ text = "Item 1" })
                            }
                        })
                    })
                })
            }
        })
    "#;

    #[test]
    fn test_lua_vnode_with_popup() {
        let lua = Lua::new();
        register_cranky_api(&lua).expect("DSL registration failed");

        let val = lua.load(POPUP_SCRIPT).eval::<mlua::Value>().unwrap();
        let node = value_to_vnode(&lua, val).unwrap();
        assert_eq!(node.children().len(), 1);
        let button_node = &node.children()[0];
        assert!(button_node.popup().is_some());
        let popup = button_node.popup().unwrap();
        assert_eq!(popup.content().children().len(), 1);
        assert_eq!(
            popup.content().tag(),
            crate::features::vdom::domain::NodeTag::Flex
        );
        assert_eq!(
            popup.anchor_direction(),
            crate::features::vdom::domain::AnchorDirection::Bottom
        );
        assert_eq!(
            popup.offset(),
            Some(crate::features::vdom::domain::PopupOffset::new(0, 8))
        );
    }

    #[test]
    fn test_lua_vnode_with_popup_default_offset_none() {
        let lua = Lua::new();
        register_cranky_api(&lua).expect("DSL registration failed");

        let script = r#"
        return ui.text({
            text = "Trigger",
            popup = ui.popup({
                content = ui.text({ text = "Popup Content" })
            })
        })
    "#;
        let val = lua.load(script).eval::<mlua::Value>().unwrap();
        let node = value_to_vnode(&lua, val).unwrap();
        assert!(node.popup().is_some());
        let popup = node.popup().unwrap();
        assert_eq!(popup.offset(), None);
    }

    const PANEL_SCRIPT: &str = r#"
        return ui.text({
            text = "Panel Trigger",
            panel = ui.panel({
                layer = "overlay",
                anchor = { "top", "right" },
                margin = { top = 10, right = 20 },
                exclusive_zone = 0,
                content = ui.flex({
                    class = "control-center",
                    children = {
                        ui.text({ text = "Volume" })
                    }
                })
            })
        })
    "#;

    #[test]
    fn test_lua_vnode_with_panel() {
        let lua = Lua::new();
        register_cranky_api(&lua).expect("DSL registration failed");

        let val = lua.load(PANEL_SCRIPT).eval::<mlua::Value>().unwrap();
        let node = value_to_vnode(&lua, val).unwrap();
        assert!(node.panel().is_some());
        let panel = node.panel().unwrap();
        assert_eq!(
            panel.layer(),
            crate::features::vdom::domain::PanelLayer::Overlay
        );
        let anchor = panel.anchor();
        assert_eq!(
            (anchor.top(), anchor.right(), anchor.bottom()),
            (true, true, false)
        );
        let margin = panel.margin();
        assert_eq!((margin.top().value(), margin.right().value()), (10, 20));
        assert_eq!(panel.content().children().len(), 1);
    }

    /// Whether a panel is anchored to its top, bottom, left and right edges.
    type Edges = (bool, bool, bool, bool);

    /// Panel `anchor` values with the (top, bottom, left, right) edges each anchors.
    const PANEL_ANCHOR_CASES: [(&str, Edges); 9] = [
        (r#"{ "bottom", "left" }"#, (false, true, true, false)),
        (
            r#"{ "TOP", "right", "middle" }"#,
            (true, false, false, true),
        ),
        (
            r#"{ "top", "bottom", "left", "right" }"#,
            (true, true, true, true),
        ),
        (
            r#"{ "bottom", "top", "right", "left" }"#,
            (true, true, true, true),
        ),
        (r#""top""#, (true, false, false, false)),
        (r#""Bottom""#, (false, true, false, false)),
        (r#""left""#, (false, false, true, false)),
        (r#""right""#, (false, false, false, true)),
        (r#""middle""#, (false, false, false, false)),
    ];

    /// Every edge a panel's `anchor` names, as a list or as one string, in
    /// any case, and only those: unknown names anchor nothing.
    #[test]
    fn test_lua_panel_anchor_edges() {
        let lua = Lua::new();
        register_cranky_api(&lua).expect("DSL registration failed");
        for (anchor, expected) in PANEL_ANCHOR_CASES {
            let script = format!(
                "return ui.text({{ text = \"t\", panel = ui.panel({{ anchor = {anchor}, content = ui.text({{ text = \"p\" }}) }}) }})"
            );
            let val = lua.load(script).eval::<mlua::Value>().unwrap();
            let node = value_to_vnode(&lua, val).unwrap();
            let edges = node.panel().unwrap().anchor();
            assert_eq!(
                (edges.top(), edges.bottom(), edges.left(), edges.right()),
                expected,
                "{anchor}"
            );
        }
    }

    #[test]
    fn test_lua_vnode_grid() {
        let lua = Lua::new();
        register_cranky_api(&lua).expect("DSL registration failed");

        let script = r#"
        return ui.grid({
            class = "my-grid",
            children = {
                ui.text({ text = "Cell 1" }),
                ui.text({ text = "Cell 2" })
            }
        })
    "#;
        let val = lua.load(script).eval::<mlua::Value>().unwrap();
        let node = value_to_vnode(&lua, val).unwrap();
        assert_eq!(node.tag(), crate::features::vdom::domain::NodeTag::Grid);
        assert_eq!(node.children().len(), 2);
    }

    const SHORTHAND_SCRIPT: &str = r#"
        local t = ui.text("Hello World", "greeting-cls")
        local p = ui.progress(0.75, "vertical", "cpu-bar")
        local r = ui.rect("separator")
        local m = ui.module("clock")
        local root = ui.flex({ t, p, r, m })
        return root
    "#;

    #[test]
    fn test_lua_shorthand_dsl() {
        let lua = Lua::new();
        register_cranky_api(&lua).expect("Registration failed");

        let val = lua.load(SHORTHAND_SCRIPT).eval::<mlua::Value>().unwrap();
        let vnode = value_to_vnode(&lua, val).unwrap();
        assert_eq!(vnode.tag(), crate::features::vdom::domain::NodeTag::Flex);
        assert_eq!(
            child_tags(&vnode),
            [
                NodeTag::Text,
                NodeTag::Progress,
                NodeTag::Rect,
                NodeTag::Module
            ]
        );
    }
    #[test]
    fn test_lua_vnode_key_is_reconciliation_identity() {
        let lua = Lua::new();
        register_cranky_api(&lua).expect("DSL registration failed");

        // `key` is distinct from `id`: both may be set, and either may be set
        // alone, since one is a CSS selector and the other is reconciliation
        // identity (see `NodeKey`).
        let script = r#"
        return ui.flex({
            key = "ws-3",
            id = "ws-3",
            children = {}
        })
    "#;
        let val = lua.load(script).eval::<mlua::Value>().expect("Eval failed");
        let vnode = value_to_vnode(&lua, val).expect("Conversion failed");
        assert_eq!(
            vnode
                .key()
                .map(crate::features::vdom::domain::NodeKey::as_str),
            Some("ws-3")
        );
        assert_eq!(
            vnode
                .element_id()
                .map(crate::features::styling::domain::ElementId::as_str),
            Some("ws-3")
        );
    }

    #[test]
    fn test_lua_vnode_without_key_has_none() {
        let lua = Lua::new();
        register_cranky_api(&lua).expect("DSL registration failed");

        let val = lua
            .load(r#"return ui.rect("box")"#)
            .eval::<mlua::Value>()
            .expect("Eval failed");
        let vnode = value_to_vnode(&lua, val).expect("Conversion failed");
        assert_eq!(vnode.key(), None);
    }

    /// The value, orientation and class names of the progress node `script` returns.
    fn progress_of(
        script: &str,
    ) -> (
        f32,
        crate::features::styling::domain::Orientation,
        Vec<String>,
    ) {
        let lua = Lua::new();
        register_cranky_api(&lua).expect("DSL registration failed");
        let val = lua.load(script).eval::<mlua::Value>().expect("Eval failed");
        let vnode = value_to_vnode(&lua, val).expect("Conversion failed");
        let crate::features::vdom::domain::kind::VNodeKind::Progress { value, orientation } =
            vnode.kind()
        else {
            panic!("expected a progress node, got {:?}", vnode.kind());
        };
        let classes = vnode.class_names().map_or_else(Vec::new, |list| {
            list.iter().map(|name| name.as_str().to_string()).collect()
        });
        (value.value(), *orientation, classes)
    }

    #[test]
    fn test_lua_progress_table_form_reads_its_own_keys() {
        use crate::features::styling::domain::Orientation;
        let (value, orientation, classes) = progress_of(
            r#"return ui.progress({ value = 0.25, orientation = "Vertical", class = "cpu" })"#,
        );
        assert!((value - 0.25).abs() < f32::EPSILON);
        assert_eq!(orientation, Orientation::Vertical);
        assert_eq!(classes, ["cpu"]);

        let (value, orientation, classes) = progress_of("return ui.progress({})");
        assert!(value.abs() < f32::EPSILON);
        assert_eq!(orientation, Orientation::Horizontal);
        assert_eq!(classes, Vec::<String>::new());
    }

    /// A positional `ui.progress` call with the value, orientation and classes of its node.
    type PositionalProgress = (
        &'static str,
        f32,
        crate::features::styling::domain::Orientation,
        &'static [&'static str],
    );

    const POSITIONAL_PROGRESS_CASES: [PositionalProgress; 4] = {
        use crate::features::styling::domain::Orientation;
        [
            (
                r#"return ui.progress(0.75, "VERTICAL", "cpu-bar")"#,
                0.75,
                Orientation::Vertical,
                &["cpu-bar"],
            ),
            ("return ui.progress(0.5)", 0.5, Orientation::Horizontal, &[]),
            (
                r#"return ui.progress(1, "vertical", "full")"#,
                1.0,
                Orientation::Vertical,
                &["full"],
            ),
            (
                r#"return ui.progress(0, "sideways", "empty")"#,
                0.0,
                Orientation::Horizontal,
                &["empty"],
            ),
        ]
    };

    #[test]
    fn test_lua_progress_positional_form_reads_orientation_and_class() {
        for (script, expected_value, expected_orientation, expected_classes) in
            POSITIONAL_PROGRESS_CASES
        {
            let (value, orientation, classes) = progress_of(script);
            assert!((value - expected_value).abs() < f32::EPSILON, "{script}");
            assert_eq!(orientation, expected_orientation, "{script}");
            assert_eq!(classes, expected_classes, "{script}");
        }
    }

    #[test]
    fn test_lua_progress_of_another_value_is_an_empty_horizontal_bar() {
        use crate::features::styling::domain::Orientation;
        let (value, orientation, classes) =
            progress_of(r#"return ui.progress("full", "vertical", "ignored")"#);
        assert!(value.abs() < f32::EPSILON);
        assert_eq!(orientation, Orientation::Horizontal);
        assert_eq!(classes, Vec::<String>::new());
    }
}
