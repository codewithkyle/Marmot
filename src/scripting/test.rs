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
fn layer_script_can_toggle_visibility() {
    let mut runtime = LuaRuntime::new();
    let state = runtime
        .exec_layer("LAYER_1", "layer.visible = false", None)
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
fn invalid_layer_visible_assignment_fails() {
    let mut runtime = LuaRuntime::new();
    let err = runtime
        .exec_layer("LAYER_1", "layer.visible = \"no\"", None)
        .unwrap_err()
        .to_string();

    assert!(err.contains("layer.visible"));
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

#[test]
fn builtin_helpers_formatting_outputs() {
    let mut runtime = LuaRuntime::new();

    let state = runtime
        .exec("FRAME_1", "frame.value = percent(5.0, 20.0)", None)
        .unwrap();
    assert_eq!(state.value_override, Some("25%".to_string()));

    let state = runtime
        .exec("FRAME_1", "frame.value = currency(12.5)", None)
        .unwrap();
    assert_eq!(state.value_override, Some("$12.50".to_string()));

    let state = runtime
        .exec(
            "FRAME_1",
            "frame.value = concat(\"BUY \", 1, \" GET \", 1)",
            None,
        )
        .unwrap();
    assert_eq!(state.value_override, Some("BUY 1 GET 1".to_string()));

    let state = runtime
        .exec("FRAME_1", "frame.value = pad_left(42, 5, \"0\")", None)
        .unwrap();
    assert_eq!(state.value_override, Some("00042".to_string()));

    let state = runtime
        .exec("FRAME_1", "frame.value = pad_right(42, 5, \"0\")", None)
        .unwrap();
    assert_eq!(state.value_override, Some("42000".to_string()));

    let state = runtime
        .exec(
            "FRAME_1",
            "frame.value = unit_price(5.99, 16, \"oz\")",
            None,
        )
        .unwrap();
    assert_eq!(state.value_override, Some("$0.37/oz".to_string()));

    let state = runtime
        .exec("FRAME_1", "frame.value = unit_price_each(10.00, 4)", None)
        .unwrap();
    assert_eq!(state.value_override, Some("$2.50 ea".to_string()));

    let state = runtime
        .exec(
            "FRAME_1",
            "frame.value = truncate(\"Organic Honeycrisp Apples\", 12)",
            None,
        )
        .unwrap();
    assert_eq!(state.value_override, Some("Organic Hon…".to_string()));

    let state = runtime
        .exec(
            "FRAME_1",
            "frame.value = date_format(\"2026-05-15\", \"MM/DD/YYYY\")",
            None,
        )
        .unwrap();
    assert_eq!(state.value_override, Some("05/15/2026".to_string()));

    let state = runtime
        .exec("FRAME_1", "frame.value = trim(\" example \")", None)
        .unwrap();
    assert_eq!(state.value_override, Some("example".to_string()));

    let state = runtime
        .exec("FRAME_1", "frame.value = trim_left(\" example \")", None)
        .unwrap();
    assert_eq!(state.value_override, Some("example ".to_string()));

    let state = runtime
        .exec("FRAME_1", "frame.value = trim_right(\" example \")", None)
        .unwrap();
    assert_eq!(state.value_override, Some(" example".to_string()));
}

#[test]
fn builtin_helpers_numeric_and_table_outputs() {
    let mut runtime = LuaRuntime::new();
    let state = runtime
        .exec(
            "FRAME_1",
            r#"
            local rounded = round(12.345, 2)
            local fallback = default(nil, "N/A")
            local saved = save_amount(9.99, 7.49)
            local parts = price_parts(12.99)

            if rounded == 12.35
                and fallback == "N/A"
                and saved == 2.5
                and parts.dollars == "12"
                and parts.cents == "99" then
                frame.visible = false
            end
        "#,
            None,
        )
        .unwrap();

    assert!(!state.visible);
}

#[test]
fn builtin_helper_percent_uses_part_over_total() {
    let mut runtime = LuaRuntime::new();
    let state = runtime
        .exec("FRAME_1", "frame.value = percent(5.0, 10.0)", None)
        .unwrap();

    assert_eq!(state.value_override, Some("50%".to_string()));
}
