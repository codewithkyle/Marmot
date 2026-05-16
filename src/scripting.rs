#[cfg(test)]
mod test;

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use mlua::{
    Error as LuaError, Lua, RegistryKey, Table, UserData, UserDataFields, Value as LuaValue,
    Variadic,
};
use serde_json::Value as JsonValue;

use crate::renderer::{FrameRuntimeState, LayerRuntimeState};

#[derive(Clone)]
struct FrameHandle(Rc<RefCell<FrameRuntimeState>>);

#[derive(Clone)]
struct LayerHandle(Rc<RefCell<LayerRuntimeState>>);

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

impl UserData for LayerHandle {
    fn add_fields<F: UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("visible", |_, this| Ok(this.0.borrow().visible));
        fields.add_field_method_set("visible", |_, this, v: LuaValue| match v {
            LuaValue::Boolean(b) => {
                this.0.borrow_mut().visible = b;
                Ok(())
            }
            _ => Err(LuaError::RuntimeError(
                "layer.visible expects boolean".to_string(),
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

    pub fn exec_layer(
        &mut self,
        layer_id: &str,
        source: &str,
        data: Option<&JsonValue>,
    ) -> mlua::Result<LayerRuntimeState> {
        let data_api = self.build_data_api(data)?;
        self.exec_layer_with_data(layer_id, source, &data_api)
    }

    pub fn exec_layer_with_data(
        &mut self,
        layer_id: &str,
        source: &str,
        data_api: &mlua::Table,
    ) -> mlua::Result<LayerRuntimeState> {
        self.compile_if_needed(layer_id, source)?;

        let state = Rc::new(RefCell::new(LayerRuntimeState { visible: true }));

        let env = self.lua.create_table()?;
        env.set("data", data_api.clone())?;
        env.set("layer", LayerHandle(state.clone()))?;

        let metatable = self.lua.create_table()?;
        metatable.set("__index", self.sandbox_base()?)?;
        env.set_metatable(Some(metatable))?;

        let key = self.compiled_scripts.get(layer_id).ok_or_else(|| {
            LuaError::RuntimeError(format!("compiled script not found for layer '{layer_id}'"))
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

        self.register_builtin_helpers(&base)?;

        let key = self.lua.create_registry_value(base.clone())?;
        self.sandbox_base = Some(key);
        Ok(base)
    }

    fn register_builtin_helpers(&self, base: &Table) -> mlua::Result<()> {
        let percent = self.lua.create_function(|_, (part, total): (f64, f64)| {
            if total == 0.0 {
                return Err(LuaError::RuntimeError(
                    "percent total must not be zero".to_string(),
                ));
            }
            let pct = (part / total) * 100.0;
            Ok(format!("{}%", Self::format_decimal_trimmed(pct, 2)))
        })?;
        base.set("percent", percent)?;

        let currency = self
            .lua
            .create_function(|_, amount: f64| Ok(format!("${:.2}", amount)))?;
        base.set("currency", currency)?;

        let round = self.lua.create_function(|_, (value, places): (f64, i64)| {
            Ok(Self::round_to_places(value, places))
        })?;
        base.set("round", round)?;

        let default_fn =
            self.lua
                .create_function(|_, (value, fallback): (LuaValue, LuaValue)| {
                    if matches!(value, LuaValue::Nil) {
                        Ok(fallback)
                    } else {
                        Ok(value)
                    }
                })?;
        base.set("default", default_fn)?;

        let concat = self.lua.create_function(|_, values: Variadic<LuaValue>| {
            let mut out = String::new();
            for value in values {
                out.push_str(&Self::lua_value_to_string(&value)?);
            }
            Ok(out)
        })?;
        base.set("concat", concat)?;

        let pad_left =
            self.lua
                .create_function(|_, (value, width, pad): (LuaValue, usize, String)| {
                    let value = Self::lua_value_to_string(&value)?;
                    Ok(Self::pad_string(&value, width, &pad, true)?)
                })?;
        base.set("pad_left", pad_left)?;

        let pad_right =
            self.lua
                .create_function(|_, (value, width, pad): (LuaValue, usize, String)| {
                    let value = Self::lua_value_to_string(&value)?;
                    Ok(Self::pad_string(&value, width, &pad, false)?)
                })?;
        base.set("pad_right", pad_right)?;

        let price_parts = self.lua.create_function(|lua, amount: f64| {
            let cents_total = (amount * 100.0).round() as i64;
            let dollars = cents_total / 100;
            let cents = cents_total.abs() % 100;

            let parts = lua.create_table()?;
            parts.set("dollars", dollars.to_string())?;
            parts.set("cents", format!("{:02}", cents))?;
            Ok(parts)
        })?;
        base.set("price_parts", price_parts)?;

        let unit_price =
            self.lua
                .create_function(|_, (total, qty, unit): (f64, f64, String)| {
                    if qty <= 0.0 {
                        return Err(LuaError::RuntimeError(
                            "unit_price quantity must be greater than zero".to_string(),
                        ));
                    }
                    Ok(format!("${:.2}/{}", total / qty, unit))
                })?;
        base.set("unit_price", unit_price)?;

        let unit_price_each = self.lua.create_function(|_, (total, count): (f64, f64)| {
            if count <= 0.0 {
                return Err(LuaError::RuntimeError(
                    "unit_price_each count must be greater than zero".to_string(),
                ));
            }
            Ok(format!("${:.2} ea", total / count))
        })?;
        base.set("unit_price_each", unit_price_each)?;

        let save_amount = self.lua.create_function(|_, (regular, sale): (f64, f64)| {
            Ok(Self::round_to_places(regular - sale, 2))
        })?;
        base.set("save_amount", save_amount)?;

        let truncate = self
            .lua
            .create_function(|_, (value, max_len): (String, usize)| {
                Ok(Self::truncate_string(&value, max_len))
            })?;
        base.set("truncate", truncate)?;

        let date_format = self
            .lua
            .create_function(|_, (input, pattern): (String, String)| {
                Self::format_date(&input, &pattern)
            })?;
        base.set("date_format", date_format)?;

        let trim = self
            .lua
            .create_function(|_, value: String| Ok(value.trim().to_string()))?;
        base.set("trim", trim)?;

        let trim_left = self
            .lua
            .create_function(|_, value: String| Ok(value.trim_start().to_string()))?;
        base.set("trim_left", trim_left)?;

        let trim_right = self
            .lua
            .create_function(|_, value: String| Ok(value.trim_end().to_string()))?;
        base.set("trim_right", trim_right)?;

        Ok(())
    }

    fn round_to_places(value: f64, places: i64) -> f64 {
        let places = places.clamp(0, 9) as i32;
        let factor = 10_f64.powi(places);
        (value * factor).round() / factor
    }

    fn format_decimal_trimmed(value: f64, places: usize) -> String {
        let mut s = format!("{value:.places$}");
        if s.contains('.') {
            while s.ends_with('0') {
                s.pop();
            }
            if s.ends_with('.') {
                s.pop();
            }
        }
        s
    }

    fn lua_value_to_string(value: &LuaValue) -> mlua::Result<String> {
        match value {
            LuaValue::Nil => Ok("nil".to_string()),
            LuaValue::Boolean(v) => Ok(v.to_string()),
            LuaValue::Integer(v) => Ok(v.to_string()),
            LuaValue::Number(v) => Ok(Self::format_decimal_trimmed(*v, 6)),
            LuaValue::String(v) => Ok(v.to_str()?.to_string()),
            _ => Err(LuaError::RuntimeError(
                "value cannot be converted to string".to_string(),
            )),
        }
    }

    fn pad_string(value: &str, width: usize, pad: &str, left: bool) -> mlua::Result<String> {
        if pad.is_empty() {
            return Err(LuaError::RuntimeError(
                "pad string must not be empty".to_string(),
            ));
        }
        let value_len = value.chars().count();
        if value_len >= width {
            return Ok(value.to_string());
        }

        let needed = width - value_len;
        let mut filler = String::new();
        while filler.chars().count() < needed {
            filler.push_str(pad);
        }
        let filler = filler.chars().take(needed).collect::<String>();
        if left {
            Ok(format!("{filler}{value}"))
        } else {
            Ok(format!("{value}{filler}"))
        }
    }

    fn truncate_string(value: &str, max_len: usize) -> String {
        if max_len == 0 {
            return String::new();
        }
        let char_count = value.chars().count();
        if char_count <= max_len {
            return value.to_string();
        }
        if max_len == 1 {
            return "…".to_string();
        }
        let keep = max_len - 1;
        let mut out = value.chars().take(keep).collect::<String>();
        out.push('…');
        out
    }

    fn format_date(input: &str, pattern: &str) -> mlua::Result<String> {
        let parts = input.split('-').collect::<Vec<_>>();
        if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
            return Err(LuaError::RuntimeError(
                "date_format expects YYYY-MM-DD input".to_string(),
            ));
        }
        let (year, month, day) = (parts[0], parts[1], parts[2]);
        if !year.chars().all(|c| c.is_ascii_digit())
            || !month.chars().all(|c| c.is_ascii_digit())
            || !day.chars().all(|c| c.is_ascii_digit())
        {
            return Err(LuaError::RuntimeError(
                "date_format expects numeric YYYY-MM-DD input".to_string(),
            ));
        }

        let mut out = pattern.to_string();
        out = out.replace("YYYY", year);
        out = out.replace("MM", month);
        out = out.replace("DD", day);
        Ok(out)
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
