//! Tests that a whole `config.toml` reaches the domain `Config`, table by table.
use super::*;

const FULL_CONFIG: &str = r#"
    [root]
    name = "topbar"
    height = 42
    vertical_alignment = "bottom"
    margin = { top = 1, bottom = 2, left = 3, right = 4 }
    left = ["clock"]

    [root.unfocused]
    height = 24
    margin = { top = 5, bottom = 6, left = 7, right = 8 }

    [modules.clock]
    enable = false
    engine = "rhai"
    format = "%H"

    [rendering]
    mode = "immediate"
    fps_limit = 30

    [metrics]
    update_interval_ms = 2500

    [tooltip]
    padding = 9

    [popup]
    behavior = "per_monitor"
"#;

struct AnyFamily;

impl crate::shared::rendering::ports::font::FontValidatorPort for AnyFamily {
    fn is_valid_family(&self, _family: &str) -> bool {
        true
    }
}

fn parse_config(toml_str: &str) -> Result<crate::shared::config::domain::Config, toml::de::Error> {
    let dto: ConfigDto = toml::from_str(toml_str)?;
    Ok(dto.into_domain(&AnyFamily))
}

/// The `[top, bottom, left, right]` offsets of `margin`.
fn sides(margin: &crate::shared::config::domain::MarginConfig) -> [i32; 4] {
    [margin.top(), margin.bottom(), margin.left(), margin.right()]
        .map(crate::shared::config::domain::MarginOffset::value)
}

/// The module `name` of `config`, if it has one.
fn module<'config>(
    config: &'config crate::shared::config::domain::Config,
    name: &str,
) -> Option<&'config crate::shared::config::domain::ModuleConfig> {
    config
        .modules()
        .get(&crate::shared::primitives::ModuleName::new(name))
}

#[test]
fn config_dto_carries_every_table_into_the_domain() {
    let config = parse_config(FULL_CONFIG).unwrap();

    let root = config.root();
    assert_eq!(root.name().as_str(), "topbar");
    assert_eq!(root.height().value(), 42);
    assert_eq!(
        root.vertical_alignment(),
        crate::shared::config::domain::VerticalAlignment::Bottom
    );
    assert_eq!(sides(root.margin()), [1, 2, 3, 4]);
    assert!(root.options().get("left").is_some());
}

#[test]
fn config_dto_carries_the_unfocused_root_table_into_the_domain() {
    let config = parse_config(FULL_CONFIG).unwrap();

    let unfocused = config.root().as_unfocused();
    assert_eq!(unfocused.height().value(), 24);
    assert_eq!(sides(unfocused.margin()), [5, 6, 7, 8]);
}

#[test]
fn config_dto_carries_the_module_tables_into_the_domain() {
    let config = parse_config(FULL_CONFIG).unwrap();

    let clock = module(&config, "clock").unwrap();
    assert!(!clock.is_enabled());
    assert_eq!(
        clock
            .engine()
            .as_explicit()
            .map(crate::shared::config::domain::EngineId::as_str),
        Some("rhai")
    );
    assert!(clock.options().get("format").is_some());
}

#[test]
fn config_dto_carries_the_metrics_tooltip_and_popup_tables_into_the_domain() {
    let config = parse_config(FULL_CONFIG).unwrap();

    assert_eq!(config.metrics().update_interval_ms().value(), 2500);
    assert_eq!(
        config.tooltip().padding(),
        crate::shared::config::domain::PaddingOffset::new(9)
    );
    assert_eq!(
        config.popup().behavior(),
        crate::shared::config::domain::PopupBehavior::PerMonitor
    );
}

#[test]
fn config_dto_carries_the_rendering_table_into_the_domain() {
    let timebased = FULL_CONFIG.replace(
        "mode = \"immediate\"\n    fps_limit = 30",
        "mode = \"timebased\"",
    );
    assert_ne!(timebased, FULL_CONFIG);
    assert_ne!(
        parse_config(&timebased).unwrap(),
        parse_config(FULL_CONFIG).unwrap()
    );
}

/// One module whose `enable` is not a bool, and one with options only.
const LOOSE_MODULES: &str = r#"
    [modules.options_only]
    enable = "yes"
    format = "a"

    [modules.detailed]
    format = "b"
"#;

#[test]
fn config_dto_enables_modules_on_the_auto_engine_unless_told_otherwise() {
    let config = parse_config(LOOSE_MODULES).unwrap();

    for name in ["options_only", "detailed"] {
        let module = module(&config, name).unwrap();
        assert!(module.is_enabled(), "{name} should be enabled");
        assert_eq!(
            module.engine(),
            &crate::shared::config::domain::EngineSelection::Auto,
            "{name} should pick its engine automatically"
        );
        assert!(
            module.options().get("format").is_some(),
            "{name} keeps its options"
        );
    }
    let options_only = module(&config, "options_only").unwrap();
    assert!(
        options_only.options().get("enable").is_some(),
        "a non-bool enable lands among the options"
    );
}
