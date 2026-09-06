use super::api::register_vdom_dsl;
use crate::shared::dbus::domain::DBusSubscription;
use crate::shared::events::signals::SignalKind;
use mlua::Lua;
use std::sync::Mutex;

#[cfg(test)]
pub struct LuaScriptLoader;

#[cfg(test)]
impl LuaScriptLoader {
    #[must_use]
    pub fn load_built_in(name: &str) -> Option<String> {
        match name {
            "clock" => Some(include_str!("../../../../../assets/widgets/clock.lua").to_string()),
            "calendar" => {
                Some(include_str!("../../../../../assets/widgets/calendar.lua").to_string())
            }
            "workspace" => {
                Some(include_str!("../../../../../assets/widgets/workspace.lua").to_string())
            }
            "systray" => {
                Some(include_str!("../../../../../assets/widgets/systray.lua").to_string())
            }
            "metrics" => {
                Some(include_str!("../../../../../assets/widgets/metrics.lua").to_string())
            }
            _ => None,
        }
    }
}

pub struct LuaModule {
    pub(crate) lua: Mutex<Lua>,
    pub(crate) source: String,
    pub(crate) name: String,
    pub(crate) cached_subs: Vec<SignalKind>,
    pub(crate) cached_dbus_subs: Vec<DBusSubscription>,
    pub(crate) cached_styles: Vec<crate::features::styling::domain::StyleSheetName>,
}

impl LuaModule {
    #[must_use]
    pub fn new(name: String, source: String) -> Self {
        let lua = Lua::new();
        let _ = register_vdom_dsl(&lua);
        Self {
            lua: Mutex::new(lua),
            source,
            name,
            cached_subs: Vec::new(),
            cached_dbus_subs: Vec::new(),
            cached_styles: Vec::new(),
        }
    }

    #[cfg(test)]
    #[must_use]
    pub fn built_in(name: &str) -> Option<Self> {
        LuaScriptLoader::load_built_in(name).map(|source| Self::new(name.to_string(), source))
    }
}
