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
    fs::{File, read_to_string},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::{
    lexer::Lexer,
    package::{MarmotPackage, PackageBuilderOptions, create_package},
    parser::Parser,
    renderer::render_pdf,
    resources::build_render_context,
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
                        .help("Filename template, supports {id} and {index}, e.g. {id}.pdf"),
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

fn batch(args: BatchArgs) -> Result<()> {
    let package = MarmotPackage::open(&args.package_file)?;
    let template_source = package.read_template_source()?;
    let template = parse_template_source(&template_source)?;
    let render_context = build_render_context(&template, &package)?;

    let file = File::open(&args.records_file).with_context(|| {
        format!(
            "failed to open records file: {}",
            args.records_file.display()
        )
    })?;
    let reader = BufReader::new(file);

    let mut success = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;

    for (line_index, line_result) in reader.lines().enumerate() {
        let line_no = line_index + 1;
        let line = match line_result {
            Ok(v) => v,
            Err(err) => {
                eprintln!("line {} read error: {}", line_no, err);
                failed += 1;
                continue;
            }
        };

        if line.trim().is_empty() {
            skipped += 1;
            continue;
        }

        let record: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(err) => {
                eprintln!("line {} json parse error: {}", line_no, err);
                failed += 1;
                continue;
            }
        };

        if !record.is_object() {
            eprintln!("line {} json must be object", line_no);
            failed += 1;
            continue;
        }

        if let Err(errors) = validate_data(&template, &record) {
            eprintln!("line {} validation failed:", line_no);
            for error in errors {
                eprintln!("    {:?}", error);
            }
            failed += 1;
            continue;
        }

        let file_name = match format_output_name(&args.output_name, &record, line_no) {
            Ok(v) => v,
            Err(err) => {
                eprintln!("line {} output-name error: {}", line_no, err);
                failed += 1;
                continue;
            }
        };

        let output_path = args.output_dir.join(file_name);

        if let Some(parent) = output_path.parent() {
            if !parent.exists() || !parent.is_dir() {
                eprintln!(
                    "line {} output path parent missing/not dir: {}",
                    line_no,
                    parent.display()
                );
                failed += 1;
                continue;
            }
        }

        match render_pdf(
            &template.page,
            &template.draw,
            &output_path,
            Some(&record),
            &render_context,
        ) {
            Ok(()) => {
                success += 1;
            }
            Err(err) => {
                eprintln!(
                    "line {} render failed for {}: {:?}",
                    line_no,
                    output_path.display(),
                    err
                );
                failed += 1;
            }
        }
    }

    println!(
        "batch complete: success={}, failed={}, skipped={}",
        success, failed, skipped
    );

    if success == 0 && failed > 0 {
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
    format!("{:.3} ms", d.as_secs_f64() * 1000.0)
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

    ensure_file_exists(&records_file)?;
    ensure_dir_exists(&output_dir)?;

    Ok(BatchArgs {
        package_file,
        records_file,
        output_dir,
        output_name,
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
        bail!("path does not exist: {}", path.display());
    }
    if !path.is_dir() {
        bail!("path is not a directory: {}", path.display());
    }
    Ok(())
}

fn ensure_parent_exists(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            bail!("output directory does not exist: {}", parent.display());
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
    let mut out = template.replace("{index}", &index.to_string());

    if out.contains("{id}") {
        let id_value = record
            .get("id")
            .ok_or_else(|| anyhow!("record missing id field required by output template"))?;
        let id_text = value_to_filename_fragment(id_value)
            .ok_or_else(|| anyhow!("record id must be string/number/bool for output template"))?;

        out = out.replace("{id}", &id_text);
    }

    let out = sanitize_filename(&out);

    if out.contains("..") {
        bail!("output file contains unsafe '..': {}", out);
    }

    if Path::new(&out).is_absolute() {
        bail!("output filename must be relative: {}", out);
    }

    Ok(out)
}
