#[cfg(test)]
mod test;

use std::{
    collections::HashMap,
    fs,
    fs::File,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use dither::{
    clamp_f64_to_u8,
    color::{RGB, palette},
    ditherer::{
        ATKINSON, BURKES, Dither, Ditherer, FLOYD_STEINBERG, JARVIS_JUDICE_NINKE, SIERRA_3, STUCKI,
    },
    prelude::Img,
};

use crate::{
    DitherType,
    resources::{RegisteredAsset, RegisteredFont, RenderContext, load_host_image_asset},
};
use crate::{
    parser::{
        AssetType, BarcodeSymbology, DrawOp, FrameDecl, FrameDrawBlock, NumberValue, Page,
        TextValue,
    },
    scripting::LuaRuntime,
};
use barcoders::sym::{
    code39::Code39,
    code128::Code128,
    ean8::EAN8,
    ean13::{EAN13, UPCA},
};
use cairo::{Antialias, Context, Filter, Format, ImageSurface, PdfSurface, SurfacePattern};
use datamatrix::{DataMatrix, SymbolList};
use pango::FontDescription;
use qrcode::{Color, EcLevel, QrCode};
use serde_json::Value;
use unicode_segmentation::UnicodeSegmentation;

pub struct RenderOutcome {
    pub warnings: RenderWarnings,
    pub script_time: Duration,
    pub draw_time: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrameRuntimeState {
    pub visible: bool,
    pub value_override: Option<String>,
}

#[allow(dead_code)]
impl FrameRuntimeState {
    pub fn default_visible() -> Self {
        Self {
            visible: true,
            value_override: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ScaledImageKey {
    source_key: String,
    width_px: i32,
    height_px: i32,
}

#[derive(Debug, Clone)]
struct RuntimeImageAsset {
    registered: RegisteredAsset,
    source_key: String,
}

#[derive(Debug, Clone)]
pub struct HostAssetPolicy {
    pub allow: bool,
    pub cwd: PathBuf,
}

impl Default for HostAssetPolicy {
    fn default() -> Self {
        Self {
            allow: false,
            cwd: PathBuf::from("."),
        }
    }
}

#[derive(Clone)]
struct ImageRemapConfig {
    dithered: Ditherer<'static>,
    palette: Vec<RGB<u8>>,
}

const IMAGE_RESAMPLE_FILTER: cairo::Filter = cairo::Filter::Good;
const IMAGE_CACHE_SCALE: f64 = 3.0;

pub struct RenderCache {
    image_surfaces: HashMap<String, cairo::ImageSurface>,
    scaled_image_surfaces: HashMap<ScaledImageKey, cairo::ImageSurface>,
    runtime_image_assets: HashMap<String, RuntimeImageAsset>,
    script_runtime: LuaRuntime,
    image_remap: Option<ImageRemapConfig>,
}

impl Default for RenderCache {
    fn default() -> Self {
        Self {
            image_surfaces: HashMap::new(),
            scaled_image_surfaces: HashMap::new(),
            runtime_image_assets: HashMap::new(),
            script_runtime: LuaRuntime::new(),
            image_remap: None,
        }
    }
}

#[derive(Debug, PartialEq)]
struct EncodedQr {
    size: usize,
    modules: Vec<bool>,
}

#[derive(Debug, PartialEq)]
struct EncodedMatrix {
    width: usize,
    height: usize,
    modules: Vec<bool>,
}

#[derive(Debug, Default)]
pub struct RenderWarnings {
    pub empty_value_frames: Vec<u32>,
}

#[derive(Debug, PartialEq)]
pub enum RenderError {
    RemapPaletteParse {
        message: String,
    },
    RemapMissingPalette,
    ImageWrite {
        path: PathBuf,
        message: String,
    },
    ScriptRuntime {
        frame_index: u32,
        frame_id: String,
        message: String,
    },
    MissingSlot {
        slot: String,
    },
    MissingData {
        slot: String,
    },
    InvalidNumberSlot {
        slot: String,
    },
    InvalidTextSlot {
        slot: String,
    },
    WrongAssetType {
        alias: String,
        found: AssetType,
    },
    InvalidImageGeometry {
        alias: String,
        width: f64,
        height: f64,
    },
    MissingAssetAlias {
        alias: String,
    },
    EmptyDynamicAssetAlias {
        path: String,
    },
    HostAssetAccessDenied {
        path: String,
    },
    HostAssetResolve {
        path: String,
        message: String,
    },
    HostAssetLoad {
        path: PathBuf,
        message: String,
    },
    ImageDecode {
        alias: String,
        path: PathBuf,
        message: String,
    },
    InvalidBarcodeGeometry {
        width: f64,
        height: f64,
    },
    BarcodeEncode {
        symbology: String,
        data: String,
        message: String,
    },
    Cairo(cairo::Error),
}

impl From<cairo::Error> for RenderError {
    fn from(err: cairo::Error) -> Self {
        RenderError::Cairo(err)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImageFit {
    Contain,
    Cover,
    Stretch,
}

impl ImageFit {
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "contain" => Some(Self::Contain),
            "cover" => Some(Self::Cover),
            "stretch" => Some(Self::Stretch),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum PendingPath {
    Line {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

impl TextAlign {
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "center" => Some(Self::Center),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VerticalAlign {
    Top,
    Middle,
    Bottom,
}

impl VerticalAlign {
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "top" => Some(Self::Top),
            "bottom" => Some(Self::Bottom),
            "middle" => Some(Self::Middle),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineBreakMode {
    Word,
    Char,
    None,
}

impl LineBreakMode {
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "word" => Some(Self::Word),
            "char" => Some(Self::Char),
            "none" => Some(Self::None),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextFit {
    Fixed,
    ShrinkToFit,
    GrowToFit,
    Fit,
}

impl TextFit {
    pub fn from_word(word: &str) -> Option<Self> {
        match word {
            "fit" => Some(Self::Fit),
            "fixed" => Some(Self::Fixed),
            "shrink" => Some(Self::ShrinkToFit),
            "grow" => Some(Self::GrowToFit),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum CurrentFont {
    System(String),
    Packaged { alias: String, font: RegisteredFont },
}

#[derive(Debug, Clone)]
struct RenderState {
    font: CurrentFont,
    font_size: f64,
    text_align: TextAlign,
    vertical_align: VerticalAlign,
    line_break: LineBreakMode,
    crop_text: bool,
    text_fit: TextFit,
    text_fit_min_size: f64,
    text_fit_max_size: f64,
    image_fit: ImageFit,
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            font: CurrentFont::System("Sans".to_string()),
            font_size: 12.0,
            text_align: TextAlign::Left,
            vertical_align: VerticalAlign::Top,
            line_break: LineBreakMode::Word,
            crop_text: true,
            text_fit: TextFit::Fixed,
            text_fit_min_size: 4.0,
            text_fit_max_size: 96.0,
            image_fit: ImageFit::Contain,
        }
    }
}

fn run_frame_scripts(
    frame_state: &mut HashMap<u32, FrameRuntimeState>,
    data: Option<&Value>,
    context: &RenderContext,
    runtime: &mut LuaRuntime,
) -> Result<(), RenderError> {
    if context.scripts.is_empty() {
        return Ok(());
    }

    let prepared_data = runtime
        .build_data_api(data)
        .map_err(|err| RenderError::ScriptRuntime {
            frame_index: 0,
            frame_id: "<data>".to_string(),
            message: err.to_string(),
        })?;

    for planned in &context.script_plan {
        let Some(source) = context.scripts.get(&planned.frame_id) else {
            continue;
        };

        let updated = runtime
            .exec_with_data(&planned.frame_id, source, &prepared_data)
            .map_err(|err| RenderError::ScriptRuntime {
                frame_index: planned.frame_index,
                frame_id: planned.frame_id.clone(),
                message: err.to_string(),
            })?;

        frame_state.insert(planned.frame_index, updated);
    }
    Ok(())
}

fn to_title_case(input: &str) -> String {
    input
        .split_word_bounds()
        .map(|chunk| {
            if chunk.chars().any(|c| c.is_alphabetic()) {
                let mut graphemes = chunk.graphemes(true);
                match graphemes.next() {
                    Some(first) => {
                        let rest: String = graphemes.collect();
                        format!("{}{}", first.to_uppercase(), rest.to_lowercase())
                    }
                    None => String::new(),
                }
            } else {
                chunk.to_string()
            }
        })
        .collect()
}

fn to_capitalize(input: &str) -> String {
    let mut graphemes = input.graphemes(true);
    match graphemes.next() {
        Some(first) => {
            let rest: String = graphemes.collect();
            format!("{}{}", first.to_uppercase(), rest.to_lowercase())
        }
        None => String::new(),
    }
}

fn eval_text(value: &TextValue, data: Option<&Value>) -> Result<String, RenderError> {
    match value {
        TextValue::Number(number) => {
            let value = eval_number(number, data)?;
            Ok(format!("{value}"))
        }
        TextValue::Literal(text) => Ok(text.clone()),
        TextValue::Slot(name) => {
            let data = data.ok_or_else(|| RenderError::MissingData { slot: name.clone() })?;
            let value = data
                .get(name)
                .ok_or_else(|| RenderError::MissingSlot { slot: name.clone() })?;
            value
                .as_str()
                .map(|s| s.to_string())
                .ok_or_else(|| RenderError::InvalidTextSlot { slot: name.clone() })
        }
        TextValue::Concat(parts) => {
            let mut out = String::new();
            for part in parts {
                out.push_str(&eval_text(part, data)?);
            }
            Ok(out)
        }
        TextValue::UpperCase(v) => {
            let value = eval_text(v.as_ref(), data)?;
            Ok(value.to_uppercase())
        }
        TextValue::LowerCase(v) => {
            let value = eval_text(v.as_ref(), data)?;
            Ok(value.to_lowercase())
        }
        TextValue::TitleCase(v) => {
            let value = eval_text(v.as_ref(), data)?;
            Ok(to_title_case(&value))
        }
        TextValue::Capitalize(v) => {
            let value = eval_text(v.as_ref(), data)?;
            Ok(to_capitalize(&value))
        }
    }
}

fn eval_number(value: &NumberValue, data: Option<&Value>) -> Result<f64, RenderError> {
    match value {
        NumberValue::Literal(n) => Ok(*n),
        NumberValue::Slot(name) => {
            let data = data.ok_or_else(|| RenderError::MissingData { slot: name.clone() })?;

            let value = data
                .get(name)
                .ok_or_else(|| RenderError::MissingSlot { slot: name.clone() })?;

            value
                .as_f64()
                .ok_or_else(|| RenderError::InvalidNumberSlot { slot: name.clone() })
        }
    }
}

fn encode_code39_modules(data: &str) -> Result<Vec<u8>, RenderError> {
    let code = Code39::new(data).map_err(|err| RenderError::BarcodeEncode {
        symbology: "c39".to_string(),
        data: data.to_string(),
        message: err.to_string(),
    })?;
    Ok(code.encode())
}

fn render_code39(
    ctx: &Context,
    value: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), RenderError> {
    if width <= 0.0 || height <= 0.0 {
        return Err(RenderError::InvalidBarcodeGeometry { width, height });
    }
    let modules = encode_code39_modules(value)?;
    render_barcode(
        ctx,
        x,
        y,
        width,
        height,
        modules,
        "c39".to_string(),
        value.to_string(),
    )?;
    Ok(())
}

fn encode_code128_modules(symbol: &BarcodeSymbology, value: &str) -> Result<Vec<u8>, RenderError> {
    let marker = symbol.to_marker();
    let payload = format!("{marker}{value}");
    let code = Code128::new(payload).map_err(|err| RenderError::BarcodeEncode {
        symbology: symbol.to_word(),
        data: value.to_string(),
        message: err.to_string(),
    })?;
    Ok(code.encode())
}

fn render_code128(
    ctx: &Context,
    symbol: &BarcodeSymbology,
    value: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), RenderError> {
    if width <= 0.0 || height <= 0.0 {
        return Err(RenderError::InvalidBarcodeGeometry { width, height });
    }
    let modules = encode_code128_modules(symbol, value)?;
    render_barcode(
        ctx,
        x,
        y,
        width,
        height,
        modules,
        symbol.to_word(),
        value.to_string(),
    )?;
    Ok(())
}

fn encode_upca_modules(value: &str) -> Result<Vec<u8>, RenderError> {
    let code = UPCA::new(value).map_err(|err| RenderError::BarcodeEncode {
        symbology: "upca".to_string(),
        data: value.to_string(),
        message: err.to_string(),
    })?;
    Ok(code.encode())
}

fn is_retail_guard_module(i: usize) -> bool {
    (0..3).contains(&i) || (45..50).contains(&i) || (92..95).contains(&i)
}

fn is_ean8_guard_module(i: usize) -> bool {
    (0..3).contains(&i) || (31..36).contains(&i) || (64..67).contains(&i)
}

fn render_upca(
    ctx: &Context,
    value: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), RenderError> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err(RenderError::InvalidBarcodeGeometry { width, height });
    }

    let modules = encode_upca_modules(value)?;
    if modules.is_empty() {
        return Err(RenderError::BarcodeEncode {
            symbology: "upca".to_string(),
            data: value.to_string(),
            message: "empty module stream".to_string(),
        });
    }

    if modules.len() != 95 {
        return render_barcode(
            ctx,
            x,
            y,
            width,
            height,
            modules,
            "upca".to_string(),
            value.to_string(),
        );
    }

    let x_dim = width / modules.len() as f64;
    let guard_extra = 5.0 * x_dim;
    let data_h = (height - guard_extra).max(0.0);

    ctx.save()?;
    ctx.set_antialias(Antialias::None);

    for (i, bit) in modules.iter().enumerate() {
        if *bit == 1 {
            let bx = x + (i as f64 * x_dim);
            let bar_h = if is_retail_guard_module(i) {
                height
            } else {
                data_h
            };
            ctx.rectangle(bx, y, x_dim, bar_h);
        }
    }

    ctx.fill()?;
    ctx.restore()?;

    Ok(())
}

fn encode_ean8_modules(value: &str) -> Result<Vec<u8>, RenderError> {
    let code = EAN8::new(value).map_err(|err| RenderError::BarcodeEncode {
        symbology: "ean8".to_string(),
        data: value.to_string(),
        message: err.to_string(),
    })?;
    Ok(code.encode())
}

fn render_ean8(
    ctx: &Context,
    value: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), RenderError> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err(RenderError::InvalidBarcodeGeometry { width, height });
    }

    let modules = encode_ean8_modules(value)?;
    if modules.is_empty() {
        return Err(RenderError::BarcodeEncode {
            symbology: "ean8".to_string(),
            data: value.to_string(),
            message: "empty module stream".to_string(),
        });
    }

    if modules.len() != 67 {
        return render_barcode(
            ctx,
            x,
            y,
            width,
            height,
            modules,
            "ean8".to_string(),
            value.to_string(),
        );
    }

    let x_dim = width / modules.len() as f64;
    let guard_extra = 5.0 * x_dim;
    let data_h = (height - guard_extra).max(0.0);

    ctx.save()?;
    ctx.set_antialias(Antialias::None);

    for (i, bit) in modules.iter().enumerate() {
        if *bit == 1 {
            let bx = x + (i as f64 * x_dim);
            let bar_h = if is_ean8_guard_module(i) {
                height
            } else {
                data_h
            };
            ctx.rectangle(bx, y, x_dim, bar_h);
        }
    }

    ctx.fill()?;
    ctx.restore()?;

    Ok(())
}

fn encode_ean13_modules(value: &str) -> Result<Vec<u8>, RenderError> {
    let code = EAN13::new(value).map_err(|err| RenderError::BarcodeEncode {
        symbology: "ean13".to_string(),
        data: value.to_string(),
        message: err.to_string(),
    })?;
    Ok(code.encode())
}

fn render_ean13(
    ctx: &Context,
    value: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), RenderError> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err(RenderError::InvalidBarcodeGeometry { width, height });
    }

    let modules = encode_ean13_modules(value)?;
    if modules.is_empty() {
        return Err(RenderError::BarcodeEncode {
            symbology: "ean13".to_string(),
            data: value.to_string(),
            message: "empty module stream".to_string(),
        });
    }

    if modules.len() != 95 {
        return render_barcode(
            ctx,
            x,
            y,
            width,
            height,
            modules,
            "ean13".to_string(),
            value.to_string(),
        );
    }

    let x_dim = width / modules.len() as f64;
    let guard_extra = 5.0 * x_dim;
    let data_h = (height - guard_extra).max(0.0);

    ctx.save()?;
    ctx.set_antialias(Antialias::None);

    for (i, bit) in modules.iter().enumerate() {
        if *bit == 1 {
            let bx = x + (i as f64 * x_dim);
            let bar_h = if is_retail_guard_module(i) {
                height
            } else {
                data_h
            };
            ctx.rectangle(bx, y, x_dim, bar_h);
        }
    }

    ctx.fill()?;
    ctx.restore()?;

    Ok(())
}

fn render_barcode(
    ctx: &Context,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    modules: Vec<u8>,
    symbol: String,
    value: String,
) -> Result<(), RenderError> {
    if modules.is_empty() {
        return Err(RenderError::BarcodeEncode {
            symbology: symbol,
            data: value,
            message: "empty module stream".to_string(),
        });
    }

    let module_w = width / modules.len() as f64;

    ctx.save()?;
    ctx.set_antialias(Antialias::None);

    for (i, bit) in modules.iter().enumerate() {
        if *bit == 1 {
            let bx = x + (i as f64 * module_w);
            ctx.rectangle(bx, y, module_w, height);
        }
    }

    ctx.fill()?;
    ctx.restore()?;

    Ok(())
}

fn encode_data_matrix(value: &str) -> Result<EncodedMatrix, RenderError> {
    let code = DataMatrix::encode_str(value, SymbolList::default()).map_err(|err| {
        RenderError::BarcodeEncode {
            symbology: "datamatrix".to_string(),
            data: value.to_string(),
            message: format!("{err:?}"),
        }
    })?;

    let bitmap = code.bitmap();
    let width = bitmap.width();
    let height = bitmap.height();

    let mut modules = vec![false; width * height];
    for (x, y) in bitmap.pixels() {
        let idx = y * width + x;
        modules[idx] = true;
    }

    Ok(EncodedMatrix {
        width,
        height,
        modules,
    })
}

fn render_datamatrix(
    ctx: &Context,
    value: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), RenderError> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err(RenderError::InvalidBarcodeGeometry { width, height });
    }

    let dm = encode_data_matrix(value)?;
    if dm.width == 0 || dm.height == 0 || dm.modules.len() != dm.width * dm.height {
        return Err(RenderError::BarcodeEncode {
            symbology: "datamatrix".to_string(),
            data: value.to_string(),
            message: "invalid data matrix".to_string(),
        });
    }

    let quiet_zone_modules: usize = 1;
    let total_w_modules = dm.width + (quiet_zone_modules * 2);
    let total_h_modules = dm.height + (quiet_zone_modules * 2);

    let cell = (width / total_w_modules as f64).min(height / total_h_modules as f64);
    if !cell.is_finite() || cell <= 0.0 {
        return Err(RenderError::InvalidBarcodeGeometry { width, height });
    }

    let draw_w = cell * total_w_modules as f64;
    let draw_h = cell * total_h_modules as f64;
    let origin_x = x + (width - draw_w) / 2.0;
    let origin_y = y + (height - draw_h) / 2.0;

    ctx.save()?;
    ctx.set_antialias(Antialias::None);
    ctx.rectangle(x, y, width, height);
    ctx.clip();

    for row in 0..dm.height {
        for col in 0..dm.width {
            let idx = row * dm.width + col;
            if dm.modules[idx] {
                let px = origin_x + (col + quiet_zone_modules) as f64 * cell;
                let py = origin_y + (row + quiet_zone_modules) as f64 * cell;
                ctx.rectangle(px, py, cell, cell);
            }
        }
    }

    ctx.fill()?;
    ctx.restore()?;

    Ok(())
}

fn encode_qr_matrix(value: &str) -> Result<EncodedQr, RenderError> {
    let code =
        QrCode::with_error_correction_level(value.as_bytes(), EcLevel::M).map_err(|err| {
            RenderError::BarcodeEncode {
                symbology: "qr".to_string(),
                data: value.to_string(),
                message: err.to_string(),
            }
        })?;

    let size = code.width();
    let modules = code
        .into_colors()
        .into_iter()
        .map(|c| matches!(c, Color::Dark))
        .collect::<Vec<bool>>();

    Ok(EncodedQr { size, modules })
}

fn render_qr(
    ctx: &Context,
    value: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), RenderError> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err(RenderError::InvalidBarcodeGeometry { width, height });
    }

    let qr = encode_qr_matrix(value)?;
    if qr.size == 0 || qr.modules.len() != qr.size * qr.size {
        return Err(RenderError::BarcodeEncode {
            symbology: "qr".to_string(),
            data: value.to_string(),
            message: "invalid qr matrix".to_string(),
        });
    }

    let quiet_zone_modules: usize = 4;
    let total_modules = qr.size + (quiet_zone_modules * 2);

    let cell = (width / total_modules as f64).min(height / total_modules as f64);
    if !cell.is_finite() || cell <= 0.0 {
        return Err(RenderError::InvalidBarcodeGeometry { width, height });
    }

    let draw_w = cell * total_modules as f64;
    let draw_h = cell * total_modules as f64;
    let origin_x = x + (width - draw_w) / 2.0;
    let origin_y = y + (height - draw_h) / 2.0;

    ctx.save()?;
    ctx.set_antialias(Antialias::None);
    ctx.rectangle(x, y, width, height);
    ctx.clip();

    for row in 0..qr.size {
        for col in 0..qr.size {
            let idx = row * qr.size + col;
            if qr.modules[idx] {
                let px = origin_x + (col + quiet_zone_modules) as f64 * cell;
                let py = origin_y + (row + quiet_zone_modules) as f64 * cell;
                ctx.rectangle(px, py, cell, cell);
            }
        }
    }

    ctx.fill()?;
    ctx.restore()?;

    Ok(())
}

fn to_pango_units(value: f64) -> i32 {
    (value * pango::SCALE as f64) as i32
}

fn current_font_description_name(font: &CurrentFont) -> &str {
    match font {
        CurrentFont::System(name) => name,
        CurrentFont::Packaged { font, .. } => &font.family_name,
    }
}

fn configure_text_layout(
    layout: &pango::Layout,
    state: &RenderState,
    text: &str,
    font_size: f64,
    width: f64,
    height: f64,
) {
    layout.set_text(text);

    let font_name = current_font_description_name(&state.font);
    let mut font = FontDescription::new();
    font.set_family(font_name);
    font.set_size(to_pango_units(font_size));
    layout.set_font_description(Some(&font));

    layout.set_height(to_pango_units(height));

    layout.set_alignment(match state.text_align {
        TextAlign::Left => pango::Alignment::Left,
        TextAlign::Center => pango::Alignment::Center,
        TextAlign::Right => pango::Alignment::Right,
    });

    match state.line_break {
        LineBreakMode::Word => {
            layout.set_width(to_pango_units(width));
            layout.set_wrap(pango::WrapMode::Word);
            layout.set_single_paragraph_mode(false);
        }
        LineBreakMode::Char => {
            layout.set_width(to_pango_units(width));
            layout.set_wrap(pango::WrapMode::Char);
            layout.set_single_paragraph_mode(false);
        }
        LineBreakMode::None => {
            layout.set_width(-1);
            layout.set_single_paragraph_mode(true);
        }
    }
}

fn layout_fits(layout: &pango::Layout, width: f64, height: f64) -> bool {
    let (_, logical_rect) = layout.pixel_extents();
    logical_rect.width() as f64 <= width && logical_rect.height() as f64 <= height
}

fn find_largest_fitting_font_size(
    layout: &pango::Layout,
    state: &RenderState,
    text: &str,
    width: f64,
    height: f64,
    min_size: f64,
    max_size: f64,
) -> f64 {
    if max_size < min_size {
        return min_size;
    }

    let mut low = min_size;
    let mut high = max_size;
    let mut best = min_size;

    for _ in 0..8 {
        let mid = (low + high) / 2.0;
        configure_text_layout(&layout, state, text, mid, width, height);

        if layout_fits(&layout, width, height) {
            best = mid;
            low = mid;
        } else {
            high = mid;
        }
    }

    best
}

fn fitted_font_size(
    layout: &pango::Layout,
    state: &RenderState,
    text: &str,
    width: f64,
    height: f64,
) -> f64 {
    match state.text_fit {
        TextFit::Fixed => state.font_size,
        TextFit::ShrinkToFit => {
            let max = state.font_size.min(state.text_fit_max_size);
            find_largest_fitting_font_size(
                layout,
                state,
                text,
                width,
                height,
                state.text_fit_min_size,
                max,
            )
        }
        TextFit::GrowToFit => {
            let min = state.font_size.max(state.text_fit_min_size);
            find_largest_fitting_font_size(
                layout,
                state,
                text,
                width,
                height,
                min,
                state.text_fit_max_size,
            )
        }
        TextFit::Fit => find_largest_fitting_font_size(
            layout,
            state,
            text,
            width,
            height,
            state.text_fit_min_size,
            state.text_fit_max_size,
        ),
    }
}

fn to_dithered(kind: DitherType) -> Ditherer<'static> {
    match kind {
        DitherType::Floyd => FLOYD_STEINBERG,
        DitherType::Atkinson => ATKINSON,
        DitherType::Stucki => STUCKI,
        DitherType::Burkes => BURKES,
        DitherType::Jarvis => JARVIS_JUDICE_NINKE,
        DitherType::Sierra3 => SIERRA_3,
    }
}

fn build_image_remap_config(
    dither: Option<DitherType>,
    remap_palette_source: Option<&str>,
) -> Result<Option<ImageRemapConfig>, RenderError> {
    let Some(kind) = dither else {
        return Ok(None);
    };
    let source = remap_palette_source.ok_or(RenderError::RemapMissingPalette)?;
    let palette_vec =
        palette::parse::<Vec<RGB<u8>>>(source).map_err(|err| RenderError::RemapPaletteParse {
            message: err.to_string(),
        })?;

    Ok(Some(ImageRemapConfig {
        dithered: to_dithered(kind),
        palette: palette_vec,
    }))
}

fn apply_remap_to_rgba(
    rgba: &mut image::RgbaImage,
    cfg: &ImageRemapConfig,
) -> Result<(), RenderError> {
    let width = rgba.width();
    let src_pixels = rgba
        .pixels()
        .map(|p| RGB(f64::from(p[0]), f64::from(p[1]), f64::from(p[2])));

    let img = Img::new(src_pixels, width).ok_or_else(|| RenderError::RemapPaletteParse {
        message: "failed to biuld dither image".to_string(),
    })?;

    let quantize = palette::quantize(&cfg.palette);
    let out = cfg
        .dithered
        .dither(img, quantize)
        .convert_with(|rgb| rgb.convert_with(clamp_f64_to_u8));

    for (dst, RGB(r, g, b)) in rgba.pixels_mut().zip(out.into_iter()) {
        dst[0] = r;
        dst[1] = g;
        dst[2] = b;
    }

    Ok(())
}

fn premultiply(channel: u8, alpha: u8) -> u8 {
    ((u16::from(channel) * u16::from(alpha) + 127) / 255) as u8
}

fn load_image_surface(
    alias: &str,
    path: &Path,
    remap: Option<&ImageRemapConfig>,
) -> Result<cairo::ImageSurface, RenderError> {
    let dyn_img = image::open(path).map_err(|err| RenderError::ImageDecode {
        alias: alias.to_string(),
        path: path.to_path_buf(),
        message: err.to_string(),
    })?;

    let mut rgba = dyn_img.to_rgba8();
    if let Some(cfg) = remap {
        apply_remap_to_rgba(&mut rgba, cfg)?;
    }
    let (width, height) = rgba.dimensions();

    let mut surface =
        cairo::ImageSurface::create(cairo::Format::ARgb32, width as i32, height as i32)?;
    let stride = surface.stride() as usize;
    let src = rgba.as_raw();

    {
        let mut dst = surface.data().map_err(|err| RenderError::ImageDecode {
            alias: alias.to_string(),
            path: path.to_path_buf(),
            message: format!("cairo surface borrow failed: {err}"),
        })?;

        for y in 0..height as usize {
            let src_row = &src[y * width as usize * 4..(y + 1) * width as usize * 4];
            let dst_row = &mut dst[y * stride..y * stride + width as usize * 4];

            for x in 0..width as usize {
                let si = x * 4;
                let di = x * 4;

                let r = src_row[si];
                let g = src_row[si + 1];
                let b = src_row[si + 2];
                let a = src_row[si + 3];

                dst_row[di] = premultiply(b, a);
                dst_row[di + 1] = premultiply(g, a);
                dst_row[di + 2] = premultiply(r, a);
                dst_row[di + 3] = a;
            }
        }
    }

    surface.mark_dirty();
    Ok(surface)
}

fn get_or_load_image_surface<'a>(
    cache: &'a mut RenderCache,
    source_key: &str,
    alias: &str,
    path: &Path,
) -> Result<&'a cairo::ImageSurface, RenderError> {
    if !cache.image_surfaces.contains_key(source_key) {
        let remap = cache.image_remap.as_ref();
        let surface = load_image_surface(alias, path, remap)?;
        cache.image_surfaces.insert(source_key.to_string(), surface);
    }

    Ok(cache
        .image_surfaces
        .get(source_key)
        .expect("cache insert must exist"))
}

fn normalize_surface_dims(width: f64, height: f64) -> (i32, i32) {
    let w = width.round().max(1.0) as i32;
    let h = height.round().max(1.0) as i32;
    (w, h)
}

fn build_scaled_surface(
    source: &cairo::ImageSurface,
    target_w: i32,
    target_h: i32,
) -> Result<cairo::ImageSurface, RenderError> {
    if source.width() == target_w && source.height() == target_h {
        return Ok(source.clone());
    }

    let dest = cairo::ImageSurface::create(Format::ARgb32, target_w, target_h)?;
    let ctx = Context::new(&dest)?;

    ctx.scale(
        target_w as f64 / source.width() as f64,
        target_h as f64 / source.height() as f64,
    );

    let pattern = SurfacePattern::create(source);
    pattern.set_extend(cairo::Extend::Pad);
    pattern.set_filter(IMAGE_RESAMPLE_FILTER);

    ctx.set_source(&pattern)?;
    ctx.paint()?;
    dest.mark_dirty();

    Ok(dest)
}

fn get_or_load_scaled_image_surface<'a>(
    cache: &'a mut RenderCache,
    source_key: &str,
    alias: &str,
    path: &Path,
    target_w: f64,
    target_h: f64,
) -> Result<&'a ImageSurface, RenderError> {
    let (width_px, height_px) = normalize_surface_dims(target_w, target_h);
    let key = ScaledImageKey {
        source_key: source_key.to_string(),
        width_px,
        height_px,
    };

    if !cache.scaled_image_surfaces.contains_key(&key) {
        let source = get_or_load_image_surface(cache, source_key, alias, path)?.clone();
        let scaled = build_scaled_surface(&source, width_px, height_px)?;
        cache.scaled_image_surfaces.insert(key.clone(), scaled);
    }

    Ok(cache
        .scaled_image_surfaces
        .get(&key)
        .expect("scaled cache insert must exist"))
}

fn path_cache_key(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn resolve_host_asset_path(raw_path: &str, policy: &HostAssetPolicy) -> Result<PathBuf, RenderError> {
    if !policy.allow {
        return Err(RenderError::HostAssetAccessDenied {
            path: raw_path.to_string(),
        });
    }

    let input = Path::new(raw_path);
    let joined = if input.is_absolute() {
        input.to_path_buf()
    } else {
        policy.cwd.join(input)
    };

    fs::canonicalize(&joined).map_err(|err| RenderError::HostAssetResolve {
        path: raw_path.to_string(),
        message: err.to_string(),
    })
}

fn load_runtime_image_asset(
    cache: &mut RenderCache,
    policy: &HostAssetPolicy,
    alias: &str,
    path: &str,
) -> Result<(), RenderError> {
    if alias.trim().is_empty() {
        return Err(RenderError::EmptyDynamicAssetAlias {
            path: path.to_string(),
        });
    }

    let resolved_path = resolve_host_asset_path(path, policy)?;
    let registered = load_host_image_asset(alias, &resolved_path).map_err(|err| {
        RenderError::HostAssetLoad {
            path: resolved_path.clone(),
            message: err.to_string(),
        }
    })?;
    let source_key = path_cache_key(&resolved_path);

    cache.runtime_image_assets.insert(
        alias.to_string(),
        RuntimeImageAsset {
            registered,
            source_key,
        },
    );

    Ok(())
}

fn resolve_image_alias(
    cache: &RenderCache,
    context: &RenderContext,
    asset: &str,
) -> Option<(PathBuf, String, AssetType)> {
    if let Some(runtime) = cache.runtime_image_assets.get(asset) {
        return Some((
            runtime.registered.path.clone(),
            runtime.source_key.clone(),
            runtime.registered.ty.clone(),
        ));
    }

    context
        .resolve_asset(asset)
        .map(|registered| {
            (
                registered.path.clone(),
                path_cache_key(&registered.path),
                registered.ty.clone(),
            )
        })
}

fn render_image(
    ctx: &Context,
    context: &RenderContext,
    cache: &mut RenderCache,
    asset: &str,
    fit: ImageFit,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), RenderError> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err(RenderError::InvalidImageGeometry {
            alias: asset.to_string(),
            width,
            height,
        });
    }

    let (registered_path, source_key, asset_type) =
        resolve_image_alias(cache, context, asset).ok_or_else(|| RenderError::MissingAssetAlias {
            alias: asset.to_string(),
        })?;

    if asset_type != AssetType::Image {
        return Err(RenderError::WrongAssetType {
            alias: asset.to_string(),
            found: asset_type,
        });
    }

    let (src_w, src_h) = {
        let source = get_or_load_image_surface(cache, &source_key, asset, &registered_path)?;
        (source.width() as f64, source.height() as f64)
    };

    let (draw_x, draw_y, draw_w, draw_h) = match fit {
        ImageFit::Stretch => (x, y, width, height),
        ImageFit::Contain => {
            let s = (width / src_w).min(height / src_h);
            let draw_w = src_w * s;
            let draw_h = src_h * s;
            let dx = x + (width - draw_w) / 2.0;
            let dy = y + (height - draw_h) / 2.0;
            (dx, dy, draw_w, draw_h)
        }
        ImageFit::Cover => {
            let s = (width / src_w).max(height / src_h);
            let draw_w = src_w * s;
            let draw_h = src_h * s;
            let dx = x + (width - draw_w) / 2.0;
            let dy = y + (height - draw_h) / 2.0;
            (dx, dy, draw_w, draw_h)
        }
    };

    let cache_w = draw_w * IMAGE_CACHE_SCALE;
    let cache_h = draw_h * IMAGE_CACHE_SCALE;
    let scaled_surface = get_or_load_scaled_image_surface(
        cache,
        &source_key,
        asset,
        &registered_path,
        cache_w,
        cache_h,
    )?;

    ctx.save()?;
    ctx.rectangle(x, y, width, height);
    ctx.clip();

    ctx.translate(draw_x, draw_y);
    ctx.scale(1.0 / IMAGE_CACHE_SCALE, 1.0 / IMAGE_CACHE_SCALE);
    ctx.set_source_surface(scaled_surface, 0.0, 0.0)?;
    let pattern = ctx.source();
    pattern.set_filter(Filter::Best);
    ctx.paint()?;
    ctx.restore()?;

    Ok(())
}

fn render_textbox(
    layout: &pango::Layout,
    ctx: &Context,
    state: &RenderState,
    text: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), RenderError> {
    let font_size = fitted_font_size(layout, state, text, width, height);
    configure_text_layout(layout, state, text, font_size, width, height);

    ctx.save()?;

    if state.crop_text {
        ctx.rectangle(x, y, width, height);
        ctx.clip();
    }

    let (_, logical_rect) = layout.pixel_extents();
    let text_height = logical_rect.height() as f64;

    let draw_y = match state.vertical_align {
        VerticalAlign::Top => y,
        VerticalAlign::Middle => y + ((height - text_height) / 2.0),
        VerticalAlign::Bottom => y + (height - text_height),
    };

    ctx.move_to(x, draw_y);
    pangocairo::functions::show_layout(ctx, layout);

    ctx.restore()?;

    Ok(())
}

fn resolve_current_font(context: &RenderContext, requested: &str) -> CurrentFont {
    match context.resolve_font(requested) {
        Some(font) => CurrentFont::Packaged {
            alias: requested.to_string(),
            font: font.clone(),
        },
        None => CurrentFont::System(requested.to_string()),
    }
}

fn should_render_frame(state: Option<&FrameRuntimeState>) -> bool {
    let Some(state) = state else {
        return true;
    };
    state.visible
}

fn build_initial_frame_state(frames: &[FrameDecl]) -> HashMap<u32, FrameRuntimeState> {
    let mut out = HashMap::new();
    for frame in frames {
        out.insert(frame.index, FrameRuntimeState::default_visible());
    }
    out
}

fn warn_if_empty_set_value(
    frame_index: u32,
    runtime: Option<&FrameRuntimeState>,
    warnings: &mut RenderWarnings,
) {
    if let Some(FrameRuntimeState {
        value_override: Some(v),
        ..
    }) = runtime
    {
        if v.is_empty() {
            warnings.empty_value_frames.push(frame_index)
        }
    }
}

fn execute_draw(
    ctx: &Context,
    draw_frames: &[FrameDrawBlock],
    frame_state: &HashMap<u32, FrameRuntimeState>,
    data: Option<&Value>,
    context: &RenderContext,
    cache: &mut RenderCache,
    host_assets: &HostAssetPolicy,
) -> Result<RenderWarnings, RenderError> {
    let mut state = RenderState::default();
    let mut pending_path: Option<PendingPath> = None;
    let layout = pangocairo::functions::create_layout(ctx);
    let mut warnings = RenderWarnings::default();

    for frame in draw_frames {
        let runtime = frame_state.get(&frame.index);

        warn_if_empty_set_value(frame.index, runtime, &mut warnings);

        if !should_render_frame(runtime) {
            continue;
        }

        for op in &frame.ops {
            match op {
                DrawOp::Barcode {
                    value,
                    symbology,
                    x,
                    y,
                    width,
                    height,
                } => {
                    let value = match runtime.and_then(|r| r.value_override.as_ref()) {
                        Some(v) if !v.is_empty() => v.clone(),
                        _ => eval_text(value, data)?,
                    };
                    let x = eval_number(x, data)?;
                    let y = eval_number(y, data)?;
                    let width = eval_number(width, data)?;
                    let height = eval_number(height, data)?;

                    match symbology {
                        BarcodeSymbology::Code39 => {
                            render_code39(ctx, &value, x, y, width, height)?;
                        }
                        BarcodeSymbology::Code128A => {
                            render_code128(ctx, symbology, &value, x, y, width, height)?;
                        }
                        BarcodeSymbology::Code128B => {
                            render_code128(ctx, symbology, &value, x, y, width, height)?;
                        }
                        BarcodeSymbology::Code128C => {
                            render_code128(ctx, symbology, &value, x, y, width, height)?;
                        }
                        BarcodeSymbology::UPCA => {
                            render_upca(ctx, &value, x, y, width, height)?;
                        }
                        BarcodeSymbology::EAN13 => {
                            render_ean13(ctx, &value, x, y, width, height)?;
                        }
                        BarcodeSymbology::EAN8 => {
                            render_ean8(ctx, &value, x, y, width, height)?;
                        }
                        BarcodeSymbology::QR => {
                            render_qr(ctx, &value, x, y, width, height)?;
                        }
                        BarcodeSymbology::DataMatrix => {
                            render_datamatrix(ctx, &value, x, y, width, height)?;
                        }
                    }
                }
                DrawOp::SetImageFit { fit } => {
                    state.image_fit = *fit;
                }
                DrawOp::SetFontFamily { font } => {
                    let requested = eval_text(font, data)?;
                    state.font = resolve_current_font(context, &requested);
                }
                DrawOp::SetTextFitMaxSize { max } => {
                    state.text_fit_max_size = eval_number(max, data)?;
                }
                DrawOp::SetTextFitMinSize { min } => {
                    state.text_fit_min_size = eval_number(min, data)?;
                }
                DrawOp::SetTextFit { fit } => {
                    state.text_fit = *fit;
                }
                DrawOp::SetLineBreakMode { line_break } => {
                    state.line_break = *line_break;
                }
                DrawOp::SetVerticalAlignment { align } => {
                    state.vertical_align = *align;
                }
                DrawOp::SetTextAlignment { align } => {
                    state.text_align = *align;
                }
                DrawOp::SetRgb { r, g, b } => {
                    ctx.set_source_rgb(
                        eval_number(r, data)?.clamp(0.0, 1.0),
                        eval_number(g, data)?.clamp(0.0, 1.0),
                        eval_number(b, data)?.clamp(0.0, 1.0),
                    );
                }
                DrawOp::SetCmyk { c, m, y, k } => {
                    let c_actual = eval_number(c, data)?;
                    let m_actual = eval_number(m, data)?;
                    let y_actual = eval_number(y, data)?;
                    let k_actual = eval_number(k, data)?;

                    let r = (1.0 - c_actual) * (1.0 - k_actual);
                    let g = (1.0 - m_actual) * (1.0 - k_actual);
                    let b = (1.0 - y_actual) * (1.0 - k_actual);

                    ctx.set_source_rgb(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0));
                }
                DrawOp::SetStrokeWidth { width } => {
                    ctx.set_line_width(eval_number(width, data)?);
                }
                DrawOp::SetFontSize { size } => {
                    state.font_size = eval_number(size, data)?;
                }
                DrawOp::LinePath { x1, y1, x2, y2 } => {
                    pending_path = Some(PendingPath::Line {
                        x1: eval_number(x1, data)?,
                        y1: eval_number(y1, data)?,
                        x2: eval_number(x2, data)?,
                        y2: eval_number(y2, data)?,
                    });
                }
                DrawOp::RectPath {
                    x,
                    y,
                    width,
                    height,
                } => {
                    pending_path = Some(PendingPath::Rect {
                        x: eval_number(x, data)?,
                        y: eval_number(y, data)?,
                        width: eval_number(width, data)?,
                        height: eval_number(height, data)?,
                    });
                }
                DrawOp::Stroke => {
                    let path = pending_path
                        .take()
                        .expect("parser should prevent stroke without a current path");
                    match path {
                        PendingPath::Line { x1, y1, x2, y2 } => {
                            ctx.move_to(x1, y1);
                            ctx.line_to(x2, y2);
                            ctx.stroke()?;
                        }
                        PendingPath::Rect {
                            x,
                            y,
                            width,
                            height,
                        } => {
                            ctx.rectangle(x, y, width, height);
                            ctx.stroke()?;
                        }
                    }
                }
                DrawOp::Fill => {
                    let path = pending_path
                        .take()
                        .expect("parser should prevent fill without a current path");
                    match path {
                        PendingPath::Rect {
                            x,
                            y,
                            width,
                            height,
                        } => {
                            ctx.rectangle(x, y, width, height);
                            ctx.fill()?;
                        }
                        PendingPath::Line { .. } => {
                            panic!("parser should prevent filling a line");
                        }
                    }
                }
                DrawOp::Image {
                    asset,
                    x,
                    y,
                    width,
                    height,
                } => {
                    let value = match runtime.and_then(|r| r.value_override.as_ref()) {
                        Some(v) if !v.is_empty() => v.clone(),
                        _ => eval_text(asset, data)?,
                    };
                    render_image(
                        ctx,
                        context,
                        cache,
                        &value,
                        state.image_fit,
                        eval_number(x, data)?,
                        eval_number(y, data)?,
                        eval_number(width, data)?,
                        eval_number(height, data)?,
                    )?;
                }
                DrawOp::LoadImage { path, alias } => {
                    let path = eval_text(path, data)?;
                    let alias = eval_text(alias, data)?;
                    load_runtime_image_asset(cache, host_assets, &alias, &path)?;
                }
                DrawOp::TextBox {
                    text,
                    x,
                    y,
                    width,
                    height,
                } => {
                    let value = match runtime.and_then(|r| r.value_override.as_ref()) {
                        Some(v) if !v.is_empty() => v.clone(),
                        _ => eval_text(text, data)?,
                    };
                    render_textbox(
                        &layout,
                        ctx,
                        &state,
                        &value,
                        eval_number(x, data)?,
                        eval_number(y, data)?,
                        eval_number(width, data)?,
                        eval_number(height, data)?,
                    )?;
                }
            }
        }
    }

    Ok(warnings)
}

pub fn render_pdf(
    page: &Page,
    frames: &[FrameDecl],
    draw_frames: &[FrameDrawBlock],
    output_path: &Path,
    data: Option<&Value>,
    context: &RenderContext,
    host_assets: &HostAssetPolicy,
) -> Result<RenderOutcome, RenderError> {
    let mut cache = RenderCache::default();
    render_pdf_with_cache(
        page,
        frames,
        draw_frames,
        output_path,
        data,
        context,
        host_assets,
        &mut cache,
    )
}

pub fn render_pdf_with_cache(
    page: &Page,
    frames: &[FrameDecl],
    draw_frames: &[FrameDrawBlock],
    output_path: &Path,
    data: Option<&Value>,
    context: &RenderContext,
    host_assets: &HostAssetPolicy,
    cache: &mut RenderCache,
) -> Result<RenderOutcome, RenderError> {
    let surface = PdfSurface::new(page.width, page.height, output_path)?;
    let ctx = Context::new(&surface)?;
    let mut frame_state = build_initial_frame_state(frames);
    let script_start = Instant::now();
    run_frame_scripts(&mut frame_state, data, context, &mut cache.script_runtime)?;
    let script_time = script_start.elapsed();

    let draw_start = Instant::now();
    let warnings = execute_draw(&ctx, draw_frames, &frame_state, data, context, cache, host_assets)?;
    let draw_time = draw_start.elapsed();
    surface.finish();

    Ok(RenderOutcome {
        warnings,
        script_time,
        draw_time,
    })
}

pub fn render_png(
    page: &Page,
    frames: &[FrameDecl],
    draw_frames: &[FrameDrawBlock],
    output_path: &Path,
    data: Option<&Value>,
    context: &RenderContext,
    host_assets: &HostAssetPolicy,
    dpi: u16,
    dither: Option<DitherType>,
    remap_palette_source: Option<&str>,
) -> Result<RenderOutcome, RenderError> {
    let mut cache = RenderCache::default();
    render_png_with_cache(
        page,
        frames,
        draw_frames,
        output_path,
        data,
        context,
        host_assets,
        dpi,
        dither,
        remap_palette_source,
        &mut cache,
    )
}

pub fn render_png_with_cache(
    page: &Page,
    frames: &[FrameDecl],
    draw_frames: &[FrameDrawBlock],
    output_path: &Path,
    data: Option<&Value>,
    context: &RenderContext,
    host_assets: &HostAssetPolicy,
    dpi: u16,
    dither: Option<DitherType>,
    remap_palette_source: Option<&str>,
    cache: &mut RenderCache,
) -> Result<RenderOutcome, RenderError> {
    cache.image_remap = build_image_remap_config(dither, remap_palette_source)?;

    let scale = dpi as f64 / 72.0;
    let (w, h) = normalize_surface_dims(page.width * scale, page.height * scale);
    let surface = ImageSurface::create(Format::ARgb32, w, h)?;
    let ctx = Context::new(&surface)?;
    ctx.scale(scale, scale);

    let mut frame_state = build_initial_frame_state(frames);
    let script_start = Instant::now();
    run_frame_scripts(&mut frame_state, data, context, &mut cache.script_runtime)?;
    let script_time = script_start.elapsed();

    let draw_start = Instant::now();
    let warnings = execute_draw(&ctx, draw_frames, &frame_state, data, context, cache, host_assets)?;
    let draw_time = draw_start.elapsed();
    surface.flush();

    let mut file = File::create(output_path).map_err(|e| RenderError::ImageDecode {
        alias: "<output>".to_string(),
        path: output_path.to_path_buf(),
        message: format!("failed to create output: {e}"),
    })?;
    surface
        .write_to_png(&mut file)
        .map_err(|e| RenderError::ImageDecode {
            alias: "<output>".to_string(),
            path: output_path.to_path_buf(),
            message: format!("failed to create png: {e}"),
        })?;

    Ok(RenderOutcome {
        warnings,
        script_time,
        draw_time,
    })
}
