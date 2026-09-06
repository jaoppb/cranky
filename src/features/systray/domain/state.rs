use super::identifiers::SystrayId;
use super::item::SystrayItem;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SystrayState {
    items: std::collections::BTreeMap<SystrayId, SystrayItem>,
}

impl serde::Serialize for SystrayState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SystrayState", 1)?;
        let items: Vec<&SystrayItem> = self.items.values().collect();
        state.serialize_field("items", &items)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for SystrayState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct Helper {
            items: Vec<SystrayItem>,
        }
        let helper = Helper::deserialize(deserializer)?;
        let mut items = std::collections::BTreeMap::new();
        for item in helper.items {
            items.insert(item.id().clone(), item);
        }
        Ok(Self { items })
    }
}

impl SystrayState {
    #[must_use]
    pub const fn new(items: std::collections::BTreeMap<SystrayId, SystrayItem>) -> Self {
        Self { items }
    }

    #[must_use]
    pub const fn items(&self) -> &std::collections::BTreeMap<SystrayId, SystrayItem> {
        &self.items
    }
}
