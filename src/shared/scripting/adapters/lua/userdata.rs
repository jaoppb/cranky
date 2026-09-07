use crate::features::vdom::domain::{PanelSpec, PopupSpec, VNode};
use crate::shared::primitives::ScriptMonitorInfo;
use mlua::{UserData, UserDataFields, UserDataMethods};

#[derive(Clone)]
pub struct LuaVNode(pub VNode);

impl UserData for LuaVNode {}

#[derive(Clone)]
pub struct LuaPopup(pub PopupSpec);

impl UserData for LuaPopup {}

#[derive(Clone)]
pub struct LuaPanel(pub PanelSpec);

impl UserData for LuaPanel {}

#[derive(Clone)]
pub struct LuaMonitor(pub ScriptMonitorInfo);

impl UserData for LuaMonitor {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("id", |_, this| Ok(this.0.id().as_str().to_string()));
        fields.add_field_method_get("name", |_, this| {
            let name = this.0.name();
            if name.is_empty() {
                Ok(this.0.id().as_str().to_string())
            } else {
                Ok(name.to_string())
            }
        });
        fields.add_field_method_get("width", |_, this| Ok(this.0.size().width()));
        fields.add_field_method_get("height", |_, this| Ok(this.0.size().height()));
        fields.add_field_method_get("scale", |_, this| Ok(this.0.scale().value()));
        fields.add_field_method_get("is_focused", |_, this| Ok(this.0.is_focused()));
        fields.add_field_method_get("active_workspace_id", |_, this| {
            Ok(this.0.active_workspace_id())
        });
        fields.add_field_method_get("special_workspace_id", |_, this| {
            Ok(this.0.special_workspace_id())
        });
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, this, ()| {
            Ok(this.0.id().as_str().to_string())
        });
    }
}
