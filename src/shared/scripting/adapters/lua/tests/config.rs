//! The `config` table a Lua module reads: the bar's root settings and the module's options.
use super::support::text_of;
use crate::features::module_runtime::ports::AnyModulePort;
use crate::shared::config::domain::{Config, ModuleConfig};
use crate::shared::primitives::{DynamicValue, ModuleOptions, MonitorId};
use crate::shared::scripting::adapters::lua::LuaModule;

const CONFIG_SOURCE: &str = r#"
    seen = config.root.name .. "|" .. config.root.height .. "|" .. config.module.label .. "|"
        .. config.options.label .. "|" .. tostring(config.module == config.options)
    config = nil
    function render(monitor) return ui.text(seen) end
"#;

fn labelled_config() -> ModuleConfig {
    let options = std::collections::HashMap::from([(
        "label".to_string(),
        DynamicValue::String("hi".to_string()),
    )]);
    ModuleConfig::new("configured".into(), ModuleOptions::new(options))
}

#[test]
fn test_lua_config_holds_the_root_settings_and_the_module_options() {
    let full_config = Config::default();
    let mut module = LuaModule::new("configured".to_string(), CONFIG_SOURCE.to_string());
    module.init(&labelled_config(), &full_config).unwrap();
    let expected = format!(
        "{}|{}|hi|hi|true",
        full_config.root().name().as_str(),
        full_config.root().height().value()
    );
    assert_eq!(text_of(&module.render(&MonitorId::new("DP-1"))), expected);
}

#[test]
fn test_lua_config_is_created_again_when_the_script_removed_it() {
    let full_config = Config::default();
    let mut module = LuaModule::new("configured".to_string(), CONFIG_SOURCE.to_string());
    module.init(&labelled_config(), &full_config).unwrap();
    module.init(&labelled_config(), &full_config).unwrap();
    let rendered = text_of(&module.render(&MonitorId::new("DP-1")));
    assert!(rendered.ends_with("|hi|hi|true"), "{rendered}");
}

#[test]
fn test_lua_config_that_is_no_table_fails_the_next_init() {
    let mut module = LuaModule::new("configured".to_string(), "config = 5".to_string());
    module.init(&labelled_config(), &Config::default()).unwrap();
    let error = module.init(&labelled_config(), &Config::default());
    assert!(
        matches!(&error, Err(crate::features::module_runtime::ports::ModuleInitError::ScriptError(message)) if message.contains("integer to table")),
        "{error:?}"
    );
}
