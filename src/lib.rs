pub mod cli;
mod lexer;
mod ops;
mod package;
mod parser;
mod renderer;
mod resources;
mod scripting;
mod util;
mod validator;

use anyhow::{Result, anyhow, bail};
use std::{path::PathBuf, time::Duration};

pub struct RenderStats {
    pub avg: Duration,
    pub min: Duration,
    pub max: Duration,
    pub p90: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub p999: Duration,
}

pub struct CheckArgs {
    pub package_file: PathBuf,
    pub data_file: PathBuf,
}

pub struct RenderArgs {
    pub package_file: PathBuf,
    pub data_file: Option<PathBuf>,
    pub output_file: PathBuf,
    pub enable_timings: bool,
    pub output_type: OutputType,
    pub dpi: u16,
    pub dither: Option<DitherType>,
    pub allow_host_assets: bool,
}

pub struct PackArgs {
    pub template_file: PathBuf,
    pub name: String,
    pub output_dir: Option<PathBuf>,
    pub assets: Vec<PathBuf>,
    pub fonts: Vec<PathBuf>,
    pub scripts: Vec<PathBuf>,
    pub remap_file: Option<PathBuf>,
}

pub struct BatchArgs {
    pub package_file: PathBuf,
    pub records_file: PathBuf,
    pub output_dir: PathBuf,
    pub output_name: String,
    pub jobs: usize,
    pub trust_data: bool,
    pub enable_timings: bool,
    pub output_type: OutputType,
    pub dpi: u16,
    pub dither: Option<DitherType>,
    pub allow_host_assets: bool,
}

#[derive(Clone, Copy)]
pub enum DitherType {
    Floyd,
    Atkinson,
    Stucki,
    Burkes,
    Jarvis,
    Sierra3,
}

impl DitherType {
    pub fn try_from_word(word: &str) -> Result<Self> {
        match word.to_ascii_lowercase().as_str() {
            "floyd" | "floyd-steinberg" | "steinberg" => Ok(Self::Floyd),
            "atkinson" => Ok(Self::Atkinson),
            "stucki" => Ok(Self::Stucki),
            "burkes" => Ok(Self::Burkes),
            "jarvis" | "jarvis-judice-ninke" => Ok(Self::Jarvis),
            "sierra3" | "sierra-3" | "sierra" => Ok(Self::Sierra3),
            _ => bail!("invalid dither type: {}", word),
        }
    }
}

#[derive(Clone)]
pub enum OutputType {
    PDF,
    PNG,
}

impl OutputType {
    pub fn try_from_word(word: &str) -> Result<Self> {
        match word.to_lowercase().as_str() {
            "png" => Ok(Self::PNG),
            "pdf" => Ok(Self::PDF),
            _ => Err(anyhow!("invalid output type value: {}", word)),
        }
    }
}
