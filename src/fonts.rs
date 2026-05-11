use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow, bail};
use ttf_parser::{Face, name_id};

use crate::{package::MarmotPackage, parser::Template};

#[derive(Debug, Clone, Default)]
pub struct RenderContext {
    pub fonts: HashMap<String, RegisteredFont>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegisteredFont {
    pub path: PathBuf,
    pub family_name: String,
}

impl RenderContext {
    pub fn resolve_font(&self, name: &str) -> Option<&RegisteredFont> {
        self.fonts.get(name)
    }
}

fn register_font(alias: &str, path: PathBuf) -> Result<RegisteredFont> {
    fontconfig::add_app_font_file(&path).map_err(|err| anyhow!("{err}"))?;

    let family_name = read_font_family_name(&path)?;

    Ok(RegisteredFont { path, family_name })
}

pub fn build_render_context(template: &Template, package: &MarmotPackage) -> Result<RenderContext> {
    let mut fonts = HashMap::new();

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

        let registered = register_font(&font_decl.name, path)?;

        fonts.insert(font_decl.name.clone(), registered);
    }

    Ok(RenderContext { fonts })
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
        return Ok(full_name)
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
        os::raw::{c_char, c_int, c_uchar},
        path::Path,
        ptr,
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
