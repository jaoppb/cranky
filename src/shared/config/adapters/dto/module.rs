use std::collections::HashMap;

use serde::Deserialize;

use crate::shared::config::domain;
use crate::shared::primitives::ModuleName;

use super::helpers::{default_true, json_map_to_options};

#[derive(Debug, Deserialize, Default)]
pub struct ModulesConfigDto {
    #[serde(flatten)]
    modules: HashMap<String, ModuleEntryDto>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ModuleEntryDto {
    Detailed {
        #[serde(default = "default_true")]
        enable: bool,
        #[serde(default)]
        engine: Option<String>,
        #[serde(flatten)]
        options: HashMap<String, serde_json::Value>,
    },
    OptionsOnly(HashMap<String, serde_json::Value>),
}

impl ModulesConfigDto {
    #[must_use]
    pub fn into_domain(self) -> domain::ModulesConfig {
        let mut map = HashMap::new();
        for (name_str, entry) in self.modules {
            let mod_name = ModuleName::new(name_str);
            let mod_cfg = match entry {
                ModuleEntryDto::Detailed {
                    enable,
                    engine,
                    options,
                } => {
                    let selection = engine.map_or(domain::EngineSelection::Auto, |e| {
                        domain::EngineSelection::Explicit(domain::EngineId::new(e))
                    });
                    domain::ModuleConfig::new(
                        mod_name.clone(),
                        enable,
                        selection,
                        json_map_to_options(options),
                    )
                }
                ModuleEntryDto::OptionsOnly(options) => domain::ModuleConfig::new(
                    mod_name.clone(),
                    true,
                    domain::EngineSelection::Auto,
                    json_map_to_options(options),
                ),
            };
            map.insert(mod_name, mod_cfg);
        }
        domain::ModulesConfig::new(map)
    }
}
