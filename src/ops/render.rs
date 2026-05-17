use anyhow::{Context, Result, anyhow, bail};
use serde_json::Value;
use std::{fs::read_to_string, time::Instant};

use crate::{
    OutputType, RenderArgs,
    ops::{common::load_remap_palette_if_needed, parse_template_source},
    package::MarmotPackage,
    renderer::{HostAssetPolicy, render_pdf, render_png},
    resources::build_render_context,
    util::format_duration,
    validator::validate_data,
};

pub fn render(args: RenderArgs) -> Result<()> {
    let total_start = Instant::now();
    let prep_start = Instant::now();

    let package = MarmotPackage::open(&args.package_file)?;
    let template_source = package.read_template_source()?;
    let template = parse_template_source(&template_source)?;

    let data: Option<Value> = if let Some(data_file) = args.data_file {
        let data_source = read_to_string(&data_file)
            .with_context(|| format!("failed to read data: {}", data_file.display()))?;
        let data: Value = serde_json::from_str(&data_source)
            .with_context(|| format!("failed to parse JSON: {}", data_file.display()))?;

        if let Err(errors) = validate_data(&template, &data) {
            for error in errors {
                eprintln!("validation error: {error:?}");
            }
            bail!("data does not match template slots")
        }

        Some(data)
    } else {
        None
    };

    let render_context = build_render_context(&template, &package)?;
    let remap_source = load_remap_palette_if_needed(&package, args.dither)?;
    let host_asset_policy = HostAssetPolicy {
        allow: args.allow_host_assets,
        cwd: std::env::current_dir().context("failed to read current working directory")?,
    };

    let prep = prep_start.elapsed();
    let render_start = Instant::now();

    let outcome = match args.output_type {
        OutputType::PDF => render_pdf(
            &template.page,
            &template.frames,
            &template.layers,
            &template.draw_entries,
            &args.output_file,
            data.as_ref(),
            &render_context,
            &host_asset_policy,
        )
        .map_err(|err| anyhow!("failed to render PDF: {err:?}"))?,
        OutputType::PNG => render_png(
            &template.page,
            &template.frames,
            &template.layers,
            &template.draw_entries,
            &args.output_file,
            data.as_ref(),
            &render_context,
            &host_asset_policy,
            args.dpi,
            args.dither,
            remap_source.as_deref(),
        )
        .map_err(|err| anyhow!("failed to render PNG: {err:?}"))?,
    };

    for frame_idx in outcome.warnings.empty_value_frames {
        eprintln!("warning: frame {} has empty value", frame_idx);
    }
    for frame_idx in outcome.warnings.unused_fill_color_frames {
        eprintln!(
            "warning: frame {} has unused fill_color override",
            frame_idx
        );
    }
    for frame_idx in outcome.warnings.unused_stroke_color_frames {
        eprintln!(
            "warning: frame {} has unused stroke_color override",
            frame_idx
        );
    }
    for frame_idx in outcome.warnings.unused_stroke_width_frames {
        eprintln!(
            "warning: frame {} has unused stroke_width override",
            frame_idx
        );
    }
    for frame_idx in outcome.warnings.unused_text_color_frames {
        eprintln!(
            "warning: frame {} has unused text_color override",
            frame_idx
        );
    }

    let render = render_start.elapsed();
    let total = total_start.elapsed();

    println!("wrote {}", args.output_file.display());
    if args.enable_timings {
        eprintln!("timings:");
        eprintln!("    prep:    {}", format_duration(prep));
        eprintln!("    render:  {}", format_duration(render));
        eprintln!("    script:  {}", format_duration(outcome.script_time));
        eprintln!("    draw:    {}", format_duration(outcome.draw_time));
        eprintln!("    total:   {}", format_duration(total));
    }

    Ok(())
}
