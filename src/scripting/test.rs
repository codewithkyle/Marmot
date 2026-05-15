use super::*;
use serde_json::json;

#[test]
fn script_can_toggle_visibility() {
    let mut runtime = LuaRuntime::new();
    runtime.load("frame.visible = false".to_string(), None);

    let state = runtime.exec().unwrap();

    assert!(!state.visible);
}

#[test]
fn script_can_set_value_override() {
    let mut runtime = LuaRuntime::new();
    runtime.load("frame.value = \"HELLO\"".to_string(), None);

    let state = runtime.exec().unwrap();

    assert_eq!(state.value_override, Some("HELLO".to_string()));
}

#[test]
fn script_can_clear_value_override_with_nil() {
    let mut runtime = LuaRuntime::new();
    runtime.load("frame.value = nil".to_string(), None);

    let state = runtime.exec().unwrap();

    assert_eq!(state.value_override, None);
}

#[test]
fn data_get_slot_supports_scalar_values() {
    let mut runtime = LuaRuntime::new();
    let data = json!({
        "name": "qr",
        "qty": 12,
        "enabled": true,
        "missing": null
    });
    runtime.load(
        r#"
            local name = data.getSlot("name")
            local qty = data.getSlot("qty")
            local enabled = data.getSlot("enabled")
            local missing = data.getSlot("missing")

            if name == "qr" and qty == 12 and enabled == true and missing == nil then
                frame.visible = false
            end
        "#
        .to_string(),
        Some(&data),
    );

    let state = runtime.exec().unwrap();

    assert!(!state.visible);
}

#[test]
fn data_get_slot_errors_for_object_values() {
    let mut runtime = LuaRuntime::new();
    let data = json!({ "nested": { "a": 1 } });
    runtime.load("local v = data.getSlot(\"nested\")".to_string(), Some(&data));

    let err = runtime.exec().unwrap_err().to_string();

    assert!(err.contains("data.getSlot"));
    assert!(err.contains("array") || err.contains("object"));
}

#[test]
fn invalid_visible_assignment_fails() {
    let mut runtime = LuaRuntime::new();
    runtime.load("frame.visible = \"no\"".to_string(), None);

    let err = runtime.exec().unwrap_err().to_string();

    assert!(err.contains("visible"));
    assert!(err.contains("boolean") || err.contains("bool"));
}

#[test]
fn invalid_value_assignment_fails() {
    let mut runtime = LuaRuntime::new();
    runtime.load("frame.value = 123".to_string(), None);

    let err = runtime.exec().unwrap_err().to_string();

    assert!(err.contains("value"));
    assert!(err.contains("string") || err.contains("nil"));
}

#[test]
fn runtime_error_fails_exec() {
    let mut runtime = LuaRuntime::new();
    runtime.load("error(\"boom\")".to_string(), None);

    let err = runtime.exec().unwrap_err().to_string();

    assert!(err.contains("boom"));
}
