use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use image::{ImageFormat, ImageReader};
use ttf_parser::{Face, name_id};

use crate::{
    package::MarmotPackage,
    parser::{AssetType, DrawEntry, Template},
};

const MAX_ASSET_BYTES: u64 = 50 * 1024 * 1024; // NOTE: 50 MiB
const MAX_IMAGE_PIXELS: u64 = 40_000_000; // NOTE: 40 MP

#[derive(Debug, Clone, Default)]
pub struct RenderContext {
    pub fonts: HashMap<String, RegisteredFont>,
    pub assets: HashMap<String, RegisteredAsset>,
    pub scripts: HashMap<String, String>,
    pub layer_script_plan: Vec<LayerScriptPlanEntry>,
    pub frame_script_plan: Vec<FrameScriptPlanEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerScriptPlanEntry {
    pub layer_index: u32,
    pub layer_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameScriptPlanEntry {
    pub frame_index: u32,
    pub frame_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisteredFont {
    pub path: PathBuf,
    pub family_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisteredAsset {
    pub path: PathBuf,
    pub name: String,
    pub ty: AssetType,
    pub byte_len: u64,
    pub image: Option<RegisteredImageInfo>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisteredImageInfo {
    pub format: String,
    pub width: u32,
    pub height: u32,
}

impl RenderContext {
    pub fn resolve_font(&self, name: &str) -> Option<&RegisteredFont> {
        self.fonts.get(name)
    }
    pub fn resolve_asset(&self, name: &str) -> Option<&RegisteredAsset> {
        self.assets.get(name)
    }
}

fn is_allowed_image_format(format: ImageFormat) -> bool {
    matches!(
        format,
        ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP
    )
}

fn image_format_label(format: ImageFormat) -> String {
    format!("{format:?}").to_lowercase()
}

fn register_font(path: PathBuf) -> Result<RegisteredFont> {
    fontconfig::add_app_font_file(&path).map_err(|err| anyhow!("{err}"))?;

    let family_name = read_font_family_name(&path)?;

    Ok(RegisteredFont { path, family_name })
}

fn register_asset(name: String, path: PathBuf, ty: AssetType) -> Result<RegisteredAsset> {
    let (byte_len, bytes) = read_asset_bytes(&name, &path)?;

    let image = match ty {
        AssetType::Image => Some(validate_image_bytes(&name, &path, &bytes)?),
    };

    Ok(RegisteredAsset {
        path,
        name,
        ty,
        byte_len,
        image: image,
    })
}

fn read_asset_bytes(name: &str, path: &Path) -> Result<(u64, Vec<u8>)> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("asset {name} metadata read failed: {}", path.display()))?;

    if !metadata.is_file() {
        bail!("asset {name} path is not file: {}", path.display());
    }

    let byte_len = metadata.len();
    if byte_len == 0 {
        bail!("asset {name} file is empty: {}", path.display());
    }
    if byte_len > MAX_ASSET_BYTES {
        bail!(
            "asset {name} too large: {} bytes (max {}) at {}",
            byte_len,
            MAX_ASSET_BYTES,
            path.display()
        );
    }

    let bytes =
        fs::read(path).with_context(|| format!("asset {name} read failed: {}", path.display()))?;

    Ok((byte_len, bytes))
}

pub fn load_host_image_asset(alias: &str, path: &Path) -> Result<RegisteredAsset> {
    let (byte_len, bytes) = read_asset_bytes(alias, path)?;
    let image = validate_image_bytes(alias, path, &bytes)?;

    Ok(RegisteredAsset {
        path: path.to_path_buf(),
        name: alias.to_string(),
        ty: AssetType::Image,
        byte_len,
        image: Some(image),
    })
}

pub fn validate_image_bytes(name: &str, path: &Path, bytes: &[u8]) -> Result<RegisteredImageInfo> {
    let format = image::guess_format(bytes).with_context(|| {
        format!(
            "asset {name} iamge probe failed (unknown/invalid header): {}",
            path.display()
        )
    })?;

    if !is_allowed_image_format(format) {
        bail!(
            "asset {name} image format not allowed: {} at {}",
            image_format_label(format),
            path.display()
        );
    }

    let (width, height) = ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .with_context(|| {
            format!(
                "asset {name} image dimensions read failed ({}) at {}",
                image_format_label(format),
                path.display()
            )
        })?;

    if width == 0 || height == 0 {
        bail!(
            "asset {name} image has zero dimension: {}x{} at {}",
            width,
            height,
            path.display()
        );
    }

    let pixels = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or_else(|| {
            anyhow!(
                "asset {name} image pixel count overflow at {}",
                path.display()
            )
        })?;

    if pixels > MAX_IMAGE_PIXELS {
        bail!(
            "asset {name} iamge too large: {}x{} ({} px, max {}) at {}",
            width,
            height,
            pixels,
            MAX_IMAGE_PIXELS,
            path.display()
        );
    }

    Ok(RegisteredImageInfo {
        format: image_format_label(format),
        width,
        height,
    })
}

pub fn build_render_context(template: &Template, package: &MarmotPackage) -> Result<RenderContext> {
    let mut fonts = HashMap::new();
    let mut assets = HashMap::new();
    let mut scripts = HashMap::new();

    for font_decl in &template.fonts {
        if fonts.contains_key(&font_decl.name) {
            bail!("duplicate font alias: {}", font_decl.name);
        }

        let path = package.resolve_path(&font_decl.path).with_context(|| {
            format!(
                "failed to resolve font alias {} at {}",
                font_decl.name, font_decl.path
            )
        })?;

        let registered = register_font(path)?;

        fonts.insert(font_decl.name.clone(), registered);
    }

    for asset_decl in &template.assets {
        if assets.contains_key(&asset_decl.name) {
            bail!("duplicate asset alias: {}", asset_decl.name);
        }

        let path = package.resolve_path(&asset_decl.path).with_context(|| {
            format!(
                "failed to resolve asset alias {} at {}",
                asset_decl.name, asset_decl.path
            )
        })?;

        let registered = register_asset(asset_decl.name.clone(), path, asset_decl.ty.clone())?;

        assets.insert(asset_decl.name.clone(), registered);
    }

    let layer_ids: HashSet<&str> = template.layers.iter().map(|l| l.id.as_str()).collect();
    let frame_ids: HashSet<&str> = template.frames.iter().map(|frame| frame.id.as_str()).collect();

    for layer_id in &layer_ids {
        if frame_ids.contains(layer_id) {
            bail!(
                "ambiguous script id '{}' is declared as both layer id and frame id",
                layer_id
            );
        }
    }

    let script_files = package.list_files_under("scripts")?;
    for filename in script_files {
        let path = package.resolve_path(format!("scripts/{filename}").as_str())?;
        let ext = path.extension().and_then(|s| s.to_str());
        if ext != Some("lua") {
            bail!(
                "invalid script file extension (expected .lua): {}",
                path.display()
            );
        }

        let frame_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow!("invalid script filename: {}", path.display()))?
            .to_string();

        if !frame_ids.contains(frame_id.as_str()) && !layer_ids.contains(frame_id.as_str()) {
            bail!(
                "unknown script file '{}' (no matching layer/frame id '{}')",
                path.display(),
                frame_id
            );
        }

        if scripts.contains_key(&frame_id) {
            bail!("duplicate script for frame id: {}", frame_id);
        }

        let source = fs::read_to_string(&path)
            .with_context(|| format!("failed to read script source: {}", path.display()))?;

        scripts.insert(frame_id, source);
    }

    let mut drawn_layer_indices: HashSet<u32> = HashSet::new();
    let mut drawn_frame_indices: HashSet<u32> = HashSet::new();
    for entry in &template.draw_entries {
        match entry {
            DrawEntry::Frame(frame) => {
                drawn_frame_indices.insert(frame.index);
            }
            DrawEntry::Layer(layer) => {
                drawn_layer_indices.insert(layer.index);
                for frame in &layer.frames {
                    drawn_frame_indices.insert(frame.index);
                }
            }
        }
    }

    let mut layer_script_plan = Vec::new();
    for layer in &template.layers {
        if !drawn_layer_indices.contains(&layer.index) {
            continue;
        }
        if scripts.contains_key(&layer.id) {
            layer_script_plan.push(LayerScriptPlanEntry {
                layer_index: layer.index,
                layer_id: layer.id.clone(),
            });
        }
    }

    let mut frame_script_plan = Vec::new();
    for frame in &template.frames {
        if !drawn_frame_indices.contains(&frame.index) {
            continue;
        }
        if scripts.contains_key(&frame.id) {
            frame_script_plan.push(FrameScriptPlanEntry {
                frame_index: frame.index,
                frame_id: frame.id.clone(),
            });
        }
    }

    Ok(RenderContext {
        fonts,
        assets,
        scripts,
        layer_script_plan,
        frame_script_plan,
    })
}

fn read_name(face: &Face, id: u16) -> Option<String> {
    face.names()
        .into_iter()
        .filter(|name| name.name_id == id)
        .find_map(|name| name.to_string())
}

fn read_font_family_name(path: &Path) -> Result<String> {
    let bytes =
        fs::read(path).with_context(|| format!("failed to read font file: {}", path.display()))?;

    let face = Face::parse(&bytes, 0)
        .map_err(|err| anyhow!("failed to parse font file {}: {err:?}", path.display()))?;

    if let Some(family) = read_name(&face, name_id::FAMILY) {
        return Ok(family);
    }

    if let Some(full_name) = read_name(&face, name_id::FULL_NAME) {
        return Ok(full_name);
    }

    if let Some(postscript_name) = read_name(&face, name_id::POST_SCRIPT_NAME) {
        return Ok(postscript_name);
    }

    for name in face.names() {
        eprintln!(
            "name_id={}, encoding_id={}, language_id={}, string={:?}",
            name.name_id,
            name.encoding_id,
            name.language_id,
            name.to_string(),
        );
    }

    bail!("font file has no readable family name: {}", path.display())
}

mod fontconfig {
    use std::{
        ffi::CString,
        os::raw::{c_int, c_uchar},
        path::Path,
    };

    type FcBool = c_int;

    #[repr(C)]
    struct FcConfig {
        _private: [u8; 0],
    }

    #[link(name = "fontconfig")]
    unsafe extern "C" {
        fn FcConfigGetCurrent() -> *mut FcConfig;
        fn FcConfigAppFontAddFile(config: *mut FcConfig, file: *const c_uchar) -> FcBool;
        fn FcConfigBuildFonts(config: *mut FcConfig) -> FcBool;
    }

    pub fn add_app_font_file(path: &Path) -> Result<(), String> {
        let path = path
            .to_str()
            .ok_or_else(|| format!("font path is not valid UTF-8: {}", path.display()))?;

        let c_path =
            CString::new(path).map_err(|_| format!("font path contains NUL byte: {path}"))?;

        unsafe {
            let config = FcConfigGetCurrent();
            if config.is_null() {
                return Err("fontconfig returned null current config".to_string());
            }

            let added = FcConfigAppFontAddFile(config, c_path.as_ptr() as *const c_uchar);
            if added == 0 {
                return Err(format!("fontconfig failed to add font file: {path}"));
            }

            let built = FcConfigBuildFonts(config);
            if built == 0 {
                return Err("fontconfig failed to rebuild font set".to_string());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::package::{MarmotPackage, PackageBuilderOptions, create_package};
    use crate::parser::{AssetDecl, FontDecl, Page, Template};
    use std::fs;
    use tempfile::tempdir;
    fn write_file(name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let dir = tempdir().unwrap();
        let path = dir.path().join(name);
        fs::write(&path, bytes).unwrap();
        // keep dir alive by leaking for test lifetime
        std::mem::forget(dir);
        path
    }

    #[test]
    fn register_asset_accepts_valid_png_image() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let path = dir.path().join("ok.png");
        // Build real 1x1 PNG with encoder
        let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]));
        img.save_with_format(&path, image::ImageFormat::Png)
            .unwrap();
        let asset = register_asset("logo".to_string(), path.clone(), AssetType::Image).unwrap();
        assert_eq!(asset.name, "logo");
        assert_eq!(asset.path, path);
        assert_eq!(asset.ty, AssetType::Image);
        assert!(asset.byte_len > 0);
        let image = asset.image.expect("expected image metadata");
        assert_eq!(image.width, 1);
        assert_eq!(image.height, 1);
        assert_eq!(image.format.to_ascii_lowercase(), "png");
    }

    #[test]
    fn register_asset_errors_for_missing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("missing.png");
        let err = register_asset("missing".to_string(), path, AssetType::Image).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("asset missing"));
        assert!(msg.contains("metadata"));
    }

    #[test]
    fn register_asset_errors_for_empty_file() {
        let path = write_file("empty.png", &[]);
        let err = register_asset("empty".to_string(), path, AssetType::Image).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("asset empty"));
        assert!(msg.contains("empty"));
    }

    #[test]
    fn register_asset_errors_for_non_image_bytes() {
        let path = write_file("not-image.bin", b"this is not image data");
        let err = register_asset("bad".to_string(), path, AssetType::Image).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("asset bad"));
        assert!(msg.contains("probe failed") || msg.contains("image format not allowed"));
    }

    #[test]
    fn register_asset_errors_for_truncated_image() {
        // PNG signature only: format may probe as png, decode/dimensions must fail
        let path = write_file("truncated.png", &[137, 80, 78, 71, 13, 10, 26, 10]);
        let err = register_asset("trunc".to_string(), path, AssetType::Image).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("asset trunc"));
        assert!(
            msg.contains("dimensions read failed")
                || msg.contains("decode failed")
                || msg.contains("probe failed")
        );
    }

    fn make_minimal_package() -> (tempfile::TempDir, MarmotPackage) {
        let dir = tempdir().unwrap();
        let template_file = dir.path().join("template.psl");
        let package_file = dir.path().join("bundle.marmot");

        fs::write(
            &template_file,
            "%!PSL 0.1\npage 10 10\nframes begin\n  1 FRAME_1\nend\nlayers begin\n  layer 1 LAYER_1 begin\n    1 FRAME_1\n  end\nend\ndraw begin\n  layer 1 begin\n    frame 1 begin\n    end\n  end\nend\n",
        )
        .unwrap();

        create_package(PackageBuilderOptions {
            template_file,
            output_file: package_file.clone(),
            assets: vec![],
            fonts: vec![],
            scripts: vec![],
            remap_file: None,
        })
        .unwrap();

        let package = MarmotPackage::open(&package_file).unwrap();
        (dir, package)
    }

    fn make_package_with_real_font_and_asset() -> (tempfile::TempDir, MarmotPackage) {
        let dir = tempdir().unwrap();
        let template_file = dir.path().join("template.psl");
        let package_file = dir.path().join("bundle.marmot");

        let root = std::env::current_dir().unwrap();
        let font = root.join("test/fonts/Kablammo.ttf");
        let asset = root.join("test/images/sprout-basket.png");

        fs::write(
            &template_file,
            "%!PSL 0.1\npage 10 10\nframes begin\n  1 FRAME_1\nend\nlayers begin\n  layer 1 LAYER_1 begin\n    1 FRAME_1\n  end\nend\ndraw begin\n  layer 1 begin\n    frame 1 begin\n    end\n  end\nend\n",
        )
        .unwrap();

        create_package(PackageBuilderOptions {
            template_file,
            output_file: package_file.clone(),
            assets: vec![asset],
            fonts: vec![font],
            scripts: vec![],
            remap_file: None,
        })
        .unwrap();

        let package = MarmotPackage::open(&package_file).unwrap();
        (dir, package)
    }

    fn empty_template() -> Template {
        Template {
            version: "0.1".to_string(),
            page: Page {
                width: 10.0,
                height: 10.0,
            },
            slots: vec![],
            fonts: vec![],
            assets: vec![],
            frames: vec![],
            layers: vec![],
            draw_layers: vec![],
            draw_entries: vec![],
        }
    }

    #[test]
    fn build_render_context_errors_on_duplicate_font_alias() {
        let (_dir, package) = make_package_with_real_font_and_asset();

        let mut template = empty_template();
        template.fonts = vec![
            FontDecl {
                name: "brand".to_string(),
                path: "fonts/Kablammo.ttf".to_string(),
            },
            FontDecl {
                name: "brand".to_string(),
                path: "fonts/B.ttf".to_string(),
            },
        ];

        let err = build_render_context(&template, &package)
            .unwrap_err()
            .to_string();
        assert!(err.contains("duplicate font alias: brand"));
    }

    #[test]
    fn build_render_context_errors_on_duplicate_asset_alias() {
        let (_dir, package) = make_package_with_real_font_and_asset();

        let mut template = empty_template();
        template.assets = vec![
            AssetDecl {
                name: "logo".to_string(),
                path: "assets/sprout-basket.png".to_string(),
                ty: AssetType::Image,
            },
            AssetDecl {
                name: "logo".to_string(),
                path: "assets/B.png".to_string(),
                ty: AssetType::Image,
            },
        ];

        let err = build_render_context(&template, &package)
            .unwrap_err()
            .to_string();
        assert!(err.contains("duplicate asset alias: logo"));
    }

    #[test]
    fn build_render_context_errors_when_asset_path_missing() {
        let (_dir, package) = make_minimal_package();

        let mut template = empty_template();
        template.assets = vec![AssetDecl {
            name: "logo".to_string(),
            path: "assets/missing.png".to_string(),
            ty: AssetType::Image,
        }];

        let err = build_render_context(&template, &package)
            .unwrap_err()
            .to_string();
        assert!(err.contains("failed to resolve asset alias logo"));
        assert!(err.contains("assets/missing.png"));
    }
}
