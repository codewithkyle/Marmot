use super::*;
use serde_json::json;

#[test]
fn script_can_toggle_visibility() {
    let mut runtime = LuaRuntime::new();
    let state = runtime
        .exec("FRAME_1", "frame.visible = false", None)
        .unwrap();

    assert!(!state.visible);
}

#[test]
fn script_can_set_value_override() {
    let mut runtime = LuaRuntime::new();
    let state = runtime
        .exec("FRAME_1", "frame.value = \"HELLO\"", None)
        .unwrap();

    assert_eq!(state.value_override, Some("HELLO".to_string()));
}

#[test]
fn script_can_clear_value_override_with_nil() {
    let mut runtime = LuaRuntime::new();
    let state = runtime.exec("FRAME_1", "frame.value = nil", None).unwrap();

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
    let state = runtime
        .exec(
            "FRAME_1",
            r#"
            local name = data.getSlot("name")
            local qty = data.getSlot("qty")
            local enabled = data.getSlot("enabled")
            local missing = data.getSlot("missing")

            if name == "qr" and qty == 12 and enabled == true and missing == nil then
                frame.visible = false
            end
        "#,
            Some(&data),
        )
        .unwrap();

    assert!(!state.visible);
}

#[test]
fn data_get_slot_errors_for_object_values() {
    let mut runtime = LuaRuntime::new();
    let data = json!({ "nested": { "a": 1 } });
    let err = runtime
        .exec("FRAME_1", "local v = data.getSlot(\"nested\")", Some(&data))
        .unwrap_err()
        .to_string();

    assert!(err.contains("data.getSlot"));
    assert!(err.contains("string/number/boolean/null"));
}

#[test]
fn invalid_visible_assignment_fails() {
    let mut runtime = LuaRuntime::new();
    let err = runtime
        .exec("FRAME_1", "frame.visible = \"no\"", None)
        .unwrap_err()
        .to_string();

    assert!(err.contains("visible"));
    assert!(err.contains("boolean") || err.contains("bool"));
}

#[test]
fn invalid_value_assignment_fails() {
    let mut runtime = LuaRuntime::new();
    let err = runtime
        .exec("FRAME_1", "frame.value = 123", None)
        .unwrap_err()
        .to_string();

    assert!(err.contains("value"));
    assert!(err.contains("string") || err.contains("nil"));
}

#[test]
fn runtime_error_fails_exec() {
    let mut runtime = LuaRuntime::new();
    let err = runtime
        .exec("FRAME_1", "error(\"boom\")", None)
        .unwrap_err()
        .to_string();

    assert!(err.contains("boom"));
}
