use std::{collections::HashMap, path::PathBuf};

use super::*;
use crate::{
    parser::{
        DrawEntry, DrawOp, FrameDecl, FrameDrawBlock, LayerDecl, LayerDrawBlock, NumberValue, Page,
    },
    resources::{FrameScriptPlanEntry, LayerScriptPlanEntry, RegisteredAsset},
};
use serde_json::Value;

fn default_layers() -> Vec<LayerDecl> {
    vec![LayerDecl {
        index: 1,
        id: "LAYER_1".to_string(),
        frames: vec![FrameDecl {
            index: 1,
            id: "FRAME_1".to_string(),
        }],
    }]
}

fn as_draw_layers(draw_ops: &[DrawOp]) -> Vec<LayerDrawBlock> {
    vec![LayerDrawBlock {
        index: 1,
        frames: vec![FrameDrawBlock {
            index: 1,
            ops: draw_ops.to_vec(),
        }],
    }]
}

fn default_frames() -> Vec<FrameDecl> {
    default_layers()[0].frames.clone()
}

fn as_draw_entries(draw_ops: &[DrawOp]) -> Vec<DrawEntry> {
    as_draw_layers(draw_ops)
        .into_iter()
        .map(DrawEntry::Layer)
        .collect()
}

fn as_top_level_draw_entries(draw_ops: &[DrawOp]) -> Vec<DrawEntry> {
    vec![DrawEntry::Frame(FrameDrawBlock {
        index: 1,
        ops: draw_ops.to_vec(),
    })]
}

fn empty_render_context() -> RenderContext {
    RenderContext {
        fonts: HashMap::new(),
        assets: HashMap::new(),
        scripts: HashMap::new(),
        layer_script_plan: Vec::new(),
        frame_script_plan: Vec::new(),
    }
}

fn host_assets_disabled() -> HostAssetPolicy {
    HostAssetPolicy {
        allow: false,
        cwd: std::env::current_dir().unwrap(),
    }
}

fn host_assets_enabled(cwd: PathBuf) -> HostAssetPolicy {
    HostAssetPolicy { allow: true, cwd }
}

fn render_context_with_assets(assets: HashMap<String, RegisteredAsset>) -> RenderContext {
    RenderContext {
        fonts: HashMap::new(),
        assets,
        scripts: HashMap::new(),
        layer_script_plan: Vec::new(),
        frame_script_plan: Vec::new(),
    }
}

fn scripted_context_for_default_frame(scripts: HashMap<String, String>) -> RenderContext {
    let mut frame_script_plan = Vec::new();
    if scripts.contains_key("FRAME_1") {
        frame_script_plan.push(FrameScriptPlanEntry {
            frame_index: 1,
            frame_id: "FRAME_1".to_string(),
        });
    }

    RenderContext {
        fonts: HashMap::new(),
        assets: HashMap::new(),
        scripts,
        layer_script_plan: Vec::new(),
        frame_script_plan,
    }
}

fn scripted_context_for_default_layer(scripts: HashMap<String, String>) -> RenderContext {
    let mut layer_script_plan = Vec::new();
    if scripts.contains_key("LAYER_1") {
        layer_script_plan.push(LayerScriptPlanEntry {
            layer_index: 1,
            layer_id: "LAYER_1".to_string(),
        });
    }

    RenderContext {
        fonts: HashMap::new(),
        assets: HashMap::new(),
        scripts,
        layer_script_plan,
        frame_script_plan: Vec::new(),
    }
}

fn execute_draw_ops_for_test(draw_ops: &[DrawOp], data: Option<&Value>) -> Result<(), RenderError> {
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 256, 256)?;
    let ctx = cairo::Context::new(&surface)?;
    let mut cache = RenderCache::default();
    let render_context = empty_render_context();
    let layers = default_layers();
    let draw_entries = as_draw_entries(draw_ops);
    let layer_state = build_initial_layer_state(&layers);
    let frame_state = build_initial_frame_state(&layers);

    execute_draw(
        &ctx,
        &draw_entries,
        &layer_state,
        &frame_state,
        data,
        &render_context,
        &mut cache,
        &host_assets_disabled(),
    )
    .map(|_| ())
}

fn execute_draw_ops_with_runtime_for_test(
    draw_ops: &[DrawOp],
    runtime: FrameRuntimeState,
) -> Result<RenderWarnings, RenderError> {
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 256, 256)?;
    let ctx = cairo::Context::new(&surface)?;
    let mut cache = RenderCache::default();
    let render_context = empty_render_context();
    let layers = default_layers();
    let draw_entries = as_draw_entries(draw_ops);
    let layer_state = build_initial_layer_state(&layers);
    let mut frame_state = build_initial_frame_state(&layers);
    frame_state.insert(1, runtime);

    execute_draw(
        &ctx,
        &draw_entries,
        &layer_state,
        &frame_state,
        None,
        &render_context,
        &mut cache,
        &host_assets_disabled(),
    )
}

fn render_pdf_for_test(
    page: &Page,
    draw_ops: &[DrawOp],
    output_path: &std::path::Path,
    data: Option<&Value>,
    context: &RenderContext,
) -> Result<(), RenderError> {
    let layers = default_layers();
    let frames = default_frames();
    let draw_entries = as_draw_entries(draw_ops);
    render_pdf(
        page,
        &frames,
        &layers,
        &draw_entries,
        output_path,
        data,
        context,
        &host_assets_disabled(),
    )
    .map(|_| ())
}

fn render_pdf_for_test_with_entries(
    page: &Page,
    draw_entries: &[DrawEntry],
    output_path: &std::path::Path,
    data: Option<&Value>,
    context: &RenderContext,
) -> Result<(), RenderError> {
    let layers = default_layers();
    let frames = default_frames();
    render_pdf(
        page,
        &frames,
        &layers,
        draw_entries,
        output_path,
        data,
        context,
        &host_assets_disabled(),
    )
    .map(|_| ())
}

fn render_png_for_test(
    page: &Page,
    draw_ops: &[DrawOp],
    output_path: &std::path::Path,
    data: Option<&Value>,
    context: &RenderContext,
) -> Result<(), RenderError> {
    let layers = default_layers();
    let frames = default_frames();
    let draw_entries = as_draw_entries(draw_ops);
    render_png(
        page,
        &frames,
        &layers,
        &draw_entries,
        output_path,
        data,
        context,
        &host_assets_disabled(),
        74,
        None,
        None,
    )
    .map(|_| ())
}

fn render_pdf_with_cache_for_test(
    page: &Page,
    draw_ops: &[DrawOp],
    output_path: &std::path::Path,
    data: Option<&Value>,
    context: &RenderContext,
    cache: &mut RenderCache,
) -> Result<(), RenderError> {
    let layers = default_layers();
    let draw_entries = as_draw_entries(draw_ops);
    let surface = PdfSurface::new(page.width, page.height, output_path)?;
    let ctx = Context::new(&surface)?;

    let mut layer_state = build_initial_layer_state(&layers);
    let mut frame_state = build_initial_frame_state(&layers);
    run_layer_scripts(&mut layer_state, data, context, &mut cache.script_runtime)?;
    run_frame_scripts(&mut frame_state, data, context, &mut cache.script_runtime)?;
    execute_draw(
        &ctx,
        &draw_entries,
        &layer_state,
        &frame_state,
        data,
        context,
        cache,
        &host_assets_disabled(),
    )?;
    surface.finish();

    Ok(())
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

    let render_context = empty_render_context();

    render_pdf_for_test(&page, &draw_ops, &output_path, None, &render_context).unwrap();

    let metadata = fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);

    let _ = fs::remove_file(output_path);
}

#[test]
fn renders_basic_png() {
    use crate::parser::{DrawOp, NumberValue, Page};
    use std::fs;

    let page = Page {
        width: 128.0,
        height: 96.0,
    };

    let draw_ops = vec![
        DrawOp::SetRgb {
            r: NumberValue::Literal(0.0),
            g: NumberValue::Literal(1.0),
            b: NumberValue::Literal(0.0),
        },
        DrawOp::RectPath {
            x: NumberValue::Literal(0.0),
            y: NumberValue::Literal(0.0),
            width: NumberValue::Literal(128.0),
            height: NumberValue::Literal(96.0),
        },
        DrawOp::Fill,
    ];

    let output_path = std::env::temp_dir().join("marmot_basic_render_test.png");
    let render_context = empty_render_context();

    render_png_for_test(&page, &draw_ops, &output_path, None, &render_context).unwrap();

    let metadata = fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);

    let png_bytes = fs::read(&output_path).unwrap();
    assert!(png_bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]));

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

    let render_context = empty_render_context();

    render_pdf_for_test(&page, &draw_ops, &output_path, None, &render_context).unwrap();

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
        ..empty_render_context()
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
    let context = empty_render_context();

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
    let render_context = empty_render_context();

    let err =
        render_pdf_for_test(&page, &draw_ops, &output_path, None, &render_context).unwrap_err();

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
fn loadimage_errors_when_host_assets_disabled() {
    use crate::parser::TextValue;

    let draw_ops = vec![DrawOp::LoadImage {
        path: TextValue::Literal("./logos/sprout-basket.png".to_string()),
        alias: TextValue::Literal("customer_logo".to_string()),
    }];

    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 64, 64).unwrap();
    let ctx = cairo::Context::new(&surface).unwrap();
    let mut cache = RenderCache::default();
    let context = empty_render_context();
    let layers = default_layers();
    let draw_entries = as_draw_entries(&draw_ops);
    let layer_state = build_initial_layer_state(&layers);
    let frame_state = build_initial_frame_state(&layers);

    let err = execute_draw(
        &ctx,
        &draw_entries,
        &layer_state,
        &frame_state,
        None,
        &context,
        &mut cache,
        &host_assets_disabled(),
    )
    .unwrap_err();

    assert!(matches!(err, RenderError::HostAssetAccessDenied { .. }));
}

#[test]
fn loadimage_reads_host_asset_and_renders_image() {
    use crate::parser::TextValue;
    use image::{ImageFormat, Rgba, RgbaImage};

    let dir = tempfile::tempdir().unwrap();
    let logos_dir = dir.path().join("logos");
    std::fs::create_dir_all(&logos_dir).unwrap();
    let image_path = logos_dir.join("customer.png");
    let img = RgbaImage::from_pixel(8, 8, Rgba([255, 0, 0, 255]));
    img.save_with_format(&image_path, ImageFormat::Png).unwrap();

    let draw_ops = vec![
        DrawOp::LoadImage {
            path: TextValue::Literal("./logos/customer.png".to_string()),
            alias: TextValue::Literal("customer_logo".to_string()),
        },
        DrawOp::Image {
            asset: TextValue::Literal("customer_logo".to_string()),
            x: NumberValue::Literal(0.0),
            y: NumberValue::Literal(0.0),
            width: NumberValue::Literal(32.0),
            height: NumberValue::Literal(32.0),
        },
    ];

    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 64, 64).unwrap();
    let ctx = cairo::Context::new(&surface).unwrap();
    let mut cache = RenderCache::default();
    let context = empty_render_context();
    let layers = default_layers();
    let draw_entries = as_draw_entries(&draw_ops);
    let layer_state = build_initial_layer_state(&layers);
    let frame_state = build_initial_frame_state(&layers);

    execute_draw(
        &ctx,
        &draw_entries,
        &layer_state,
        &frame_state,
        None,
        &context,
        &mut cache,
        &host_assets_enabled(dir.path().to_path_buf()),
    )
    .unwrap();

    assert_eq!(cache.runtime_image_assets.len(), 1);
    assert_eq!(cache.image_surfaces.len(), 1);
    assert_eq!(cache.scaled_image_surfaces.len(), 1);
}

#[test]
fn loadimage_reuses_cache_for_same_path_under_two_aliases() {
    use crate::parser::TextValue;
    use image::{ImageFormat, Rgba, RgbaImage};

    let dir = tempfile::tempdir().unwrap();
    let logos_dir = dir.path().join("logos");
    std::fs::create_dir_all(&logos_dir).unwrap();
    let image_path = logos_dir.join("customer.png");
    let img = RgbaImage::from_pixel(8, 8, Rgba([255, 0, 0, 255]));
    img.save_with_format(&image_path, ImageFormat::Png).unwrap();

    let draw_ops = vec![
        DrawOp::LoadImage {
            path: TextValue::Literal("./logos/customer.png".to_string()),
            alias: TextValue::Literal("logo_a".to_string()),
        },
        DrawOp::LoadImage {
            path: TextValue::Literal("./logos/customer.png".to_string()),
            alias: TextValue::Literal("logo_b".to_string()),
        },
        DrawOp::Image {
            asset: TextValue::Literal("logo_a".to_string()),
            x: NumberValue::Literal(0.0),
            y: NumberValue::Literal(0.0),
            width: NumberValue::Literal(32.0),
            height: NumberValue::Literal(32.0),
        },
        DrawOp::Image {
            asset: TextValue::Literal("logo_b".to_string()),
            x: NumberValue::Literal(40.0),
            y: NumberValue::Literal(0.0),
            width: NumberValue::Literal(32.0),
            height: NumberValue::Literal(32.0),
        },
    ];

    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 96, 64).unwrap();
    let ctx = cairo::Context::new(&surface).unwrap();
    let mut cache = RenderCache::default();
    let context = empty_render_context();
    let layers = default_layers();
    let draw_entries = as_draw_entries(&draw_ops);
    let layer_state = build_initial_layer_state(&layers);
    let frame_state = build_initial_frame_state(&layers);

    execute_draw(
        &ctx,
        &draw_entries,
        &layer_state,
        &frame_state,
        None,
        &context,
        &mut cache,
        &host_assets_enabled(dir.path().to_path_buf()),
    )
    .unwrap();

    assert_eq!(cache.runtime_image_assets.len(), 2);
    assert_eq!(cache.image_surfaces.len(), 1);
    assert_eq!(cache.scaled_image_surfaces.len(), 1);
}

#[test]
fn loadimage_accepts_absolute_host_path() {
    use crate::parser::TextValue;
    use image::{ImageFormat, Rgba, RgbaImage};

    let dir = tempfile::tempdir().unwrap();
    let image_path = dir.path().join("customer.png");
    let img = RgbaImage::from_pixel(8, 8, Rgba([255, 0, 0, 255]));
    img.save_with_format(&image_path, ImageFormat::Png).unwrap();

    let draw_ops = vec![
        DrawOp::LoadImage {
            path: TextValue::Literal(image_path.to_string_lossy().to_string()),
            alias: TextValue::Literal("customer_logo".to_string()),
        },
        DrawOp::Image {
            asset: TextValue::Literal("customer_logo".to_string()),
            x: NumberValue::Literal(0.0),
            y: NumberValue::Literal(0.0),
            width: NumberValue::Literal(32.0),
            height: NumberValue::Literal(32.0),
        },
    ];

    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 64, 64).unwrap();
    let ctx = cairo::Context::new(&surface).unwrap();
    let mut cache = RenderCache::default();
    let context = empty_render_context();
    let layers = default_layers();
    let draw_entries = as_draw_entries(&draw_ops);
    let layer_state = build_initial_layer_state(&layers);
    let frame_state = build_initial_frame_state(&layers);

    execute_draw(
        &ctx,
        &draw_entries,
        &layer_state,
        &frame_state,
        None,
        &context,
        &mut cache,
        &host_assets_enabled(PathBuf::from("/")),
    )
    .unwrap();

    assert_eq!(cache.runtime_image_assets.len(), 1);
    assert_eq!(cache.image_surfaces.len(), 1);
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
    let render_context = empty_render_context();
    let err =
        render_pdf_for_test(&page, &draw_ops, &output_path, None, &render_context).unwrap_err();
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
    let render_context = render_context_with_assets(assets);
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
    render_pdf_for_test(&page, &draw_ops, &output_path, None, &render_context).unwrap();
    let metadata = fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);
    let _ = fs::remove_file(output_path);
}

#[test]
fn reuses_scaled_surface_for_same_asset_geometry_and_fit() {
    use crate::parser::{AssetType, Page, TextValue};
    use crate::resources::RegisteredImageInfo;
    use image::{ImageFormat, Rgba, RgbaImage};
    use std::fs;

    let dir = tempfile::tempdir().unwrap();
    let image_path = dir.path().join("logo.png");
    let img = RgbaImage::from_pixel(16, 16, Rgba([255, 0, 0, 255]));
    img.save_with_format(&image_path, ImageFormat::Png).unwrap();
    let byte_len = fs::metadata(&image_path).unwrap().len();

    let mut assets = HashMap::new();
    assets.insert(
        "logo".to_string(),
        RegisteredAsset {
            path: image_path,
            name: "logo".to_string(),
            ty: AssetType::Image,
            byte_len,
            image: Some(RegisteredImageInfo {
                format: "png".to_string(),
                width: 16,
                height: 16,
            }),
        },
    );

    let render_context = render_context_with_assets(assets);

    let page = Page {
        width: 120.0,
        height: 120.0,
    };

    let draw_ops = vec![
        DrawOp::SetImageFit {
            fit: ImageFit::Contain,
        },
        DrawOp::Image {
            asset: TextValue::Literal("logo".to_string()),
            x: NumberValue::Literal(10.0),
            y: NumberValue::Literal(10.0),
            width: NumberValue::Literal(80.0),
            height: NumberValue::Literal(60.0),
        },
    ];

    let mut cache = RenderCache::default();
    let output_a = dir.path().join("a.pdf");
    let output_b = dir.path().join("b.pdf");

    render_pdf_with_cache_for_test(
        &page,
        &draw_ops,
        &output_a,
        None,
        &render_context,
        &mut cache,
    )
    .unwrap();
    render_pdf_with_cache_for_test(
        &page,
        &draw_ops,
        &output_b,
        None,
        &render_context,
        &mut cache,
    )
    .unwrap();

    assert_eq!(cache.image_surfaces.len(), 1);
    assert_eq!(cache.scaled_image_surfaces.len(), 1);
}

#[test]
fn creates_distinct_scaled_surfaces_for_distinct_geometry() {
    use crate::parser::{AssetType, Page, TextValue};
    use crate::resources::RegisteredImageInfo;
    use image::{ImageFormat, Rgba, RgbaImage};
    use std::fs;

    let dir = tempfile::tempdir().unwrap();
    let image_path = dir.path().join("logo.png");
    let img = RgbaImage::from_pixel(16, 16, Rgba([255, 0, 0, 255]));
    img.save_with_format(&image_path, ImageFormat::Png).unwrap();
    let byte_len = fs::metadata(&image_path).unwrap().len();

    let mut assets = HashMap::new();
    assets.insert(
        "logo".to_string(),
        RegisteredAsset {
            path: image_path,
            name: "logo".to_string(),
            ty: AssetType::Image,
            byte_len,
            image: Some(RegisteredImageInfo {
                format: "png".to_string(),
                width: 16,
                height: 16,
            }),
        },
    );

    let render_context = render_context_with_assets(assets);

    let page = Page {
        width: 120.0,
        height: 120.0,
    };

    let mut cache = RenderCache::default();
    let output_a = dir.path().join("c.pdf");
    let output_b = dir.path().join("d.pdf");

    let draw_ops_a = vec![
        DrawOp::SetImageFit {
            fit: ImageFit::Stretch,
        },
        DrawOp::Image {
            asset: TextValue::Literal("logo".to_string()),
            x: NumberValue::Literal(10.0),
            y: NumberValue::Literal(10.0),
            width: NumberValue::Literal(80.0),
            height: NumberValue::Literal(60.0),
        },
    ];

    let draw_ops_b = vec![
        DrawOp::SetImageFit {
            fit: ImageFit::Stretch,
        },
        DrawOp::Image {
            asset: TextValue::Literal("logo".to_string()),
            x: NumberValue::Literal(10.0),
            y: NumberValue::Literal(10.0),
            width: NumberValue::Literal(60.0),
            height: NumberValue::Literal(80.0),
        },
    ];

    render_pdf_with_cache_for_test(
        &page,
        &draw_ops_a,
        &output_a,
        None,
        &render_context,
        &mut cache,
    )
    .unwrap();
    render_pdf_with_cache_for_test(
        &page,
        &draw_ops_b,
        &output_b,
        None,
        &render_context,
        &mut cache,
    )
    .unwrap();

    assert_eq!(cache.image_surfaces.len(), 1);
    assert_eq!(cache.scaled_image_surfaces.len(), 2);
}

#[test]
fn errors_when_registered_image_geometry_is_invalid() {
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
            path: image_path,
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
        scripts: HashMap::new(),
        layer_script_plan: Vec::new(),
        frame_script_plan: Vec::new(),
    };

    let page = Page {
        width: 120.0,
        height: 120.0,
    };
    let draw_ops = vec![DrawOp::Image {
        asset: TextValue::Literal("logo".to_string()),
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(10.0),
        width: NumberValue::Literal(0.0),
        height: NumberValue::Literal(10.0),
    }];

    let output = dir.path().join("invalid-geom.pdf");
    let err = render_pdf_for_test(&page, &draw_ops, &output, None, &render_context).unwrap_err();
    assert!(matches!(
        err,
        RenderError::InvalidImageGeometry { width, height, .. }
            if width == 0.0 && height == 10.0
    ));
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
fn executes_msi_barcode_draw_op() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("1234567".to_string()),
        symbology: crate::parser::BarcodeSymbology::MSI,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(220.0),
        height: NumberValue::Literal(50.0),
    }];

    execute_draw_ops_for_test(&draw_ops, None).unwrap();
}

#[test]
fn errors_when_msi_data_is_invalid() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("12A45".to_string()),
        symbology: crate::parser::BarcodeSymbology::MSI,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(20.0),
        width: NumberValue::Literal(220.0),
        height: NumberValue::Literal(50.0),
    }];

    let err = execute_draw_ops_for_test(&draw_ops, None).unwrap_err();
    assert!(matches!(
        err,
        RenderError::BarcodeEncode { symbology, data, .. }
            if symbology == "msi" && data == "12A45"
    ));
}

#[test]
fn errors_when_msi_barcode_geometry_is_invalid() {
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("1234567".to_string()),
        symbology: crate::parser::BarcodeSymbology::MSI,
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

#[test]
fn caches_scaled_surface_once_for_repeated_same_geometry() {
    use crate::parser::AssetType;
    use crate::resources::RegisteredImageInfo;
    use image::{ImageFormat, Rgba, RgbaImage};
    use std::fs;
    let dir = tempfile::tempdir().unwrap();
    let image_path = dir.path().join("logo.png");
    let img = RgbaImage::from_pixel(8, 8, Rgba([255, 0, 0, 255]));
    img.save_with_format(&image_path, ImageFormat::Png).unwrap();
    let mut assets = HashMap::new();
    assets.insert(
        "logo".to_string(),
        RegisteredAsset {
            path: image_path.clone(),
            name: "logo".to_string(),
            ty: AssetType::Image,
            byte_len: fs::metadata(&image_path).unwrap().len(),
            image: Some(RegisteredImageInfo {
                format: "png".to_string(),
                width: 8,
                height: 8,
            }),
        },
    );
    let context = render_context_with_assets(assets);
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 300, 300).unwrap();
    let ctx = cairo::Context::new(&surface).unwrap();
    let mut cache = RenderCache::default();
    render_image(
        &ctx,
        &context,
        &mut cache,
        "logo",
        ImageFit::Stretch,
        10.0,
        10.0,
        80.0,
        40.0,
    )
    .unwrap();
    render_image(
        &ctx,
        &context,
        &mut cache,
        "logo",
        ImageFit::Stretch,
        20.0,
        20.0,
        80.0,
        40.0,
    )
    .unwrap();
    assert_eq!(cache.image_surfaces.len(), 1);
    assert_eq!(cache.scaled_image_surfaces.len(), 1);
}

#[test]
fn scaled_cache_dimensions_match_expected_oversample_size() {
    use crate::parser::AssetType;
    use crate::resources::RegisteredImageInfo;
    use image::{ImageFormat, Rgba, RgbaImage};
    use std::fs;
    let dir = tempfile::tempdir().unwrap();
    let image_path = dir.path().join("logo.png");
    let img = RgbaImage::from_pixel(8, 8, Rgba([255, 0, 0, 255]));
    img.save_with_format(&image_path, ImageFormat::Png).unwrap();
    let mut assets = HashMap::new();
    assets.insert(
        "logo".to_string(),
        RegisteredAsset {
            path: image_path.clone(),
            name: "logo".to_string(),
            ty: AssetType::Image,
            byte_len: fs::metadata(&image_path).unwrap().len(),
            image: Some(RegisteredImageInfo {
                format: "png".to_string(),
                width: 8,
                height: 8,
            }),
        },
    );
    let context = render_context_with_assets(assets);
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 300, 300).unwrap();
    let ctx = cairo::Context::new(&surface).unwrap();
    let mut cache = RenderCache::default();
    render_image(
        &ctx,
        &context,
        &mut cache,
        "logo",
        ImageFit::Stretch,
        0.0,
        0.0,
        80.0,
        40.0,
    )
    .unwrap();
    let key = cache.scaled_image_surfaces.keys().next().unwrap();
    let expected_w = (80.0 * IMAGE_CACHE_SCALE).round() as i32;
    let expected_h = (40.0 * IMAGE_CACHE_SCALE).round() as i32;
    assert_eq!(key.width_px, expected_w);
    assert_eq!(key.height_px, expected_h);
}

#[test]
fn errors_when_image_geometry_is_invalid() {
    use crate::parser::{AssetType, TextValue};
    use crate::resources::RegisteredImageInfo;
    use image::{ImageFormat, Rgba, RgbaImage};
    use std::fs;
    let dir = tempfile::tempdir().unwrap();
    let image_path = dir.path().join("logo.png");
    let img = RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255]));
    img.save_with_format(&image_path, ImageFormat::Png).unwrap();
    let mut assets = HashMap::new();
    assets.insert(
        "logo".to_string(),
        RegisteredAsset {
            path: image_path.clone(),
            name: "logo".to_string(),
            ty: AssetType::Image,
            byte_len: fs::metadata(&image_path).unwrap().len(),
            image: Some(RegisteredImageInfo {
                format: "png".to_string(),
                width: 2,
                height: 2,
            }),
        },
    );
    let draw_ops = vec![DrawOp::Image {
        asset: TextValue::Literal("logo".to_string()),
        x: NumberValue::Literal(0.0),
        y: NumberValue::Literal(0.0),
        width: NumberValue::Literal(0.0),
        height: NumberValue::Literal(10.0),
    }];
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 64, 64).unwrap();
    let ctx = cairo::Context::new(&surface).unwrap();
    let mut cache = RenderCache::default();
    let context = render_context_with_assets(assets);
    let layers = default_layers();
    let draw_entries = as_draw_entries(&draw_ops);
    let layer_state = build_initial_layer_state(&layers);
    let frame_state = build_initial_frame_state(&layers);
    let err = execute_draw(
        &ctx,
        &draw_entries,
        &layer_state,
        &frame_state,
        None,
        &context,
        &mut cache,
        &host_assets_disabled(),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        RenderError::InvalidImageGeometry { width, height, .. }
            if width == 0.0 && height == 10.0
    ));
}

#[test]
fn script_visibility_false_skips_frame_draw() {
    use crate::parser::{BarcodeSymbology, TextValue};

    let page = Page {
        width: 100.0,
        height: 100.0,
    };
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("ABC".to_string()),
        symbology: BarcodeSymbology::EAN8,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(10.0),
        width: NumberValue::Literal(60.0),
        height: NumberValue::Literal(20.0),
    }];
    let output_path = std::env::temp_dir().join("marmot_script_visibility_skip_test.pdf");

    let mut scripts = HashMap::new();
    scripts.insert("FRAME_1".to_string(), "frame.visible = false".to_string());

    let render_context = scripted_context_for_default_frame(scripts);

    render_pdf_for_test(&page, &draw_ops, &output_path, None, &render_context).unwrap();

    let metadata = std::fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn script_visibility_false_skips_top_level_frame_draw() {
    use crate::parser::{BarcodeSymbology, TextValue};

    let page = Page {
        width: 100.0,
        height: 100.0,
    };
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Slot("sku".to_string()),
        symbology: BarcodeSymbology::EAN8,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(10.0),
        width: NumberValue::Literal(60.0),
        height: NumberValue::Literal(20.0),
    }];
    let draw_entries = as_top_level_draw_entries(&draw_ops);
    let output_path = std::env::temp_dir().join("marmot_top_level_frame_visibility_skip_test.pdf");

    let mut scripts = HashMap::new();
    scripts.insert("FRAME_1".to_string(), "frame.visible = false".to_string());

    let render_context = scripted_context_for_default_frame(scripts);

    // If top-level frame visibility is ignored, this would fail because data is missing for $(sku).
    render_pdf_for_test_with_entries(&page, &draw_entries, &output_path, None, &render_context)
        .unwrap();

    let metadata = std::fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn layer_visibility_false_skips_all_frames_in_layer() {
    use crate::parser::{BarcodeSymbology, TextValue};

    let page = Page {
        width: 100.0,
        height: 100.0,
    };
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Literal("ABC".to_string()),
        symbology: BarcodeSymbology::EAN8,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(10.0),
        width: NumberValue::Literal(60.0),
        height: NumberValue::Literal(20.0),
    }];
    let output_path = std::env::temp_dir().join("marmot_layer_visibility_skip_test.pdf");

    let mut scripts = HashMap::new();
    scripts.insert("LAYER_1".to_string(), "layer.visible = false".to_string());

    let render_context = scripted_context_for_default_layer(scripts);

    render_pdf_for_test(&page, &draw_ops, &output_path, None, &render_context).unwrap();

    let metadata = std::fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn script_value_override_applies_to_textbox() {
    use crate::parser::TextValue;

    let page = Page {
        width: 120.0,
        height: 120.0,
    };
    let draw_ops = vec![DrawOp::TextBox {
        text: TextValue::Slot("product_name".to_string()),
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(10.0),
        width: NumberValue::Literal(100.0),
        height: NumberValue::Literal(40.0),
    }];
    let output_path = std::env::temp_dir().join("marmot_script_text_override_test.pdf");

    let mut scripts = HashMap::new();
    scripts.insert(
        "FRAME_1".to_string(),
        "frame.value = \"OVERRIDE\"".to_string(),
    );

    let render_context = scripted_context_for_default_frame(scripts);

    render_pdf_for_test(&page, &draw_ops, &output_path, None, &render_context).unwrap();

    let metadata = std::fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn script_value_override_applies_to_image() {
    use crate::parser::{AssetType, TextValue};
    use crate::resources::RegisteredImageInfo;
    use image::{ImageFormat, Rgba, RgbaImage};

    let dir = tempfile::tempdir().unwrap();
    let image_path = dir.path().join("logo.png");
    let img = RgbaImage::from_pixel(4, 4, Rgba([255, 0, 0, 255]));
    img.save_with_format(&image_path, ImageFormat::Png).unwrap();

    let mut assets = HashMap::new();
    assets.insert(
        "logo".to_string(),
        RegisteredAsset {
            path: image_path.clone(),
            name: "logo".to_string(),
            ty: AssetType::Image,
            byte_len: std::fs::metadata(&image_path).unwrap().len(),
            image: Some(RegisteredImageInfo {
                format: "png".to_string(),
                width: 4,
                height: 4,
            }),
        },
    );

    let page = Page {
        width: 120.0,
        height: 120.0,
    };
    let draw_ops = vec![DrawOp::Image {
        asset: TextValue::Slot("asset_name".to_string()),
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(10.0),
        width: NumberValue::Literal(60.0),
        height: NumberValue::Literal(40.0),
    }];
    let output_path = dir.path().join("marmot_script_image_override_test.pdf");

    let mut scripts = HashMap::new();
    scripts.insert("FRAME_1".to_string(), "frame.value = \"logo\"".to_string());

    let mut render_context = scripted_context_for_default_frame(scripts);
    render_context.assets = assets;

    render_pdf_for_test(&page, &draw_ops, &output_path, None, &render_context).unwrap();
    let metadata = std::fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);
}

#[test]
fn script_value_override_applies_to_barcode() {
    use crate::parser::{BarcodeSymbology, TextValue};

    let page = Page {
        width: 120.0,
        height: 120.0,
    };
    let draw_ops = vec![DrawOp::Barcode {
        value: TextValue::Slot("barcode_value".to_string()),
        symbology: BarcodeSymbology::Code39,
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(10.0),
        width: NumberValue::Literal(80.0),
        height: NumberValue::Literal(20.0),
    }];
    let output_path = std::env::temp_dir().join("marmot_script_barcode_override_test.pdf");

    let mut scripts = HashMap::new();
    scripts.insert(
        "FRAME_1".to_string(),
        "frame.value = \"ABC123\"".to_string(),
    );

    let render_context = scripted_context_for_default_frame(scripts);

    render_pdf_for_test(&page, &draw_ops, &output_path, None, &render_context).unwrap();

    let metadata = std::fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn script_style_overrides_apply_to_fill_stroke_and_text() {
    use crate::parser::TextValue;

    let page = Page {
        width: 140.0,
        height: 120.0,
    };
    let draw_ops = vec![
        DrawOp::RectPath {
            x: NumberValue::Literal(10.0),
            y: NumberValue::Literal(10.0),
            width: NumberValue::Literal(50.0),
            height: NumberValue::Literal(30.0),
        },
        DrawOp::Fill,
        DrawOp::RectPath {
            x: NumberValue::Literal(10.0),
            y: NumberValue::Literal(10.0),
            width: NumberValue::Literal(50.0),
            height: NumberValue::Literal(30.0),
        },
        DrawOp::Stroke,
        DrawOp::LinePath {
            x1: NumberValue::Literal(70.0),
            y1: NumberValue::Literal(10.0),
            x2: NumberValue::Literal(120.0),
            y2: NumberValue::Literal(40.0),
        },
        DrawOp::Stroke,
        DrawOp::TextBox {
            text: TextValue::Literal("demo".to_string()),
            x: NumberValue::Literal(10.0),
            y: NumberValue::Literal(60.0),
            width: NumberValue::Literal(100.0),
            height: NumberValue::Literal(30.0),
        },
    ];
    let output_path = std::env::temp_dir().join("marmot_script_style_overrides_test.pdf");

    let mut scripts = HashMap::new();
    scripts.insert(
        "FRAME_1".to_string(),
        r#"
            frame.fill_color = { r = 0.25, g = 0.25, b = 0.25 }
            frame.stroke_color = parse_rgb("0.1 0.2 0.3")
            frame.stroke_width = 2
            frame.text_color = cmyk_to_rgb(0.25, 0.25, 0.25, 0.25)
        "#
        .to_string(),
    );

    let render_context = scripted_context_for_default_frame(scripts);

    render_pdf_for_test(&page, &draw_ops, &output_path, None, &render_context).unwrap();

    let metadata = std::fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn invalid_script_visible_assignment_fails_render() {
    use crate::parser::TextValue;

    let page = Page {
        width: 100.0,
        height: 100.0,
    };
    let draw_ops = vec![DrawOp::TextBox {
        text: TextValue::Literal("ok".to_string()),
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(10.0),
        width: NumberValue::Literal(80.0),
        height: NumberValue::Literal(20.0),
    }];
    let output_path = std::env::temp_dir().join("marmot_script_invalid_visible_test.pdf");

    let mut scripts = HashMap::new();
    scripts.insert("FRAME_1".to_string(), "frame.visible = \"no\"".to_string());

    let render_context = scripted_context_for_default_frame(scripts);

    let err =
        render_pdf_for_test(&page, &draw_ops, &output_path, None, &render_context).unwrap_err();

    assert!(matches!(err, RenderError::ScriptRuntime { .. }));
    let msg = format!("{err:?}");
    assert!(msg.contains("frame.visible"));
}

#[test]
fn invalid_script_value_assignment_fails_render() {
    use crate::parser::TextValue;

    let page = Page {
        width: 100.0,
        height: 100.0,
    };
    let draw_ops = vec![DrawOp::TextBox {
        text: TextValue::Literal("ok".to_string()),
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(10.0),
        width: NumberValue::Literal(80.0),
        height: NumberValue::Literal(20.0),
    }];
    let output_path = std::env::temp_dir().join("marmot_script_invalid_value_test.pdf");

    let mut scripts = HashMap::new();
    scripts.insert("FRAME_1".to_string(), "frame.value = 123".to_string());

    let render_context = scripted_context_for_default_frame(scripts);

    let err =
        render_pdf_for_test(&page, &draw_ops, &output_path, None, &render_context).unwrap_err();

    assert!(matches!(err, RenderError::ScriptRuntime { .. }));
    let msg = format!("{err:?}");
    assert!(msg.contains("frame.value"));
}

#[test]
fn script_runtime_error_fails_render() {
    use crate::parser::TextValue;

    let page = Page {
        width: 100.0,
        height: 100.0,
    };
    let draw_ops = vec![DrawOp::TextBox {
        text: TextValue::Literal("ok".to_string()),
        x: NumberValue::Literal(10.0),
        y: NumberValue::Literal(10.0),
        width: NumberValue::Literal(80.0),
        height: NumberValue::Literal(20.0),
    }];
    let output_path = std::env::temp_dir().join("marmot_script_runtime_error_test.pdf");

    let mut scripts = HashMap::new();
    scripts.insert("FRAME_1".to_string(), "error(\"boom\")".to_string());

    let render_context = scripted_context_for_default_frame(scripts);

    let err =
        render_pdf_for_test(&page, &draw_ops, &output_path, None, &render_context).unwrap_err();

    assert!(matches!(err, RenderError::ScriptRuntime { .. }));
    let msg = format!("{err:?}");
    assert!(msg.contains("boom"));
}

#[test]
fn warns_when_text_color_override_is_unused() {
    let draw_ops = vec![
        DrawOp::RectPath {
            x: NumberValue::Literal(10.0),
            y: NumberValue::Literal(20.0),
            width: NumberValue::Literal(30.0),
            height: NumberValue::Literal(40.0),
        },
        DrawOp::Fill,
    ];

    let runtime = FrameRuntimeState {
        visible: true,
        value_override: None,
        fill_color_override: None,
        stroke_color_override: None,
        stroke_width_override: None,
        text_color_override: Some(RuntimeRgb {
            r: 0.2,
            g: 0.3,
            b: 0.4,
        }),
    };

    let warnings = execute_draw_ops_with_runtime_for_test(&draw_ops, runtime).unwrap();
    assert_eq!(warnings.unused_text_color_frames, vec![1]);
}

#[test]
fn remap_config_errors_when_dither_enabled_but_palette_missing() {
    let err = match build_image_remap_config(Some(DitherType::Floyd), None) {
        Ok(_) => panic!("expected missing-palette error"),
        Err(err) => err,
    };
    assert!(matches!(err, RenderError::RemapMissingPalette));
}

#[test]
fn remap_config_errors_on_invalid_palette_source() {
    let err = match build_image_remap_config(Some(DitherType::Floyd), Some("not-a-color\n")) {
        Ok(_) => panic!("expected palette parse error"),
        Err(err) => err,
    };
    assert!(matches!(err, RenderError::RemapPaletteParse { .. }));
}

#[test]
fn dithered_remap_output_uses_only_palette_colors() {
    use image::{Rgba, RgbaImage};

    let source = "FFFFFF\n000000\nFF0000\n00FF00\n0000FF\n";
    let cfg = build_image_remap_config(Some(DitherType::Floyd), Some(source))
        .unwrap()
        .expect("expected remap config");

    let mut rgba = RgbaImage::new(2, 2);
    rgba.put_pixel(0, 0, Rgba([12, 200, 180, 255]));
    rgba.put_pixel(1, 0, Rgba([250, 22, 12, 255]));
    rgba.put_pixel(0, 1, Rgba([20, 20, 20, 255]));
    rgba.put_pixel(1, 1, Rgba([240, 240, 240, 255]));

    apply_remap_to_rgba(&mut rgba, &cfg).unwrap();

    let allowed = [
        [255u8, 255u8, 255u8],
        [0u8, 0u8, 0u8],
        [255u8, 0u8, 0u8],
        [0u8, 255u8, 0u8],
        [0u8, 0u8, 255u8],
    ];
    for px in rgba.pixels() {
        assert!(allowed.contains(&[px[0], px[1], px[2]]));
    }
}

#[test]
fn renders_pdf_stream_to_bytes() {
    use crate::parser::{DrawOp, NumberValue, Page};

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
    ];

    let layers = default_layers();
    let frames = default_frames();
    let draw_entries = as_draw_entries(&draw_ops);
    let render_context = empty_render_context();

    let (bytes, outcome) = render_pdf_stream(
        &page,
        &frames,
        &layers,
        &draw_entries,
        None,
        &render_context,
        &host_assets_disabled(),
    )
    .unwrap();

    assert!(!bytes.is_empty());
    assert!(bytes.starts_with(b"%PDF"));
    assert!(outcome.warnings.empty_value_frames.is_empty());
}

#[test]
fn renders_png_stream_to_bytes() {
    use crate::parser::{DrawOp, NumberValue, Page};

    let page = Page {
        width: 128.0,
        height: 96.0,
    };

    let draw_ops = vec![
        DrawOp::SetRgb {
            r: NumberValue::Literal(0.0),
            g: NumberValue::Literal(1.0),
            b: NumberValue::Literal(0.0),
        },
        DrawOp::RectPath {
            x: NumberValue::Literal(0.0),
            y: NumberValue::Literal(0.0),
            width: NumberValue::Literal(128.0),
            height: NumberValue::Literal(96.0),
        },
        DrawOp::Fill,
    ];

    let layers = default_layers();
    let frames = default_frames();
    let draw_entries = as_draw_entries(&draw_ops);
    let render_context = empty_render_context();

    let (bytes, outcome) = render_png_stream(
        &page,
        &frames,
        &layers,
        &draw_entries,
        None,
        &render_context,
        &host_assets_disabled(),
        72,
        None,
        None,
    )
    .unwrap();

    assert!(!bytes.is_empty());
    assert!(bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]));
    assert!(outcome.warnings.empty_value_frames.is_empty());
}
