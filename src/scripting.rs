#[cfg(test)]
mod test;

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use mlua::{Error as LuaError, Lua, RegistryKey, UserData, UserDataFields, Value as LuaValue};
use serde_json::Value as JsonValue;

use crate::renderer::FrameRuntimeState;

#[derive(Clone)]
struct FrameHandle(Rc<RefCell<FrameRuntimeState>>);

impl UserData for FrameHandle {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("visible", |_, this| Ok(this.0.borrow().visible));
        fields.add_field_method_set("visible", |_, this, v: LuaValue| match v {
            LuaValue::Boolean(b) => {
                this.0.borrow_mut().visible = b;
                Ok(())
            }
            _ => Err(LuaError::RuntimeError(
                "frame.visible expects boolean".to_string(),
            )),
        });

        fields.add_field_method_get("value", |_, this| {
            Ok(this.0.borrow().value_override.clone())
        });
        fields.add_field_method_set("value", |_, this, v: LuaValue| match v {
            LuaValue::Nil => {
                this.0.borrow_mut().value_override = None;
                Ok(())
            }
            LuaValue::String(s) => {
                this.0.borrow_mut().value_override = Some(s.to_str()?.to_string());
                Ok(())
            }
            _ => Err(LuaError::RuntimeError(
                "frame.value expects string or nil".to_string(),
            )),
        });
    }
}

#[derive(Debug)]
pub struct LuaRuntime {
    lua: Lua,
    compiled_scripts: HashMap<String, RegistryKey>,
    compiled_sources: HashMap<String, String>,
    sandbox_base: Option<RegistryKey>,
}

impl LuaRuntime {
    pub fn new() -> Self {
        Self {
            lua: Lua::new(),
            compiled_scripts: HashMap::new(),
            compiled_sources: HashMap::new(),
            sandbox_base: None,
        }
    }

    pub fn exec(
        &mut self,
        frame_id: &str,
        source: &str,
        data: Option<&JsonValue>,
    ) -> mlua::Result<FrameRuntimeState> {
        let data_api = self.build_data_api(data)?;
        self.exec_with_data(frame_id, source, &data_api)
    }

    pub fn build_data_api(&self, data: Option<&JsonValue>) -> mlua::Result<mlua::Table> {
        let data_api = self.lua.create_table()?;
        let data_owned = data.cloned();

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
        Ok(data_api)
    }

    pub fn exec_with_data(
        &mut self,
        frame_id: &str,
        source: &str,
        data_api: &mlua::Table,
    ) -> mlua::Result<FrameRuntimeState> {
        self.compile_if_needed(frame_id, source)?;

        let state = Rc::new(RefCell::new(FrameRuntimeState {
            visible: true,
            value_override: None,
        }));

        let env = self.lua.create_table()?;
        env.set("data", data_api.clone())?;
        env.set("frame", FrameHandle(state.clone()))?;

        let metatable = self.lua.create_table()?;
        metatable.set("__index", self.sandbox_base()?)?;
        env.set_metatable(Some(metatable))?;

        let key = self.compiled_scripts.get(frame_id).ok_or_else(|| {
            LuaError::RuntimeError(format!("compiled script not found for frame '{frame_id}'"))
        })?;
        let func: mlua::Function = self.lua.registry_value(key)?;
        func.call::<()>(env)?;

        Ok(state.borrow().clone())
    }

    fn compile_if_needed(&mut self, frame_id: &str, source: &str) -> mlua::Result<()> {
        if let Some(prev_source) = self.compiled_sources.get(frame_id) {
            if prev_source == source {
                return Ok(());
            }
        }

        if let Some(old_key) = self.compiled_scripts.remove(frame_id) {
            self.lua.remove_registry_value(old_key)?;
        }

        let wrapped = format!(
            "return function(__marmot_env) local _ENV = __marmot_env; {} end",
            source
        );
        let func: mlua::Function = self.lua.load(&wrapped).eval()?;
        let key = self.lua.create_registry_value(func)?;

        self.compiled_scripts.insert(frame_id.to_string(), key);
        self.compiled_sources
            .insert(frame_id.to_string(), source.to_string());

        Ok(())
    }

    fn sandbox_base(&mut self) -> mlua::Result<mlua::Table> {
        if let Some(key) = &self.sandbox_base {
            return self.lua.registry_value(key);
        }

        let globals = self.lua.globals();
        let base = self.lua.create_table()?;

        for name in [
            "assert", "error", "ipairs", "next", "pairs", "pcall", "select", "tonumber",
            "tostring", "type", "xpcall", "math", "string", "table", "utf8",
        ] {
            let value = globals.get::<LuaValue>(name)?;
            if !matches!(value, LuaValue::Nil) {
                base.set(name, value)?;
            }
        }

        let key = self.lua.create_registry_value(base.clone())?;
        self.sandbox_base = Some(key);
        Ok(base)
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
            JsonValue::Array(_) | JsonValue::Object(_) => Err(LuaError::RuntimeError(
                "data.getSlot only supports string/number/boolean/null in v1".to_string(),
            )),
        }
    }
}
