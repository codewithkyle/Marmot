mod lexer;
mod package;
mod parser;
mod renderer;
mod resources;
mod scripting;
mod validator;
pub mod cli;

use serde_json::Value;
use anyhow::{Context, Result, anyhow, bail};
use std::{
    fs::{File, create_dir_all, read_to_string},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

use crate::{
    cli::{BatchArgs, CheckArgs, PackArgs, RenderArgs, RenderStats}, lexer::Lexer, package::{MarmotPackage, PackageBuilderOptions, create_package}, parser::{Parser, Template}, renderer::{
        HostAssetPolicy, RenderCache, RenderWarnings, render_pdf, render_pdf_with_cache,
        render_png, render_png_with_cache,
    }, resources::{RenderContext, build_render_context}, validator::validate_data
};

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
    fn try_from_word(word: &str) -> Result<Self> {
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

fn parse_template_source(template_source: &str) -> Result<parser::Template> {
    let mut lexer = Lexer::new(template_source);
    let tokens = lexer
        .tokenize()
        .map_err(|err| anyhow!("failed to tokenize template: {err:?}"))?;
    let mut parser = Parser::new(tokens);
    parser
        .parse_template()
        .map_err(|err| anyhow!("failed to parse template: {err:?}"))
}

fn pack(args: PackArgs) -> Result<()> {
    let output_dir = args.output_dir.unwrap_or_else(|| PathBuf::from("."));
    ensure_dir_exists(&output_dir)?;

    let output_file = output_dir.join(format!("{}.marmot", args.name));

    let options = PackageBuilderOptions {
        template_file: args.template_file,
        output_file: output_file.clone(),
        assets: args.assets,
        fonts: args.fonts,
        scripts: args.scripts,
        remap_file: args.remap_file,
    };

    create_package(options)?;

    println!("wrote {}", output_file.display());

    Ok(())
}

struct BatchJob {
    line_no: usize,
    line: String,
}

enum BatchResult {
    Success {
        render_time: Duration,
        script_time: Duration,
        draw_time: Duration,
        warnings: RenderWarnings,
        line_no: usize,
        output_path: PathBuf,
    },
    Failed {
        line_no: usize,
        message: String,
        render_time: Option<Duration>,
    },
}

fn batch(args: BatchArgs) -> Result<()> {
    let total_start = Instant::now();
    let prep_start = Instant::now();

    let package = MarmotPackage::open(&args.package_file)?;
    let template_source = package.read_template_source()?;
    let template = Arc::new(parse_template_source(&template_source)?);
    let render_context = Arc::new(build_render_context(&template, &package)?);

    let jobs = resolve_jobs(args.jobs);
    println!("batch: jobs={}", jobs);

    let file = File::open(&args.records_file).with_context(|| {
        format!(
            "failed to open records file: {}",
            args.records_file.display()
        )
    })?;
    let reader = BufReader::new(file);

    let output_dir = Arc::new(args.output_dir.clone());
    let output_name = Arc::new(args.output_name.clone());
    let output_type = Arc::new(args.output_type.clone());
    let dpi = args.dpi;
    let dither = args.dither;
    let remap_source = Arc::new(load_remap_palette_if_needed(&package, args.dither)?);
    let trust_data = args.trust_data;
    let host_asset_policy = Arc::new(HostAssetPolicy {
        allow: args.allow_host_assets,
        cwd: std::env::current_dir().context("failed to read current working directory")?,
    });

    let (job_tx, job_rx) = mpsc::sync_channel::<BatchJob>(jobs * 4);
    let (result_tx, result_rx) = mpsc::channel::<BatchResult>();

    let shared_rx = Arc::new(Mutex::new(job_rx));
    let mut workers = Vec::with_capacity(jobs);

    let prep = prep_start.elapsed();
    let process_start = Instant::now();

    for _ in 0..jobs {
        let rx = Arc::clone(&shared_rx);
        let tx = result_tx.clone();
        let template = Arc::clone(&template);
        let render_context = Arc::clone(&render_context);
        let output_dir = Arc::clone(&output_dir);
        let output_name = Arc::clone(&output_name);
        let output_type = Arc::clone(&output_type);
        let remap_source = Arc::clone(&remap_source);
        let host_asset_policy = Arc::clone(&host_asset_policy);

        let handle = thread::spawn(move || {
            let mut render_cache = RenderCache::default();

            loop {
                let job = {
                    let guard = rx.lock().expect("job receiver lock poisoned");
                    guard.recv()
                };

                let job = match job {
                    Ok(v) => v,
                    Err(_) => break,
                };

                let result = process_batch_line(
                    job.line_no,
                    &job.line,
                    &template,
                    &render_context,
                    &output_dir,
                    &output_name,
                    &output_type,
                    trust_data,
                    &mut render_cache,
                    &dpi,
                    dither,
                    remap_source.as_deref(),
                    &host_asset_policy,
                );

                if tx.send(result).is_err() {
                    break;
                }
            }
        });

        workers.push(handle);
    }

    drop(result_tx);

    let mut dispatched = 0usize;
    let mut skipped = 0usize;

    for (line_index, line_result) in reader.lines().enumerate() {
        let line_no = line_index + 1;
        let line = match line_result {
            Ok(v) => v,
            Err(err) => {
                eprintln!("line {} read error: {}", line_no, err);
                continue;
            }
        };

        if line.trim().is_empty() {
            skipped += 1;
            continue;
        }

        let job = BatchJob { line_no, line };
        if job_tx.send(job).is_err() {
            bail!("worker queue closed unexpectedly");
        }
        dispatched += 1;
    }

    drop(job_tx);

    let mut success = 0usize;
    let mut failed = 0usize;
    let mut completed = 0usize;
    let mut render_times = Vec::with_capacity(dispatched);
    let mut script_times = Vec::with_capacity(dispatched);
    let mut draw_times = Vec::with_capacity(dispatched);

    while completed < dispatched {
        match result_rx.recv() {
            Ok(BatchResult::Success {
                render_time,
                script_time,
                draw_time,
                warnings,
                line_no,
                output_path,
            }) => {
                success += 1;
                completed += 1;
                render_times.push(render_time);
                script_times.push(script_time);
                draw_times.push(draw_time);
                for frame_idx in warnings.empty_value_frames {
                    eprintln!(
                        "warning: line {} ({}): frame {} has empty value",
                        line_no,
                        output_path.display(),
                        frame_idx
                    );
                }
                for frame_idx in warnings.unused_fill_color_frames {
                    eprintln!(
                        "warning: line {} ({}): frame {} has unused fill_color override",
                        line_no,
                        output_path.display(),
                        frame_idx
                    );
                }
                for frame_idx in warnings.unused_stroke_color_frames {
                    eprintln!(
                        "warning: line {} ({}): frame {} has unused stroke_color override",
                        line_no,
                        output_path.display(),
                        frame_idx
                    );
                }
                for frame_idx in warnings.unused_stroke_width_frames {
                    eprintln!(
                        "warning: line {} ({}): frame {} has unused stroke_width override",
                        line_no,
                        output_path.display(),
                        frame_idx
                    );
                }
                for frame_idx in warnings.unused_text_color_frames {
                    eprintln!(
                        "warning: line {} ({}): frame {} has unused text_color override",
                        line_no,
                        output_path.display(),
                        frame_idx
                    );
                }
            }
            Ok(BatchResult::Failed {
                line_no,
                message,
                render_time,
            }) => {
                eprintln!("line {} {}", line_no, message);
                failed += 1;
                completed += 1;
                if let Some(render_time) = render_time {
                    render_times.push(render_time);
                }
            }
            Err(_) => {
                bail!(
                    "result channel closed early: completed {} of {}",
                    completed,
                    dispatched
                );
            }
        }
    }

    for handle in workers {
        if handle.join().is_err() {
            bail!("worker thread panicked");
        }
    }

    println!(
        "batch complete: success={}, failed={}, skipped={}",
        success, failed, skipped
    );

    let process = process_start.elapsed();
    let total = total_start.elapsed();

    if args.enable_timings {
        let render_stats = summarize_render_times(&render_times);
        let script_stats = summarize_render_times(&script_times);
        let draw_stats = summarize_render_times(&draw_times);

        eprintln!("timings:");
        eprintln!("    prep:    {}", format_duration(prep));
        eprintln!("    process: {}", format_duration(process));
        eprintln!("    total:   {}", format_duration(total));

        if let Some(stats) = render_stats {
            eprintln!("    render avg:   {}", format_duration(stats.avg));
            eprintln!("    render min:   {}", format_duration(stats.min));
            eprintln!("    render max:   {}", format_duration(stats.max));
            eprintln!("    render p90:   {}", format_duration(stats.p90));
            eprintln!("    render p95:   {}", format_duration(stats.p95));
            eprintln!("    render p99:   {}", format_duration(stats.p99));
            eprintln!("    render p99.9: {}", format_duration(stats.p999));
        }
        if let Some(stats) = script_stats {
            eprintln!("    script avg:   {}", format_duration(stats.avg));
            eprintln!("    script min:   {}", format_duration(stats.min));
            eprintln!("    script max:   {}", format_duration(stats.max));
        }
        if let Some(stats) = draw_stats {
            eprintln!("    draw avg:     {}", format_duration(stats.avg));
            eprintln!("    draw min:     {}", format_duration(stats.min));
            eprintln!("    draw max:     {}", format_duration(stats.max));
        }
    }

    if success == 0 && (failed > 0 || dispatched > 0) {
        bail!("batch produced no outputs");
    }

    Ok(())
}

fn render(args: RenderArgs) -> Result<()> {
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

fn format_duration(d: Duration) -> String {
    let seconds = d.as_secs_f64();

    if seconds >= 60.0 {
        format!("{:.3} min", seconds / 60.0)
    } else if seconds >= 1.0 {
        format!("{:.3} s", seconds)
    } else {
        format!("{:.3} ms", seconds * 1000.0)
    }
}

fn check(args: CheckArgs) -> Result<()> {
    let package = MarmotPackage::open(&args.package_file)?;
    let template_source = package.read_template_source()?;
    let template = parse_template_source(&template_source)?;

    let data_source = read_to_string(&args.data_file)
        .with_context(|| format!("failed to read data: {}", args.data_file.display()))?;

    let data: Value = serde_json::from_str(&data_source)
        .with_context(|| format!("failed to parse JSON: {}", args.data_file.display()))?;

    match validate_data(&template, &data) {
        Ok(()) => {
            println!("OK");
            Ok(())
        }
        Err(errors) => {
            for error in errors {
                eprintln!("validation error: {error:?}");
            }
            bail!("data does not match template slots")
        }
    }
}

fn load_remap_palette_if_needed(
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

fn ensure_file_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        bail!("file does not exist: {}", path.display());
    }
    if !path.is_file() {
        bail!("path is not a file: {}", path.display());
    }
    Ok(())
}

fn ensure_dir_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        create_dir_all(path)
            .with_context(|| format!("failed to create directory: {}", path.display()))?;
        return Ok(());
    }

    if !path.is_dir() {
        bail!("path is not a directory: {}", path.display());
    }

    Ok(())
}

fn ensure_parent_exists(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            create_dir_all(parent)
                .with_context(|| format!("failed to create directory: {}", parent.display()))?;
            return Ok(());
        }
        if !parent.as_os_str().is_empty() && !parent.is_dir() {
            bail!("output parent is not a directory: {}", parent.display());
        }
    }
    Ok(())
}

fn value_to_filename_fragment(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn sanitize_filename(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        let bad = matches!(
            ch,
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' | '\0'
        );
        if bad || ch.is_control() {
            out.push('_');
        } else {
            out.push(ch);
        }
    }

    let out = out.trim();
    if out.is_empty() {
        "_".to_string()
    } else {
        out.to_string()
    }
}

fn format_output_name(template: &str, record: &Value, index: usize) -> Result<String> {
    let mut out = String::with_capacity(template.len() + 16);
    let mut cursor = 0usize;

    while let Some(open_rel) = template[cursor..].find('{') {
        let open = cursor + open_rel;
        out.push_str(&template[cursor..open]);

        let after_open = open + 1;
        if let Some(close_rel) = template[after_open..].find('}') {
            let close = after_open + close_rel;
            let key = &template[after_open..close];

            let replacement = if key == "index" {
                index.to_string()
            } else {
                let value = record.get(key).ok_or_else(|| {
                    anyhow!("record missing field '{}' required by output template", key)
                })?;
                value_to_filename_fragment(value).ok_or_else(|| {
                    anyhow!(
                        "record field '{}' must be string/number/bool for output template",
                        key
                    )
                })?
            };

            out.push_str(&replacement);
            cursor = close + 1;
        } else {
            out.push_str(&template[open..]);
            cursor = template.len();
            break;
        }
    }

    if cursor < template.len() {
        out.push_str(&template[cursor..]);
    }

    let out = sanitize_filename(&out);

    if out.contains("..") {
        bail!("output file contains unsafe '..': {}", out);
    }

    Ok(out)
}

fn resolve_jobs(requested: usize) -> usize {
    if requested == 0 {
        thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    } else {
        requested.max(1)
    }
}

fn process_batch_line(
    line_no: usize,
    line: &str,
    template: &Template,
    render_context: &RenderContext,
    output_dir: &Path,
    output_name_template: &str,
    output_type: &OutputType,
    trust_data: bool,
    render_cache: &mut RenderCache,
    dpi: &u16,
    dither: Option<DitherType>,
    remap_source: Option<&str>,
    host_assets: &HostAssetPolicy,
) -> BatchResult {
    let record: Value = match serde_json::from_str(&line) {
        Ok(v) => v,
        Err(err) => {
            return BatchResult::Failed {
                line_no,
                message: format!("json parse error: {err}"),
                render_time: None,
            };
        }
    };

    if !record.is_object() {
        return BatchResult::Failed {
            line_no,
            message: "json must be object".to_string(),
            render_time: None,
        };
    }

    if !trust_data {
        if let Err(errors) = validate_data(&template, &record) {
            return BatchResult::Failed {
                line_no,
                message: format!("validation failed: {:?}", errors),
                render_time: None,
            };
        }
    }

    let file_name = match format_output_name(output_name_template, &record, line_no) {
        Ok(v) => v,
        Err(err) => {
            return BatchResult::Failed {
                line_no,
                message: format!("output-name error: {err}",),
                render_time: None,
            };
        }
    };

    let output_path = output_dir.join(file_name);

    let render_start = Instant::now();

    let outcome = match output_type {
        OutputType::PDF => render_pdf_with_cache(
            &template.page,
            &template.frames,
            &template.layers,
            &template.draw_entries,
            &output_path,
            Some(&record),
            &render_context,
            host_assets,
            render_cache,
        ),
        OutputType::PNG => render_png_with_cache(
            &template.page,
            &template.frames,
            &template.layers,
            &template.draw_entries,
            &output_path,
            Some(&record),
            &render_context,
            host_assets,
            *dpi,
            dither,
            remap_source,
            render_cache,
        ),
    };

    match outcome {
        Ok(outcome) => BatchResult::Success {
            render_time: render_start.elapsed(),
            script_time: outcome.script_time,
            draw_time: outcome.draw_time,
            warnings: outcome.warnings,
            output_path: output_path.clone(),
            line_no,
        },
        Err(err) => BatchResult::Failed {
            line_no,
            message: format!("render failed for {}: {:?}", output_path.display(), err),
            render_time: Some(render_start.elapsed()),
        },
    }
}

fn summarize_render_times(render_times: &[Duration]) -> Option<RenderStats> {
    if render_times.is_empty() {
        return None;
    }

    let mut sorted = render_times.to_vec();
    sorted.sort_unstable();

    let min = sorted[0];
    let max = sorted[sorted.len() - 1];

    let total_nanos: u128 = sorted.iter().map(Duration::as_nanos).sum();
    let avg_nanos = total_nanos / sorted.len() as u128;
    let avg = Duration::from_nanos(avg_nanos.min(u64::MAX as u128) as u64);

    Some(RenderStats {
        avg,
        min,
        max,
        p90: percentile_duration(&sorted, 0.90),
        p95: percentile_duration(&sorted, 0.95),
        p99: percentile_duration(&sorted, 0.99),
        p999: percentile_duration(&sorted, 0.999),
    })
}

fn percentile_duration(sorted: &[Duration], percentile: f64) -> Duration {
    let n = sorted.len();
    if n == 0 {
        return Duration::from_nanos(0);
    }

    let rank = (percentile.clamp(0.0, 1.0) * n as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(n - 1);
    sorted[index]
}
