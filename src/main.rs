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
            let source = r#"
%!PSL 0.1

page 612 792

slots begin
  product_name text required
  base_price text required
  sale_price text required
  buy int required
  get int required
end

fonts begin
  helvetica "fonts/Helvetica.ttf"
  helvetica_bold "fonts/Helvetica-Bold.ttf"
end

assets begin
  logo image "assets/logo.png"
  badge image "assets/logo.png"
end

draw begin
  % Background and border
  1 1 1 rgb
  0 0 612 792 rect fill

  1 0 0 rgb
  72 72 468 648 rect stroke

  % Embedded image
  logo 420 40 120 60 image contain

  % Product name
  helvetica_bold font
  28 fontsize
  center align
  middle valign
  0 0 0 1 cmyk
  $(product_name) 72 100 468 80 textbox

  % Offer
  helvetica font
  64 fontsize
  (BUY) $(buy) (GET) $(get) 72 240 468 100 textbox

  % Price
  helvetica_bold font
  96 fontsize
  0.5 grey
  $(sale_price) 72 380 468 130 textbox
end
            "#;
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
