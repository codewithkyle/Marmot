use std::{collections::HashMap, path::PathBuf};

use super::*;
use crate::{
    parser::{DrawOp, NumberValue},
    resources::RegisteredAsset,
};

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
        assets: HashMap::<String, RegisteredAsset>::new(),
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
        assets: HashMap::<String, RegisteredAsset>::new(),
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
fn lowers_text_style_and_font_ops() {
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

    let render_ops = lower_draw_ops(&draw_ops, None).unwrap();

    assert_eq!(
        render_ops,
        vec![
            RenderOp::SetFontFamily {
                font: "Helvetica-Bold".to_string(),
            },
            RenderOp::SetFontSize { size: 14.0 },
            RenderOp::SetTextAlignment {
                align: TextAlign::Right,
            },
            RenderOp::SetVerticalAlignment {
                align: VerticalAlign::Bottom,
            },
            RenderOp::SetLineBreakMode {
                line_break: LineBreakMode::None,
            },
            RenderOp::SetTextFit {
                fit: TextFit::ShrinkToFit,
            },
            RenderOp::SetTextFitMinSize { min: 8.0 },
            RenderOp::SetTextFitMaxSize { max: 24.0 },
        ]
    );
}

#[test]
fn lowers_slot_driven_style_values_from_json_data() {
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

    let render_ops = lower_draw_ops(&draw_ops, Some(&data)).unwrap();

    assert_eq!(
        render_ops,
        vec![
            RenderOp::SetFontFamily {
                font: "Helvetica-Bold".to_string(),
            },
            RenderOp::SetRgb {
                r: 0.1,
                g: 0.2,
                b: 0.3,
            },
            RenderOp::SetStrokeWidth { width: 2.0 },
            RenderOp::SetFontSize { size: 16.0 },
            RenderOp::SetTextFitMinSize { min: 9.0 },
            RenderOp::SetTextFitMaxSize { max: 32.0 },
        ]
    );
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

    let err = lower_draw_ops(&draw_ops, None).unwrap_err();

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

    let err = lower_draw_ops(&draw_ops, Some(&data)).unwrap_err();

    assert!(matches!(
        err,
        RenderError::MissingSlot { slot } if slot == "product_name"
    ));
}

#[test]
#[should_panic(expected = "parser should prevent stroke without a current path")]
fn panics_when_stroke_has_no_pending_path() {
    let draw_ops = vec![DrawOp::Stroke];

    let _ = lower_draw_ops(&draw_ops, None);
}

#[test]
#[should_panic(expected = "parser should prevent fill without a current path")]
fn panics_when_fill_has_no_pending_path() {
    let draw_ops = vec![DrawOp::Fill];

    let _ = lower_draw_ops(&draw_ops, None);
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

    let _ = lower_draw_ops(&draw_ops, None);
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
fn lowers_image_draw_op() {
    use crate::parser::TextValue;
    let draw_ops = vec![DrawOp::Image {
        asset: TextValue::Literal("logo".to_string()),
        x: NumberValue::Literal(12.0),
        y: NumberValue::Literal(24.0),
        width: NumberValue::Literal(80.0),
        height: NumberValue::Literal(40.0),
    }];
    let render_ops = lower_draw_ops(&draw_ops, None).unwrap();
    assert_eq!(
        render_ops,
        vec![RenderOp::Image {
            asset: "logo".to_string(),
            x: 12.0,
            y: 24.0,
            width: 80.0,
            height: 40.0,
        }]
    );
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
    // create temp png
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
fn lowers_set_imagefit() {
    let draw_ops = vec![DrawOp::SetImageFit {
        fit: ImageFit::Cover,
    }];
    let render_ops = lower_draw_ops(&draw_ops, None).unwrap();
    assert_eq!(
        render_ops,
        vec![RenderOp::SetImageFit {
            fit: ImageFit::Cover,
        }]
    );
}
