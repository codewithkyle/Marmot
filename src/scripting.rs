use std::{cell::RefCell, rc::Rc};

use mlua::{Lua, UserData, UserDataFields, Value as LuaValue, Error as LuaError};
use serde_json::Value as JsonValue;

use crate::renderer::{FrameRuntimeState};

#[derive(Clone)]
struct FrameHandle(Rc<RefCell<FrameRuntimeState>>);

impl UserData for FrameHandle {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("visible", |_, this| Ok(this.0.borrow().visible));
        fields.add_field_method_set("visible", |_, this, v: bool| {
            this.0.borrow_mut().visible = v;
            Ok(())
        });

        fields.add_field_method_get("value", |_, this| {
            Ok(this.0.borrow().value_override.clone())
        });
        fields.add_field_method_set("value", |_, this, v: Option<String>| {
            this.0.borrow_mut().value_override = v;
            Ok(())
        });
    }
}

#[derive(Debug, Clone)]
pub struct LuaRuntime {
    source: String,
    lua: Lua,
    data: Option<serde_json::Value>,
}

impl LuaRuntime {
    pub fn new() -> Self {
        let lua = Lua::new();

        Self {
            source: String::new(),
            lua,
            data: Option::None,
        }
    }

    pub fn load(&mut self, source: String, data: Option<&JsonValue>) {
        self.source = source;
        self.data = data.cloned();
    }

    pub fn exec(&self) -> mlua::Result<FrameRuntimeState> {
        let globals = self.lua.globals();

        let data_api = self.lua.create_table()?;
        let data_owned = self.data.clone();

        let get_slot = self.lua.create_function(move |lua, key: String| {
            let Some(JsonValue::Object(map)) = data_owned.as_ref() else {
                return Ok(LuaValue::Nil);
            };
            let Some(v) = map.get(&key) else {
                return Ok(LuaValue::Nil);
            };
            Self::json_scalar_to_lua(lua, v)
        })?;

        data_api.set("getSlot", get_slot)?;
        globals.set("data", data_api)?;

        let state = Rc::new(RefCell::new(FrameRuntimeState {
            visible: true,
            value_override: None,
        }));

        globals.set("frame", FrameHandle(state.clone()))?;
        self.lua.load(&self.source).exec()?;

        Ok(state.borrow().clone())
    }

    fn json_scalar_to_lua(lua: &Lua, v: &JsonValue) -> mlua::Result<LuaValue> {
        match v {
            JsonValue::Null => Ok(LuaValue::Nil),
            JsonValue::Bool(b) => Ok(LuaValue::Boolean(*b)),
            JsonValue::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(LuaValue::Integer(i))
                } else if let Some(f) = n.as_f64() {
                    Ok(LuaValue::Number(f))
                } else {
                    Err(LuaError::RuntimeError("invalid JSON number".to_string()))
                }
            }
            JsonValue::String(s) => Ok(LuaValue::String(lua.create_string(s)?)),
            JsonValue::Array(_) | JsonValue::Object(_) => Err(LuaError::RuntimeError("arrays/objects not supported in data.getSlot".to_string()))
        }
    }
}
