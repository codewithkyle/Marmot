use std::fs::read_to_string;

use anyhow::{Context, Result, anyhow};

use crate::{
    DitherType,
    lexer::Lexer,
    package::MarmotPackage,
    parser::{Parser, Template},
};

pub fn parse_template_source(template_source: &str) -> Result<Template> {
    let mut lexer = Lexer::new(template_source);
    let tokens = lexer
        .tokenize()
        .map_err(|err| anyhow!("failed to tokenize template: {err:?}"))?;
    let mut parser = Parser::new(tokens);
    parser
        .parse_template()
        .map_err(|err| anyhow!("failed to parse template: {err:?}"))
}

pub fn load_remap_palette_if_needed(
    pkg: &MarmotPackage,
    dither: Option<DitherType>,
) -> Result<Option<String>> {
    if dither.is_none() {
        return Ok(None);
    }
    let path = pkg
        .resolve_path("remap.plt")
        .context("--dither requires remap.plt in package")?;
    Ok(Some(read_to_string(&path).with_context(|| {
        format!("failed to read {}", path.display())
    })?))
}
