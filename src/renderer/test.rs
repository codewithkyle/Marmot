use std::{collections::HashMap, path::PathBuf};

use super::*;
use crate::{
    parser::{DrawOp, NumberValue},
    resources::RegisteredAsset,
};
use serde_json::Value;

fn execute_draw_ops_for_test(draw_ops: &[DrawOp], data: Option<&Value>) -> Result<(), RenderError> {
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 256, 256)?;
    let ctx = cairo::Context::new(&surface)?;
    let mut cache = RenderCache::default();
    let render_context = RenderContext {
        fonts: HashMap::<String, RegisteredFont>::new(),
        assets: HashMap::<String, RegisteredAsset>::new(),
    };

    execute_draw_ops(&ctx, draw_ops, data, &render_context, &mut cache)
}

#[test]
fn executes_rect_fill() {
    let draw_ops = vec![
        DrawOp::RectPath {
            x: NumberValue::Literal(10.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(30.0),
            height: NumberValue::Literal(40.0),
        },
        DrawOp::Fill,
    ];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
}

#[test]
fn executes_rect_stroke() {
    let draw_ops = vec![
        DrawOp::RectPath {
            x: NumberValue::Literal(10.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(30.0),
            height: NumberValue::Literal(40.0),
        },
        DrawOp::Stroke,
    ];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
}

#[test]
fn executes_line_stroke() {
    let draw_ops = vec![
        DrawOp::LinePath {
            x1: NumberValue::Literal(0.0),
            y1: NumberValue::Literal(0.0),
            x2: NumberValue::Literal(100.0),
            y2: NumberValue::Literal(100.0),
        },
        DrawOp::Stroke,
    ];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
}

#[test]
fn executes_cmyk_literal_color() {
    let draw_ops = vec![DrawOp::SetCmyk {
        c: NumberValue::Literal(0.1),
        m: NumberValue::Literal(0.2),
        y: NumberValue::Literal(0.3),
        k: NumberValue::Literal(0.4),
    }];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
}

#[test]
fn executes_clamped_cmyk_literal_color() {
    let draw_ops = vec![DrawOp::SetCmyk {
        c: NumberValue::Literal(2.0),
        m: NumberValue::Literal(-0.5),
        y: NumberValue::Literal(0.3),
        k: NumberValue::Literal(1.2),
    }];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
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
        assets: HashMap::<String, RegisteredAsset>::new(),
    };

    render_pdf(&page, &draw_ops, &output_path, None, &render_context).unwrap();

    let metadata = fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);

    let _ = fs::remove_file(output_path);
}

#[test]
fn executes_literal_rect_without_data() {
    let draw_ops = vec![
        DrawOp::RectPath {
            x: NumberValue::Literal(10.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(30.0),
            height: NumberValue::Literal(40.0),
        },
        DrawOp::Fill,
    ];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
}

#[test]
fn executes_numeric_slot_from_json_data() {
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

    execute_draw_ops_for_test(&draw_ops, Some(&data)).unwrap();
}

#[test]
fn executes_integer_slot_from_json_data() {
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

    execute_draw_ops_for_test(&draw_ops, Some(&data)).unwrap();
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

    let err = execute_draw_ops_for_test(&draw_ops, None).unwrap_err();

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

    let err = execute_draw_ops_for_test(&draw_ops, Some(&data)).unwrap_err();

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

    let err = execute_draw_ops_for_test(&draw_ops, Some(&data)).unwrap_err();

    assert!(matches!(
        err,
        RenderError::InvalidNumberSlot { slot } if slot == "x"
    ));
}

#[test]
fn executes_static_textbox() {
    let draw_ops = vec![DrawOp::TextBox {
        text: TextValue::Literal("Hello Marmot".to_string()),
        x: NumberValue::Literal(20.0),
        y: NumberValue::Literal(40.0),
        width: NumberValue::Literal(160.0),
        height: NumberValue::Literal(40.0),
    }];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
}

#[test]
fn executes_dynamic_textbox_from_json_data() {
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

    execute_draw_ops_for_test(&draw_ops, Some(&data)).unwrap();
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

    let err = execute_draw_ops_for_test(&draw_ops, Some(&data)).unwrap_err();

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
        assets: HashMap::<String, RegisteredAsset>::new(),
    };

    render_pdf(&page, &draw_ops, &output_path, None, &render_context).unwrap();

    let metadata = std::fs::metadata(&output_path).unwrap();
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

    let context = RenderContext {
        fonts,
        assets: HashMap::<String, RegisteredAsset>::new(),
    };

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
        assets: HashMap::<String, RegisteredAsset>::new(),
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

#[test]
fn executes_text_style_and_font_ops() {
    let draw_ops = vec![
        DrawOp::SetFontFamily {
            font: TextValue::Literal("Helvetica-Bold".to_string()),
        },
        DrawOp::SetFontSize {
            size: NumberValue::Literal(14.0),
        },
        DrawOp::SetTextAlignment {
            align: TextAlign::Right,
        },
        DrawOp::SetVerticalAlignment {
            align: VerticalAlign::Bottom,
        },
        DrawOp::SetLineBreakMode {
            line_break: LineBreakMode::None,
        },
        DrawOp::SetTextFit {
            fit: TextFit::ShrinkToFit,
        },
        DrawOp::SetTextFitMinSize {
            min: NumberValue::Literal(8.0),
        },
        DrawOp::SetTextFitMaxSize {
            max: NumberValue::Literal(24.0),
        },
    ];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
}

#[test]
fn executes_slot_driven_style_values_from_json_data() {
    let data = serde_json::json!({
        "font": "Helvetica-Bold",
        "r": 0.1,
        "g": 0.2,
        "b": 0.3,
        "stroke": 2,
        "size": 16,
        "min": 9,
        "max": 32
    });

    let draw_ops = vec![
        DrawOp::SetFontFamily {
            font: TextValue::Slot("font".to_string()),
        },
        DrawOp::SetRgb {
            r: NumberValue::Slot("r".to_string()),
            g: NumberValue::Slot("g".to_string()),
            b: NumberValue::Slot("b".to_string()),
        },
        DrawOp::SetStrokeWidth {
            width: NumberValue::Slot("stroke".to_string()),
        },
        DrawOp::SetFontSize {
            size: NumberValue::Slot("size".to_string()),
        },
        DrawOp::SetTextFitMinSize {
            min: NumberValue::Slot("min".to_string()),
        },
        DrawOp::SetTextFitMaxSize {
            max: NumberValue::Slot("max".to_string()),
        },
    ];

    execute_draw_ops_for_test(&draw_ops, Some(&data)).unwrap();
}

#[test]
fn executes_slot_driven_cmyk_values_from_json_data() {
    let data = serde_json::json!({
        "c": 0.1,
        "m": 0.2,
        "y": 0.3,
        "k": 0.4
    });

    let draw_ops = vec![DrawOp::SetCmyk {
        c: NumberValue::Slot("c".to_string()),
        m: NumberValue::Slot("m".to_string()),
        y: NumberValue::Slot("y".to_string()),
        k: NumberValue::Slot("k".to_string()),
    }];

    execute_draw_ops_for_test(&draw_ops, Some(&data)).unwrap();
}

#[test]
fn errors_when_cmyk_slot_field_is_missing() {
    let data = serde_json::json!({
        "c": 0.1,
        "m": 0.2,
        "y": 0.3
    });

    let draw_ops = vec![DrawOp::SetCmyk {
        c: NumberValue::Slot("c".to_string()),
        m: NumberValue::Slot("m".to_string()),
        y: NumberValue::Slot("y".to_string()),
        k: NumberValue::Slot("k".to_string()),
    }];

    let err = execute_draw_ops_for_test(&draw_ops, Some(&data)).unwrap_err();

    assert!(matches!(
        err,
        RenderError::MissingSlot { slot } if slot == "k"
    ));
}

#[test]
fn errors_when_text_slot_is_used_without_data() {
    let draw_ops = vec![DrawOp::TextBox {
        text: TextValue::Slot("product_name".to_string()),
        x: NumberValue::Literal(20.0),
        y: NumberValue::Literal(40.0),
        width: NumberValue::Literal(160.0),
        height: NumberValue::Literal(40.0),
    }];

    let err = execute_draw_ops_for_test(&draw_ops, None).unwrap_err();

    assert!(matches!(
        err,
        RenderError::MissingData { slot } if slot == "product_name"
    ));
}

#[test]
fn errors_when_text_slot_field_is_missing() {
    let data = serde_json::json!({
        "other": "Coffee Beans"
    });

    let draw_ops = vec![DrawOp::TextBox {
        text: TextValue::Slot("product_name".to_string()),
        x: NumberValue::Literal(20.0),
        y: NumberValue::Literal(40.0),
        width: NumberValue::Literal(160.0),
        height: NumberValue::Literal(40.0),
    }];

    let err = execute_draw_ops_for_test(&draw_ops, Some(&data)).unwrap_err();

    assert!(matches!(
        err,
        RenderError::MissingSlot { slot } if slot == "product_name"
    ));
}

#[test]
#[should_panic(expected = "parser should prevent stroke without a current path")]
fn panics_when_stroke_has_no_pending_path() {
    let draw_ops = vec![DrawOp::Stroke];

    let _ = execute_draw_ops_for_test(&draw_ops, None);
}

#[test]
#[should_panic(expected = "parser should prevent fill without a current path")]
fn panics_when_fill_has_no_pending_path() {
    let draw_ops = vec![DrawOp::Fill];

    let _ = execute_draw_ops_for_test(&draw_ops, None);
}

#[test]
#[should_panic(expected = "parser should prevent filling a line")]
fn panics_when_fill_is_applied_to_line() {
    let draw_ops = vec![
        DrawOp::LinePath {
            x1: NumberValue::Literal(0.0),
            y1: NumberValue::Literal(0.0),
            x2: NumberValue::Literal(10.0),
            y2: NumberValue::Literal(10.0),
        },
        DrawOp::Fill,
    ];

    let _ = execute_draw_ops_for_test(&draw_ops, None);
}

#[test]
fn returns_cairo_error_for_invalid_output_path() {
    use crate::parser::Page;

    let page = Page {
        width: 200.0,
        height: 200.0,
    };
    let draw_ops = vec![];
    let output_path = PathBuf::from("/definitely/missing/path/marmot-render-test.pdf");
    let render_context = RenderContext {
        fonts: HashMap::<String, RegisteredFont>::new(),
        assets: HashMap::<String, RegisteredAsset>::new(),
    };

    let err = render_pdf(&page, &draw_ops, &output_path, None, &render_context).unwrap_err();

    assert!(matches!(err, RenderError::Cairo(_)));
}

#[test]
fn executes_image_draw_op() {
    use crate::parser::TextValue;
    let draw_ops = vec![DrawOp::Image {
        asset: TextValue::Literal("logo".to_string()),
        x: NumberValue::Literal(12.0),
        y: NumberValue::Literal(24.0),
        width: NumberValue::Literal(80.0),
        height: NumberValue::Literal(40.0),
    }];

    let err = execute_draw_ops_for_test(&draw_ops, None).unwrap_err();
    assert!(matches!(
        err,
        RenderError::MissingAssetAlias { alias } if alias == "logo"
    ));
}

#[test]
fn errors_when_rendering_image_with_missing_asset_alias() {
    use crate::parser::{Page, TextValue};
    let page = Page {
        width: 100.0,
        height: 100.0,
    };
    let draw_ops = vec![DrawOp::Image {
        asset: TextValue::Literal("missing_logo".to_string()),
        x: NumberValue::Literal(0.0),
        y: NumberValue::Literal(0.0),
        width: NumberValue::Literal(50.0),
        height: NumberValue::Literal(50.0),
    }];
    let output_path = std::env::temp_dir().join("marmot_missing_image_alias_test.pdf");
    let render_context = RenderContext {
        fonts: HashMap::<String, RegisteredFont>::new(),
        assets: HashMap::<String, RegisteredAsset>::new(),
    };
    let err = render_pdf(&page, &draw_ops, &output_path, None, &render_context).unwrap_err();
    assert!(matches!(
        err,
        RenderError::MissingAssetAlias { alias } if alias == "missing_logo"
    ));
}

#[test]
fn renders_pdf_with_registered_image_asset() {
    use crate::parser::{AssetType, Page, TextValue};
    use crate::resources::RegisteredImageInfo;
    use image::{ImageFormat, Rgba, RgbaImage};
    use std::fs;

    let dir = tempfile::tempdir().unwrap();
    let image_path = dir.path().join("logo.png");
    let img = RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255]));
    img.save_with_format(&image_path, ImageFormat::Png).unwrap();
    let byte_len = fs::metadata(&image_path).unwrap().len();
    let mut assets = HashMap::new();
    assets.insert(
        "logo".to_string(),
        RegisteredAsset {
            path: image_path.clone(),
            name: "logo".to_string(),
            ty: AssetType::Image,
            byte_len,
            image: Some(RegisteredImageInfo {
                format: "png".to_string(),
                width: 2,
                height: 2,
            }),
        },
    );
    let render_context = RenderContext {
        fonts: HashMap::<String, RegisteredFont>::new(),
        assets,
    };
    let page = Page {
        width: 120.0,
        height: 120.0,
    };
    let draw_ops = vec![DrawOp::Image {
        asset: TextValue::Literal("logo".to_string()),
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(10.0),
        width: NumberValue::Literal(80.0),
        height: NumberValue::Literal(80.0),
    }];
    let output_path = std::env::temp_dir().join("marmot_image_render_test.pdf");
    render_pdf(&page, &draw_ops, &output_path, None, &render_context).unwrap();
    let metadata = fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);
    let _ = fs::remove_file(output_path);
}

#[test]
fn executes_set_imagefit() {
    let draw_ops = vec![DrawOp::SetImageFit {
        fit: ImageFit::Cover,
    }];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
}

#[test]
fn executes_textbox_with_concat_text() {
    let data = serde_json::json!({
        "B": "2",
        "G": "1"
    });
    let draw_ops = vec![DrawOp::TextBox {
        text: TextValue::Concat(vec![
            TextValue::Literal("BUY ".to_string()),
            TextValue::Slot("B".to_string()),
            TextValue::Literal(" GET ".to_string()),
            TextValue::Slot("G".to_string()),
        ]),
        x: NumberValue::Literal(20.0),
        y: NumberValue::Literal(40.0),
        width: NumberValue::Literal(160.0),
        height: NumberValue::Literal(40.0),
    }];
    execute_draw_ops_for_test(&draw_ops, Some(&data)).unwrap();
}
#[test]
fn errors_when_concat_contains_missing_slot() {
    let data = serde_json::json!({
        "B": "2"
    });
    let draw_ops = vec![DrawOp::TextBox {
        text: TextValue::Concat(vec![
            TextValue::Literal("BUY ".to_string()),
            TextValue::Slot("B".to_string()),
            TextValue::Literal(" GET ".to_string()),
            TextValue::Slot("G".to_string()),
        ]),
        x: NumberValue::Literal(20.0),
        y: NumberValue::Literal(40.0),
        width: NumberValue::Literal(160.0),
        height: NumberValue::Literal(40.0),
    }];
    let err = execute_draw_ops_for_test(&draw_ops, Some(&data)).unwrap_err();
    assert!(matches!(
        err,
        RenderError::MissingSlot { slot } if slot == "G"
    ));
}

#[test]
fn eval_text_uppercase() {
    let value = TextValue::UpperCase(Box::new(TextValue::Literal("hELLo".to_string())));
    let text = eval_text(&value, None).unwrap();
    assert_eq!(text, "HELLO");
}

#[test]
fn eval_text_lowercase() {
    let value = TextValue::LowerCase(Box::new(TextValue::Literal("hELLo".to_string())));
    let text = eval_text(&value, None).unwrap();
    assert_eq!(text, "hello");
}

#[test]
fn eval_text_titlecase() {
    let value = TextValue::TitleCase(Box::new(TextValue::Literal("hELLo wORLd".to_string())));
    let text = eval_text(&value, None).unwrap();
    assert_eq!(text, "Hello World");
}

#[test]
fn eval_text_capitalize() {
    let value = TextValue::Capitalize(Box::new(TextValue::Literal("hELLo wORLd".to_string())));
    let text = eval_text(&value, None).unwrap();
    assert_eq!(text, "Hello world");
}

#[test]
fn executes_code39_barcode_draw_op() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("ABC-123".to_string()),
        symbology: crate::parser::BarcodeSymbology::Code39,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(220.0),
        height: NumberValue::Literal(50.0),
    }];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
}

#[test]
fn executes_code39_barcode_draw_op_with_slot_data() {
    let data = serde_json::json!({
        "sku": "MARMOT-42"
    });

    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Slot("sku".to_string()),
        symbology: crate::parser::BarcodeSymbology::Code39,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(220.0),
        height: NumberValue::Literal(50.0),
    }];

    execute_draw_ops_for_test(&draw_ops, Some(&data)).unwrap();
}

#[test]
fn errors_when_code39_data_contains_unsupported_characters() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("abc".to_string()),
        symbology: crate::parser::BarcodeSymbology::Code39,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(220.0),
        height: NumberValue::Literal(50.0),
    }];

    let err = execute_draw_ops_for_test(&draw_ops, None).unwrap_err();
    assert!(matches!(
        err,
        RenderError::BarcodeEncode { symbology, data, .. }
            if symbology == "c39" && data == "abc"
    ));
}

#[test]
fn errors_when_code39_barcode_geometry_is_invalid() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("ABC123".to_string()),
        symbology: crate::parser::BarcodeSymbology::Code39,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(0.0),
        height: NumberValue::Literal(50.0),
    }];

    let err = execute_draw_ops_for_test(&draw_ops, None).unwrap_err();
    assert!(matches!(
        err,
        RenderError::InvalidBarcodeGeometry { width, height }
            if width == 0.0 && height == 50.0
    ));
}

#[test]
fn executes_code128a_barcode_draw_op() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("ABC123".to_string()),
        symbology: crate::parser::BarcodeSymbology::Code128A,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(220.0),
        height: NumberValue::Literal(50.0),
    }];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
}

#[test]
fn executes_code128b_barcode_draw_op() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("Abc-123".to_string()),
        symbology: crate::parser::BarcodeSymbology::Code128B,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(220.0),
        height: NumberValue::Literal(50.0),
    }];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
}

#[test]
fn executes_code128c_barcode_draw_op() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("12345678".to_string()),
        symbology: crate::parser::BarcodeSymbology::Code128C,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(220.0),
        height: NumberValue::Literal(50.0),
    }];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
}

#[test]
fn errors_when_code128c_data_is_invalid() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("12345".to_string()),
        symbology: crate::parser::BarcodeSymbology::Code128C,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(220.0),
        height: NumberValue::Literal(50.0),
    }];

    let err = execute_draw_ops_for_test(&draw_ops, None).unwrap_err();
    assert!(matches!(
        err,
        RenderError::BarcodeEncode { symbology, data, .. }
            if symbology == "c128c" && data == "12345"
    ));
}

#[test]
fn executes_upca_barcode_draw_op() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("036000291452".to_string()),
        symbology: crate::parser::BarcodeSymbology::UPCA,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(220.0),
        height: NumberValue::Literal(50.0),
    }];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
}

#[test]
fn errors_when_upca_data_is_invalid() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("ABC123".to_string()),
        symbology: crate::parser::BarcodeSymbology::UPCA,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(220.0),
        height: NumberValue::Literal(50.0),
    }];

    let err = execute_draw_ops_for_test(&draw_ops, None).unwrap_err();
    assert!(matches!(
        err,
        RenderError::BarcodeEncode { symbology, data, .. }
            if symbology == "upca" && data == "ABC123"
    ));
}

#[test]
fn errors_when_upca_barcode_geometry_is_invalid() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("036000291452".to_string()),
        symbology: crate::parser::BarcodeSymbology::UPCA,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(0.0),
        height: NumberValue::Literal(50.0),
    }];

    let err = execute_draw_ops_for_test(&draw_ops, None).unwrap_err();
    assert!(matches!(
        err,
        RenderError::InvalidBarcodeGeometry { width, height }
            if width == 0.0 && height == 50.0
    ));
}

#[test]
fn retail_guard_module_boundaries() {
    assert!(is_retail_guard_module(0));
    assert!(is_retail_guard_module(2));
    assert!(!is_retail_guard_module(3));

    assert!(is_retail_guard_module(45));
    assert!(is_retail_guard_module(49));
    assert!(!is_retail_guard_module(50));

    assert!(is_retail_guard_module(92));
    assert!(is_retail_guard_module(94));
    assert!(!is_retail_guard_module(95));
}

#[test]
fn executes_ean13_barcode_draw_op() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("4006381333931".to_string()),
        symbology: crate::parser::BarcodeSymbology::EAN13,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(220.0),
        height: NumberValue::Literal(50.0),
    }];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
}

#[test]
fn errors_when_ean13_data_is_invalid() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("ABC123".to_string()),
        symbology: crate::parser::BarcodeSymbology::EAN13,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(220.0),
        height: NumberValue::Literal(50.0),
    }];

    let err = execute_draw_ops_for_test(&draw_ops, None).unwrap_err();
    assert!(matches!(
        err,
        RenderError::BarcodeEncode { symbology, data, .. }
            if symbology == "ean13" && data == "ABC123"
    ));
}

#[test]
fn errors_when_ean13_barcode_geometry_is_invalid() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("4006381333931".to_string()),
        symbology: crate::parser::BarcodeSymbology::EAN13,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(0.0),
        height: NumberValue::Literal(50.0),
    }];

    let err = execute_draw_ops_for_test(&draw_ops, None).unwrap_err();
    assert!(matches!(
        err,
        RenderError::InvalidBarcodeGeometry { width, height }
            if width == 0.0 && height == 50.0
    ));
}

#[test]
fn executes_ean8_barcode_draw_op() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("55123457".to_string()),
        symbology: crate::parser::BarcodeSymbology::EAN8,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(220.0),
        height: NumberValue::Literal(50.0),
    }];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
}

#[test]
fn errors_when_ean8_data_is_invalid() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("ABC123".to_string()),
        symbology: crate::parser::BarcodeSymbology::EAN8,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(220.0),
        height: NumberValue::Literal(50.0),
    }];

    let err = execute_draw_ops_for_test(&draw_ops, None).unwrap_err();
    assert!(matches!(
        err,
        RenderError::BarcodeEncode { symbology, data, .. }
            if symbology == "ean8" && data == "ABC123"
    ));
}

#[test]
fn errors_when_ean8_barcode_geometry_is_invalid() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("55123457".to_string()),
        symbology: crate::parser::BarcodeSymbology::EAN8,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(0.0),
        height: NumberValue::Literal(50.0),
    }];

    let err = execute_draw_ops_for_test(&draw_ops, None).unwrap_err();
    assert!(matches!(
        err,
        RenderError::InvalidBarcodeGeometry { width, height }
            if width == 0.0 && height == 50.0
    ));
}

#[test]
fn ean8_guard_module_boundaries() {
    assert!(is_ean8_guard_module(0));
    assert!(is_ean8_guard_module(2));
    assert!(!is_ean8_guard_module(3));

    assert!(is_ean8_guard_module(31));
    assert!(is_ean8_guard_module(35));
    assert!(!is_ean8_guard_module(36));

    assert!(is_ean8_guard_module(64));
    assert!(is_ean8_guard_module(66));
    assert!(!is_ean8_guard_module(67));
}

#[test]
fn executes_qr_barcode_draw_op() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("https://example.com/marmot".to_string()),
        symbology: crate::parser::BarcodeSymbology::QR,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(140.0),
        height: NumberValue::Literal(140.0),
    }];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
}

#[test]
fn errors_when_qr_data_is_too_long() {
    let too_long = "A".repeat(6000);
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal(too_long.clone()),
        symbology: crate::parser::BarcodeSymbology::QR,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(140.0),
        height: NumberValue::Literal(140.0),
    }];

    let err = execute_draw_ops_for_test(&draw_ops, None).unwrap_err();
    assert!(matches!(
        err,
        RenderError::BarcodeEncode { symbology, data, .. }
            if symbology == "qr" && data == too_long
    ));
}

#[test]
fn errors_when_qr_barcode_geometry_is_invalid() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("https://example.com".to_string()),
        symbology: crate::parser::BarcodeSymbology::QR,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(0.0),
        height: NumberValue::Literal(140.0),
    }];

    let err = execute_draw_ops_for_test(&draw_ops, None).unwrap_err();
    assert!(matches!(
        err,
        RenderError::InvalidBarcodeGeometry { width, height }
            if width == 0.0 && height == 140.0
    ));
}

#[test]
fn executes_datamatrix_barcode_draw_op() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("MARMOT-DM-001".to_string()),
        symbology: crate::parser::BarcodeSymbology::DataMatrix,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(140.0),
        height: NumberValue::Literal(140.0),
    }];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
}

#[test]
fn errors_when_datamatrix_data_is_too_long() {
    let too_long = "A".repeat(10000);
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal(too_long.clone()),
        symbology: crate::parser::BarcodeSymbology::DataMatrix,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(140.0),
        height: NumberValue::Literal(140.0),
    }];

    let err = execute_draw_ops_for_test(&draw_ops, None).unwrap_err();
    assert!(matches!(
        err,
        RenderError::BarcodeEncode { symbology, data, .. }
            if symbology == "datamatrix" && data == too_long
    ));
}

#[test]
fn errors_when_datamatrix_barcode_geometry_is_invalid() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("MARMOT-DM-001".to_string()),
        symbology: crate::parser::BarcodeSymbology::DataMatrix,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(0.0),
        height: NumberValue::Literal(140.0),
    }];

    let err = execute_draw_ops_for_test(&draw_ops, None).unwrap_err();
    assert!(matches!(
        err,
        RenderError::InvalidBarcodeGeometry { width, height }
            if width == 0.0 && height == 140.0
    ));
}
