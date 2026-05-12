#[cfg(test)]
mod test;

use std::{collections::HashMap, path::{Path, PathBuf}};

use crate::parser::{AssetType, DrawOp, NumberValue, Page, TextValue};
use crate::resources::{RegisteredFont, RenderContext};
use cairo::{Context, PdfSurface};
use pango::FontDescription;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum RenderOp {
    SetRgb {
        r: f64,
        g: f64,
        b: f64,
    },
    SetStrokeWidth {
        width: f64,
    },
    SetFontSize {
        size: f64,
    },
    SetTextAlignment {
        align: TextAlign,
    },
    SetVerticalAlignment {
        align: VerticalAlign,
    },
    SetLineBreakMode {
        line_break: LineBreakMode,
    },
    SetTextFit {
        fit: TextFit,
    },
    SetTextFitMinSize {
        min: f64,
    },
    SetTextFitMaxSize {
        max: f64,
    },
    SetFontFamily {
        font: String,
    },
    SetImageFit {
        fit: ImageFit,
    },
    StrokeLine {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    StrokeRect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    FillRect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    TextBox {
        text: String,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    Image {
        asset: String,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
}

#[derive(Debug, PartialEq)]
pub enum RenderError {
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
    ImageDecode {
        alias: String,
        path: PathBuf,
        message: String,
    },
    Cairo(cairo::Error),
}

#[derive(Default)]
pub struct RenderCache {
    image_surfaces: HashMap<String, cairo::ImageSurface>,
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

fn eval_text(value: &TextValue, data: Option<&Value>) -> Result<String, RenderError> {
    match value {
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

fn lower_draw_ops(draw_ops: &[DrawOp], data: Option<&Value>) -> Result<Vec<RenderOp>, RenderError> {
    let mut render_ops = Vec::new();
    let mut pending_path: Option<PendingPath> = None;

    for draw_op in draw_ops {
        match draw_op {
            DrawOp::SetImageFit { fit } => {
                render_ops.push(RenderOp::SetImageFit { fit: *fit });
            }
            DrawOp::SetFontFamily { font } => {
                render_ops.push(RenderOp::SetFontFamily {
                    font: eval_text(font, data)?,
                });
            }
            DrawOp::SetTextFitMaxSize { max } => {
                render_ops.push(RenderOp::SetTextFitMaxSize {
                    max: eval_number(max, data)?,
                });
            }
            DrawOp::SetTextFitMinSize { min } => {
                render_ops.push(RenderOp::SetTextFitMinSize {
                    min: eval_number(min, data)?,
                });
            }
            DrawOp::SetTextFit { fit } => {
                render_ops.push(RenderOp::SetTextFit { fit: *fit });
            }
            DrawOp::SetLineBreakMode { line_break } => {
                render_ops.push(RenderOp::SetLineBreakMode {
                    line_break: *line_break,
                });
            }
            DrawOp::SetVerticalAlignment { align } => {
                render_ops.push(RenderOp::SetVerticalAlignment { align: *align });
            }
            DrawOp::SetTextAlignment { align } => {
                render_ops.push(RenderOp::SetTextAlignment { align: *align });
            }
            DrawOp::SetFontSize { size } => {
                render_ops.push(RenderOp::SetFontSize {
                    size: eval_number(size, data)?,
                });
            }
            DrawOp::SetRgb { r, g, b } => {
                render_ops.push(RenderOp::SetRgb {
                    r: eval_number(r, data)?,
                    g: eval_number(g, data)?,
                    b: eval_number(b, data)?,
                });
            }
            DrawOp::SetStrokeWidth { width } => {
                render_ops.push(RenderOp::SetStrokeWidth {
                    width: eval_number(width, data)?,
                });
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
                })
            }
            DrawOp::Stroke => {
                let path = pending_path
                    .take()
                    .expect("parser should prevent stroke without a current path");
                match path {
                    PendingPath::Line { x1, y1, x2, y2 } => {
                        render_ops.push(RenderOp::StrokeLine { x1, y1, x2, y2 });
                    }
                    PendingPath::Rect {
                        x,
                        y,
                        width,
                        height,
                    } => {
                        render_ops.push(RenderOp::StrokeRect {
                            x,
                            y,
                            width,
                            height,
                        });
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
                        render_ops.push(RenderOp::FillRect {
                            x,
                            y,
                            width,
                            height,
                        });
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
                render_ops.push(RenderOp::Image {
                    asset: eval_text(asset, data)?,
                    x: eval_number(x, data)?,
                    y: eval_number(y, data)?,
                    width: eval_number(width, data)?,
                    height: eval_number(height, data)?,
                });
            }
            DrawOp::TextBox {
                text,
                x,
                y,
                width,
                height,
            } => {
                render_ops.push(RenderOp::TextBox {
                    text: eval_text(text, data)?,
                    x: eval_number(x, data)?,
                    y: eval_number(y, data)?,
                    width: eval_number(width, data)?,
                    height: eval_number(height, data)?,
                });
            }
        }
    }

    Ok(render_ops)
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

fn premultiply(channel: u8, alpha: u8) -> u8 {
    ((u16::from(channel) * u16::from(alpha) + 127) / 255) as u8
}

fn load_image_surface(alias: &str, path: &Path) -> Result<cairo::ImageSurface, RenderError> {
    let dyn_img = image::open(path).map_err(|err| RenderError::ImageDecode {
        alias: alias.to_string(),
        path: path.to_path_buf(),
        message: err.to_string(),
    })?;

    let rgba = dyn_img.to_rgba8();
    let (width, height) = rgba.dimensions();

    let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, width as i32, height as i32)?;
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
    alias: &str,
    path: &Path,
) -> Result<&'a cairo::ImageSurface, RenderError> {
    if !cache.image_surfaces.contains_key(alias) {
        let surface = load_image_surface(alias, path)?;
        cache.image_surfaces.insert(alias.to_string(), surface);
    }

    Ok(cache
        .image_surfaces
        .get(alias)
        .expect("cache insert must exist"))
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

    let registered =
        context
            .resolve_asset(asset)
            .ok_or_else(|| RenderError::MissingAssetAlias {
                alias: asset.to_string(),
            })?;

    if registered.ty != AssetType::Image {
        return Err(RenderError::WrongAssetType {
            alias: asset.to_string(),
            found: registered.ty.clone(),
        });
    }

    let surface = get_or_load_image_surface(cache, asset, &registered.path)?;
    let src_w = surface.width() as f64;
    let src_h = surface.height() as f64;

    let (draw_x, draw_y, scale_x, scale_y) = match fit {
        ImageFit::Stretch => (x, y, width / src_w, height / src_h),
        ImageFit::Contain => {
            let s = (width / src_w).min(height / src_h);
            let draw_w = src_w * s;
            let draw_h = src_h * s;
            let dx = x + (width - draw_w) / 2.0;
            let dy = y + (height - draw_h) / 2.0;
            (dx, dy, s, s)
        }
        ImageFit::Cover => {
            let s = (width / src_w).max(height / src_h);
            let draw_w = src_w * s;
            let draw_h = src_h * s;
            let dx = x + (width - draw_w) / 2.0;
            let dy = y + (height - draw_h) / 2.0;
            (dx, dy, s, s)
        }
    };

    ctx.save()?;
    ctx.rectangle(x, y, width, height);
    ctx.clip();

    ctx.translate(draw_x, draw_y);
    ctx.scale(scale_x, scale_y);

    let pattern = cairo::SurfacePattern::create(surface);
    pattern.set_extend(cairo::Extend::Pad);
    pattern.set_filter(cairo::Filter::Best);

    ctx.set_source(&pattern)?;
    ctx.paint()?;
    ctx.restore()?;

    Ok(())
}

fn render_textbox(
    ctx: &Context,
    state: &RenderState,
    text: &str,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), RenderError> {
    let layout = pangocairo::functions::create_layout(ctx);

    let font_size = fitted_font_size(&layout, state, text, width, height);
    configure_text_layout(&layout, state, text, font_size, width, height);

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
    pangocairo::functions::show_layout(ctx, &layout);

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

fn execute_render_ops(
    ctx: &Context,
    render_ops: &[RenderOp],
    context: &RenderContext,
    cache: &mut RenderCache,
) -> Result<(), RenderError> {
    let mut state = RenderState::default();

    for op in render_ops {
        match op {
            RenderOp::SetImageFit { fit } => {
                state.image_fit = *fit;
            }
            RenderOp::SetFontFamily { font } => {
                state.font = resolve_current_font(context, font);
            }
            RenderOp::SetTextFitMaxSize { max } => {
                state.text_fit_max_size = *max;
            }
            RenderOp::SetTextFitMinSize { min } => {
                state.text_fit_min_size = *min;
            }
            RenderOp::SetTextFit { fit } => {
                state.text_fit = *fit;
            }
            RenderOp::SetLineBreakMode { line_break } => {
                state.line_break = *line_break;
            }
            RenderOp::SetVerticalAlignment { align } => {
                state.vertical_align = *align;
            }
            RenderOp::SetTextAlignment { align } => {
                state.text_align = *align;
            }
            RenderOp::SetRgb { r, g, b } => {
                ctx.set_source_rgb(*r, *g, *b);
            }
            RenderOp::SetStrokeWidth { width } => {
                ctx.set_line_width(*width);
            }
            RenderOp::SetFontSize { size } => {
                state.font_size = *size;
            }
            RenderOp::StrokeLine { x1, y1, x2, y2 } => {
                ctx.move_to(*x1, *y1);
                ctx.line_to(*x2, *y2);
                ctx.stroke()?;
            }
            RenderOp::StrokeRect {
                x,
                y,
                width,
                height,
            } => {
                ctx.rectangle(*x, *y, *width, *height);
                ctx.stroke()?;
            }
            RenderOp::FillRect {
                x,
                y,
                width,
                height,
            } => {
                ctx.rectangle(*x, *y, *width, *height);
                ctx.fill()?;
            }
            RenderOp::Image {
                asset,
                x,
                y,
                width,
                height,
            } => {
                render_image(
                    ctx,
                    context,
                    cache,
                    asset,
                    state.image_fit,
                    *x,
                    *y,
                    *width,
                    *height,
                )?;
            }
            RenderOp::TextBox {
                text,
                x,
                y,
                width,
                height,
            } => {
                render_textbox(ctx, &state, text, *x, *y, *width, *height)?;
            }
        }
    }

    Ok(())
}

pub fn render_pdf_with_cache(
    page: &Page,
    draw_ops: &[DrawOp],
    output_path: &Path,
    data: Option<&Value>,
    context: &RenderContext,
    cache: &mut RenderCache,
) -> Result<(), RenderError> {
    let render_ops = lower_draw_ops(draw_ops, data)?;

    let surface = PdfSurface::new(page.width, page.height, output_path)?;
    let ctx = Context::new(&surface)?;

    execute_render_ops(&ctx, &render_ops, context, cache)?;

    surface.finish();

    Ok(())
}

pub fn render_pdf(
    page: &Page,
    draw_ops: &[DrawOp],
    output_path: &Path,
    data: Option<&Value>,
    context: &RenderContext,
) -> Result<(), RenderError> {
    let mut cache = RenderCache::default();
    render_pdf_with_cache(page, draw_ops, output_path, data, context, &mut cache)
}
