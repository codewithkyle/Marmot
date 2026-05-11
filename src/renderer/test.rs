use std::{collections::HashMap, path::PathBuf};

use super::*;
use crate::parser::{DrawOp, NumberValue};

#[test]
fn lowers_rect_fill() {
    let draw_ops = vec![
        DrawOp::RectPath {
            x: NumberValue::Literal(10.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(30.0),
            height: NumberValue::Literal(40.0),
        },
        DrawOp::Fill,
    ];

    let data: Option<&Value> = None;
    let render_ops = lower_draw_ops(&draw_ops, data).unwrap();

    assert_eq!(
        render_ops,
        vec![RenderOp::FillRect {
            x: 10.0,
            y: 20.0,
            width: 30.0,
            height: 40.0,
        }]
    );
}

#[test]
fn lowers_rect_stroke() {
    let draw_ops = vec![
        DrawOp::RectPath {
            x: NumberValue::Literal(10.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(30.0),
            height: NumberValue::Literal(40.0),
        },
        DrawOp::Stroke,
    ];

    let data: Option<&Value> = None;
    let render_ops = lower_draw_ops(&draw_ops, data).unwrap();

    assert_eq!(
        render_ops,
        vec![RenderOp::StrokeRect {
            x: 10.0,
            y: 20.0,
            width: 30.0,
            height: 40.0,
        }]
    );
}

#[test]
fn lowers_line_stroke() {
    let draw_ops = vec![
        DrawOp::LinePath {
            x1: NumberValue::Literal(0.0),
            y1: NumberValue::Literal(0.0),
            x2: NumberValue::Literal(100.0),
            y2: NumberValue::Literal(100.0),
        },
        DrawOp::Stroke,
    ];

    let data: Option<&Value> = None;
    let render_ops = lower_draw_ops(&draw_ops, data).unwrap();

    assert_eq!(
        render_ops,
        vec![RenderOp::StrokeLine {
            x1: 0.0,
            y1: 0.0,
            x2: 100.0,
            y2: 100.0,
        }]
    );
}

#[test]
fn renders_basic_pdf() {
    use crate::parser::{DrawOp, NumberValue, Page};
    use std::fs;

    let page = Page {
        width: 200.0,
        height: 200.0,
    };

    let draw_ops = vec![
        DrawOp::SetRgb {
            r: NumberValue::Literal(1.0),
            g: NumberValue::Literal(1.0),
            b: NumberValue::Literal(1.0),
        },
        DrawOp::RectPath {
            x: NumberValue::Literal(0.0),
            y: NumberValue::Literal(0.0),
            width: NumberValue::Literal(200.0),
            height: NumberValue::Literal(200.0),
        },
        DrawOp::Fill,
        DrawOp::SetRgb {
            r: NumberValue::Literal(1.0),
            g: NumberValue::Literal(0.0),
            b: NumberValue::Literal(0.0),
        },
        DrawOp::RectPath {
            x: NumberValue::Literal(20.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(80.0),
            height: NumberValue::Literal(60.0),
        },
        DrawOp::Fill,
        DrawOp::SetRgb {
            r: NumberValue::Literal(0.0),
            g: NumberValue::Literal(0.0),
            b: NumberValue::Literal(0.0),
        },
        DrawOp::SetStrokeWidth {
            width: NumberValue::Literal(2.0),
        },
        DrawOp::RectPath {
            x: NumberValue::Literal(20.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(80.0),
            height: NumberValue::Literal(60.0),
        },
        DrawOp::Stroke,
        DrawOp::LinePath {
            x1: NumberValue::Literal(0.0),
            y1: NumberValue::Literal(0.0),
            x2: NumberValue::Literal(200.0),
            y2: NumberValue::Literal(200.0),
        },
        DrawOp::Stroke,
    ];

    let output_path = std::env::temp_dir().join("marmot_basic_render_test.pdf");

    let render_context = RenderContext {
        fonts: HashMap::<String, RegisteredFont>::new(),
    };

    let data: Option<&Value> = None;
    render_pdf(&page, &draw_ops, &output_path, data, &render_context).unwrap();

    let metadata = fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);

    let _ = fs::remove_file(output_path);
}

#[test]
fn lowers_literal_rect_without_data() {
    let draw_ops = vec![
        DrawOp::RectPath {
            x: NumberValue::Literal(10.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(30.0),
            height: NumberValue::Literal(40.0),
        },
        DrawOp::Fill,
    ];

    let render_ops = lower_draw_ops(&draw_ops, None).unwrap();

    assert_eq!(
        render_ops,
        vec![RenderOp::FillRect {
            x: 10.0,
            y: 20.0,
            width: 30.0,
            height: 40.0,
        }]
    );
}

#[test]
fn lowers_numeric_slot_from_json_data() {
    let data = serde_json::json!({
        "x": 25.0
    });

    let draw_ops = vec![
        DrawOp::RectPath {
            x: NumberValue::Slot("x".to_string()),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(30.0),
            height: NumberValue::Literal(40.0),
        },
        DrawOp::Fill,
    ];

    let render_ops = lower_draw_ops(&draw_ops, Some(&data)).unwrap();

    assert_eq!(
        render_ops,
        vec![RenderOp::FillRect {
            x: 25.0,
            y: 20.0,
            width: 30.0,
            height: 40.0,
        }]
    );
}

#[test]
fn lowers_integer_slot_from_json_data() {
    let data = serde_json::json!({
        "x": 25
    });

    let draw_ops = vec![
        DrawOp::LinePath {
            x1: NumberValue::Slot("x".to_string()),
            y1: NumberValue::Literal(0.0),
            x2: NumberValue::Literal(100.0),
            y2: NumberValue::Literal(100.0),
        },
        DrawOp::Stroke,
    ];

    let render_ops = lower_draw_ops(&draw_ops, Some(&data)).unwrap();

    assert_eq!(
        render_ops,
        vec![RenderOp::StrokeLine {
            x1: 25.0,
            y1: 0.0,
            x2: 100.0,
            y2: 100.0,
        }]
    );
}

#[test]
fn errors_when_slot_is_used_without_data() {
    let draw_ops = vec![
        DrawOp::RectPath {
            x: NumberValue::Slot("x".to_string()),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(30.0),
            height: NumberValue::Literal(40.0),
        },
        DrawOp::Fill,
    ];

    let err = lower_draw_ops(&draw_ops, None).unwrap_err();

    assert!(matches!(
        err,
        RenderError::MissingData { slot } if slot == "x"
    ));
}

#[test]
fn errors_when_json_field_is_missing() {
    let data = serde_json::json!({
        "other": 25.0
    });

    let draw_ops = vec![
        DrawOp::RectPath {
            x: NumberValue::Slot("x".to_string()),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(30.0),
            height: NumberValue::Literal(40.0),
        },
        DrawOp::Fill,
    ];

    let err = lower_draw_ops(&draw_ops, Some(&data)).unwrap_err();

    assert!(matches!(
        err,
        RenderError::MissingSlot { slot } if slot == "x"
    ));
}

#[test]
fn errors_when_json_field_is_not_a_number() {
    let data = serde_json::json!({
        "x": "25"
    });

    let draw_ops = vec![
        DrawOp::RectPath {
            x: NumberValue::Slot("x".to_string()),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(30.0),
            height: NumberValue::Literal(40.0),
        },
        DrawOp::Fill,
    ];

    let err = lower_draw_ops(&draw_ops, Some(&data)).unwrap_err();

    assert!(matches!(
        err,
        RenderError::InvalidNumberSlot { slot } if slot == "x"
    ));
}

#[test]
fn lowers_static_textbox() {
    let draw_ops = vec![DrawOp::TextBox {
        text: TextValue::Literal("Hello Marmot".to_string()),
        x: NumberValue::Literal(20.0),
        y: NumberValue::Literal(40.0),
        width: NumberValue::Literal(160.0),
        height: NumberValue::Literal(40.0),
    }];

    let render_ops = lower_draw_ops(&draw_ops, None).unwrap();

    assert_eq!(
        render_ops,
        vec![RenderOp::TextBox {
            text: "Hello Marmot".to_string(),
            x: 20.0,
            y: 40.0,
            width: 160.0,
            height: 40.0,
        }]
    );
}

#[test]
fn lowers_dynamic_textbox_from_json_data() {
    let data = serde_json::json!({
        "product_name": "Coffee Beans"
    });

    let draw_ops = vec![DrawOp::TextBox {
        text: TextValue::Slot("product_name".to_string()),
        x: NumberValue::Literal(20.0),
        y: NumberValue::Literal(40.0),
        width: NumberValue::Literal(160.0),
        height: NumberValue::Literal(40.0),
    }];

    let render_ops = lower_draw_ops(&draw_ops, Some(&data)).unwrap();

    assert_eq!(
        render_ops,
        vec![RenderOp::TextBox {
            text: "Coffee Beans".to_string(),
            x: 20.0,
            y: 40.0,
            width: 160.0,
            height: 40.0,
        }]
    );
}

#[test]
fn errors_when_text_slot_is_not_a_string() {
    let data = serde_json::json!({
        "product_name": 123
    });

    let draw_ops = vec![DrawOp::TextBox {
        text: TextValue::Slot("product_name".to_string()),
        x: NumberValue::Literal(20.0),
        y: NumberValue::Literal(40.0),
        width: NumberValue::Literal(160.0),
        height: NumberValue::Literal(40.0),
    }];

    let err = lower_draw_ops(&draw_ops, Some(&data)).unwrap_err();

    assert!(matches!(
        err,
        RenderError::InvalidTextSlot { slot } if slot == "product_name"
    ));
}

#[test]
fn renders_static_text_pdf() {
    use crate::parser::{DrawOp, NumberValue, Page, TextValue};
    use std::fs;

    let page = Page {
        width: 200.0,
        height: 100.0,
    };

    let draw_ops = vec![
        DrawOp::SetRgb {
            r: NumberValue::Literal(1.0),
            g: NumberValue::Literal(1.0),
            b: NumberValue::Literal(1.0),
        },
        DrawOp::RectPath {
            x: NumberValue::Literal(0.0),
            y: NumberValue::Literal(0.0),
            width: NumberValue::Literal(200.0),
            height: NumberValue::Literal(100.0),
        },
        DrawOp::Fill,
        DrawOp::SetRgb {
            r: NumberValue::Literal(0.0),
            g: NumberValue::Literal(0.0),
            b: NumberValue::Literal(0.0),
        },
        DrawOp::TextBox {
            text: TextValue::Literal("Hello Marmot".to_string()),
            x: NumberValue::Literal(20.0),
            y: NumberValue::Literal(35.0),
            width: NumberValue::Literal(160.0),
            height: NumberValue::Literal(40.0),
        },
    ];

    let output_path = std::env::temp_dir().join("marmot_text_render_test.pdf");

    let render_context = RenderContext {
        fonts: HashMap::<String, RegisteredFont>::new(),
    };

    render_pdf(&page, &draw_ops, &output_path, None, &render_context).unwrap();

    let metadata = fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);

    let _ = fs::remove_file(output_path);
}

#[test]
fn resolves_declared_font_as_registered_packaged_font() {
    let registered = RegisteredFont {
        path: PathBuf::from("/tmp/package/fonts/Helvetica-Bold.ttf"),
        family_name: "Helvetica Bold".to_string(),
    };

    let mut fonts = HashMap::new();
    fonts.insert("helvetica_bold".to_string(), registered.clone());

    let context = RenderContext { fonts };

    let font = resolve_current_font(&context, "helvetica_bold");

    assert_eq!(
        font,
        CurrentFont::Packaged {
            alias: "helvetica_bold".to_string(),
            font: registered,
        }
    );
}

#[test]
fn resolves_unknown_font_as_system_font() {
    let context = RenderContext {
        fonts: HashMap::new(),
    };

    let font = resolve_current_font(&context, "Sans");

    assert_eq!(font, CurrentFont::System("Sans".to_string()));
}

#[test]
fn packaged_font_uses_registered_family_name_for_pango() {
    let font = CurrentFont::Packaged {
        alias: "helvetica_bold".to_string(),
        font: RegisteredFont {
            path: PathBuf::from("/tmp/package/fonts/Helvetica-Bold.ttf"),
            family_name: "Helvetica Bold".to_string(),
        },
    };

    assert_eq!(current_font_description_name(&font), "Helvetica Bold");
}
