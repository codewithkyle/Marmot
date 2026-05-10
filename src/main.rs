mod lexer;
use lexer::Lexer;
use anyhow::{Result, bail};
use clap::{Arg, ArgMatches, Command};
use std::path::{Path, PathBuf};

struct CheckArgs {
    template_file: PathBuf,
    data_file: PathBuf,
}

struct RenderArgs {
    template_file: PathBuf,
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
                .about("Check a template against a data file")
                .arg(Arg::new("template").required(true))
                .arg(Arg::new("data").required(true)),
        )
        .subcommand(
            Command::new("render")
                .about("Render a template with a data file")
                .arg(Arg::new("template").required(true))
                .arg(Arg::new("data"))
                .arg(Arg::new("output").short('o').long("output").required(true)),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("check", sub_matches)) => {
            let _args = parse_check_args(sub_matches)?;
            let source = "page 612.25 792.9974";
            let mut lexer = Lexer::new(source);
            match lexer.tokenize() {
                Ok(tokens) => {
                    for token in tokens {
                        println!("{token:?}");
                    }
                }
                Err(err) => {
                    eprintln!("Lexer error: {err:?}");
                }
            }
        }
        Some(("render", sub_matches)) => {
            let _args = parse_render_args(sub_matches)?;
            // TODO: kick off render(args)?
        }
        _ => unreachable!("Exhausted list of subcommands."),
    };

    Ok(())
}

fn parse_check_args(matches: &ArgMatches) -> Result<CheckArgs> {
    let template_file = matches
        .get_one::<String>("template")
        .expect("template is required")
        .into();
    let data_file = matches
        .get_one::<String>("data")
        .expect("data is required")
        .into();
    let args = CheckArgs {
        template_file,
        data_file,
    };
    ensure_file_exists(&args.template_file)?;
    ensure_file_exists(&args.data_file)?;
    Ok(args)
}

fn parse_render_args(matches: &ArgMatches) -> Result<RenderArgs> {
    let template_file = matches
        .get_one::<String>("template")
        .expect("template is required")
        .into();
    let data_file = matches.get_one::<String>("data").map(PathBuf::from);
    let output_file = matches
        .get_one::<String>("output")
        .expect("output is required")
        .into();

    let args = RenderArgs {
        template_file,
        data_file,
        output_file,
    };

    ensure_file_exists(&args.template_file)?;
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
