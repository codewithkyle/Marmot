mod lexer;
mod package;
mod parser;
mod renderer;
mod resources;
mod validator;

use anyhow::{Context, Result, anyhow, bail};
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde_json::Value;
use std::{
    fs::{File, create_dir_all, read_to_string},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

use crate::{
    lexer::Lexer,
    package::{MarmotPackage, PackageBuilderOptions, create_package},
    parser::{Parser, Template},
    renderer::{RenderCache, render_pdf, render_pdf_with_cache},
    resources::{RenderContext, build_render_context},
    validator::validate_data,
};

struct CheckArgs {
    package_file: PathBuf,
    data_file: PathBuf,
}

struct RenderArgs {
    package_file: PathBuf,
    data_file: Option<PathBuf>,
    output_file: PathBuf,
    enable_timings: bool,
}

struct PackArgs {
    template_file: PathBuf,
    name: String,
    output_dir: Option<PathBuf>,
    assets: Vec<PathBuf>,
    fonts: Vec<PathBuf>,
}

struct BatchArgs {
    package_file: PathBuf,
    records_file: PathBuf,
    output_dir: PathBuf,
    output_name: String,
    jobs: usize,
    trust_data: bool,
    enable_timings: bool,
}

fn main() -> Result<()> {
    let matches = Command::new("marmot")
        .version("0.1")
        .about("A PostScript-inspired template language for rendering dynamic PDFs and images.")
        .propagate_version(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("check")
                .about("Check a .marmot package against a data file")
                .arg(Arg::new("package").required(true))
                .arg(Arg::new("data").required(true)),
        )
        .subcommand(
            Command::new("render")
                .about("Render a .marmot package with a data file")
                .arg(Arg::new("package").required(true))
                .arg(Arg::new("data"))
                .arg(Arg::new("output").short('o').long("output").required(true))
                .arg(
                    Arg::new("timings")
                        .long("timings")
                        .action(ArgAction::SetTrue),
                ),
        )
        .subcommand(
            Command::new("pack")
                .about("Create a .marmot package")
                .arg(Arg::new("template").required(true))
                .arg(Arg::new("name").required(true))
                .arg(
                    Arg::new("asset")
                        .short('a')
                        .long("asset")
                        .value_name("PATH")
                        .action(ArgAction::Append)
                        .help("Add an asset file to the package"),
                )
                .arg(
                    Arg::new("font")
                        .short('f')
                        .long("font")
                        .value_name("PATH")
                        .action(ArgAction::Append)
                        .help("Add a font file to the package"),
                )
                .arg(Arg::new("output").short('o').long("output-dir")),
        )
        .subcommand(
            Command::new("batch")
                .about("Render many PDFs from a .marmot package and JSONL records")
                .arg(Arg::new("package").required(true))
                .arg(Arg::new("records").required(true))
                .arg(
                    Arg::new("output-dir")
                        .long("output-dir")
                        .required(true)
                        .value_name("DIR"),
                )
                .arg(
                    Arg::new("output-name")
                        .long("output-name")
                        .required(true)
                        .value_name("TEMPlATE")
                        .help("Filename template with {index} and top-level record fiels, e.g. {sku}.pdf or {sku}-{id}.pdf"),
                )
                .arg(
                    Arg::new("jobs")
                        .short('j')
                        .long("jobs")
                        .value_name("N")
                        .default_value("0")
                        .value_parser(clap::value_parser!(usize))
                        .help("Worker count. 0 = auto"),
                )
                .arg(
                    Arg::new("timings")
                        .long("timings")
                        .action(ArgAction::SetTrue)
                        .help("Print batch timing breakdown"),
                )
                .arg(
                    Arg::new("trust-data")
                        .long("trust-data")
                        .action(ArgAction::SetTrue)
                        .help("Skip upfront slot validation for each record"),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("check", sub_matches)) => {
            let args = parse_check_args(sub_matches)?;
            check(args)?;
        }
        Some(("render", sub_matches)) => {
            let args = parse_render_args(sub_matches)?;
            render(args)?;
        }
        Some(("pack", sub_matches)) => {
            let args = parse_pack_args(sub_matches)?;
            pack(args)?;
        }
        Some(("batch", sub_matches)) => {
            let args = parse_batch_args(sub_matches)?;
            batch(args)?;
        }
        _ => unreachable!("Exhausted list of subcommands."),
    };

    Ok(())
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
    let trust_data = args.trust_data;

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
                    trust_data,
                    &mut render_cache,
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

    while completed < dispatched {
        match result_rx.recv() {
            Ok(BatchResult::Success { render_time }) => {
                success += 1;
                completed += 1;
                render_times.push(render_time);
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

    let prep = prep_start.elapsed();
    let render_start = Instant::now();

    render_pdf(
        &template.page,
        &template.draw,
        &args.output_file,
        data.as_ref(),
        &render_context,
    )
    .map_err(|err| anyhow!("failed to render PDF: {err:?}"))?;

    let render = render_start.elapsed();
    let total = total_start.elapsed();

    println!("wrote {}", args.output_file.display());
    if args.enable_timings {
        eprintln!("timings:");
        eprintln!("    prep:    {}", format_duration(prep));
        eprintln!("    render:  {}", format_duration(render));
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

fn parse_check_args(matches: &ArgMatches) -> Result<CheckArgs> {
    let package_file = matches
        .get_one::<String>("package")
        .expect("package is required")
        .into();
    let data_file = matches
        .get_one::<String>("data")
        .expect("data is required")
        .into();
    let args = CheckArgs {
        package_file,
        data_file,
    };
    ensure_file_exists(&args.data_file)?;
    Ok(args)
}

fn parse_batch_args(matches: &ArgMatches) -> Result<BatchArgs> {
    let package_file: PathBuf = matches
        .get_one::<String>("package")
        .expect("package is required")
        .into();
    let records_file: PathBuf = matches
        .get_one::<String>("records")
        .expect("records is required")
        .into();
    let output_dir: PathBuf = matches
        .get_one::<String>("output-dir")
        .expect("output-dir is required")
        .into();
    let output_name = matches
        .get_one::<String>("output-name")
        .expect("output-name is required")
        .into();
    let jobs = *matches.get_one::<usize>("jobs").expect("jobs has default");
    let trust_data = *matches.get_one::<bool>("trust-data").unwrap_or(&false);
    let enable_timings = *matches.get_one::<bool>("timings").unwrap_or(&false);

    ensure_file_exists(&records_file)?;
    ensure_dir_exists(&output_dir)?;

    Ok(BatchArgs {
        package_file,
        records_file,
        output_dir,
        output_name,
        jobs,
        trust_data,
        enable_timings,
    })
}

fn parse_pack_args(matches: &ArgMatches) -> Result<PackArgs> {
    let template_file = matches
        .get_one::<String>("template")
        .expect("template is required")
        .into();
    let name = matches
        .get_one::<String>("name")
        .expect("name is required")
        .into();
    let output_dir = matches.get_one::<String>("output").map(PathBuf::from);
    let fonts: Vec<PathBuf> = matches
        .get_many::<String>("font")
        .unwrap_or_default()
        .map(PathBuf::from)
        .collect();
    let assets: Vec<PathBuf> = matches
        .get_many::<String>("asset")
        .unwrap_or_default()
        .map(PathBuf::from)
        .collect();

    let args = PackArgs {
        template_file,
        name,
        output_dir,
        fonts,
        assets,
    };

    Ok(args)
}

fn parse_render_args(matches: &ArgMatches) -> Result<RenderArgs> {
    let package_file = matches
        .get_one::<String>("package")
        .expect("package is required")
        .into();
    let data_file = matches.get_one::<String>("data").map(PathBuf::from);
    let output_file = matches
        .get_one::<String>("output")
        .expect("output is required")
        .into();

    let enable_timings = matches.get_one::<bool>("timings").unwrap_or(&false);

    let args = RenderArgs {
        package_file,
        data_file,
        output_file,
        enable_timings: *enable_timings,
    };

    if let Some(data_file) = &args.data_file {
        ensure_file_exists(data_file)?;
    }
    ensure_parent_exists(&args.output_file)?;

    Ok(args)
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
    trust_data: bool,
    render_cache: &mut RenderCache,
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

    match render_pdf_with_cache(
        &template.page,
        &template.draw,
        &output_path,
        Some(&record),
        &render_context,
        render_cache,
    ) {
        Ok(()) => BatchResult::Success {
            render_time: render_start.elapsed(),
        },
        Err(err) => BatchResult::Failed {
            line_no,
            message: format!("render failed for {}: {:?}", output_path.display(), err),
            render_time: Some(render_start.elapsed()),
        },
    }
}

struct RenderStats {
    avg: Duration,
    min: Duration,
    max: Duration,
    p90: Duration,
    p95: Duration,
    p99: Duration,
    p999: Duration,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn output_name_supports_single_top_level_field() {
        let record = json!({ "sku": "49000000001" });
        let out = format_output_name("{sku}.pdf", &record, 1).unwrap();
        assert_eq!(out, "49000000001.pdf");
    }
    #[test]
    fn output_name_supports_many_fields() {
        let record = json!({
            "sku": "49000000001",
            "buy_qty": 1,
            "get_qty": 2
        });
        let out = format_output_name("{index}-{sku}-{buy_qty}-{get_qty}.pdf", &record, 7).unwrap();
        assert_eq!(out, "7-49000000001-1-2.pdf");
    }
    #[test]
    fn output_name_supports_repeated_fields() {
        let record = json!({ "sku": "ABC123" });
        let out = format_output_name("{sku}-{sku}.pdf", &record, 1).unwrap();
        assert_eq!(out, "ABC123-ABC123.pdf");
    }
    #[test]
    fn output_name_errors_when_field_missing() {
        let record = json!({ "id": "x" });
        let err = format_output_name("{sku}.pdf", &record, 1)
            .unwrap_err()
            .to_string();
        assert!(err.contains("record missing field 'sku'"));
    }
    #[test]
    fn output_name_errors_when_field_not_scalar() {
        let record = json!({ "sku": { "nested": "x" } });
        let err = format_output_name("{sku}.pdf", &record, 1)
            .unwrap_err()
            .to_string();
        assert!(err.contains("record field 'sku' must be string/number/bool"));
    }
    #[test]
    fn output_name_keeps_malformed_open_brace_literal() {
        let record = json!({ "sku": "49000000001" });
        let out = format_output_name("prefix-{sku.pdf", &record, 1).unwrap();
        assert_eq!(out, "prefix-{sku.pdf");
    }
    #[test]
    fn output_name_sanitizes_invalid_filename_chars() {
        let record = json!({ "sku": "49/000:000?01" });
        let out = format_output_name("{sku}.pdf", &record, 1).unwrap();
        assert_eq!(out, "49_000_000_01.pdf");
    }
    #[test]
    fn output_name_rejects_dotdot_segments() {
        let record = json!({ "sku": ".." });
        let err = format_output_name("{sku}.pdf", &record, 1)
            .unwrap_err()
            .to_string();
        assert!(err.contains("unsafe '..'"));
    }
}
