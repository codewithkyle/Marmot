mod lexer;
mod parser;
mod renderer;
mod validator;
mod package;

use anyhow::{Context, Result, anyhow, bail};
use clap::{Arg, ArgMatches, Command};
use serde_json::Value;
use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
};

use crate::{lexer::Lexer, package::MarmotPackage, parser::Parser, renderer::render_pdf, validator::validate_data};

struct CheckArgs {
    package_file: PathBuf,
    data_file: PathBuf,
}

struct RenderArgs {
    package_file: PathBuf,
    data_file: Option<PathBuf>,
    output_file: PathBuf,
}

struct BatchArgs {
    template_file: PathBuf,
    data_file: PathBuf,
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
                .arg(Arg::new("output").short('o').long("output").required(true)),
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
        _ => unreachable!("Exhausted list of subcommands."),
    };

    Ok(())
}

fn parse_template_source(template_source: &str) -> Result<parser::Template> {
    let mut lexer = Lexer::new(template_source);
    let tokens = lexer.tokenize().map_err(|err| anyhow!("failed to tokenize template: {err:?}"))?;
    let mut parser = Parser::new(tokens);
    parser.parse_template().map_err(|err| anyhow!("failed to parse template: {err:?}"))
}

fn render(args: RenderArgs) -> Result<()> {
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

    render_pdf(&template.page, &template.draw, &args.output_file, data.as_ref())
        .map_err(|err| anyhow!("failed to render PDF: {err:?}"))?;

    println!("wrote {}", args.output_file.display());

    Ok(())
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

    let args = RenderArgs {
        package_file,
        data_file,
        output_file,
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
