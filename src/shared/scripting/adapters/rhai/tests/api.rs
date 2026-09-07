#[cfg(test)]
mod tests {
    use crate::features::module_runtime::ports::AnyModulePort;
    use crate::features::vdom::domain::UiAction;
    use crate::shared::events::core::PointerButton;
    use crate::shared::primitives::{MonitorId, ScriptMonitorInfo};
    use crate::shared::scripting::adapters::rhai::RhaiModule;

#[test]
fn test_rhai_cranky_namespaces_and_aliases() {
    let source = r#"
        fn init() {
            // cranky namespaces
            assert(cranky != ());
            assert(cranky.ui != ());
            assert(cranky.signals != ());
            assert(cranky.config != ());
            assert(cranky.sys != ());

            // aliases
            assert(ui != ());
            assert(signals != ());
            assert(config != ());
            assert(sys != ());
        }
        fn refresh() {}
        fn render(monitor) {
            let txt = ui.text("Namespace Test", "title");
            let btn = ui.element.text("Element Alias");
            return ui.flex([txt, btn]);
        }
    "#;
    let mut module = RhaiModule::new("test_ns".into(), source).unwrap();
    let mod_config = crate::shared::config::domain::ModuleConfig::new(
        "test_ns".into(),
        true,
        crate::shared::config::domain::EngineSelection::Auto,
        crate::shared::primitives::ModuleOptions::default(),
    );
    let config = crate::shared::config::domain::Config::default();
    assert!(module.init(&mod_config, &config).is_ok());
    let node = module.render(&MonitorId::new("DP-1"));
    assert_eq!(node.tag(), crate::features::vdom::domain::NodeTag::Flex);
    assert_eq!(node.children().len(), 2);
}

#[test]
fn test_rhai_ui_actions() {
    let source = r#"
        fn init() {}
        fn refresh() {}
        fn render(monitor) {
            let a1 = ui.action.exec("echo test");
            let a2 = ui.action.script_call("toggle_popup");
            let a3 = ui.action.systray(42, "Activate");

            return ui.flex(#{
                children: [
                    ui.text(#{ text: "Click", on_click: a1 }),
                    ui.text(#{ text: "Call", on_click: a2 }),
                    ui.text(#{ text: "Systray", on_click: a3 })
                ]
            });
        }
    "#;
    let module = RhaiModule::new("test_actions".into(), source).unwrap();
    let node = module.render(&MonitorId::new("DP-1"));
    assert_eq!(node.children().len(), 3);

    let click1 = node.children()[0].on_click().unwrap();
    assert_eq!(
        click1.get(&PointerButton::Left),
        Some(&UiAction::Exec("echo test".into()))
    );

    let click2 = node.children()[1].on_click().unwrap();
    assert_eq!(
        click2.get(&PointerButton::Left),
        Some(&UiAction::ScriptCall(
            crate::shared::primitives::FunctionName::new("toggle_popup")
        ))
    );

    let click3 = node.children()[2].on_click().unwrap();
    assert_eq!(
        click3.get(&PointerButton::Left),
        Some(&UiAction::SystrayAction {
            id: crate::features::systray::domain::SystrayId::new("42"),
            action: crate::features::systray::domain::SystrayActionName::parse_str("Activate"),
            pos: None
        })
    );
}

#[test]
fn test_rhai_rich_monitor_and_sys() {
    let source = r#"
        fn init() {}
        fn refresh() {}
        fn render(monitor) {
            // Assert monitor properties
            assert(monitor.id == "DP-1");
            assert(monitor.name == "DP-1");
            assert(monitor.width == 1920);
            assert(monitor.height == 1080);
            assert(monitor.scale == 1.5);
            assert(monitor.is_focused == true);
            assert(monitor.active_workspace_id == 1);
            assert(monitor.special_workspace_id == 2);
            assert(to_string(monitor) == "DP-1");

            // Assert sys
            assert(sys.hostname() != "");
            sys.log.info("Rendering on monitor " + monitor.name);

            let cur = sys.monitors.current(monitor);
            assert(cur.id == monitor.id);

            return ui.text("Mon: " + monitor.name);
        }
    "#;
    let mut module = RhaiModule::new("test_sys".into(), source).unwrap();
    let test_mon = ScriptMonitorInfo::new(
        MonitorId::new("DP-1"),
        "DP-1".to_string(),
        crate::shared::primitives::geometry::Size::new(1920, 1080),
        crate::shared::primitives::geometry::Scale::new(1.5),
        true,
        Some(1),
        Some(2),
    );
    module.cached_monitors = vec![test_mon];

    let node = module.render(&MonitorId::new("DP-1"));
    assert_eq!(node.tag(), crate::features::vdom::domain::NodeTag::Text);
}

#[test]
fn test_rhai_render_grid() {
    let source = r#"
        fn init() {}
        fn refresh() {}
        fn render(monitor) {
            return ui.grid([
                ui.text("Grid item 1"),
                ui.text("Grid item 2")
            ]);
        }
    "#;
    let module = RhaiModule::new("test_grid".into(), source).unwrap();
    let render_node = module.render(&MonitorId::new("DP-1"));
    assert_eq!(
        render_node.tag(),
        crate::features::vdom::domain::NodeTag::Grid
    );
    assert_eq!(render_node.children().len(), 2);
}

#[test]
fn test_rhai_vnode_with_popup_and_panel() {
    let source = r#"
        fn init() {}
        fn refresh() {}
        fn render(monitor) {
            let p_content = ui.text("Popup content");
            let pop = ui.popup(#{
                content: p_content,
                anchor: "bottom",
                offset: [5, 10],
                dismiss_on_unfocus: true,
            });

            let panel_content = ui.text("Panel content");
            let pan = ui.panel(#{
                content: panel_content,
                layer: "overlay",
                anchor: ["top", "right"],
                margin: #{ top: 15, right: 25 },
                exclusive_zone: 0,
                keyboard: "none",
            });

            return ui.rect(#{
                popup: pop,
                panel: pan,
            });
        }
    "#;
    let module = RhaiModule::new("test_pop_pan".into(), source).unwrap();
    let node = module.render(&MonitorId::new("DP-1"));
    assert!(node.popup().is_some());
    let popup = node.popup().unwrap();
    assert_eq!(popup.anchor_direction(), crate::features::vdom::domain::AnchorDirection::Bottom);
    assert_eq!(popup.offset().unwrap().dx(), 5);
    assert_eq!(popup.offset().unwrap().dy(), 10);
    assert!(popup.dismiss_on_unfocus());

    assert!(node.panel().is_some());
    let panel = node.panel().unwrap();
    assert_eq!(panel.layer(), crate::features::vdom::domain::PanelLayer::Overlay);
    assert!(panel.anchor().top());
    assert!(panel.anchor().right());
    assert!(!panel.anchor().bottom());
    assert_eq!(panel.margin().top().value(), 15);
    assert_eq!(panel.margin().right().value(), 25);
}

#[test]
fn test_rhai_vnode_with_popup_default_offset_none() {
    let source = r#"
        fn init() {}
        fn refresh() {}
        fn render(monitor) {
            let p_content = ui.text("Popup content");
            let pop = ui.popup(#{
                content: p_content,
                anchor: "bottom",
                dismiss_on_unfocus: true,
            });
            return ui.rect(#{
                popup: pop,
            });
        }
    "#;
    let module = RhaiModule::new("test_pop_none".into(), source).unwrap();
    let node = module.render(&MonitorId::new("DP-1"));
    assert!(node.popup().is_some());
    let popup = node.popup().unwrap();
    assert_eq!(popup.offset(), None);
}
}
