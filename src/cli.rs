#[cfg(test)]
mod test;

use anyhow::Result;
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::path::PathBuf;

use crate::{
    BatchArgs, CheckArgs, DitherType, OutputType, PackArgs, RenderArgs,
    ops::{batch, check, pack, render},
    util::{ensure_dir_exists, ensure_file_exists, ensure_parent_exists},
};

pub fn run() -> Result<()> {
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
                .arg(Arg::new("output-type").long("output-type").value_name("TYPE").value_parser(["pdf", "png"]).default_value("pdf"))
                .arg(
                    Arg::new("timings")
                        .long("timings")
                        .action(ArgAction::SetTrue),
                )
                .arg(Arg::new("dpi").long("dpi").value_name("NUMBER").value_parser(clap::value_parser!(u16).range(72..=1200)).default_value("300"))
                .arg(Arg::new("dither").long("dither").value_name("TYPE").value_parser(["floyd","atkinson","stucki","burkes","jarvis","sierra3"]))
                .arg(
                    Arg::new("allow-host-assets")
                        .long("allow-host-assets")
                        .action(ArgAction::SetTrue)
                        .help("Allow loadimage to read host filesystem paths"),
                )
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
                .arg(
                    Arg::new("script")
                        .short('s')
                        .long("script")
                        .value_name("PATH")
                        .action(ArgAction::Append)
                        .help("Add a lua script file to the package")
                )
                .arg(Arg::new("output").short('o').long("output-dir"))
                .arg(Arg::new("remap").long("remap").value_name("PATH"))
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
                .arg(Arg::new("output-type").long("output-type").value_name("TYPE").value_parser(["pdf", "png"]).default_value("pdf"))
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
                )
                .arg(Arg::new("dpi").long("dpi").value_name("NUMBER").value_parser(clap::value_parser!(u16).range(72..=1200)).default_value("300"))
                .arg(Arg::new("dither").long("dither").value_name("TYPE").value_parser(["floyd","atkinson","stucki","burkes","jarvis","sierra3"]))
                .arg(
                    Arg::new("allow-host-assets")
                        .long("allow-host-assets")
                        .action(ArgAction::SetTrue)
                        .help("Allow loadimage to read host filesystem paths"),
                )
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
    let output_type = matches
        .get_one::<String>("output-type")
        .map(|s| OutputType::try_from_word(s))
        .unwrap_or(Ok(OutputType::PDF))?;
    let dpi = matches.get_one::<u16>("dpi").expect("dpi has default");
    let dither = matches
        .get_one::<String>("dither")
        .map(|s| DitherType::try_from_word(s))
        .transpose()?;
    let allow_host_assets = *matches
        .get_one::<bool>("allow-host-assets")
        .unwrap_or(&false);

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
        output_type,
        dpi: *dpi,
        dither,
        allow_host_assets,
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
    let scripts: Vec<PathBuf> = matches
        .get_many::<String>("script")
        .unwrap_or_default()
        .map(PathBuf::from)
        .collect();
    let remap_file = matches.get_one::<String>("remap").map(PathBuf::from);

    let args = PackArgs {
        template_file,
        name,
        output_dir,
        fonts,
        assets,
        scripts,
        remap_file,
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
    let output_type = matches
        .get_one::<String>("output-type")
        .map(|s| OutputType::try_from_word(s))
        .unwrap_or(Ok(OutputType::PDF))?;

    let enable_timings = matches.get_one::<bool>("timings").unwrap_or(&false);

    let dpi = matches.get_one::<u16>("dpi").expect("dpi has default");

    let dither = matches
        .get_one::<String>("dither")
        .map(|s| DitherType::try_from_word(s))
        .transpose()?;
    let allow_host_assets = *matches
        .get_one::<bool>("allow-host-assets")
        .unwrap_or(&false);

    let args = RenderArgs {
        package_file,
        data_file,
        output_file,
        enable_timings: *enable_timings,
        output_type,
        dpi: *dpi,
        dither,
        allow_host_assets,
    };

    if let Some(data_file) = &args.data_file {
        ensure_file_exists(data_file)?;
    }
    ensure_parent_exists(&args.output_file)?;

    Ok(args)
}
