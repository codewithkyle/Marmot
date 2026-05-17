use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

use crate::{
    BatchArgs, DitherType, OutputType,
    ops::{common::load_remap_palette_if_needed, parse_template_source},
    package::MarmotPackage,
    parser::Template,
    renderer::{
        HostAssetPolicy, RenderCache, RenderWarnings, render_pdf_with_cache, render_png_with_cache,
    },
    resources::{RenderContext, build_render_context},
    util::{format_duration, format_output_name, summarize_render_times},
    validator::validate_data,
};

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

pub fn batch(args: BatchArgs) -> Result<()> {
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
