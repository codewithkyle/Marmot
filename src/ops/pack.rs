use std::path::PathBuf;

use anyhow::Result;

use crate::{
    PackArgs,
    package::{PackageBuilderOptions, create_package},
    util::ensure_dir_exists,
};

pub fn pack(args: PackArgs) -> Result<()> {
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
